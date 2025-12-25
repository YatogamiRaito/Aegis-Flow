//! Aegis-Telemetry: Energy Telemetry for Aegis-Flow
//!
//! Provides kernel-level energy metrics using eBPF with software fallback.
//!
//! # Features
//! - `ebpf`: Enable eBPF-based metrics (requires Linux kernel 5.8+)
//!
//! # Example
//! ```rust,ignore
//! use aegis_telemetry::{EnergyEstimator, EnergyMetrics};
//!
//! let estimator = EnergyEstimator::new();
//! let metrics = estimator.measure_request(|| {
//!     // Your request handling code
//! });
//! println!("Energy: {} J", metrics.total_joules());
//! ```

pub mod ebpf;
pub mod energy;
pub mod estimator;
pub mod prometheus;

pub use ebpf::{EbpfLoader, EbpfMetrics};
pub use energy::{EnergyBreakdown, EnergyMetrics, EnergySource};
pub use estimator::EnergyEstimator;
pub use prometheus::EnergyPrometheusExporter;

/// Error types for telemetry operations
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("eBPF not supported on this system")]
    EbpfNotSupported,

    #[error("Failed to initialize metrics: {0}")]
    MetricsInitError(String),

    #[error("Measurement failed: {0}")]
    MeasurementError(String),
}

pub type Result<T> = std::result::Result<T, TelemetryError>;
