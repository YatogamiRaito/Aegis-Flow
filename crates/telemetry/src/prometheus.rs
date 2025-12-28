//! Prometheus Metrics Exporter
//!
//! Exports energy metrics to Prometheus.

use crate::energy::EnergyMetrics;
use metrics::{counter, gauge, histogram};
use std::sync::Once;
use tracing::info;

static INIT: Once = Once::new();

/// Initialize energy metrics for Prometheus
pub fn init_energy_metrics() {
    INIT.call_once(|| {
        info!("⚡ Initializing energy telemetry metrics");
    });
}

/// Prometheus exporter for energy metrics
#[derive(Debug, Default)]
pub struct EnergyPrometheusExporter;

impl EnergyPrometheusExporter {
    /// Create a new exporter
    pub fn new() -> Self {
        init_energy_metrics();
        Self
    }

    /// Record energy metrics to Prometheus
    pub fn record(&self, metrics: &EnergyMetrics) {
        let endpoint = metrics.endpoint.clone();
        let method = metrics.method.clone();

        // Total energy histogram (no labels for simplicity)
        histogram!("aegis_request_energy_joules").record(metrics.total_joules());

        // Energy breakdown gauges
        gauge!("aegis_request_cpu_energy_joules").set(metrics.breakdown.cpu_joules);
        gauge!("aegis_request_memory_energy_joules").set(metrics.breakdown.memory_joules);
        gauge!("aegis_request_network_energy_joules").set(metrics.breakdown.network_joules);
        gauge!("aegis_request_storage_energy_joules").set(metrics.breakdown.storage_joules);

        // Request duration
        histogram!("aegis_request_duration_seconds").record(metrics.duration.as_secs_f64());

        // Bytes transferred
        counter!("aegis_request_bytes_total").increment(metrics.bytes_transferred);

        // CPU cycles (if available)
        if let Some(cycles) = metrics.cpu_cycles {
            counter!("aegis_request_cpu_cycles_total").increment(cycles);
        }

        // Carbon footprint (assuming 400 gCO2/kWh average)
        let carbon_g = metrics.carbon_grams(400.0);
        histogram!("aegis_request_carbon_grams").record(carbon_g);

        // Log endpoint/method for debugging
        tracing::debug!("Recorded energy metrics for {} {}", method, endpoint);
    }

    /// Record aggregated statistics
    pub fn record_totals(&self, total_requests: u64, total_energy: f64) {
        gauge!("aegis_total_requests").set(total_requests as f64);
        gauge!("aegis_total_energy_joules").set(total_energy);
        gauge!("aegis_average_energy_joules").set(if total_requests > 0 {
            total_energy / total_requests as f64
        } else {
            0.0
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy::EnergyBreakdown;
    use std::time::Duration;

    #[test]
    fn test_exporter_creation() {
        let _exporter = EnergyPrometheusExporter::new();
        // Just verify it creates without panicking
    }

    #[test]
    fn test_record_metrics() {
        let exporter = EnergyPrometheusExporter::new();

        let metrics = EnergyMetrics::new("/health", "GET")
            .with_breakdown(EnergyBreakdown::new(0.001, 0.0005, 0.0002, 0.0))
            .with_duration(Duration::from_millis(10))
            .with_bytes(512);

        // Should not panic
        exporter.record(&metrics);
    }

    #[test]
    fn test_record_totals() {
        let exporter = EnergyPrometheusExporter::new();
        exporter.record_totals(100, 0.5);
    }

    #[test]
    fn test_concurrent_recording() {
        use std::sync::Arc;
        use std::thread;

        let exporter = Arc::new(EnergyPrometheusExporter::new());
        let mut handles = vec![];

        for i in 0..10 {
            let exp_clone = exporter.clone();
            handles.push(thread::spawn(move || {
                let metrics = EnergyMetrics::new("/api", "POST")
                    .with_breakdown(EnergyBreakdown::new(0.001, 0.0, 0.0, 0.0));
                exp_clone.record(&metrics);
                // Also record totals
                exp_clone.record_totals(i, 0.1 * i as f64);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_record_with_cpu_cycles() {
        let exporter = EnergyPrometheusExporter::new();

        let metrics = EnergyMetrics::new("/api", "POST")
            .with_breakdown(EnergyBreakdown::new(0.002, 0.001, 0.0005, 0.0001))
            .with_duration(Duration::from_millis(50))
            .with_bytes(4096)
            .with_cpu_cycles(1_000_000);

        exporter.record(&metrics);
        // Verifies that cpu_cycles branch is taken (line 52)
    }

    #[test]
    fn test_record_totals_zero_requests() {
        let exporter = EnergyPrometheusExporter::new();
        // Test division by zero protection
        exporter.record_totals(0, 0.0);
    }

    #[test]
    fn test_init_idempotency() {
        // Calling init multiple times should be safe due to Once
        init_energy_metrics();
        init_energy_metrics();
    }

    #[test]
    fn test_record_extreme_values() {
        let exporter = EnergyPrometheusExporter::new();
        let metrics = EnergyMetrics::new("/api", "POST")
            .with_bytes(u64::MAX)
            .with_cpu_cycles(u64::MAX);
        exporter.record(&metrics);
    }
}
