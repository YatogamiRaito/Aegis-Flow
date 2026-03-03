use crate::process::ProcessConfig;
use crate::daemon::{DaemonError, ProcessManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Unsupported format")]
    UnsupportedFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfigEntry {
    pub name: String,
    pub script: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub env_production: Option<HashMap<String, String>>,
    #[serde(default)]
    pub env_staging: Option<HashMap<String, String>>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub instances: usize, // 0 means cluster using cpu_count
    pub max_memory_bytes: Option<u64>,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

fn default_max_restarts() -> u32 {
    15
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EcosystemConfig {
    #[serde(rename = "apps", alias = "app")]
    pub apps: Vec<AppConfigEntry>,
}

impl EcosystemConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let content = std::fs::read_to_string(path)?;

        let config: EcosystemConfig = match ext {
            "toml" => toml::from_str(&content)?,
            "yaml" | "yml" => serde_yaml::from_str(&content)?,
            _ => return Err(ConfigError::UnsupportedFormat),
        };

        config.validate(path.parent().unwrap_or(Path::new(".")))?;
        Ok(config)
    }

    fn validate(&self, _base_dir: &Path) -> Result<(), ConfigError> {
        if self.apps.is_empty() {
            return Err(ConfigError::Validation("No apps defined in ecosystem".to_string()));
        }

        let mut names = std::collections::HashSet::new();
        for app in &self.apps {
            if app.name.is_empty() {
                return Err(ConfigError::Validation("App name cannot be empty".to_string()));
            }
            if app.script.is_empty() {
                return Err(ConfigError::Validation(format!("App '{}' has no script defined", app.name)));
            }
            if !names.insert(&app.name) {
                return Err(ConfigError::Validation(format!("Duplicate app name: {}", app.name)));
            }
        }
        Ok(())
    }

    /// Convert AppConfigEntry to a concrete ProcessConfig for a specified environment
    pub fn build_process_config(&self, app: &AppConfigEntry, env_profile: Option<&str>) -> ProcessConfig {
        let mut env = app.env.clone();

        match env_profile {
            Some("production") => {
                if let Some(prod_env) = &app.env_production {
                    for (k, v) in prod_env {
                        env.insert(k.clone(), v.clone());
                    }
                }
            }
            Some("staging") => {
                if let Some(staging_env) = &app.env_staging {
                    for (k, v) in staging_env {
                        env.insert(k.clone(), v.clone());
                    }
                }
            }
            _ => {}
        }

        ProcessConfig {
            script: app.script.clone(),
            args: app.args.clone(),
            env,
            cwd: app.cwd.clone(),
            instances: app.instances,
            max_memory_bytes: app.max_memory_bytes,
            max_restarts: app.max_restarts,
        }
    }
}

pub struct EcosystemManager {
    pm: Arc<ProcessManager>,
}

impl EcosystemManager {
    pub fn new(pm: Arc<ProcessManager>) -> Self {
        Self { pm }
    }

    pub async fn start_ecosystem(&self, config: &EcosystemConfig, env_profile: Option<&str>) -> Result<(), DaemonError> {
        for app in &config.apps {
            let proc_config = config.build_process_config(app, env_profile);
            
            // To be robust, one would spawn multiple if instances > 1, 
            // but for simplicity here we assume the Daemon/ClusterManager handles that,
            // or the ecosystem loop expands them. For now, just spawn named app.
            
            if proc_config.instances > 1 {
                // Should invoke cluster manager logic, or we implement loop here:
                for i in 0..proc_config.instances {
                    let mut inst_config = proc_config.clone();
                    inst_config.env.insert("INSTANCE_ID".to_string(), i.to_string());
                    self.pm.spawn_process(&format!("{}-{}", app.name, i), &inst_config).await?;
                }
            } else {
                self.pm.spawn_process(&app.name, &proc_config).await?;
            }
        }
        Ok(())
    }

    pub async fn stop_all(&self, config: &EcosystemConfig) -> Result<(), DaemonError> {
        for app in &config.apps {
            if app.instances > 1 {
                for i in 0..app.instances {
                    let _ = self.pm.stop_process(&format!("{}-{}", app.name, i), std::time::Duration::from_millis(500)).await;
                }
            } else {
                let _ = self.pm.stop_process(&app.name, std::time::Duration::from_millis(500)).await;
            }
        }
        Ok(())
    }

    pub async fn restart_all(&self, config: &EcosystemConfig, env_profile: Option<&str>) -> Result<(), DaemonError> {
        let delay = std::time::Duration::from_millis(500);
        for app in &config.apps {
            let proc_config = config.build_process_config(app, env_profile);
            if app.instances > 1 {
                for i in 0..app.instances {
                    let _ = self.pm.restart_process(&format!("{}-{}", app.name, i), &proc_config, delay).await;
                }
            } else {
                let _ = self.pm.restart_process(&app.name, &proc_config, delay).await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ProcessTable;

    #[test]
    fn test_parse_toml_ecosystem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ecosystem.toml");
        
        let toml_content = r#"
        [[apps]]
        name = "api-server"
        script = "./server"
        instances = 4
        
        [apps.env]
        PORT = "3000"
        
        [apps.env_production]
        NODE_ENV = "production"
        
        [[apps]]
        name = "worker"
        script = "python"
        args = ["worker.py"]
        "#;
        
        std::fs::write(&path, toml_content).unwrap();
        
        let config = EcosystemConfig::from_file(&path).unwrap();
        assert_eq!(config.apps.len(), 2);
        
        let api = &config.apps[0];
        assert_eq!(api.name, "api-server");
        assert_eq!(api.instances, 4);
        assert_eq!(api.env.get("PORT").unwrap(), "3000");
        
        let worker = &config.apps[1];
        assert_eq!(worker.name, "worker");
        assert_eq!(worker.args, vec!["worker.py"]);
    }

    #[test]
    fn test_parse_yaml_ecosystem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ecosystem.yaml");
        
        let yaml_content = r#"
apps:
  - name: "api"
    script: "./api"
    env_staging:
      DEBUG: "true"
"#;
        std::fs::write(&path, yaml_content).unwrap();
        
        let config = EcosystemConfig::from_file(&path).unwrap();
        assert_eq!(config.apps.len(), 1);
        let api = &config.apps[0];
        assert_eq!(api.name, "api");
        
        let proc_config = config.build_process_config(api, Some("staging"));
        assert_eq!(proc_config.env.get("DEBUG").unwrap(), "true");
        
        let proc_config_dev = config.build_process_config(api, None);
        assert!(proc_config_dev.env.get("DEBUG").is_none());
    }

    #[test]
    fn test_config_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ecosystem.toml");
        
        // Empty apps
        std::fs::write(&path, "apps = []").unwrap();
        assert!(EcosystemConfig::from_file(&path).is_err());
        
        // Missing name
        let toml_content = r#"
        [[apps]]
        script = "./server"
        "#;
        std::fs::write(&path, toml_content).unwrap();
        // Since struct deserialization itself enforces name via serde, it will fail at toml::from_str
        assert!(EcosystemConfig::from_file(&path).is_err());
    }

    #[tokio::test]
    async fn test_start_stop_ecosystem() {
        let table = Arc::new(ProcessTable::new());
        let pm = Arc::new(ProcessManager::new(table.clone()));
        let manager = EcosystemManager::new(pm.clone());
        
        let config = EcosystemConfig {
            apps: vec![
                AppConfigEntry {
                    name: "app1".to_string(),
                    script: "sleep".to_string(),
                    args: vec!["10".to_string()],
                    env: HashMap::new(),
                    env_production: None,
                    env_staging: None,
                    cwd: None,
                    instances: 1,
                    max_memory_bytes: None,
                    max_restarts: 15,
                }
            ]
        };
        
        manager.start_ecosystem(&config, None).await.unwrap();
        assert!(table.get("app1").is_ok());
        
        manager.stop_all(&config).await.unwrap();
        let p = table.get("app1").unwrap();
        assert_eq!(p.status, crate::process::ProcessStatus::Stopped);
    }
}
