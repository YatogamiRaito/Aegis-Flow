//! Energy Estimator Module
//!
//! Software-based energy estimation with optional eBPF support.

use crate::energy::{EnergyBreakdown, EnergyMetrics, EnergySource};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

/// Energy model coefficients for software estimation
#[derive(Debug, Clone)]
pub struct EnergyModel {
    /// Joules per CPU cycle
    pub joules_per_cycle: f64,
    /// Joules per byte of memory access
    pub joules_per_memory_byte: f64,
    /// Joules per byte of network I/O
    pub joules_per_network_byte: f64,
    /// Joules per byte of storage I/O
    pub joules_per_storage_byte: f64,
    /// Base overhead per request (idle energy)
    pub base_overhead_joules: f64,
}

impl Default for EnergyModel {
    fn default() -> Self {
        // Based on typical server hardware energy profiles
        // Intel Xeon: ~50-100W for CPU, ~5-10W for memory
        Self {
            joules_per_cycle: 5e-11,        // ~50pJ per cycle
            joules_per_memory_byte: 1e-9,   // ~1nJ per byte
            joules_per_network_byte: 5e-10, // ~0.5nJ per byte
            joules_per_storage_byte: 1e-8,  // ~10nJ per byte (SSD)
            base_overhead_joules: 1e-4,     // 0.1mJ base overhead
        }
    }
}

/// Energy estimator for per-request measurements
#[derive(Debug)]
pub struct EnergyEstimator {
    /// Energy model for estimation
    model: EnergyModel,
    /// Total requests measured
    request_count: AtomicU64,
    /// Total energy consumed (in micro-joules for precision)
    total_energy_uj: AtomicU64,
    /// Source of measurements
    source: EnergySource,
}

impl EnergyEstimator {
    /// Create a new energy estimator with default model
    pub fn new() -> Self {
        Self {
            model: EnergyModel::default(),
            request_count: AtomicU64::new(0),
            total_energy_uj: AtomicU64::new(0),
            source: EnergySource::Software,
        }
    }

    /// Create with custom energy model
    pub fn with_model(model: EnergyModel) -> Self {
        Self {
            model,
            request_count: AtomicU64::new(0),
            total_energy_uj: AtomicU64::new(0),
            source: EnergySource::Software,
        }
    }

    /// Get the measurement source
    pub fn source(&self) -> EnergySource {
        self.source
    }

    /// Get total request count
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get total energy consumed in joules
    pub fn total_energy_joules(&self) -> f64 {
        self.total_energy_uj.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    /// Measure energy for a synchronous operation
    #[instrument(skip(self, f))]
    pub fn measure<T, F: FnOnce() -> T>(
        &self,
        endpoint: &str,
        method: &str,
        f: F,
    ) -> (T, EnergyMetrics) {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        let metrics = self.estimate_from_duration(endpoint, method, duration, 0);
        self.record_metrics(&metrics);

        (result, metrics)
    }

    /// Measure energy with known byte count
    #[instrument(skip(self, f))]
    pub fn measure_with_bytes<T, F: FnOnce() -> T>(
        &self,
        endpoint: &str,
        method: &str,
        bytes: u64,
        f: F,
    ) -> (T, EnergyMetrics) {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        let metrics = self.estimate_from_duration(endpoint, method, duration, bytes);
        self.record_metrics(&metrics);

        (result, metrics)
    }

    /// Estimate energy from duration and bytes
    pub fn estimate_from_duration(
        &self,
        endpoint: &str,
        method: &str,
        duration: Duration,
        bytes: u64,
    ) -> EnergyMetrics {
        // Estimate CPU cycles from duration
        // Assuming ~3 GHz average CPU frequency
        let cpu_ghz = 3.0;
        let estimated_cycles = (duration.as_secs_f64() * cpu_ghz * 1e9) as u64;

        // Calculate energy breakdown
        let cpu_energy = estimated_cycles as f64 * self.model.joules_per_cycle;
        let network_energy = bytes as f64 * self.model.joules_per_network_byte;
        let memory_energy = bytes as f64 * self.model.joules_per_memory_byte;
        let base_energy = self.model.base_overhead_joules;

        let breakdown = EnergyBreakdown::new(
            cpu_energy + base_energy,
            memory_energy,
            network_energy,
            0.0, // Storage not measured in this mode
        );

        debug!(
            "Energy estimate: {:?} J for {} {} ({:?})",
            breakdown.total(),
            method,
            endpoint,
            duration
        );

        EnergyMetrics::new(endpoint, method)
            .with_duration(duration)
            .with_breakdown(breakdown)
            .with_bytes(bytes)
            .with_cpu_cycles(estimated_cycles)
    }

    /// Record metrics for aggregation
    fn record_metrics(&self, metrics: &EnergyMetrics) {
        self.request_count.fetch_add(1, Ordering::Relaxed);

        // Convert to micro-joules for better precision
        let energy_uj = (metrics.total_joules() * 1_000_000.0) as u64;
        self.total_energy_uj.fetch_add(energy_uj, Ordering::Relaxed);
    }

    /// Get average energy per request
    pub fn average_energy_joules(&self) -> f64 {
        let count = self.request_count();
        if count > 0 {
            self.total_energy_joules() / count as f64
        } else {
            0.0
        }
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.request_count.store(0, Ordering::Relaxed);
        self.total_energy_uj.store(0, Ordering::Relaxed);
    }
}

impl Default for EnergyEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe shared estimator
pub type SharedEnergyEstimator = Arc<EnergyEstimator>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimator_creation() {
        let estimator = EnergyEstimator::new();
        assert_eq!(estimator.source(), EnergySource::Software);
        assert_eq!(estimator.request_count(), 0);
    }

    #[test]
    fn test_measure_simple() {
        let estimator = EnergyEstimator::new();

        let (result, metrics) = estimator.measure("/health", "GET", || 42);

        assert_eq!(result, 42);
        assert_eq!(metrics.endpoint, "/health");
        assert_eq!(metrics.method, "GET");
        assert!(metrics.total_joules() > 0.0);
        assert_eq!(estimator.request_count(), 1);
    }

    #[test]
    fn test_measure_with_bytes() {
        let estimator = EnergyEstimator::new();

        let (_, metrics) = estimator.measure_with_bytes("/upload", "POST", 1024, || {
            std::thread::sleep(Duration::from_micros(100));
        });

        assert_eq!(metrics.bytes_transferred, 1024);
        assert!(metrics.total_joules() > 0.0);
    }

    #[test]
    fn test_average_energy() {
        let estimator = EnergyEstimator::new();

        for _ in 0..10 {
            estimator.measure("/test", "GET", || {
                std::thread::sleep(Duration::from_micros(10));
            });
        }

        assert_eq!(estimator.request_count(), 10);
        assert!(estimator.average_energy_joules() > 0.0);
    }

    #[test]
    fn test_reset() {
        let estimator = EnergyEstimator::new();

        estimator.measure("/test", "GET", || ());
        assert_eq!(estimator.request_count(), 1);

        estimator.reset();
        assert_eq!(estimator.request_count(), 0);
        assert_eq!(estimator.total_energy_joules(), 0.0);
    }

    #[test]
    fn test_custom_model() {
        let model = EnergyModel {
            joules_per_cycle: 1e-10,
            ..Default::default()
        };
        let estimator = EnergyEstimator::with_model(model);

        let (_, metrics) = estimator.measure("/test", "GET", || {
            std::thread::sleep(Duration::from_micros(100));
        });

        // Higher energy with custom model
        assert!(metrics.total_joules() > 0.0);
    }

    #[test]
    fn test_estimator_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let estimator = Arc::new(EnergyEstimator::new());
        let mut handles = vec![];

        for _ in 0..5 {
            let est_clone = Arc::clone(&estimator);
            let handle = thread::spawn(move || {
                est_clone.measure("/test", "GET", || ());
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(estimator.request_count(), 5);
    }

    #[test]
    fn test_energy_model_default() {
        let model = EnergyModel::default();
        assert!(model.joules_per_cycle > 0.0);
        assert!(model.joules_per_memory_byte > 0.0);
    }

    #[test]
    fn test_energy_model_debug() {
        let model = EnergyModel::default();
        let debug = format!("{:?}", model);
        assert!(debug.contains("EnergyModel"));
    }

    #[test]
    fn test_average_energy_zero_requests() {
        let estimator = EnergyEstimator::new();
        assert_eq!(estimator.average_energy_joules(), 0.0);
    }

    #[test]
    fn test_estimator_default() {
        let estimator = EnergyEstimator::default();
        assert_eq!(estimator.request_count(), 0);
        assert_eq!(estimator.source(), EnergySource::Software);
    }

    #[test]
    fn test_energy_model_clone() {
        let model1 = EnergyModel::default();
        let model2 = model1.clone();
        assert_eq!(model1.joules_per_cycle, model2.joules_per_cycle);
    }

    #[test]
    fn test_estimator_debug() {
        let estimator = EnergyEstimator::new();
        let debug_str = format!("{:?}", estimator);
        assert!(debug_str.contains("EnergyEstimator"));
    }

    #[test]
    fn test_estimate_from_duration_logic() {
        let estimator = EnergyEstimator::new();
        let duration = Duration::from_secs(1);
        let bytes = 1_000_000;

        let metrics = estimator.estimate_from_duration("/manual", "TEST", duration, bytes);

        // Verify calculations: ~0.1516 J
        assert!(metrics.total_joules() > 0.15);
        assert!(metrics.total_joules() < 0.16);
        assert_eq!(metrics.bytes_transferred, bytes);
        assert_eq!(metrics.endpoint, "/manual");
    }

    #[test]
    fn test_estimator_source() {
        let estimator = EnergyEstimator::new();
        let source = estimator.source();
        assert_eq!(source, EnergySource::Software);
    }

    #[test]
    fn test_shared_energy_estimator_type() {
        // Test the SharedEnergyEstimator type alias
        let shared: SharedEnergyEstimator = Arc::new(EnergyEstimator::new());
        let cloned = Arc::clone(&shared);

        // Both references should work
        assert_eq!(shared.request_count(), 0);
        assert_eq!(cloned.request_count(), 0);

        // Measure through one reference
        shared.measure("/test", "GET", || 42);

        // Both should see the update
        assert_eq!(shared.request_count(), 1);
        assert_eq!(cloned.request_count(), 1);
    }

    #[test]
    fn test_total_energy_joules_precision() {
        let estimator = EnergyEstimator::new();

        // Do multiple measurements
        for _ in 0..100 {
            estimator.measure("/precision", "GET", || {
                std::thread::sleep(Duration::from_micros(10));
            });
        }

        let total = estimator.total_energy_joules();
        assert!(total > 0.0);

        // Verify precision conversion (micro-joules to joules)
        let avg = estimator.average_energy_joules();
        let expected_total = avg * 100.0;
        assert!((total - expected_total).abs() < 1e-10);
    }
}
