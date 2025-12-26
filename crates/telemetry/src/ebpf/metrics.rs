//! eBPF Metrics Collection
//!
//! Collects and aggregates metrics from eBPF programs.

use crate::energy::{EnergyBreakdown, EnergyMetrics};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// Per-request eBPF collected data
#[derive(Debug, Clone, Default)]
pub struct EbpfRequestData {
    /// CPU cycles consumed
    pub cpu_cycles: u64,
    /// Network bytes sent
    pub network_tx_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Block I/O bytes read
    pub block_read_bytes: u64,
    /// Block I/O bytes written
    pub block_write_bytes: u64,
    /// Memory pages allocated
    pub memory_pages: u64,
}

impl EbpfRequestData {
    /// Total network bytes
    pub fn total_network_bytes(&self) -> u64 {
        self.network_tx_bytes + self.network_rx_bytes
    }

    /// Total block I/O bytes
    pub fn total_block_bytes(&self) -> u64 {
        self.block_read_bytes + self.block_write_bytes
    }
}

/// eBPF-based metrics collector
#[derive(Debug)]
pub struct EbpfMetrics {
    /// Total CPU cycles observed
    total_cpu_cycles: AtomicU64,
    /// Total network bytes
    total_network_bytes: AtomicU64,
    /// Per-request data (request_id -> data)
    request_data: Arc<RwLock<HashMap<String, EbpfRequestData>>>,
    /// Energy coefficients for conversion
    coefficients: EnergyCoefficients,
}

/// Coefficients for converting eBPF metrics to energy
#[derive(Debug, Clone)]
pub struct EnergyCoefficients {
    /// Joules per CPU cycle
    pub joules_per_cycle: f64,
    /// Joules per network byte
    pub joules_per_network_byte: f64,
    /// Joules per block I/O byte
    pub joules_per_block_byte: f64,
    /// Joules per memory page
    pub joules_per_memory_page: f64,
}

impl Default for EnergyCoefficients {
    fn default() -> Self {
        Self {
            joules_per_cycle: 5e-11,        // ~50pJ per cycle (Intel Xeon)
            joules_per_network_byte: 5e-10, // ~0.5nJ per byte
            joules_per_block_byte: 1e-8,    // ~10nJ per byte (SSD)
            joules_per_memory_page: 5e-6,   // ~5µJ per page fault
        }
    }
}

impl EbpfMetrics {
    /// Create new eBPF metrics collector
    pub fn new() -> Self {
        Self {
            total_cpu_cycles: AtomicU64::new(0),
            total_network_bytes: AtomicU64::new(0),
            request_data: Arc::new(RwLock::new(HashMap::new())),
            coefficients: EnergyCoefficients::default(),
        }
    }

    /// Create with custom coefficients
    pub fn with_coefficients(coefficients: EnergyCoefficients) -> Self {
        Self {
            total_cpu_cycles: AtomicU64::new(0),
            total_network_bytes: AtomicU64::new(0),
            request_data: Arc::new(RwLock::new(HashMap::new())),
            coefficients,
        }
    }

    /// Start tracking a request
    pub fn start_request(&self, request_id: &str) {
        let mut data = self.request_data.write();
        data.insert(request_id.to_string(), EbpfRequestData::default());
        debug!("Started eBPF tracking for request: {}", request_id);
    }

    /// Record CPU cycles for a request
    pub fn record_cpu_cycles(&self, request_id: &str, cycles: u64) {
        let mut data = self.request_data.write();
        if let Some(req_data) = data.get_mut(request_id) {
            req_data.cpu_cycles += cycles;
        }
        self.total_cpu_cycles.fetch_add(cycles, Ordering::Relaxed);
    }

    /// Record network bytes for a request
    pub fn record_network(&self, request_id: &str, tx_bytes: u64, rx_bytes: u64) {
        let mut data = self.request_data.write();
        if let Some(req_data) = data.get_mut(request_id) {
            req_data.network_tx_bytes += tx_bytes;
            req_data.network_rx_bytes += rx_bytes;
        }
        self.total_network_bytes
            .fetch_add(tx_bytes + rx_bytes, Ordering::Relaxed);
    }

    /// Finish tracking and get energy metrics
    pub fn finish_request(
        &self,
        request_id: &str,
        endpoint: &str,
        method: &str,
        duration: Duration,
    ) -> Option<EnergyMetrics> {
        let data = {
            let mut map = self.request_data.write();
            map.remove(request_id)
        }?;

        // Convert to energy
        let cpu_energy = data.cpu_cycles as f64 * self.coefficients.joules_per_cycle;
        let network_energy =
            data.total_network_bytes() as f64 * self.coefficients.joules_per_network_byte;
        let storage_energy =
            data.total_block_bytes() as f64 * self.coefficients.joules_per_block_byte;
        let memory_energy = data.memory_pages as f64 * self.coefficients.joules_per_memory_page;

        let breakdown =
            EnergyBreakdown::new(cpu_energy, memory_energy, network_energy, storage_energy);

        debug!(
            "eBPF metrics for {}: {} cycles, {} net bytes, {} J total",
            request_id,
            data.cpu_cycles,
            data.total_network_bytes(),
            breakdown.total()
        );

        Some(
            EnergyMetrics::new(endpoint, method)
                .with_request_id(request_id)
                .with_duration(duration)
                .with_breakdown(breakdown)
                .with_bytes(data.total_network_bytes())
                .with_cpu_cycles(data.cpu_cycles),
        )
    }

    /// Get total CPU cycles observed
    pub fn total_cpu_cycles(&self) -> u64 {
        self.total_cpu_cycles.load(Ordering::Relaxed)
    }

    /// Get total network bytes observed
    pub fn total_network_bytes(&self) -> u64 {
        self.total_network_bytes.load(Ordering::Relaxed)
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.total_cpu_cycles.store(0, Ordering::Relaxed);
        self.total_network_bytes.store(0, Ordering::Relaxed);
        self.request_data.write().clear();
    }
}

impl Default for EbpfMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_metrics_creation() {
        let metrics = EbpfMetrics::new();
        assert_eq!(metrics.total_cpu_cycles(), 0);
        assert_eq!(metrics.total_network_bytes(), 0);
    }

    #[test]
    fn test_request_tracking() {
        let metrics = EbpfMetrics::new();

        metrics.start_request("req-1");
        metrics.record_cpu_cycles("req-1", 1_000_000);
        metrics.record_network("req-1", 512, 1024);

        let energy = metrics.finish_request("req-1", "/api/data", "GET", Duration::from_millis(10));

        assert!(energy.is_some());
        let energy = energy.unwrap();
        assert_eq!(energy.endpoint, "/api/data");
        assert_eq!(energy.cpu_cycles, Some(1_000_000));
        assert_eq!(energy.bytes_transferred, 1536);
        assert!(energy.total_joules() > 0.0);
    }

    #[test]
    fn test_multiple_requests() {
        let metrics = EbpfMetrics::new();

        metrics.start_request("req-1");
        metrics.start_request("req-2");

        metrics.record_cpu_cycles("req-1", 100);
        metrics.record_cpu_cycles("req-2", 200);

        assert_eq!(metrics.total_cpu_cycles(), 300);
    }

    #[test]
    fn test_reset() {
        let metrics = EbpfMetrics::new();

        metrics.start_request("req-1");
        metrics.record_cpu_cycles("req-1", 1000);

        metrics.reset();

        assert_eq!(metrics.total_cpu_cycles(), 0);
    }
}
