//! Aegis-Proxy: High-performance PQC-enabled proxy server
//!
//! This is the main entry point for the Aegis-Flow proxy service.

use anyhow::Result;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod config;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    info!("🚀 Aegis-Flow Proxy starting...");
    info!("📦 Version: {}", env!("CARGO_PKG_VERSION"));
    info!("🔐 Post-Quantum Cryptography: Enabled (Kyber-768 + X25519)");

    // Initialize and run the server
    let config = config::ProxyConfig::default();
    info!("🌐 Listening on {}:{}", config.host, config.port);

    server::run(config).await?;

    Ok(())
}
