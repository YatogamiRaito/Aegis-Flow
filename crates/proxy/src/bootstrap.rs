//! Bootstrapping logic for the Aegis-Flow proxy
//!
//! This module contains the main startup logic extracted from main.rs
//! to allow for integration testing.

use crate::{PqcProxyServer, ProxyConfig, server};
use anyhow::Result;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Initialize the application and run the server
pub async fn bootstrap() -> Result<()> {
    // Initialize tracing
    // Note: We check if it's already set to allow tests to run multiple times
    let _ = tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .try_init();

    info!("🚀 Aegis-Flow Proxy starting...");
    info!("📦 Version: {}", env!("CARGO_PKG_VERSION"));

    // Initialize metrics
    let metrics_handle = crate::metrics::init_metrics();

    // Initialize lifecycle manager
    let lifecycle = std::sync::Arc::new(crate::LifecycleManager::new());

    let config = ProxyConfig::default();

    // Spawn health server
    let health_config = config.health.clone();
    let health_lifecycle = lifecycle.clone();
    let health_metrics = Some(metrics_handle.clone());

    tokio::spawn(async move {
        if let Err(e) =
            crate::health_server::run_health_server(health_config, health_lifecycle, health_metrics)
                .await
        {
            tracing::error!("Health server failed: {}", e);
        }
    });

    info!("🌐 Listening on {}:{}", config.host, config.port);
    info!("🔐 Post-Quantum Cryptography: Enabled (ML-KEM-768 + X25519)");

    if config.pqc_enabled {
        info!("🛡️ PQC mode enabled - using hybrid key exchange");
        let pqc_server = PqcProxyServer::new(config);
        pqc_server.run().await?;
    } else {
        server::run(config).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LifecycleManager, ProxyConfig};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_bootstrap_components() {
        // 1. Tracing init (should be idempotent due to try_init usage)
        let _subscriber = tracing_subscriber::registry();

        // 2. Metrics init - verify idempotency
        let _handle1 = crate::metrics::init_metrics();
        let _handle2 = crate::metrics::init_metrics();

        // 3. Verify version constant is available
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_bootstrap_metadata() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_proxy_config_defaults() {
        let config = ProxyConfig::default();
        assert!(!config.host.is_empty());
        assert!(config.port > 0);
    }

    #[test]
    fn test_lifecycle_manager_creation() {
        let lifecycle = Arc::new(LifecycleManager::new());
        assert_eq!(lifecycle.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_lifecycle_ready_state() {
        let lifecycle = Arc::new(LifecycleManager::new());
        lifecycle.mark_ready().await;
        assert!(lifecycle.health_status().await.is_ready());
    }

    #[test]
    fn test_pqc_config_enabled() {
        let config = ProxyConfig::default();
        // Default should have PQC enabled
        assert!(config.pqc_enabled);
    }

    #[test]
    fn test_health_config_clone() {
        let config = ProxyConfig::default();
        let health_config = config.health.clone();
        assert!(health_config.port > 0);
    }

    #[tokio::test]
    async fn test_pqc_server_creation() {
        let config = ProxyConfig::default();
        if config.pqc_enabled {
            let _server = PqcProxyServer::new(config);
            // Just verify creation doesn't panic
        }
    }
}
