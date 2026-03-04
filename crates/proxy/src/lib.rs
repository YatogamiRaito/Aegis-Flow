//! Aegis-Proxy Library
//!
//! High-performance PQC-enabled proxy server for Aegis-Flow.
//! This is the library crate that provides all public APIs.

pub mod access_log;
pub mod acl;
pub mod acme;
pub mod admin_api;
pub mod auth;
pub mod auth_request;
pub mod autoindex;
pub mod bootstrap;
pub mod caching;
pub mod carbon_router;
pub mod circuit_breaker;
pub mod compression;
pub mod config;
pub mod conn_limit;
pub mod discovery;
pub mod dual_stack_server;
pub mod fastcgi;
pub mod geoip;
pub mod green_wait;
pub mod headers;
pub mod health_check;
pub mod health_server;
pub mod http3_handler;
pub mod http_proxy;
pub mod image_filter;
pub mod jwt;
pub mod lb;
pub mod lifecycle;
pub mod limit_except;
pub mod limit_rate;
pub mod location;
pub mod map_directive;
pub mod master;
pub mod metrics;
pub mod mime_types;
pub mod mirror;
pub mod pqc_server;
pub mod proxy_cache;
pub mod proxy_protocol;
pub mod quic_server;
pub mod ranges;
pub mod rate_limit;
pub mod rewrite;
pub mod scgi;
pub mod server;
pub mod sni;
pub mod split_clients;
pub mod ssi;
pub mod static_files;
pub mod sticky;
pub mod stream_proto;
pub mod stream_proxy;
pub mod stub_status;
pub mod sub_filter;
pub mod syslog;
pub mod tracing_otel;
pub mod udp_proxy;
pub mod upstream;
pub mod upstream_client;
pub mod validator;
pub mod variables;
pub mod vhost;
pub mod waf;
pub mod websocket;
pub mod xslt;
pub mod zero_copy;
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
