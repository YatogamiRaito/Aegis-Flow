//! Aegis-Proxy: High-performance PQC-enabled proxy server
//!
//! This is the main entry point for the Aegis-Flow proxy service.

use anyhow::Result;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub mod carbon_router;
mod config;
pub mod discovery;
pub mod dual_stack_server;
pub mod green_wait;
pub mod http3_handler;
mod http_proxy;
pub mod metrics;
mod pqc_server;
pub mod quic_server;
mod server;
pub mod tracing_otel;

pub use carbon_router::{CarbonRouter, CarbonRouterConfig, RegionScore};
pub use config::ProxyConfig;
pub use discovery::{LoadBalanceStrategy, ServiceRegistry};
pub use dual_stack_server::{DualStackConfig, DualStackServer, DualStackStats};
pub use green_wait::{
    DeferredJob, GreenWaitConfig, GreenWaitScheduler, JobPriority, ScheduleResult,
};
pub use http_proxy::{HttpProxy, HttpProxyConfig};
pub use http3_handler::{Http3Config, Http3Handler, Http3Request, Http3Response};
pub use pqc_server::PqcProxyServer;
pub use quic_server::{QuicConfig, QuicServer, QuicStats};
pub use tracing_otel::TraceContext;

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

    if config.pqc_enabled {
        info!("🛡️ PQC mode enabled - using hybrid key exchange");
        let pqc_server = PqcProxyServer::new(config);
        pqc_server.run().await?;
    } else {
        server::run(config).await?;
    }

    Ok(())
}
