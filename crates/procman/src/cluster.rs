use crate::daemon::{DaemonError, ProcessManager};
use crate::process::{ProcessConfig, ProcessStatus};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::thread;

/// Get the number of available CPU cores. Default to 1 if detection fails.
pub fn cpu_count() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Aggregate status for a cluster of processes.
#[derive(Debug, Default)]
pub struct ClusterStatus {
    pub total_memory_bytes: u64,
    pub avg_cpu_percent: f64,
    pub online_instances: usize,
    pub errored_instances: usize,
    pub total_instances: usize,
}

/// Parse a .env file and return a HashMap of key-value pairs.
/// Keys that already exist in the environment are not overwritten by default,
/// but here we just parse them. The caller can merge them with `env::vars()`.
pub fn parse_dotenv<P: AsRef<Path>>(path: P) -> std::io::Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    let mut env = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            // Remove optional surrounding quotes
            let val = val.trim();
            let val = if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                &val[1..val.len() - 1]
            } else {
                val
            };
            env.insert(key, val.to_string());
        }
    }

    Ok(env)
}

pub struct ClusterManager {
    pm: Arc<ProcessManager>,
}

impl ClusterManager {
    pub fn new(pm: Arc<ProcessManager>) -> Self {
        Self { pm }
    }

    /// Spawn N instances of the given config, injecting instance-specific environment variables.
    pub async fn spawn_cluster(
        &self,
        base_name: &str,
        config: ProcessConfig,
    ) -> Result<Vec<String>, DaemonError> {
        let instances = if config.instances == 0 {
            cpu_count()
        } else {
            config.instances
        };

        let mut spawned_names = Vec::new();

        for i in 0..instances {
            let instance_id = i;
            let instance_name = format!("{}-{}", base_name, instance_id);

            // Clone config to inject instance specific env vars
            let mut instance_config = config.clone();

            instance_config
                .env
                .insert("INSTANCE_ID".to_string(), instance_id.to_string());
            instance_config
                .env
                .insert("PM_ID".to_string(), instance_id.to_string()); // PM2 compatibility
            instance_config
                .env
                .insert("AEGIS_APP_NAME".to_string(), base_name.to_string());

            self.pm
                .spawn_process(&instance_name, &instance_config)
                .await?;
            spawned_names.push(instance_name);
        }

        Ok(spawned_names)
    }

    /// Aggregate status for all instances with the given base name prefix
    pub fn cluster_status(
        &self,
        table: &crate::table::ProcessTable,
        base_name: &str,
    ) -> ClusterStatus {
        let mut status = ClusterStatus::default();
        let all_processes = table.list();

        let mut count = 0;
        let mut sum_cpu = 0.0;

        for p in all_processes {
            // Check if process is part of this cluster based on naming convention
            if p.name.starts_with(&format!("{}-", base_name)) {
                status.total_instances += 1;
                status.total_memory_bytes += p.memory_bytes;

                count += 1;
                sum_cpu += p.cpu_percent;

                match p.status {
                    ProcessStatus::Online => status.online_instances += 1,
                    ProcessStatus::Errored => status.errored_instances += 1,
                    _ => {}
                }
            }
        }

        if count > 0 {
            status.avg_cpu_percent = sum_cpu / (count as f64);
        }

        status
    }

    /// Restart instances one by one with a delay
    pub async fn rolling_restart(
        &self,
        base_name: &str,
        config: &ProcessConfig,
        table: &crate::table::ProcessTable,
        delay_between_instances: std::time::Duration,
    ) -> Result<(), DaemonError> {
        let all_processes = table.list();

        let mut to_restart = Vec::new();
        for p in all_processes {
            if p.name.starts_with(&format!("{}-", base_name)) {
                to_restart.push(p.name);
            }
        }

        for name in to_restart {
            self.pm
                .restart_process(&name, config, std::time::Duration::from_millis(100))
                .await?;
            tokio::time::sleep(delay_between_instances).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cpu_count() {
        let cores = cpu_count();
        assert!(cores >= 1);
    }

    #[test]
    fn test_parse_dotenv() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "PORT=8080").unwrap();
        writeln!(file, "# Comment").unwrap();
        writeln!(file, "SECRET=\"my-secret\"").unwrap();
        writeln!(file, "SINGLE='quote-secret'").unwrap();
        writeln!(file, "   SPACED   =   value   ").unwrap();

        let env = parse_dotenv(file.path()).unwrap();

        assert_eq!(env.get("PORT").unwrap(), "8080");
        assert_eq!(env.get("SECRET").unwrap(), "my-secret");
        assert_eq!(env.get("SINGLE").unwrap(), "quote-secret");
        assert_eq!(env.get("SPACED").unwrap(), "value");
        assert!(!env.contains_key("#"));
    }

    #[tokio::test]
    async fn test_spawn_cluster_and_status() {
        let table = Arc::new(crate::table::ProcessTable::new());
        let pm = Arc::new(ProcessManager::new(table.clone()));
        let cluster_manager = ClusterManager::new(pm.clone());

        let mut config = ProcessConfig::new("echo");
        config.args = vec!["hello".to_string()];
        config.instances = 3;

        let names = cluster_manager
            .spawn_cluster("cluster-app", config.clone())
            .await
            .unwrap();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "cluster-app-0");
        assert_eq!(names[1], "cluster-app-1");
        assert_eq!(names[2], "cluster-app-2");

        // Wait briefly for spawns to register completely
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let status = cluster_manager.cluster_status(&table, "cluster-app");
        assert_eq!(status.total_instances, 3);
        assert_eq!(status.online_instances, 3);

        // Test Rolling Restart (just verify it runs without error)
        cluster_manager
            .rolling_restart(
                "cluster-app",
                &config,
                &table,
                std::time::Duration::from_millis(10),
            )
            .await
            .unwrap();

        // Cleanup
        for name in names {
            pm.delete_process(&name).await.unwrap();
        }
    }
}
