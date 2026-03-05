//! OpenTelemetry Tracing Module (official SDK implementation for OTel 0.27+)
//!
//! Provides distributed tracing with OTLP/gRPC exporter.

use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::{self as sdktrace, Sampler};
use opentelemetry_sdk::{Resource, runtime, trace::TracerProvider};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[deprecated(
    since = "0.31.0",
    note = "Please use crate::telemetry::init_tracing instead"
)]
pub fn init_tracing(service_name: &str, otlp_endpoint: &str) -> anyhow::Result<()> {
    // Determine log level from env or default to info
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create OTel resource
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // Configure OTLP exporter (gRPC with tonic)
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint.to_string())
        .build()?;

    // Create Tracer Provider with ParentBased Probabilistic Sampling (10%)
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            0.1,
        ))))
        .with_resource(resource)
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    // Configure W3C Trace Context and Baggage Propagators
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(BaggagePropagator::new()),
    ]));

    // Create Tracing Layer
    let otel_layer =
        tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer("aegis-proxy"));

    // Initialize Registry with layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!(
        "🔍 OpenTelemetry tracing initialized (endpoint: {})",
        otlp_endpoint
    );
    Ok(())
}

/// Helper to gracefully shutdown the tracer provider
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}
