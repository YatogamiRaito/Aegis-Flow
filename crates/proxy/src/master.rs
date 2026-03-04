use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Worker process metadata
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: usize,
    pub pid: u32,
    pub cpu_core: Option<usize>,
}

/// Worker process table owned by the master
pub struct MasterProcess {
    workers: Vec<WorkerInfo>,
    max_workers: usize,
    pub worker_connections: usize,
}

impl MasterProcess {
    pub fn new(max_workers: usize, worker_connections: usize) -> Self {
        Self {
            workers: Vec::new(),
            max_workers,
            worker_connections,
        }
    }

    pub fn auto_worker_count() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    pub fn register_worker(&mut self, pid: u32, cpu_core: Option<usize>) -> usize {
        let id = self.workers.len();
        self.workers.push(WorkerInfo { id, pid, cpu_core });
        id
    }

    pub fn remove_worker(&mut self, pid: u32) -> Option<WorkerInfo> {
        if let Some(idx) = self.workers.iter().position(|w| w.pid == pid) {
            Some(self.workers.remove(idx))
        } else {
            None
        }
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn needs_more_workers(&self) -> bool {
        self.workers.len() < self.max_workers
    }
}

/// Connection counter for worker_connections limit
pub struct ConnectionCounter {
    current: Arc<AtomicUsize>,
    max: usize,
}

impl ConnectionCounter {
    pub fn new(max: usize) -> Self {
        Self {
            current: Arc::new(AtomicUsize::new(0)),
            max,
        }
    }

    pub fn try_acquire(&self) -> bool {
        let current = self.current.fetch_add(1, Ordering::Relaxed);
        if current >= self.max {
            self.current.fetch_sub(1, Ordering::Relaxed);
            false
        } else {
            true
        }
    }

    pub fn release(&self) {
        self.current.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn current_count(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }
}

/// Crash rate tracker: detect excessive crashes and stop respawning
pub struct CrashTracker {
    crash_times: Vec<std::time::Instant>,
    max_crashes: usize,
    window_secs: u64,
}

impl CrashTracker {
    pub fn new(max_crashes: usize, window_secs: u64) -> Self {
        Self {
            crash_times: Vec::new(),
            max_crashes,
            window_secs,
        }
    }

    pub fn record_crash(&mut self) {
        let now = std::time::Instant::now();
        // Clean old crashes outside window
        self.crash_times
            .retain(|t| now.duration_since(*t).as_secs() < self.window_secs);
        self.crash_times.push(now);
    }

    pub fn should_stop_respawning(&self) -> bool {
        self.crash_times.len() >= self.max_crashes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_auto_worker_count() {
        let count = MasterProcess::auto_worker_count();
        assert!(count >= 1);
    }

    #[test]
    fn test_master_worker_registration() {
        let mut master = MasterProcess::new(4, 1000);
        assert_eq!(master.worker_count(), 0);
        assert!(master.needs_more_workers());

        master.register_worker(1001, Some(0));
        master.register_worker(1002, Some(1));

        assert_eq!(master.worker_count(), 2);
        assert!(master.needs_more_workers());
    }

    #[test]
    fn test_master_remove_worker() {
        let mut master = MasterProcess::new(4, 1000);
        master.register_worker(1001, Some(0));

        let removed = master.remove_worker(1001);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().pid, 1001);
        assert_eq!(master.worker_count(), 0);
    }

    #[test]
    fn test_connection_counter() {
        let counter = ConnectionCounter::new(2);

        assert!(counter.try_acquire()); // 1
        assert!(counter.try_acquire()); // 2
        assert!(!counter.try_acquire()); // 3 - over limit

        counter.release(); // back to 1
        assert!(counter.try_acquire()); // back to 2
    }

    #[test]
    fn test_crash_tracker() {
        let mut tracker = CrashTracker::new(3, 60);

        assert!(!tracker.should_stop_respawning());

        tracker.record_crash();
        tracker.record_crash();
        assert!(!tracker.should_stop_respawning());

        tracker.record_crash();
        assert!(tracker.should_stop_respawning());
    }
}
