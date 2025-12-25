//! OpenTelemetry Tracing Module
//!
//! Provides distributed tracing with context propagation.

use std::collections::HashMap;
use tracing::{Level, Span, debug, info, span};

/// Trace context for distributed tracing
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// Trace ID (128-bit, hex)
    pub trace_id: String,
    /// Span ID (64-bit, hex)
    pub span_id: String,
    /// Parent span ID (optional)
    pub parent_span_id: Option<String>,
    /// Sampling decision
    pub sampled: bool,
    /// Baggage items
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new trace context with generated IDs
    pub fn new() -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            parent_span_id: None,
            sampled: true,
            baggage: HashMap::new(),
        }
    }

    /// Create a child span context
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
            baggage: self.baggage.clone(),
        }
    }

    /// Parse W3C Trace Context from headers
    pub fn from_headers(headers: &HashMap<String, String>) -> Option<Self> {
        let traceparent = headers.get("traceparent")?;
        Self::parse_traceparent(traceparent)
    }

    /// Parse traceparent header (W3C format)
    /// Format: version-trace_id-parent_id-flags
    fn parse_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        if version != "00" {
            debug!("Unknown traceparent version: {}", version);
        }

        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        let sampled = flags & 0x01 == 0x01;

        Some(Self {
            trace_id,
            span_id: generate_span_id(),
            parent_span_id: Some(span_id),
            sampled,
            baggage: HashMap::new(),
        })
    }

    /// Convert to W3C traceparent header
    pub fn to_traceparent(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id, self.span_id, flags)
    }

    /// Add baggage item
    pub fn add_baggage(&mut self, key: &str, value: &str) {
        self.baggage.insert(key.to_string(), value.to_string());
    }

    /// Get baggage item
    pub fn get_baggage(&self, key: &str) -> Option<&String> {
        self.baggage.get(key)
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random 128-bit trace ID (hex string)
fn generate_trace_id() -> String {
    use rand::Rng;
    let bytes: [u8; 16] = rand::thread_rng().r#gen();
    hex::encode(bytes)
}

/// Generate a random 64-bit span ID (hex string)
fn generate_span_id() -> String {
    use rand::Rng;
    let bytes: [u8; 8] = rand::thread_rng().r#gen();
    hex::encode(bytes)
}

/// Create a tracing span with context
pub fn create_span(name: &str, ctx: &TraceContext) -> Span {
    span!(
        Level::INFO,
        "request",
        otel.name = name,
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        parent_span_id = ?ctx.parent_span_id
    )
}

/// Span event types
#[derive(Debug, Clone, Copy)]
pub enum SpanEvent {
    /// Request received
    RequestReceived,
    /// Request forwarded to upstream
    RequestForwarded,
    /// Response received from upstream
    ResponseReceived,
    /// Response sent to client
    ResponseSent,
    /// Error occurred
    Error,
}

impl SpanEvent {
    /// Get event name
    pub fn name(&self) -> &'static str {
        match self {
            Self::RequestReceived => "request.received",
            Self::RequestForwarded => "request.forwarded",
            Self::ResponseReceived => "response.received",
            Self::ResponseSent => "response.sent",
            Self::Error => "error",
        }
    }
}

/// Record a span event
pub fn record_event(event: SpanEvent, message: &str) {
    match event {
        SpanEvent::Error => {
            tracing::error!(event = event.name(), message);
        }
        _ => {
            info!(event = event.name(), message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_new() {
        let ctx = TraceContext::new();
        assert_eq!(ctx.trace_id.len(), 32); // 16 bytes = 32 hex chars
        assert_eq!(ctx.span_id.len(), 16); // 8 bytes = 16 hex chars
        assert!(ctx.sampled);
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_child_span() {
        let parent = TraceContext::new();
        let child = parent.child();

        assert_eq!(parent.trace_id, child.trace_id);
        assert_ne!(parent.span_id, child.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }

    #[test]
    fn test_parse_traceparent() {
        let header = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = TraceContext::parse_traceparent(header).unwrap();

        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id, Some("b7ad6b7169203331".to_string()));
        assert!(ctx.sampled);
    }

    #[test]
    fn test_to_traceparent() {
        let mut ctx = TraceContext::new();
        ctx.trace_id = "0af7651916cd43dd8448eb211c80319c".to_string();
        ctx.span_id = "00f067aa0ba902b7".to_string();
        ctx.sampled = true;

        let header = ctx.to_traceparent();
        assert!(header.starts_with("00-0af7651916cd43dd8448eb211c80319c-00f067aa0ba902b7-01"));
    }

    #[test]
    fn test_baggage() {
        let mut ctx = TraceContext::new();
        ctx.add_baggage("user_id", "12345");
        ctx.add_baggage("tenant", "acme");

        assert_eq!(ctx.get_baggage("user_id"), Some(&"12345".to_string()));
        assert_eq!(ctx.get_baggage("tenant"), Some(&"acme".to_string()));
        assert_eq!(ctx.get_baggage("missing"), None);
    }
}
