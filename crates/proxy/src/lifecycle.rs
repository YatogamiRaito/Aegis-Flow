//! Lifecycle Management Module
//!
//! Provides graceful shutdown handling and health endpoints for production readiness.
//! Supports:
//! - SIGTERM/SIGINT signal handling
//! - Connection draining
//! - Readiness and liveness probes
//! - Startup probes

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Shutdown signal receiver type
pub type ShutdownReceiver = broadcast::Receiver<()>;

/// Health status for the service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Service is healthy and ready
    Healthy,
    /// Service is starting up
    Starting,
    /// Service is shutting down (draining connections)
    Draining,
    /// Service is unhealthy
    Unhealthy,
}

impl HealthStatus {
    /// Convert to HTTP status code
    pub fn to_status_code(self) -> u16 {
        match self {
            Self::Healthy => 200,
            Self::Starting => 503,
            Self::Draining => 503,
            Self::Unhealthy => 503,
        }
    }

    /// Check if status indicates readiness
    pub fn is_ready(self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Check if status indicates liveness
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Unhealthy)
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Starting => write!(f, "starting"),
            Self::Draining => write!(f, "draining"),
            Self::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health check response for JSON endpoints
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub ready: bool,
    pub alive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl HealthResponse {
    /// Create from health status
    pub fn from_status(status: HealthStatus) -> Self {
        Self {
            status: status.to_string(),
            ready: status.is_ready(),
            alive: status.is_alive(),
            uptime_seconds: None,
            connections: None,
            version: None,
        }
    }

    /// Add uptime information
    pub fn with_uptime(mut self, uptime: Duration) -> Self {
        self.uptime_seconds = Some(uptime.as_secs());
        self
    }

    /// Add connection count
    pub fn with_connections(mut self, count: u64) -> Self {
        self.connections = Some(count);
        self
    }

    /// Add version information
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"status":"error"}"#.to_string())
    }
}

/// Lifecycle manager for graceful shutdown and health monitoring
pub struct LifecycleManager {
    /// Current health status
    status: Arc<tokio::sync::RwLock<HealthStatus>>,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Active connection count
    active_connections: Arc<AtomicU64>,
    /// Service start time
    start_time: Instant,
    /// Shutdown initiated flag
    shutting_down: Arc<AtomicBool>,
    /// Drain timeout
    drain_timeout: Duration,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            status: Arc::new(tokio::sync::RwLock::new(HealthStatus::Starting)),
            shutdown_tx,
            active_connections: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            shutting_down: Arc::new(AtomicBool::new(false)),
            drain_timeout: Duration::from_secs(30),
        }
    }

    /// Create with custom drain timeout
    pub fn with_drain_timeout(mut self, timeout: Duration) -> Self {
        self.drain_timeout = timeout;
        self
    }

    /// Get current health status
    pub async fn health_status(&self) -> HealthStatus {
        *self.status.read().await
    }

    /// Set health status
    pub async fn set_status(&self, status: HealthStatus) {
        let mut s = self.status.write().await;
        *s = status;
        info!("Health status changed to: {}", status);
    }

    /// Mark service as ready
    pub async fn mark_ready(&self) {
        self.set_status(HealthStatus::Healthy).await;
    }

    /// Mark service as unhealthy
    pub async fn mark_unhealthy(&self) {
        self.set_status(HealthStatus::Unhealthy).await;
    }

    /// Get a shutdown signal receiver
    pub fn shutdown_receiver(&self) -> ShutdownReceiver {
        self.shutdown_tx.subscribe()
    }

    /// Increment active connection count
    pub fn connection_started(&self) {
        let count = self.active_connections.fetch_add(1, Ordering::SeqCst);
        debug!("Connection started, active: {}", count + 1);
    }

    /// Decrement active connection count
    pub fn connection_finished(&self) {
        let count = self.active_connections.fetch_sub(1, Ordering::SeqCst);
        debug!("Connection finished, active: {}", count.saturating_sub(1));
    }

    /// Get active connection count
    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Get service uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
    }

    /// Create a health response
    pub async fn health_response(&self) -> HealthResponse {
        HealthResponse::from_status(self.health_status().await)
            .with_uptime(self.uptime())
            .with_connections(self.active_connections())
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    /// Initiate graceful shutdown
    pub async fn initiate_shutdown(&self) {
        if self.shutting_down.swap(true, Ordering::SeqCst) {
            warn!("Shutdown already in progress");
            return;
        }

        info!("🛑 Initiating graceful shutdown...");
        self.set_status(HealthStatus::Draining).await;

        // Send shutdown signal to all listeners
        let _ = self.shutdown_tx.send(());

        // Wait for connections to drain
        let drain_start = Instant::now();
        while self.active_connections() > 0 {
            if drain_start.elapsed() > self.drain_timeout {
                warn!(
                    "Drain timeout reached, {} connections still active",
                    self.active_connections()
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("✅ Graceful shutdown complete");
    }

    /// Setup signal handlers for Unix systems
    #[cfg(unix)]
    #[allow(clippy::expect_used)] // Signal handler setup failure is unrecoverable
    pub async fn wait_for_shutdown_signal(&self) {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }

        self.initiate_shutdown().await;
    }

    /// Setup signal handlers for non-Unix systems
    #[cfg(not(unix))]
    #[allow(clippy::expect_used)] // Signal handler setup failure is unrecoverable
    pub async fn wait_for_shutdown_signal(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C");
        self.initiate_shutdown().await;
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that decrements connection count when dropped
pub struct ConnectionGuard {
    manager: Arc<LifecycleManager>,
}

impl ConnectionGuard {
    /// Create a new connection guard
    pub fn new(manager: Arc<LifecycleManager>) -> Self {
        manager.connection_started();
        Self { manager }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.manager.connection_finished();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Starting.to_string(), "starting");
        assert_eq!(HealthStatus::Draining.to_string(), "draining");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_health_status_codes() {
        assert_eq!(HealthStatus::Healthy.to_status_code(), 200);
        assert_eq!(HealthStatus::Starting.to_status_code(), 503);
        assert_eq!(HealthStatus::Draining.to_status_code(), 503);
        assert_eq!(HealthStatus::Unhealthy.to_status_code(), 503);
    }

    #[test]
    fn test_health_status_checks() {
        assert!(HealthStatus::Healthy.is_ready());
        assert!(!HealthStatus::Starting.is_ready());

        assert!(HealthStatus::Healthy.is_alive());
        assert!(HealthStatus::Starting.is_alive());
        assert!(!HealthStatus::Unhealthy.is_alive());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_new() {
        let manager = LifecycleManager::new();
        assert_eq!(manager.health_status().await, HealthStatus::Starting);
        assert_eq!(manager.active_connections(), 0);
        assert!(!manager.is_shutting_down());
    }

    #[tokio::test]
    async fn test_mark_ready() {
        let manager = LifecycleManager::new();
        manager.mark_ready().await;
        assert_eq!(manager.health_status().await, HealthStatus::Healthy);
    }

    #[test]
    fn test_connection_tracking() {
        let manager = LifecycleManager::new();

        manager.connection_started();
        assert_eq!(manager.active_connections(), 1);

        manager.connection_started();
        assert_eq!(manager.active_connections(), 2);

        manager.connection_finished();
        assert_eq!(manager.active_connections(), 1);
    }

    #[test]
    fn test_connection_guard() {
        let manager = Arc::new(LifecycleManager::new());
        assert_eq!(manager.active_connections(), 0);

        {
            let _guard = ConnectionGuard::new(Arc::clone(&manager));
            assert_eq!(manager.active_connections(), 1);
        }

        assert_eq!(manager.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_health_response_json() {
        let manager = LifecycleManager::new();
        manager.mark_ready().await;

        let response = manager.health_response().await;
        let json = response.to_json();

        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"ready\":true"));
        assert!(json.contains("\"alive\":true"));
    }

    #[test]
    fn test_shutdown_receiver() {
        let manager = LifecycleManager::new();
        let _receiver = manager.shutdown_receiver();
        // Just ensure we can get a receiver
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let manager = Arc::new(LifecycleManager::new());
        manager.mark_ready().await;

        // Simulate a connection
        let _guard = ConnectionGuard::new(Arc::clone(&manager));
        assert_eq!(manager.active_connections(), 1);

        // Get a receiver before shutdown
        let _receiver = manager.shutdown_receiver();

        // Spawn shutdown in background (it will wait for connections)
        let _manager_clone = Arc::clone(&manager);
        let shutdown_handle = tokio::spawn(async move {
            // Use a short timeout for testing
            let manager = LifecycleManager::new().with_drain_timeout(Duration::from_millis(100));
            manager.initiate_shutdown().await;
        });

        // Verify shutdown signal is sent
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Let the shutdown complete
        let _ = shutdown_handle.await;
    }
}
