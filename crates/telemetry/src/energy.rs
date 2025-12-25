//! Energy Metrics Module
//!
//! Defines energy measurement data structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Energy consumption breakdown by source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBreakdown {
    /// CPU energy in joules
    pub cpu_joules: f64,
    /// Memory energy in joules
    pub memory_joules: f64,
    /// Network I/O energy in joules
    pub network_joules: f64,
    /// Storage I/O energy in joules
    pub storage_joules: f64,
}

impl EnergyBreakdown {
    /// Create a new energy breakdown
    pub fn new(cpu: f64, memory: f64, network: f64, storage: f64) -> Self {
        Self {
            cpu_joules: cpu,
            memory_joules: memory,
            network_joules: network,
            storage_joules: storage,
        }
    }

    /// Total energy consumption
    pub fn total(&self) -> f64 {
        self.cpu_joules + self.memory_joules + self.network_joules + self.storage_joules
    }

    /// Zero energy breakdown
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

impl Default for EnergyBreakdown {
    fn default() -> Self {
        Self::zero()
    }
}

/// Source of energy measurement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EnergySource {
    /// Software-based estimation
    #[default]
    Software,
    /// eBPF-based measurement
    Ebpf,
    /// Hardware RAPL interface
    Rapl,
    /// Combined sources
    Hybrid,
}

/// Energy metrics for a single measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyMetrics {
    /// Request ID (if applicable)
    pub request_id: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Energy breakdown by source
    pub breakdown: EnergyBreakdown,
    /// Duration of the request
    pub duration: Duration,
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Source of measurement
    pub source: EnergySource,
    /// CPU cycles consumed (if available)
    pub cpu_cycles: Option<u64>,
    /// Bytes transferred
    pub bytes_transferred: u64,
}

impl EnergyMetrics {
    /// Create new energy metrics
    pub fn new(endpoint: &str, method: &str) -> Self {
        Self {
            request_id: None,
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            breakdown: EnergyBreakdown::zero(),
            duration: Duration::ZERO,
            timestamp: Utc::now(),
            source: EnergySource::Software,
            cpu_cycles: None,
            bytes_transferred: 0,
        }
    }

    /// Set request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set energy breakdown
    pub fn with_breakdown(mut self, breakdown: EnergyBreakdown) -> Self {
        self.breakdown = breakdown;
        self
    }

    /// Set bytes transferred
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes_transferred = bytes;
        self
    }

    /// Set CPU cycles
    pub fn with_cpu_cycles(mut self, cycles: u64) -> Self {
        self.cpu_cycles = Some(cycles);
        self
    }

    /// Total energy in joules
    pub fn total_joules(&self) -> f64 {
        self.breakdown.total()
    }

    /// Energy per byte (joules/byte)
    pub fn joules_per_byte(&self) -> f64 {
        if self.bytes_transferred > 0 {
            self.total_joules() / self.bytes_transferred as f64
        } else {
            0.0
        }
    }

    /// Estimated carbon footprint in grams CO2
    /// Using average grid intensity of 400 gCO2/kWh
    pub fn carbon_grams(&self, intensity_g_per_kwh: f64) -> f64 {
        // Convert joules to kWh: 1 kWh = 3,600,000 J
        let kwh = self.total_joules() / 3_600_000.0;
        kwh * intensity_g_per_kwh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_breakdown_total() {
        let breakdown = EnergyBreakdown::new(0.001, 0.0005, 0.0002, 0.0001);
        assert!((breakdown.total() - 0.0018).abs() < 1e-10);
    }

    #[test]
    fn test_energy_metrics_creation() {
        let metrics = EnergyMetrics::new("/api/data", "GET")
            .with_request_id("req-123")
            .with_bytes(1024);

        assert_eq!(metrics.endpoint, "/api/data");
        assert_eq!(metrics.method, "GET");
        assert_eq!(metrics.bytes_transferred, 1024);
    }

    #[test]
    fn test_carbon_calculation() {
        let metrics = EnergyMetrics::new("/health", "GET")
            .with_breakdown(EnergyBreakdown::new(0.001, 0.0, 0.0, 0.0));

        // 0.001 J = 2.78e-10 kWh
        // At 400 gCO2/kWh = 1.11e-7 gCO2
        let carbon = metrics.carbon_grams(400.0);
        assert!(carbon > 0.0);
        assert!(carbon < 1e-5);
    }

    #[test]
    fn test_joules_per_byte() {
        let metrics = EnergyMetrics::new("/upload", "POST")
            .with_breakdown(EnergyBreakdown::new(0.01, 0.0, 0.0, 0.0))
            .with_bytes(1000);

        assert!((metrics.joules_per_byte() - 0.00001).abs() < 1e-10);
    }

    #[test]
    fn test_zero_bytes_joules_per_byte() {
        let metrics = EnergyMetrics::new("/health", "GET");
        assert_eq!(metrics.joules_per_byte(), 0.0);
    }
}
