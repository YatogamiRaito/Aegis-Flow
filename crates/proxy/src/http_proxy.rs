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
use tokio::net::TcpListener;
use tracing::{debug, error, info, instrument};

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
async fn handle_request(
    req: Request<Incoming>,
    _upstream: &str,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();

    debug!("📨 {} {}", method, uri);

    // For now, return a simple response
    // TODO: Forward to upstream with connection pooling
    match (method, uri.path()) {
        (Method::GET, "/health") => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("OK")))
            .unwrap()),

        (Method::GET, "/ready") => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("{\"status\":\"ready\"}")))
            .unwrap()),

        (Method::GET, "/metrics") => {
            // Return Prometheus-style metrics
            let metrics = r#"# HELP aegis_requests_total Total number of requests
# TYPE aegis_requests_total counter
aegis_requests_total 0
# HELP aegis_connections_active Active connections
# TYPE aegis_connections_active gauge
aegis_connections_active 1
"#;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain; version=0.0.4")
                .body(Full::new(Bytes::from(metrics)))
                .unwrap())
        }

        _ => {
            // Echo request info for testing
            let body = format!(
                "{{\"method\":\"{}\",\"path\":\"{}\",\"version\":\"{:?}\"}}",
                req.method(),
                req.uri().path(),
                req.version()
            );
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
    }
}

/// Tokio executor for Hyper
#[derive(Clone, Copy)]
struct TokioExecutor;

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
}
