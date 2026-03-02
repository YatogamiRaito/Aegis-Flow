use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Online,
    Stopping,
    Stopped,
    Errored,
    Launching,
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self {
            Self::Online => "online",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Errored => "errored",
            Self::Launching => "launching",
        };
        write!(f, "{}", status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: Option<u32>,
    pub status: ProcessStatus,
    pub restarts: u32,
    pub uptime_seconds: u64,
    pub memory_bytes: u64,
    pub cpu_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub script: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub cwd: Option<String>,
    pub instances: usize,
    pub max_memory_bytes: Option<u64>,
    pub max_restarts: u32,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            script: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            cwd: None,
            instances: 1,
            max_memory_bytes: None,
            max_restarts: 15,
        }
    }
}

impl ProcessConfig {
    pub fn new(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_status_display() {
        assert_eq!(ProcessStatus::Online.to_string(), "online");
        assert_eq!(ProcessStatus::Stopping.to_string(), "stopping");
        assert_eq!(ProcessStatus::Stopped.to_string(), "stopped");
        assert_eq!(ProcessStatus::Errored.to_string(), "errored");
        assert_eq!(ProcessStatus::Launching.to_string(), "launching");
    }

    #[test]
    fn test_process_info_serialization() {
        let info = ProcessInfo {
            name: "test-app".to_string(),
            pid: Some(1234),
            status: ProcessStatus::Online,
            restarts: 5,
            uptime_seconds: 3600,
            memory_bytes: 1024 * 1024 * 50,
            cpu_percent: 1.5,
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        assert!(json.contains("test-app"));
        assert!(json.contains("Online"));
        assert!(json.contains("1234"));

        let deserialized: ProcessInfo = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.name, info.name);
        assert_eq!(deserialized.pid, info.pid);
        assert_eq!(deserialized.status, info.status);
    }

    #[test]
    fn test_process_config_defaults() {
        let config = ProcessConfig::new("app.sh");
        assert_eq!(config.script, "app.sh");
        assert_eq!(config.instances, 1);
        assert_eq!(config.max_restarts, 15);
        assert!(config.cwd.is_none());
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.max_memory_bytes.is_none());
    }
}
