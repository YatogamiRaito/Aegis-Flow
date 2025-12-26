//! Aegis-Proxy Library
//!
//! High-performance PQC-enabled proxy server for Aegis-Flow.
//! This is the library crate that provides all public APIs.

pub mod carbon_router;
pub mod config;
pub mod discovery;
pub mod dual_stack_server;
pub mod green_wait;
pub mod http3_handler;
mod http_proxy;
pub mod lifecycle;
pub mod metrics;
mod pqc_server;
pub mod quic_server;
pub mod server;
pub mod tracing_otel;

pub use carbon_router::{CarbonRouter, CarbonRouterConfig, RegionScore};
pub use config::{
    ConfigError, ConfigFormat, ConfigManager, HealthConfig, LogConfig, ProxyConfig, TlsConfig,
};
pub use discovery::{LoadBalanceStrategy, ServiceRegistry};
pub use dual_stack_server::{DualStackConfig, DualStackServer, DualStackStats};
pub use green_wait::{
    DeferredJob, GreenWaitConfig, GreenWaitScheduler, JobPriority, ScheduleResult,
};
pub use http_proxy::{HttpProxy, HttpProxyConfig};
pub use http3_handler::{Http3Config, Http3Handler, Http3Request, Http3Response};
pub use lifecycle::{
    ConnectionGuard, HealthResponse, HealthStatus, LifecycleManager, ShutdownReceiver,
};
pub use pqc_server::PqcProxyServer;
pub use quic_server::{QuicConfig, QuicServer, QuicStats};
pub use tracing_otel::TraceContext;
