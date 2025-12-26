//! Prometheus Metrics Module
//!
//! Provides metrics collection and export for observability.

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;
use tracing::info;

/// Global metrics handle
static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Metric names
pub mod names {
    pub const REQUESTS_TOTAL: &str = "aegis_requests_total";
    pub const REQUEST_DURATION: &str = "aegis_request_duration_seconds";
    pub const CONNECTIONS_ACTIVE: &str = "aegis_connections_active";
    pub const HANDSHAKES_TOTAL: &str = "aegis_pqc_handshakes_total";
    pub const HANDSHAKE_DURATION: &str = "aegis_pqc_handshake_duration_seconds";
    pub const BYTES_SENT: &str = "aegis_bytes_sent_total";
    pub const BYTES_RECEIVED: &str = "aegis_bytes_received_total";
    pub const ENCRYPTION_OPERATIONS: &str = "aegis_encryption_operations_total";
    pub const ERRORS_TOTAL: &str = "aegis_errors_total";
    pub const CARBON_INTENSITY: &str = "aegis_carbon_intensity_g_kwh";
    pub const ESTIMATED_ENERGY: &str = "aegis_estimated_energy_joules_total";
    pub const ESTIMATED_CARBON: &str = "aegis_estimated_carbon_grams_total";
    pub const DEFERRED_JOBS: &str = "aegis_deferred_jobs_current";
}

/// Initialize the metrics system
#[allow(clippy::expect_used)] // Panicking is acceptable during initialization
pub fn init_metrics() -> PrometheusHandle {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    // Describe metrics
    describe_counter!(names::REQUESTS_TOTAL, "Total number of requests processed");
    describe_histogram!(names::REQUEST_DURATION, "Request duration in seconds");
    describe_gauge!(names::CONNECTIONS_ACTIVE, "Number of active connections");
    describe_counter!(names::HANDSHAKES_TOTAL, "Total PQC handshakes completed");
    describe_histogram!(
        names::HANDSHAKE_DURATION,
        "PQC handshake duration in seconds"
    );
    describe_counter!(names::BYTES_SENT, "Total bytes sent");
    describe_counter!(names::BYTES_RECEIVED, "Total bytes received");
    describe_counter!(
        names::ENCRYPTION_OPERATIONS,
        "Total encryption/decryption operations"
    );
    describe_counter!(names::ERRORS_TOTAL, "Total errors");
    describe_gauge!(
        names::CARBON_INTENSITY,
        "Current carbon intensity for each region (gCO2/kWh)"
    );
    describe_counter!(
        names::ESTIMATED_ENERGY,
        "Estimated energy consumed in Joules"
    );
    describe_counter!(
        names::ESTIMATED_CARBON,
        "Estimated carbon emissions in grams"
    );
    describe_gauge!(
        names::DEFERRED_JOBS,
        "Number of jobs currently waiting in Green-Wait queue"
    );

    info!("📊 Metrics system initialized");

    METRICS_HANDLE.set(handle.clone()).ok();
    handle
}

/// Get the global metrics handle
pub fn get_metrics_handle() -> Option<&'static PrometheusHandle> {
    METRICS_HANDLE.get()
}

/// Record a request
pub fn record_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    counter!(names::REQUESTS_TOTAL, "method" => method.to_string(), "path" => path.to_string(), "status" => status.to_string()).increment(1);
    histogram!(names::REQUEST_DURATION, "method" => method.to_string()).record(duration_secs);
}

/// Record a PQC handshake
pub fn record_handshake(algorithm: &str, duration_secs: f64, success: bool) {
    counter!(names::HANDSHAKES_TOTAL, "algorithm" => algorithm.to_string(), "success" => success.to_string()).increment(1);
    if success {
        histogram!(names::HANDSHAKE_DURATION, "algorithm" => algorithm.to_string())
            .record(duration_secs);
    }
}

/// Update active connections gauge
pub fn set_active_connections(count: f64) {
    gauge!(names::CONNECTIONS_ACTIVE).set(count);
}

/// Increment active connections
pub fn increment_connections() {
    gauge!(names::CONNECTIONS_ACTIVE).increment(1.0);
}

/// Decrement active connections
pub fn decrement_connections() {
    gauge!(names::CONNECTIONS_ACTIVE).decrement(1.0);
}

/// Record bytes transferred
pub fn record_bytes(sent: u64, received: u64) {
    counter!(names::BYTES_SENT).increment(sent);
    counter!(names::BYTES_RECEIVED).increment(received);
}

/// Record encryption operation
pub fn record_encryption(operation: &str) {
    counter!(names::ENCRYPTION_OPERATIONS, "operation" => operation.to_string()).increment(1);
}

/// Record an error
pub fn record_error(error_type: &str) {
    counter!(names::ERRORS_TOTAL, "type" => error_type.to_string()).increment(1);
}

/// Update carbon intensity for a region
pub fn update_carbon_intensity(region: &str, intensity: f64) {
    gauge!(names::CARBON_INTENSITY, "region" => region.to_string()).set(intensity);
}

/// Record estimated energy and carbon
pub fn record_energy_impact(joules: f64, carbon_grams: f64, region: &str) {
    counter!(names::ESTIMATED_ENERGY, "region" => region.to_string()).increment(joules as u64);
    counter!(names::ESTIMATED_CARBON, "region" => region.to_string())
        .increment(carbon_grams as u64);
}

/// Update deferred jobs count
pub fn update_deferred_jobs(count: usize) {
    gauge!(names::DEFERRED_JOBS).set(count as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_names() {
        assert!(names::REQUESTS_TOTAL.starts_with("aegis_"));
        assert!(names::HANDSHAKE_DURATION.contains("duration"));
    }

    #[test]
    fn test_all_metric_names_have_prefix() {
        assert!(names::REQUESTS_TOTAL.starts_with("aegis_"));
        assert!(names::REQUEST_DURATION.starts_with("aegis_"));
        assert!(names::CONNECTIONS_ACTIVE.starts_with("aegis_"));
        assert!(names::HANDSHAKES_TOTAL.starts_with("aegis_"));
        assert!(names::HANDSHAKE_DURATION.starts_with("aegis_"));
        assert!(names::BYTES_SENT.starts_with("aegis_"));
        assert!(names::BYTES_RECEIVED.starts_with("aegis_"));
        assert!(names::ENCRYPTION_OPERATIONS.starts_with("aegis_"));
        assert!(names::ERRORS_TOTAL.starts_with("aegis_"));
    }

    #[test]
    fn test_metric_names_are_not_empty() {
        assert!(!names::REQUESTS_TOTAL.is_empty());
        assert!(!names::REQUEST_DURATION.is_empty());
        assert!(!names::CONNECTIONS_ACTIVE.is_empty());
        assert!(!names::HANDSHAKES_TOTAL.is_empty());
        assert!(!names::HANDSHAKE_DURATION.is_empty());
        assert!(!names::BYTES_SENT.is_empty());
        assert!(!names::BYTES_RECEIVED.is_empty());
        assert!(!names::ENCRYPTION_OPERATIONS.is_empty());
        assert!(!names::ERRORS_TOTAL.is_empty());
    }

    #[test]
    fn test_metric_names_contain_expected_keywords() {
        assert!(names::REQUESTS_TOTAL.contains("requests"));
        assert!(names::REQUEST_DURATION.contains("duration"));
        assert!(names::CONNECTIONS_ACTIVE.contains("connections"));
        assert!(names::HANDSHAKES_TOTAL.contains("handshakes"));
        assert!(names::BYTES_SENT.contains("bytes"));
        assert!(names::BYTES_RECEIVED.contains("bytes"));
        assert!(names::ENCRYPTION_OPERATIONS.contains("encryption"));
        assert!(names::ERRORS_TOTAL.contains("errors"));
    }

    // Note: Functions like record_request, record_handshake etc. require
    // a metrics recorder to be installed. These are tested in integration tests.
}
