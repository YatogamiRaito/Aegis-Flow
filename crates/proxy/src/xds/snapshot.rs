use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use prost_types::Any;

#[derive(Debug, Clone, Default)]
pub struct SnapshotInstance {
    pub version: String,
    pub resources: Vec<Any>,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub version: String,
    pub resources: Vec<Any>,
    pub listeners: SnapshotInstance,
    pub clusters: SnapshotInstance,
    pub routes: SnapshotInstance,
}

pub struct SnapshotCache {
    snapshots: Arc<RwLock<HashMap<String, Snapshot>>>,
}

impl SnapshotCache {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_snapshot(&self, node_id: &str, snapshot: Snapshot) {
        let mut lock = self.snapshots.write();
        lock.insert(node_id.to_string(), snapshot);
    }

    pub fn get_snapshot(&self, node_id: &str) -> Option<Snapshot> {
        let lock = self.snapshots.read();
        lock.get(node_id).cloned()
    }
}
