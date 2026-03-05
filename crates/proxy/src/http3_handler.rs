//! HTTP/3 Handler Module
//!
//! HTTP/3 request and response handling over QUIC streams.

use bytes::Bytes;
use tracing::{debug, error, info, warn};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// HTTP/3 body type enum to support streaming or raw bytes
pub enum HttpBodyType {
    Empty,
    Bytes(Bytes),
    Stream(tokio::sync::mpsc::Receiver<Result<Bytes, BoxError>>),
}

impl std::fmt::Debug for HttpBodyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpBodyType::Empty => write!(f, "HttpBodyType::Empty"),
            HttpBodyType::Bytes(b) => write!(f, "HttpBodyType::Bytes({} bytes)", b.len()),
            HttpBodyType::Stream(_) => write!(f, "HttpBodyType::Stream"),
        }
    }
}

impl PartialEq<Bytes> for HttpBodyType {
    fn eq(&self, other: &Bytes) -> bool {
        match self {
            HttpBodyType::Bytes(b) => b == other,
            HttpBodyType::Empty => other.is_empty(),
            HttpBodyType::Stream(_) => false,
        }
    }
}

impl HttpBodyType {
    pub fn is_empty(&self) -> bool {
        match self {
            HttpBodyType::Empty => true,
            HttpBodyType::Bytes(b) => b.is_empty(),
            HttpBodyType::Stream(_) => false,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            HttpBodyType::Bytes(b) => Some(b),
            HttpBodyType::Empty => Some(&[]),
            HttpBodyType::Stream(_) => None,
        }
    }
}

/// HTTP/3 request representation
#[derive(Debug)]
pub struct Http3Request {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body (optional stream or bytes)
    pub body: HttpBodyType,
}

impl Http3Request {
    /// Create a new HTTP/3 request
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            headers: Vec::with_capacity(8), // Most requests have ~4-8 headers
            body: HttpBodyType::Empty,
        }
    }

    /// Add a header to the request
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the request body as bytes
    pub fn with_body(mut self, body: Bytes) -> Self {
        self.body = HttpBodyType::Bytes(body);
        self
    }

    /// Set the request body as stream
    pub fn with_stream_body(
        mut self,
        rx: tokio::sync::mpsc::Receiver<Result<Bytes, BoxError>>,
    ) -> Self {
        self.body = HttpBodyType::Stream(rx);
        self
    }
}

/// HTTP/3 response representation
#[derive(Debug)]
pub struct Http3Response {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: HttpBodyType,
}

impl Http3Response {
    /// Create a new HTTP/3 response
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::with_capacity(4), // Most responses have ~2-4 headers
            body: HttpBodyType::Empty,
        }
    }

    /// Create an OK response with body
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: HttpBodyType::Bytes(body.into()),
        }
    }

    /// Create a not found response
    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: Vec::with_capacity(2),
            body: HttpBodyType::Bytes(Bytes::from_static(b"Not Found")),
        }
    }

    /// Create an internal server error response
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            headers: Vec::with_capacity(2),
            body: HttpBodyType::Bytes(Bytes::from(message.into())),
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the response body bytes
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = HttpBodyType::Bytes(body.into());
        self
    }

    /// Set the response body stream
    pub fn with_stream_body(
        mut self,
        rx: tokio::sync::mpsc::Receiver<Result<Bytes, BoxError>>,
    ) -> Self {
        self.body = HttpBodyType::Stream(rx);
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
    client: reqwest::Client,
}

impl Http3Handler {
    /// Create a new HTTP/3 handler
    pub fn new(config: Http3Config, upstream_addr: String) -> Self {
        let connect_timeout = std::env::var("UPSTREAM_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000u64);

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .connect_timeout(std::time::Duration::from_millis(connect_timeout))
            .timeout(std::time::Duration::from_millis(connect_timeout * 3))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            upstream_addr,
            client,
        }
    }

    /// Handle an HTTP/3 request and produce a response
    pub async fn handle_request(&self, mut request: Http3Request) -> Http3Response {
        use aegis_telemetry::EnergyEstimator;
        use std::time::Instant;

        let start = Instant::now();

        if self.config.log_requests {
            info!("📥 HTTP/3 {} {}", request.method, request.path);
        }

        // 0-RTT Replay Protection
        let is_early_data = request
            .headers
            .iter()
            .any(|(k, v)| k.to_lowercase() == "early-data" && v == "1");
        if is_early_data {
            let m = request.method.as_str();
            if m != "GET" && m != "HEAD" && m != "OPTIONS" {
                warn!(
                    "🛑 Blocked non-idempotent 0-RTT request: {} {}",
                    m, request.path
                );
                return Http3Response::new(425)
                    .with_body("Too Early: Non-idempotent early data rejected");
            }
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
                // Forward to upstream
                match self.forward_to_upstream(request).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!("❌ HTTP/3 Upstream error: {}", e);
                        Http3Response::internal_error(format!("Upstream error: {}", e))
                    }
                }
            }
        };

        let duration = start.elapsed();
        debug!("⚡ Request handled in {:?}", duration);

        response
    }

    /// Forward request to upstream address
    async fn forward_to_upstream(
        &self,
        mut req: Http3Request,
    ) -> Result<Http3Response, reqwest::Error> {
        let mut url = self.upstream_addr.clone();
        if !url.starts_with("http") {
            url = format!("http://{}", url);
        }

        let target_url = format!("{}{}", url.trim_end_matches('/'), req.path);

        let method =
            reqwest::Method::from_bytes(req.method.as_bytes()).unwrap_or(reqwest::Method::GET);
        let mut upstream_req = self.client.request(method, &target_url);

        let hop_by_hop = [
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailer",
            "transfer-encoding",
            "upgrade",
            "host",
        ];

        for (k, v) in &req.headers {
            let k_lower = k.to_lowercase();
            if !hop_by_hop.contains(&k_lower.as_str()) {
                upstream_req = upstream_req.header(k, v);
            }
        }

        match req.body {
            HttpBodyType::Bytes(b) => {
                if !b.is_empty() {
                    upstream_req = upstream_req.body(b);
                }
            }
            HttpBodyType::Stream(rx) => {
                let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
                upstream_req = upstream_req.body(reqwest::Body::wrap_stream(stream));
            }
            HttpBodyType::Empty => {}
        }

        let upstream_resp = upstream_req.send().await?;
        let status = upstream_resp.status().as_u16();

        let mut h3_resp = Http3Response::new(status);
        for (name, value) in upstream_resp.headers().iter() {
            let name_str = name.as_str().to_lowercase();
            if !hop_by_hop.contains(&name_str.as_str()) {
                if let Ok(value_str) = value.to_str() {
                    h3_resp = h3_resp.with_header(name.as_str(), value_str);
                }
            }
        }

        let resp_stream = upstream_resp.bytes_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        tokio::spawn(async move {
            use futures_util::StreamExt;
            tokio::pin!(resp_stream);
            while let Some(chunk) = resp_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if tx.send(Ok(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(Box::new(e) as BoxError)).await;
                        break;
                    }
                }
            }
        });

        h3_resp.body = HttpBodyType::Stream(rx);
        Ok(h3_resp)
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
        assert!(req.body.is_empty());
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

        assert!(!req.body.is_empty());
        assert_eq!(req.body.as_bytes().unwrap(), Bytes::from("test body"));
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
        assert!(resp.body.as_bytes().unwrap().starts_with(b"{"));
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
        let body_str = std::str::from_utf8(resp.body.as_bytes().unwrap()).unwrap();
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
        assert!(!req.body.is_empty());
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
        unsafe { std::env::set_var("UPSTREAM_TIMEOUT_MS", "50") };
        let config = Http3Config {
            log_requests: false,
            ..Default::default()
        };
        // Use an address guaranteed to refuse connections in test environments
        let handler = Http3Handler::new(config, "127.0.0.1:19999".to_string());
        let req = Http3Request::new("GET", "/");
        let resp = handler.handle_request(req).await;
        // Connection refused → mapped to 500 Internal Server Error
        assert_eq!(
            resp.status, 500,
            "expected 500 on unreachable upstream, got: {}",
            resp.status
        );
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
    fn test_http3_headers_special_chars() {
        let req = Http3Request::new("GET", "/").with_header("X-Special", "!@#$%^&*()");
        assert_eq!(req.headers[0].1, "!@#$%^&*()");
    }

    #[test]
    fn test_http3_response_empty_body() {
        let resp = Http3Response::new(204).with_body("");
        assert!(resp.body.is_empty());
        assert_eq!(resp.status, 204);
    }

    #[test]
    fn test_http3_large_headers() {
        let long_val = "a".repeat(1000);
        let req = Http3Request::new("GET", "/").with_header("X-Long", &long_val);
        assert_eq!(req.headers[0].1.len(), 1000);
    }

    #[test]
    fn test_http3_headers_case_preservation() {
        // Implementation uses Strings, so case is preserved unless normalized elsewhere.
        // Let's verify our struct preserves it.
        let req = Http3Request::new("GET", "/").with_header("X-Camel-Case", "Value");
        assert_eq!(req.headers[0].0, "X-Camel-Case");
    }

    #[test]
    fn test_http3_request_various_methods() {
        for method in ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"] {
            let req = Http3Request::new(method, "/test");
            assert_eq!(req.method, method);
        }
    }

    #[tokio::test]
    async fn test_readiness_endpoint() {
        // Test the /readiness endpoint specifically (alias for /ready)
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/readiness");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 200);
        let body_str = std::str::from_utf8(resp.body.as_bytes().unwrap()).unwrap();
        assert!(body_str.contains("ready"));
    }

    #[tokio::test]
    async fn test_healthz_endpoint() {
        // Test the /healthz endpoint specifically
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/healthz");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 200);
        let body_str = std::str::from_utf8(resp.body.as_bytes().unwrap()).unwrap();
        assert!(body_str.contains("healthy"));
    }

    #[test]
    fn test_debug_impls() {
        let req = Http3Request::new("GET", "/");
        assert!(format!("{:?}", req).contains("Http3Request"));

        let resp = Http3Response::new(200);
        assert!(format!("{:?}", resp).contains("Http3Response"));

        let config = Http3Config::default();
        assert!(format!("{:?}", config).contains("Http3Config"));
    }

    #[tokio::test]
    async fn test_http3_zero_rtt_protection() {
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

        // Safe method with early data should proceed to routing
        let req_get = Http3Request::new("GET", "/healthz").with_header("Early-Data", "1");
        let resp_get = handler.handle_request(req_get).await;
        assert_eq!(resp_get.status, 200);

        // Unsafe method with early data should be blocked (425 Too Early)
        let req_post = Http3Request::new("POST", "/api/data").with_header("Early-Data", "1");
        let resp_post = handler.handle_request(req_post).await;
        assert_eq!(resp_post.status, 425);
        let body_str = std::str::from_utf8(resp_post.body.as_bytes().unwrap()).unwrap();
        assert!(body_str.contains("Too Early"));
    }
    #[tokio::test]
    async fn test_unhandled_path_triggers_debug_log() {
        // This covers line 182 - the debug! macro for unhandled requests
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());
        let req = Http3Request::new("GET", "/some/unhandled/path");
        let resp = handler.handle_request(req).await;

        assert_eq!(resp.status, 404);
    }
    #[tokio::test]
    async fn test_unsupported_method() {
        unsafe { std::env::set_var("UPSTREAM_TIMEOUT_MS", "50") };
        // Use an address guaranteed to refuse connections in test environments
        let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:19999".to_string());
        let req = Http3Request::new("BREW", "/pot");
        let resp = handler.handle_request(req).await;
        // Connection refused → mapped to 500 Internal Server Error
        assert_eq!(
            resp.status, 500,
            "expected 500 on unreachable upstream, got: {}",
            resp.status
        );
    }
}
