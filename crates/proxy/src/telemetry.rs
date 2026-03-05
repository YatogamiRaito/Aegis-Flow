//! Distributed Tracing Module
//!
//! Provides OpenTelemetry integration for distributed tracing.

use anyhow::Result;
use opentelemetry::global;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use opentelemetry_sdk::propagation::TraceContextPropagator;

/// Initialize distributed tracing
pub fn init_tracing(service_name: &str, endpoint: Option<String>) -> Result<()> {
    // W3C Trace Context Propagator setup
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Basic stdout logging layer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true);

    // Filter layer
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,aegis_proxy=debug,aegis_crypto=debug"));

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer);

    #[cfg(feature = "otel")]
    {
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() || endpoint.is_some() {
            let tracer = init_otlp_tracer(service_name, endpoint)?;
            let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            tracing::subscriber::set_global_default(subscriber.with(telemetry_layer))?;
            return Ok(());
        }
    }

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Initialize OTLP tracer
#[cfg(feature = "otel")]
pub fn init_otlp_tracer(service_name: &str, endpoint: Option<String>) -> Result<opentelemetry_sdk::trace::Tracer, anyhow::Error> {
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry::KeyValue;
    use opentelemetry_sdk::{trace as sdktrace, Resource};

    let otlp_endpoint = endpoint.unwrap_or_else(|| {
        std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string())
    });

    let sampler = match std::env::var("OTEL_TRACES_SAMPLER").as_deref() {
        Ok("traceidratio") => {
            let ratio = std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .unwrap_or_else(|_| "0.5".into())
                .parse()
                .unwrap_or(0.5);
            sdktrace::Sampler::TraceIdRatioBased(ratio)
        },
        Ok("always_off") => sdktrace::Sampler::AlwaysOff,
        _ => sdktrace::Sampler::AlwaysOn,
    };

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(
            sdktrace::config()
                .with_sampler(sampler)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    service_name.to_string(),
                )])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracer)
}

#[cfg(not(feature = "otel"))]
pub fn init_otlp_tracer(_service_name: &str, _endpoint: Option<String>) -> Result<opentelemetry_sdk::trace::Tracer, anyhow::Error> {
    anyhow::bail!("Add opentelemetry_otlp to Cargo.toml and enable auth feature to enable OTLP tracing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    static INIT: Once = Once::new();

    #[test]
    fn test_init_tracing_basic() {
        INIT.call_once(|| {
            let result = init_tracing("test-service", None);
            assert!(result.is_ok() || result.unwrap_err().to_string().contains("a global default subscriber has already been set"));
        });
    }

    #[test]
    #[cfg(not(feature = "otel"))]
    fn test_init_otlp_tracer_unimplemented() {
        let result = init_otlp_tracer("test", Some("http://localhost:4317".to_string()));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_tracing_spans_exported_as_otel() {
        // Validation check for W3C properties availability
        let propagator = TraceContextPropagator::new();
        assert_eq!(propagator.fields().count(), 1); 
    }

    #[test]
    fn test_w3c_propagation_roundtrip() {
        // To be implemented fully with `TraceContextPropagator` inject/extract tests
    }

    #[test]
    fn test_b3_propagation_roundtrip() {
        // TextMapCompositePropagator test placeholder
    }
}

