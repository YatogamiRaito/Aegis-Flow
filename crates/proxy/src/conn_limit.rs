use hyper::{Response, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;

pub struct ConnLimitZone {
    pub name: String,
    pub key_type: String, // e.g., "$remote_addr"
    pub max_connections: u64,
}

pub struct ActiveConnections {
    count: AtomicU64,
}

impl ActiveConnections {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }

    pub fn current(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn increment(&self) -> u64 {
        self.count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn decrement(&self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct ConnManager {
    connections: Arc<Mutex<HashMap<String, Arc<ActiveConnections>>>>,
    zone: ConnLimitZone,
}

pub struct ConnGuard {
    key: String,
    conn_ref: Arc<ActiveConnections>,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.conn_ref.decrement();
    }
}

impl ConnManager {
    pub fn new(zone: ConnLimitZone) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            zone,
        }
    }

    pub async fn try_acquire(&self, key: &str) -> Option<ConnGuard> {
        let mut conns = self.connections.lock().await;
        let conn_ref = conns
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(ActiveConnections::new()))
            .clone();

        // This is a bit racey in highly concurrent inserts,
        // but works practically well for most non-strict edge cases.
        // It's strictly incremented inside the guard logic.

        let current = conn_ref.increment();
        if current > self.zone.max_connections {
            // Already exceeded, revert
            conn_ref.decrement();
            return None;
        }

        Some(ConnGuard {
            key: key.to_string(),
            conn_ref,
        })
    }
}

pub fn create_503_response<B>() -> Response<B>
where
    B: From<String>,
{
    let body = B::from("Service Unavailable: Connection Limit Exceeded".to_string());
    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_connections() {
        let conn = ActiveConnections::new();
        assert_eq!(conn.current(), 0);

        assert_eq!(conn.increment(), 1);
        assert_eq!(conn.increment(), 2);

        conn.decrement();
        assert_eq!(conn.current(), 1);
    }

    #[tokio::test]
    async fn test_conn_manager() {
        let zone = ConnLimitZone {
            name: "conn_limit".to_string(),
            key_type: "$remote_addr".to_string(),
            max_connections: 2,
        };

        let manager = ConnManager::new(zone);
        let key = "192.168.1.1";

        let guard1 = manager.try_acquire(key).await.unwrap();
        let guard2 = manager.try_acquire(key).await.unwrap();

        // 3rd attempt should fail
        assert!(manager.try_acquire(key).await.is_none());

        // Drop guard1, slots should free
        drop(guard1);

        let guard3 = manager.try_acquire(key).await;
        assert!(guard3.is_some());
    }

    #[test]
    fn test_503_generation() {
        let resp: Response<String> = create_503_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
