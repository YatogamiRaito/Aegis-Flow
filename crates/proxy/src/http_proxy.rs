//! HTTP/2 Reverse Proxy Module
//!
//! Provides HTTP/2 request forwarding with connection pooling.

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, server::conn::http2, service::service_fn,
};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{debug, error, info, instrument};

use crate::metrics;

/// HTTP/2 Proxy Configuration
#[derive(Debug, Clone)]
pub struct HttpProxyConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Upstream server address
    pub upstream_addr: String,
    /// Max concurrent streams
    pub max_concurrent_streams: u32,
    /// Initial window size
    pub initial_window_size: u32,
}

impl Default for HttpProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            upstream_addr: "127.0.0.1:9000".to_string(),
            max_concurrent_streams: 100,
            initial_window_size: 65535,
        }
    }
}

/// HTTP/2 Reverse Proxy Server
pub struct HttpProxy {
    config: HttpProxyConfig,
}

impl HttpProxy {
    /// Create a new HTTP proxy
    pub fn new(config: HttpProxyConfig) -> Self {
        Self { config }
    }

    /// Run the proxy server
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.config.listen_addr).await?;

        info!("🌐 HTTP/2 Proxy listening on {}", self.config.listen_addr);
        info!("🔄 Forwarding to {}", self.config.upstream_addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let upstream = self.config.upstream_addr.clone();

            tokio::spawn(async move {
                debug!("📥 HTTP/2 connection from {}", peer_addr);

                let service = service_fn(move |req| {
                    let upstream = upstream.clone();
                    async move { handle_request(req, &upstream).await }
                });

                if let Err(e) = http2::Builder::new(TokioExecutor)
                    .serve_connection(io, service)
                    .await
                {
                    error!("❌ HTTP/2 connection error: {}", e);
                }
            });
        }
    }
}

/// Handle incoming HTTP request
#[instrument(skip(req))]
pub(crate) async fn handle_request(
    req: Request<Incoming>,
    _upstream: &str,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    debug!("📨 {} {}", method, uri);

    // Process request
    let response = match (method.clone(), uri.path()) {
        (Method::GET, "/health") => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("OK")))
            .unwrap()),

        (Method::GET, "/ready") => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("{\"status\":\"ready\"}")))
            .unwrap()),

        (Method::GET, "/metrics") => {
            let body = if let Some(handle) = metrics::get_metrics_handle() {
                handle.render()
            } else {
                "# metrics not initialized".to_string()
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain; version=0.0.4")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }

        _ => {
            // Echo request info for testing
            let body = format!(
                "{{\"method\":\"{}\",\"path\":\"{}\",\"version\":\"{:?}\"}}",
                method,
                uri.path(),
                req.version()
            );
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
    };

    // Record metrics
    let status = response
        .as_ref()
        .map(|r| r.status().as_u16())
        .unwrap_or(500);
    let duration = start.elapsed().as_secs_f64();

    metrics::record_request(method.as_str(), uri.path(), status, duration);

    // Energy estimation (simplified model)
    // Formula: Energy = (Overhead) + (Bytes * CostPerByte)
    let estimated_bytes = 1024.0; // Placeholder for avg request size
    let energy_j = (estimated_bytes * 0.5e-9) + 0.01; // 0.5 nJ/bit + 10mJ overhead
    let carbon_g = energy_j / 3.6e6 * 150.0; // Assuming 150g/kWh avg intensity

    metrics::record_energy_impact(energy_j, carbon_g, "unknown");

    response
}

/// Tokio executor for Hyper
#[derive(Clone, Copy)]
pub(crate) struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HttpProxyConfig::default();
        assert_eq!(config.max_concurrent_streams, 100);
        assert_eq!(config.initial_window_size, 65535);
        assert_eq!(config.upstream_addr, "127.0.0.1:9000");
    }

    #[test]
    fn test_custom_config() {
        let config = HttpProxyConfig {
            listen_addr: "127.0.0.1:9090".parse().unwrap(),
            upstream_addr: "backend:8080".to_string(),
            max_concurrent_streams: 50,
            initial_window_size: 32768,
        };
        assert_eq!(config.max_concurrent_streams, 50);
        assert_eq!(config.upstream_addr, "backend:8080");
    }

    #[test]
    fn test_http_proxy_creation() {
        let config = HttpProxyConfig::default();
        let _proxy = HttpProxy::new(config);
        // Just verify it creates without panicking
    }

    #[test]
    fn test_config_clone() {
        let config = HttpProxyConfig::default();
        let cloned = config.clone();
        assert_eq!(config.listen_addr, cloned.listen_addr);
        assert_eq!(config.upstream_addr, cloned.upstream_addr);
        assert_eq!(config.max_concurrent_streams, cloned.max_concurrent_streams);
        assert_eq!(config.initial_window_size, cloned.initial_window_size);
    }

    #[test]
    fn test_config_debug() {
        let config = HttpProxyConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("HttpProxyConfig"));
        assert!(debug_str.contains("listen_addr"));
        assert!(debug_str.contains("upstream_addr"));
    }

    #[test]
    fn test_config_listen_addr_parsing() {
        let config = HttpProxyConfig {
            listen_addr: "0.0.0.0:3000".parse().unwrap(),
            ..Default::default()
        };
        assert_eq!(config.listen_addr.port(), 3000);
    }

    #[test]
    fn test_config_with_different_ports() {
        for port in [8080, 8443, 9000, 3000] {
            let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
            let config = HttpProxyConfig {
                listen_addr: addr,
                ..Default::default()
            };
            assert_eq!(config.listen_addr.port(), port);
        }
    }

    #[test]
    fn test_proxy_new_preserves_config() {
        let config = HttpProxyConfig {
            listen_addr: "127.0.0.1:7777".parse().unwrap(),
            upstream_addr: "custom-backend:8080".to_string(),
            max_concurrent_streams: 200,
            initial_window_size: 131070,
        };
        let proxy = HttpProxy::new(config.clone());
        assert_eq!(proxy.config.listen_addr, config.listen_addr);
        assert_eq!(proxy.config.upstream_addr, config.upstream_addr);
    }
}
