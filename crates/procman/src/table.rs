use crate::process::ProcessInfo;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TableError {
    #[error("Process not found: {0}")]
    NotFound(String),
    #[error("Process already exists: {0}")]
    AlreadyExists(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
struct TableData {
    processes: HashMap<String, ProcessInfo>,
}

pub struct ProcessTable {
    data: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    persist_path: Option<PathBuf>,
}

impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessTable {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            persist_path: None,
        }
    }

    pub fn with_persistence(path: impl Into<PathBuf>) -> Result<Self, TableError> {
        let path = path.into();
        let table = Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            persist_path: Some(path.clone()),
        };

        if path.exists() {
            table.load()?;
        }

        Ok(table)
    }

    pub fn add(&self, process: ProcessInfo) -> Result<(), TableError> {
        let mut data = self.data.write();
        if data.contains_key(&process.name) {
            return Err(TableError::AlreadyExists(process.name));
        }
        data.insert(process.name.clone(), process);
        drop(data);
        self.save()?;
        Ok(())
    }

    pub fn remove(&self, name: &str) -> Result<ProcessInfo, TableError> {
        let mut data = self.data.write();
        let process = data
            .remove(name)
            .ok_or_else(|| TableError::NotFound(name.to_string()))?;
        drop(data);
        self.save()?;
        Ok(process)
    }

    pub fn get(&self, name: &str) -> Result<ProcessInfo, TableError> {
        let data = self.data.read();
        data.get(name)
            .cloned()
            .ok_or_else(|| TableError::NotFound(name.to_string()))
    }

    pub fn list(&self) -> Vec<ProcessInfo> {
        let data = self.data.read();
        data.values().cloned().collect()
    }

    pub fn update<F>(&self, name: &str, f: F) -> Result<(), TableError>
    where
        F: FnOnce(&mut ProcessInfo),
    {
        let mut data = self.data.write();
        let process = data
            .get_mut(name)
            .ok_or_else(|| TableError::NotFound(name.to_string()))?;
        f(process);
        drop(data);
        self.save()?;
        Ok(())
    }

    fn load(&self) -> Result<(), TableError> {
        if let Some(path) = &self.persist_path {
            match fs::read_to_string(path) {
                Ok(content) => {
                    if content.trim().is_empty() {
                        return Ok(());
                    }
                    let table_data: TableData = serde_json::from_str(&content).map_err(|e| {
                        // Return error on corruption
                        TableError::Serialization(e)
                    })?;
                    *self.data.write() = table_data.processes;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // It's ok if the file doesn't exist yet
                }
                Err(e) => return Err(TableError::Io(e)),
            }
        }
        Ok(())
    }

    fn save(&self) -> Result<(), TableError> {
        if let Some(path) = &self.persist_path {
            let data = self.data.read();
            let table_data = TableData {
                processes: data.clone(),
            };
            let json = serde_json::to_string_pretty(&table_data)?;

            // Atomic write
            let tmp_path = path.with_extension("tmp");
            fs::write(&tmp_path, json)?;
            fs::rename(&tmp_path, path)?;
        }
        Ok(())
    }

    // Attempt to adopt process by PID; this function could check if PID is alive.
    // For now, it simply marks checking if we could adopt processes from loaded state.
    pub fn readopt_processes(&self) {
        let mut data = self.data.write();
        for (_, process) in data.iter_mut() {
            if let Some(_pid) = process.pid {
                // In a real implementation this would check `kill(_pid, 0)` or use `sysinfo`
                // to see if the process is actually running properly and belongs to us
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessStatus;
    use std::env::temp_dir;

    fn create_dummy_process(name: &str) -> ProcessInfo {
        ProcessInfo {
            name: name.to_string(),
            pid: Some(123),
            status: ProcessStatus::Online,
            restarts: 0,
            uptime_seconds: 10,
            memory_bytes: 1024,
            cpu_percent: 0.1,
        }
    }

    #[test]
    fn test_crud_operations() {
        let table = ProcessTable::new();
        let proc1 = create_dummy_process("app1");

        // Add
        assert!(table.add(proc1.clone()).is_ok());
        assert!(table.add(proc1.clone()).is_err()); // Already exists

        // Get
        let fetched = table.get("app1").unwrap();
        assert_eq!(fetched.name, "app1");
        assert!(table.get("app2").is_err());

        // Update
        table.update("app1", |p| p.restarts = 5).unwrap();
        let fetched2 = table.get("app1").unwrap();
        assert_eq!(fetched2.restarts, 5);

        // List
        table.add(create_dummy_process("app2")).unwrap();
        let list = table.list();
        assert_eq!(list.len(), 2);

        // Remove
        let removed = table.remove("app1").unwrap();
        assert_eq!(removed.name, "app1");
        assert!(table.get("app1").is_err());
    }

    #[test]
    fn test_persistence() {
        let dir = temp_dir().join(format!("aegis_test_persistence_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("table.json");

        {
            let table = ProcessTable::with_persistence(&path).unwrap();
            table.add(create_dummy_process("persisted1")).unwrap();
            table.update("persisted1", |p| p.pid = Some(42)).unwrap();
        } // drop and save

        assert!(path.exists());

        {
            let table2 = ProcessTable::with_persistence(&path).unwrap();
            let proc = table2.get("persisted1").unwrap();
            assert_eq!(proc.name, "persisted1");
            assert_eq!(proc.pid, Some(42));
        }

        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn test_readopt_processes() {
        let table = ProcessTable::new();
        table.add(create_dummy_process("readopt1")).unwrap();
        table.readopt_processes();
        // Since readopt_processes is currently a stub, we just verify it doesn't panic.
        let proc = table.get("readopt1").unwrap();
        assert_eq!(proc.pid, Some(123));
    }

    #[test]
    fn test_corrupted_persistence() {
        let dir = temp_dir().join(format!("aegis_test_corrupt_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("table_corrupt.json");
        
        // Write invalid JSON
        fs::write(&path, "{ invalid json ").unwrap();
        
        let table = ProcessTable::with_persistence(&path);
        assert!(table.is_err()); // Should return serialization error
        
        // Write empty string (should be handled gracefully)
        fs::write(&path, "   \n").unwrap();
        let table2 = ProcessTable::with_persistence(&path);
        assert!(table2.is_ok());

        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }
}
