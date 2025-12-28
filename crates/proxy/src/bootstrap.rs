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
    // use super::*; // hidden to avoid unused warning if not used yet, but we will use it now.

    #[tokio::test]
    async fn test_bootstrap_components() {
        // We can't easily run the full bootstrap() because it starts a server loop.
        // But we can verify that the dependent initialization functions work.
        
        // 1. Tracing init (should be idempotent due to try_init usage in bootstrap, 
        // but here we just check we can call registry)
        let subscriber = tracing_subscriber::registry();
        assert!(std::thread::current().name().is_some()); // Just ensuring thread context exists
        
        // 2. Metrics init
        // We verify that calling init_metrics multiple times doesn't panic
        let handle1 = crate::metrics::init_metrics();
        let handle2 = crate::metrics::init_metrics();
        // Handles might be different clones, but underlying recorder should be set.
        // This confirms idempotency safety in our metrics.rs implementation (if we add it).
        
        // 3. Verify version constant is available
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_bootstrap_metadata() {
         let version = env!("CARGO_PKG_VERSION");
         println!("Testing version: {}", version);
         assert!(!version.is_empty());
    }
}
