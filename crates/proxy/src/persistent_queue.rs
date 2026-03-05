use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::green_wait::DeferredJob;

const QUEUE_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("deferred_jobs");

/// A thread-safe persistent queue for DeferredJob items, using redb as the storage engine
pub struct PersistentQueue {
    db: Arc<Database>,
    /// In-memory queue containing IDs in FIFO order for fast popping
    memory_queue: Mutex<VecDeque<u64>>,
    /// Counter to generate unique IDs
    next_id: std::sync::atomic::AtomicU64,
}

impl PersistentQueue {
    /// Opens or creates the persistent queue database
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let db = Database::create(path)?;
        
        // Ensure table exists
        let write_txn = db.begin_write()?;
        write_txn.open_table(QUEUE_TABLE)?;
        write_txn.commit()?;

        let mut queue = VecDeque::new();
        let mut max_id = 0;

        // Load existing jobs into memory queue
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(QUEUE_TABLE)?;
        
        for item in table.iter()? {
            let (key_guard, _) = item?;
            let id = key_guard.value();
            queue.push_back(id);
            if id > max_id {
                max_id = id;
            }
        }

        Ok(Self {
            db: Arc::new(db),
            memory_queue: Mutex::new(queue),
            next_id: std::sync::atomic::AtomicU64::new(max_id + 1),
        })
    }

    /// Pushes a job onto the back of the queue (persists to disk first, then memory)
    pub async fn push(&self, job: &DeferredJob) -> anyhow::Result<()> {
        let job_data = bincode::serialize(job)?;
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(QUEUE_TABLE)?;
            table.insert(id, job_data.as_slice())?;
        }
        write_txn.commit()?;

        let mut mq = self.memory_queue.lock().await;
        mq.push_back(id);
        
        Ok(())
    }

    /// Pops the next job from the front of the queue
    pub async fn pop(&self) -> anyhow::Result<Option<(u64, DeferredJob)>> {
        let mut mq = self.memory_queue.lock().await;
        let id_opt = mq.pop_front();

        let Some(id) = id_opt else {
            return Ok(None);
        };

        // Read from DB
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(QUEUE_TABLE)?;
        let item_guard = table.get(id)?;

        let Some(raw_data) = item_guard else {
            // Data missing in DB but present in memory queue (should not happen normally)
            return Ok(None);
        };

        // using bincode 1.x or 2.x?
        let job: DeferredJob = bincode::deserialize(raw_data.value())?;
        
        // Remove from DB
        drop(read_txn); // Drop read transaction before write
        let write_txn = self.db.begin_write()?;
        {
            let mut w_table = write_txn.open_table(QUEUE_TABLE)?;
            w_table.remove(id)?;
        }
        write_txn.commit()?;

        Ok(Some((id, job)))
    }

    /// Removes all jobs from the queue
    pub async fn clear(&self) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(QUEUE_TABLE)?;
            // redb doesn't have truncate, we pop all elements or we can just empty the memory queue
            // A better way is to iterate and remove
            let mut keys = Vec::new();
            for item in table.iter()? {
                keys.push(item?.0.value());
            }
            for key in keys {
                table.remove(key)?;
            }
        }
        write_txn.commit()?;

        let mut mq = self.memory_queue.lock().await;
        mq.clear();
        
        Ok(())
    }

    /// Returns the number of items in the queue
    pub async fn len(&self) -> usize {
        let mq = self.memory_queue.lock().await;
        mq.len()
    }

    /// Returns queue statistics: (total, expired, critical, high, normal, low, background)
    pub async fn get_stats(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
        let (mut total, mut expired) = (0, 0);
        let mut by_priority = [0; 5];
        
        let Ok(read_txn) = self.db.begin_read() else { return (0, 0, 0, 0, 0, 0, 0) };
        let Ok(table) = read_txn.open_table(QUEUE_TABLE) else { return (0, 0, 0, 0, 0, 0, 0) };
        let Ok(iter) = table.iter() else { return (0, 0, 0, 0, 0, 0, 0) };

        for item_res in iter {
            let Ok((_, val_guard)) = item_res else { continue };
            let raw_data = val_guard.value();
            if let Ok(job) = bincode::deserialize::<DeferredJob>(raw_data) {
                total += 1;
                if job.is_expired() {
                    expired += 1;
                }
                by_priority[job.priority as usize] += 1;
            }
        }
        
        (total, expired, by_priority[0], by_priority[1], by_priority[2], by_priority[3], by_priority[4])
    }
}

#[cfg(test)]
mod tests {
    use crate::green_wait::{DeferredJob, JobPriority};
    use crate::persistent_queue::PersistentQueue;
    use aegis_energy::Region;
    use tempfile::NamedTempFile;

    fn create_job() -> DeferredJob {
        DeferredJob {
            id: "test-req-123".to_string(),
            region: Region::new("TEST", "Test"),
            carbon_threshold: 450.0,
            priority: JobPriority::Background,
            submitted_at: chrono::Utc::now(),
            payload: vec![],
        }
    }

    #[tokio::test]
    async fn test_persist_job() {
        let file = NamedTempFile::new().unwrap();
        let queue = PersistentQueue::new(file.path()).unwrap();
        
        let job = create_job();
        queue.push(&job).await.unwrap();
        
        assert_eq!(queue.len().await, 1);
        
        let (id, popped) = queue.pop().await.unwrap().unwrap();
        assert_eq!(id, 1); // First job
        assert_eq!(popped.id, job.id);
        
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_memory_and_disk_consistent() {
        let file = NamedTempFile::new().unwrap();
        let queue = PersistentQueue::new(file.path()).unwrap();
        
        queue.push(&create_job()).await.unwrap();
        queue.push(&create_job()).await.unwrap();
        
        assert_eq!(queue.len().await, 2);
        queue.clear().await.unwrap();
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_recover_after_restart() {
        let file = NamedTempFile::new().unwrap();
        let job = create_job();

        {
            let queue = PersistentQueue::new(file.path()).unwrap();
            queue.push(&job).await.unwrap();
        } // Drop the first queue instance

        // Re-open DB
        let queue2 = PersistentQueue::new(file.path()).unwrap();
        assert_eq!(queue2.len().await, 1);
        
        let (_, popped) = queue2.pop().await.unwrap().unwrap();
        assert_eq!(popped.id, job.id);
    }
}
