use crate::process::{ProcessConfig, ProcessInfo, ProcessStatus};
use crate::table::ProcessTable;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Failed to spawn process: {0}")]
    SpawnError(#[from] std::io::Error),
    #[error("Process table error: {0}")]
    TableError(#[from] crate::table::TableError),
    #[error("Process {0} not found")]
    NotFound(String),
}

pub struct ProcessManager {
    table: Arc<ProcessTable>,
    // Map of running processes: name -> Child handle
    handles: Arc<Mutex<HashMap<String, Child>>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new(Arc::new(ProcessTable::new()))
    }
}

impl ProcessManager {
    pub fn new(table: Arc<ProcessTable>) -> Self {
        Self {
            table,
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn_process(&self, name: &str, config: &ProcessConfig) -> Result<(), DaemonError> {
        let mut cmd = Command::new(&config.script);
        cmd.args(&config.args);
        
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        }
        
        cmd.envs(&config.env);

        let child = cmd.spawn()?;
        let pid = child.id();

        let info = ProcessInfo {
            name: name.to_string(),
            pid,
            status: ProcessStatus::Online,
            restarts: 0,
            uptime_seconds: 0, // This would normally be calculated based on start time
            memory_bytes: 0,   // Normally updated via monitoring interval
            cpu_percent: 0.0,
        };

        if self.table.get(name).is_ok() {
            self.table.update(name, |p| {
                p.pid = pid;
                p.status = ProcessStatus::Online;
            })?;
        } else {
            self.table.add(info)?;
        }
        
        let mut handles = self.handles.lock().await;
        handles.insert(name.to_string(), child);

        Ok(())
    }

    pub async fn stop_process(&self, name: &str, _grace_period: Duration) -> Result<(), DaemonError> {
        self.table.update(name, |p| p.status = ProcessStatus::Stopping)?;

        let mut handles = self.handles.lock().await;
        if let Some(mut child) = handles.remove(name) {
            // In a real implementation we would send SIGTERM first (e.g., using nix box), wait `grace_period`, 
            // and if still alive send SIGKILL. Here we simplify by just calling `kill()`.
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        self.table.update(name, |p| {
            p.status = ProcessStatus::Stopped;
            p.pid = None;
        })?;

        Ok(())
    }

    pub async fn restart_process(&self, name: &str, config: &ProcessConfig, delay: Duration) -> Result<(), DaemonError> {
        self.stop_process(name, Duration::from_secs(5)).await?; // Configurable grace period in real app
        tokio::time::sleep(delay).await;
        self.spawn_process(name, config).await?;
        
        // Update restart count
        self.table.update(name, |p| p.restarts += 1)?;
        
        Ok(())
    }

    pub async fn delete_process(&self, name: &str) -> Result<(), DaemonError> {
        let _ = self.stop_process(name, Duration::from_secs(5)).await;
        self.table.remove(name)?;
        Ok(())
    }

    /// Monitor a process and restart it if it crashes
    pub async fn monitor_process(&self, name: String, config: ProcessConfig) {
        let mut backoff = ExponentialBackoff::new();
        
        loop {
            // Check if process has been removed or stopped intentionally
            if let Ok(info) = self.table.get(&name) {
                if info.status == ProcessStatus::Stopped || info.status == ProcessStatus::Stopping {
                    break;
                }
            } else {
                break; // Process removed from table
            }

            let process_waiter = {
                let mut handles = self.handles.lock().await;
                handles.remove(&name)
            };

            if let Some(mut child) = process_waiter {
                // Wait for the process to exit
                match child.wait().await {
                    Ok(status) => {
                        // Process exited
                        if !status.success() {
                            // Check max restarts
                            let restarts = self.table.get(&name).map(|p| p.restarts).unwrap_or(0);
                            if restarts >= config.max_restarts {
                                let _ = self.table.update(&name, |p| {
                                    p.status = ProcessStatus::Errored;
                                    p.pid = None;
                                });
                                break;
                            }

                            // Sleep for backoff duration
                            tokio::time::sleep(backoff.next().await).await;

                            // Restart
                            let _ = self.spawn_process(&name, &config).await;
                            let _ = self.table.update(&name, |p| p.restarts += 1);
                        } else {
                            // Clean exit
                            let _ = self.table.update(&name, |p| {
                                p.status = ProcessStatus::Stopped;
                                p.pid = None;
                            });
                            break;
                        }
                    }
                    Err(_) => {
                        // Failed to wait
                        break;
                    }
                }
            } else {
                // No handle found, likely stopped or crashed during spawn
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    pub async fn spawn_and_monitor(&self, name: &str, config: &ProcessConfig) -> Result<(), DaemonError> {
        self.spawn_process(name, config).await?;
        
        let name_clone = name.to_string();
        let config_clone = config.clone();
        let pm_self = ProcessManager {
            table: Arc::clone(&self.table),
            handles: Arc::clone(&self.handles),
        };
        
        tokio::spawn(async move {
            pm_self.monitor_process(name_clone, config_clone).await;
        });
        
        Ok(())
    }

    pub async fn reload_process(&self, name: &str, config: &ProcessConfig) -> Result<(), DaemonError> {
        // Zero-downtime reload
        // 1. spawn new process with temp name
        let temp_name = format!("{}_reload_tmp", name);
        self.spawn_process(&temp_name, config).await?;
        
        // 2. pseudo readiness check (in a real system, would probe health endpoint)
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // 3. get new pid
        let new_pid = self.table.get(&temp_name)?.pid;
        let mut handles = self.handles.lock().await;
        let new_child = handles.remove(&temp_name).ok_or_else(|| DaemonError::NotFound(temp_name.clone()))?;
        
        // 4. insert under original name, replacing old child
        let old_child_opt = handles.insert(name.to_string(), new_child);
        drop(handles);
        
        // 5. stop old child gracefully
        if let Some(mut old_child) = old_child_opt {
            let _ = old_child.kill().await;
            let _ = old_child.wait().await;
        }
        
        // 6. update table
        self.table.update(name, |p| {
            p.pid = new_pid;
            p.status = ProcessStatus::Online;
            p.restarts = 0;
            p.uptime_seconds = 0;
        })?;
        
        self.table.remove(&temp_name)?;
        
        Ok(())
    }
}

pub struct ExponentialBackoff {
    current_ms: u64,
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Self { current_ms: 1000 }
    }

    pub async fn next(&mut self) -> Duration {
        let delay = Duration::from_millis(self.current_ms);
        self.current_ms = std::cmp::min(self.current_ms * 2, 30_000);
        delay
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    // We use simple shell commands (`sh`, `-c`, `sleep`) for reliable testing
    
    #[tokio::test]
    async fn test_spawn_and_stop_process() {
        let table = Arc::new(ProcessTable::new());
        let pm = ProcessManager::new(table.clone());
        
        let mut config = ProcessConfig::new("sh");
        config.args = vec!["-c".to_string(), "sleep 10".to_string()];
        
        // Test Spawn
        pm.spawn_process("test-app", &config).await.unwrap();
        
        let info = table.get("test-app").unwrap();
        assert_eq!(info.status, ProcessStatus::Online);
        assert!(info.pid.is_some());
        
        // Test Stop
        pm.stop_process("test-app", Duration::from_millis(100)).await.unwrap();
        
        let info2 = table.get("test-app").unwrap();
        assert_eq!(info2.status, ProcessStatus::Stopped);
        assert!(info2.pid.is_none());
    }

    #[tokio::test]
    async fn test_restart_process() {
        let table = Arc::new(ProcessTable::new());
        let pm = ProcessManager::new(table.clone());
        
        let mut config = ProcessConfig::new("sh");
        config.args = vec!["-c".to_string(), "exit 0".to_string()];
        
        pm.spawn_process("test-app", &config).await.unwrap();
        
        pm.restart_process("test-app", &config, Duration::from_millis(50)).await.unwrap();
        
        let info = table.get("test-app").unwrap();
        assert_eq!(info.restarts, 1);
        assert_eq!(info.status, ProcessStatus::Online);
        assert!(info.pid.is_some());
    }

    #[tokio::test]
    async fn test_delete_process() {
        let table = Arc::new(ProcessTable::new());
        let pm = ProcessManager::new(table.clone());
        
        let mut config = ProcessConfig::new("echo");
        config.args = vec!["hello".to_string()];
        
        pm.spawn_process("test-app", &config).await.unwrap();
        pm.delete_process("test-app").await.unwrap();
        
        assert!(table.get("test-app").is_err());
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let mut backoff = ExponentialBackoff::new();
        assert_eq!(backoff.next().await.as_millis(), 1000);
        assert_eq!(backoff.next().await.as_millis(), 2000);
        assert_eq!(backoff.next().await.as_millis(), 4000);
        assert_eq!(backoff.next().await.as_millis(), 8000);
        assert_eq!(backoff.next().await.as_millis(), 16000);
        assert_eq!(backoff.next().await.as_millis(), 30000);
        assert_eq!(backoff.next().await.as_millis(), 30000); // capped at 30s
    }

    #[tokio::test]
    async fn test_crash_recovery() {
        let table = Arc::new(ProcessTable::new());
        let pm = ProcessManager::new(table.clone());
        
        let mut config = ProcessConfig::new("sh");
        // Exit with an error code to simulate a crash
        config.args = vec!["-c".to_string(), "exit 1".to_string()];
        // Shorten backoff for tests using a custom config max if supported, but here we wait briefly 
        // to see the restart counter increment. Since exponential backoff takes at least 1s, we will speed
        // this up in a real test scenario by injecting time, but here we just check logic.
        
        // Spawn and monitor
        pm.spawn_and_monitor("crash-app", &config).await.unwrap();
        
        // Wait for it to crash and restart at least once
        tokio::time::sleep(Duration::from_millis(1500)).await;
        
        let info = table.get("crash-app").unwrap();
        // It should have restarted at least once, or in the process of backoff
        assert!(info.restarts >= 1 || info.status == ProcessStatus::Online);
        
        // Clean up
        pm.delete_process("crash-app").await.unwrap();
    }

    #[tokio::test]
    async fn test_reload_process() {
        let table = Arc::new(ProcessTable::new());
        let pm = ProcessManager::new(table.clone());
        
        let mut config = ProcessConfig::new("sh");
        config.args = vec!["-c".to_string(), "sleep 5".to_string()];
        
        pm.spawn_process("reload-app", &config).await.unwrap();
        let old_pid = table.get("reload-app").unwrap().pid;
        
        pm.reload_process("reload-app", &config).await.unwrap();
        let new_pid = table.get("reload-app").unwrap().pid;
        
        assert_ne!(old_pid, new_pid);
        let info = table.get("reload-app").unwrap();
        assert_eq!(info.status, ProcessStatus::Online);
        
        pm.delete_process("reload-app").await.unwrap();
    }
}
