//! Aegis-Proxy: High-performance PQC-enabled proxy server
//!
//! This is the main entry point for the Aegis-Flow proxy service.

use aegis_proxy::{PqcProxyServer, ProxyConfig, server};
use anyhow::Result;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    info!("🚀 Aegis-Flow Proxy starting...");
    info!("📦 Version: {}", env!("CARGO_PKG_VERSION"));

    // Initialize metrics
    let metrics_handle = aegis_proxy::metrics::init_metrics();

    // Initialize lifecycle manager
    let lifecycle = std::sync::Arc::new(aegis_proxy::LifecycleManager::new());
    
    let config = ProxyConfig::default();
    
    // Spawn health server
    let health_config = config.health.clone();
    let health_lifecycle = lifecycle.clone();
    let health_metrics = Some(metrics_handle.clone());
    
    tokio::spawn(async move {
        if let Err(e) = aegis_proxy::health_server::run_health_server(
            health_config, 
            health_lifecycle, 
            health_metrics
        ).await {
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
