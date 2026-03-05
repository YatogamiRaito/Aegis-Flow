use parking_lot::RwLock;
use prost_types::Any;
use std::collections::HashMap;
use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_cache_set_get() {
        let cache = SnapshotCache::new();
        let mut snapshot = Snapshot::default();
        snapshot.version = "1.0".to_string();

        cache.set_snapshot("node-test-1", snapshot);
        let result = cache
            .get_snapshot("node-test-1")
            .expect("Snapshot should exist");
        assert_eq!(result.version, "1.0");
    }

    #[test]
    fn test_snapshot_versioning() {
        let cache = SnapshotCache::new();
        let mut snapshot1 = Snapshot::default();
        snapshot1.version = "1.0".to_string();
        cache.set_snapshot("node-test-1", snapshot1);

        let mut snapshot2 = Snapshot::default();
        snapshot2.version = "2.0".to_string();
        cache.set_snapshot("node-test-1", snapshot2);

        let result = cache
            .get_snapshot("node-test-1")
            .expect("Snapshot should exist");
        assert_eq!(
            result.version, "2.0",
            "New snapshot should overwrite the old one"
        );
    }
}
