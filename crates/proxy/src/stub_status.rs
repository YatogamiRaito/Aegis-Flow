use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};

// Global metrics state for Stub Status
pub struct StubMetrics {
    pub active: AtomicUsize,
    pub accepts: AtomicUsize,
    pub handled: AtomicUsize,
    pub requests: AtomicUsize,
    pub reading: AtomicUsize,
    pub writing: AtomicUsize,
    pub waiting: AtomicUsize,
}

impl Default for StubMetrics {
    fn default() -> Self {
        Self {
            active: AtomicUsize::new(0),
            accepts: AtomicUsize::new(0),
            handled: AtomicUsize::new(0),
            requests: AtomicUsize::new(0),
            reading: AtomicUsize::new(0),
            writing: AtomicUsize::new(0),
            waiting: AtomicUsize::new(0),
        }
    }
}

use std::sync::OnceLock;

pub static GLOBAL_STUB_METRICS: OnceLock<StubMetrics> = OnceLock::new();

pub fn get_metrics() -> &'static StubMetrics {
    GLOBAL_STUB_METRICS.get_or_init(StubMetrics::default)
}

#[derive(Serialize)]
struct StubStatusResponse {
    active_connections: usize,
    server_accepts: usize,
    server_handled: usize,
    server_requests: usize,
    reading: usize,
    writing: usize,
    waiting: usize,
}

/// Helper function to generate the text response matching nginx stub_status module format.
pub fn generate_stub_status_text() -> Response<Full<Bytes>> {
    let metrics = get_metrics();
    let active = metrics.active.load(Ordering::Relaxed);
    let accepts = metrics.accepts.load(Ordering::Relaxed);
    let handled = metrics.handled.load(Ordering::Relaxed);
    let requests = metrics.requests.load(Ordering::Relaxed);
    let reading = metrics.reading.load(Ordering::Relaxed);
    let writing = metrics.writing.load(Ordering::Relaxed);
    let waiting = metrics.waiting.load(Ordering::Relaxed);

    let text_format = format!(
        "Active connections: {}\nserver accepts handled requests\n {} {} {}\nReading: {} Writing: {} Waiting: {}\n",
        active, accepts, handled, requests, reading, writing, waiting
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(text_format)))
        .unwrap()
}
/// Helper function to generate a JSON response for advanced telemetry.
pub fn generate_stub_status_json() -> Response<Full<Bytes>> {
    let metrics = get_metrics();
    let status = StubStatusResponse {
        active_connections: metrics.active.load(Ordering::Relaxed),
        server_accepts: metrics.accepts.load(Ordering::Relaxed),
        server_handled: metrics.handled.load(Ordering::Relaxed),
        server_requests: metrics.requests.load(Ordering::Relaxed),
        reading: metrics.reading.load(Ordering::Relaxed),
        writing: metrics.writing.load(Ordering::Relaxed),
        waiting: metrics.waiting.load(Ordering::Relaxed),
    };

    let json_body = serde_json::to_string(&status).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json_body)))
        .unwrap()
}
