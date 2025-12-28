//! HTTP/3 Handler Module
//!
//! HTTP/3 request and response handling over QUIC streams.

use bytes::Bytes;
use tracing::{debug, info};

/// HTTP/3 request representation
#[derive(Debug, Clone)]
pub struct Http3Request {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body
    pub body: Option<Bytes>,
}

impl Http3Request {
    /// Create a new HTTP/3 request
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            headers: Vec::with_capacity(8), // Most requests have ~4-8 headers
            body: None,
        }
    }

    /// Add a header to the request
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the request body
    pub fn with_body(mut self, body: Bytes) -> Self {
        self.body = Some(body);
        self
    }
}

/// HTTP/3 response representation
#[derive(Debug, Clone)]
pub struct Http3Response {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Bytes,
}

impl Http3Response {
    /// Create a new HTTP/3 response
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::with_capacity(4), // Most responses have ~2-4 headers
            body: Bytes::new(),
        }
    }

    /// Create an OK response with body
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.into(),
        }
    }

    /// Create a not found response
    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: Vec::with_capacity(2),
            body: Bytes::from_static(b"Not Found"),
        }
    }

    /// Create an internal server error response
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            headers: Vec::with_capacity(2),
            body: Bytes::from(message.into()),
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the response body
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }
}

/// HTTP/3 connection handler configuration
#[derive(Debug, Clone)]
pub struct Http3Config {
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    /// Request body size limit
    pub max_body_size: usize,
    /// Enable request logging
    pub log_requests: bool,
}

impl Default for Http3Config {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            max_body_size: 16 * 1024 * 1024, // 16MB
            log_requests: true,
        }
    }
}

/// HTTP/3 request handler
pub struct Http3Handler {
    config: Http3Config,
    upstream_addr: String,
}

impl Http3Handler {
    /// Create a new HTTP/3 handler
    pub fn new(config: Http3Config, upstream_addr: String) -> Self {
        Self {
            config,
            upstream_addr,
        }
    }

    /// Handle an HTTP/3 request and produce a response
    pub async fn handle_request(&self, request: Http3Request) -> Http3Response {
        use aegis_telemetry::EnergyEstimator;
        use std::time::Instant;

        let start = Instant::now();

        if self.config.log_requests {
            info!("📥 HTTP/3 {} {}", request.method, request.path);
        }

        // Route to appropriate handler
        let response = match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/healthz") | ("GET", "/health") => {
                Http3Response::ok(r#"{"status":"healthy"}"#)
            }
            ("GET", "/ready") | ("GET", "/readiness") => Http3Response::ok(r#"{"status":"ready"}"#),
            ("GET", "/metrics") => {
                // Return Prometheus metrics
                if let Some(handle) = crate::metrics::get_metrics_handle() {
                    Http3Response::ok(handle.render())
                        .with_header("content-type", "text/plain; charset=utf-8")
                } else {
                    Http3Response::internal_error("Metrics not initialized")
                }
            }
            ("GET", "/energy") => {
                // Energy telemetry endpoint
                let estimator = EnergyEstimator::new();
                let info = serde_json::json!({
                    "total_requests": estimator.request_count(),
                    "total_energy_joules": estimator.total_energy_joules(),
                    "average_energy_joules": estimator.average_energy_joules(),
                    "source": "software"
                });
                Http3Response::ok(info.to_string()).with_header("content-type", "application/json")
            }
            _ => {
                // Forward to upstream - for now return not found
                debug!(
                    "Unhandled HTTP/3 request: {} {}",
                    request.method, request.path
                );
                Http3Response::not_found()
            }
        };

        let duration = start.elapsed();
        debug!("⚡ Request handled in {:?}", duration);

        response
    }

    /// Get the upstream address
    pub fn upstream_addr(&self) -> &str {
        &self.upstream_addr
    }

    /// Check if request logging is enabled
    pub fn is_logging_enabled(&self) -> bool {
        self.config.log_requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http3_request_creation() {
        let req = Http3Request::new("GET", "/path");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/path");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }

    #[test]
    fn test_http3_request_with_headers() {
        let req = Http3Request::new("POST", "/api")
            .with_header("content-type", "application/json")
            .with_header("authorization", "Bearer token");

        assert_eq!(req.headers.len(), 2);
        assert_eq!(
            req.headers[0],
            ("content-type".to_string(), "application/json".to_string())
        );
    }

    #[test]
    fn test_http3_request_with_body() {
        let req = Http3Request::new("POST", "/api").with_body(Bytes::from("test body"));

        assert!(req.body.is_some());
        assert_eq!(req.body.unwrap(), Bytes::from("test body"));
    }

    #[test]
    fn test_http3_response_ok() {
        let resp = Http3Response::ok("test");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, Bytes::from("test"));
    }

    #[test]
    fn test_http3_response_not_found() {
        let resp = Http3Response::not_found();
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_http3_response_internal_error() {
        let resp = Http3Response::internal_error("something went wrong");
        assert_eq!(resp.status, 500);
        assert_eq!(resp.body, Bytes::from("something went wrong"));
    }

    #[test]
    fn test_http3_config_default() {
        let config = Http3Config::default();
        assert_eq!(config.max_concurrent_streams, 100);
        assert_eq!(config.max_body_size, 16 * 1024 * 1024);
        assert!(config.log_requests);
    }

    #[test]
    fn test_http3_handler_creation() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        assert_eq!(handler.upstream_addr(), "127.0.0.1:8080");
        assert!(handler.is_logging_enabled());
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

        let req = Http3Request::new("GET", "/healthz");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 200);
        assert!(resp.body.starts_with(b"{"));
    }

    #[tokio::test]
    async fn test_ready_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

        let req = Http3Request::new("GET", "/ready");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_energy_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

        let req = Http3Request::new("GET", "/energy");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 200);
        let body_str = std::str::from_utf8(&resp.body).unwrap();
        assert!(body_str.contains("total_energy_joules"));
        assert!(
            resp.headers
                .iter()
                .any(|(k, v)| k == "content-type" && v == "application/json")
        );
    }

    #[test]
    fn test_http3_request_builder() {
        let req = Http3Request::new("POST", "/api/data")
            .with_header("Content-Type", "application/json")
            .with_header("Authorization", "Bearer token")
            .with_body(Bytes::from(r#"{"key":"value"}"#));

        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/data");
        assert_eq!(req.headers.len(), 2);
        assert!(req.body.is_some());
    }

    #[test]
    fn test_http3_response_new() {
        let resp = Http3Response::new(201);
        assert_eq!(resp.status, 201);
        assert!(resp.headers.is_empty());
        assert!(resp.body.is_empty());
    }

    #[test]
    fn test_http3_response_with_header() {
        let resp = Http3Response::new(200)
            .with_header("X-Custom", "value")
            .with_body("response body");

        assert_eq!(resp.headers.len(), 1);
        assert_eq!(
            resp.headers[0],
            ("X-Custom".to_string(), "value".to_string())
        );
        assert_eq!(resp.body, Bytes::from("response body"));
    }

    #[tokio::test]
    async fn test_unknown_path_returns_404() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

        let req = Http3Request::new("GET", "/unknown/path");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 404);
    }

    #[tokio::test]
    async fn test_metrics_not_initialized() {
        // This is tricky because metrics might be initialized by other tests.
        // But the code explicitly handles the None case.
        // We can't easily force it to None if it's already Some globally.
        // However, we can test the internal_error response creation.
        let resp = Http3Response::internal_error("Metrics not initialized");
        assert_eq!(resp.status, 500);
        assert_eq!(resp.body, Bytes::from("Metrics not initialized"));
    }

    #[test]
    fn test_http3_config_custom() {
        let config = Http3Config {
            max_concurrent_streams: 50,
            max_body_size: 1024,
            log_requests: false,
        };
        assert_eq!(config.max_concurrent_streams, 50);
        assert_eq!(config.max_body_size, 1024);
        assert!(!config.log_requests);
    }

    #[test]
    fn test_http3_handler_upstream_addr() {
        let handler = Http3Handler::new(Http3Config::default(), "upstream.local:8080".to_string());
        assert_eq!(handler.upstream_addr(), "upstream.local:8080");
    }

    #[test]
    fn test_http3_handler_logging_enabled() {
        let config = Http3Config {
            log_requests: true,
            ..Default::default()
        };
        let handler = Http3Handler::new(config, "localhost:8080".to_string());
        assert!(handler.is_logging_enabled());
    }

    #[tokio::test]
    async fn test_http3_handler_health_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/health");
        let resp = handler.handle_request(req).await;
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_http3_handler_ready_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/ready");
        let resp = handler.handle_request(req).await;
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_http3_handler_energy_endpoint() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/energy");
        let resp = handler.handle_request(req).await;
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        // Try to initialize metrics, ignore if already initialized
        let _ = std::panic::catch_unwind(|| {
            crate::metrics::init_metrics();
        });

        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/metrics");
        let resp = handler.handle_request(req).await;

        // Should be 200 OK with metrics text
        assert_eq!(resp.status, 200);
        assert!(resp.headers.contains(&(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string()
        )));
    }
    #[test]
    fn test_http3_handler_logging_disabled() {
        let config = Http3Config {
            log_requests: false,
            ..Default::default()
        };
        let handler = Http3Handler::new(config, "localhost:8080".to_string());
        assert!(!handler.is_logging_enabled());
    }

    #[tokio::test]
    async fn test_handle_request_log_disabled() {
        let config = Http3Config {
            log_requests: false, // This triggers the else/skip branch
            ..Default::default()
        };
        let handler = Http3Handler::new(config, "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/");
        let resp = handler.handle_request(req).await;
        // Just verify it doesn't crash and returns 404
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_http3_config_clone() {
        let config = Http3Config {
            max_concurrent_streams: 200,
            max_body_size: 2048,
            log_requests: false,
        };
        let cloned = config.clone();
        assert_eq!(cloned.max_concurrent_streams, 200);
        assert_eq!(cloned.max_body_size, 2048);
        assert!(!cloned.log_requests);
    }

    #[tokio::test]
    async fn test_handle_request_with_body() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("POST", "/api").with_body(Bytes::from(r#"{"test": "data"}"#));
        let resp = handler.handle_request(req).await;
        // Path not found, but processing should work
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_http3_response_bad_gateway() {
        let resp = Http3Response::new(502).with_body("Bad Gateway");
        assert_eq!(resp.status, 502);
        assert_eq!(resp.body, Bytes::from("Bad Gateway"));
    }

    #[test]
    fn test_http3_request_various_methods() {
        for method in ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"] {
            let req = Http3Request::new(method, "/test");
            assert_eq!(req.method, method);
        }
    }
}
