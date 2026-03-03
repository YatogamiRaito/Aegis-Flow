//! eBPF Program Loader
//!
//! Handles loading and managing eBPF programs for energy measurement.

use crate::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

/// eBPF program loader state
#[derive(Debug)]
pub struct EbpfLoader {
    /// Whether eBPF programs are loaded
    loaded: AtomicBool,
    /// Whether running in mock mode (no actual eBPF)
    mock_mode: bool,
}

impl EbpfLoader {
    /// Create a new eBPF loader
    pub fn new() -> Self {
        Self {
            loaded: AtomicBool::new(false),
            mock_mode: !super::is_ebpf_available(),
        }
    }

    #[cfg(test)]
    /// Force mock mode for testing
    pub fn set_mock_mode(&mut self, mode: bool) {
        self.mock_mode = mode;
    }

    /// Check if running in mock mode
    pub fn is_mock(&self) -> bool {
        self.mock_mode
    }

    /// Check if eBPF programs are loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Relaxed)
    }

    /// Load eBPF programs
    pub fn load(&self) -> Result<()> {
        if self.mock_mode {
            info!("🔧 eBPF not available, using software estimation");
            self.loaded.store(true, Ordering::Relaxed);
            return Ok(());
        }

        #[cfg(feature = "ebpf")]
        {
            self.load_real_ebpf()?;
        }

        #[cfg(not(feature = "ebpf"))]
        {
            info!("🔧 eBPF feature not enabled, using software estimation");
        }

        self.loaded.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Unload eBPF programs
    pub fn unload(&self) -> Result<()> {
        if !self.loaded.load(Ordering::Relaxed) {
            return Ok(());
        }

        #[cfg(feature = "ebpf")]
        if !self.mock_mode {
            self.unload_real_ebpf()?;
        }

        self.loaded.store(false, Ordering::Relaxed);
        info!("🔧 eBPF programs unloaded");
        Ok(())
    }

    #[cfg(feature = "ebpf")]
    fn load_real_ebpf(&self) -> Result<()> {
        // Real eBPF loading would happen here using aya
        // For now, this is a placeholder
        info!("⚡ Loading eBPF programs for CPU cycle tracking");

        // TODO: Implement actual eBPF program loading
        // - Load tracepoint for task_switch
        // - Load kprobe for network I/O
        // - Set up ring buffer for data transfer

        Ok(())
    }

    #[cfg(feature = "ebpf")]
    fn unload_real_ebpf(&self) -> Result<()> {
        // Real eBPF unloading would happen here
        Ok(())
    }
}

impl Default for EbpfLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared eBPF loader
pub type SharedEbpfLoader = Arc<EbpfLoader>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = EbpfLoader::new();
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_loader_load_unload() {
        let loader = EbpfLoader::new();

        assert!(loader.load().is_ok());
        assert!(loader.is_loaded());

        assert!(loader.unload().is_ok());
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_mock_mode() {
        let loader = EbpfLoader::new();
        let _ = loader.is_mock();
    }

    #[test]
    fn test_double_unload() {
        let loader = EbpfLoader::new();
        loader.load().unwrap();

        // First unload
        assert!(loader.unload().is_ok());
        assert!(!loader.is_loaded());

        // Second unload should be fine (idempotent)
        assert!(loader.unload().is_ok());
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_consecutive_loads() {
        let loader = EbpfLoader::new();

        loader.load().unwrap();
        assert!(loader.is_loaded());

        // Loading again shouldn't fail (though implementation detail: usually fine)
        loader.load().unwrap();
        assert!(loader.is_loaded());
    }

    #[test]
    fn test_loader_mock_behavior() {
        // Force mock mode logic check
        let loader = EbpfLoader::new();
        // Just verify basic property access
        let _ = loader.is_mock();

        assert!(!loader.is_loaded());
        loader.load().unwrap();
        assert!(loader.is_loaded());
    }
    #[test]
    fn test_loader_default() {
        let loader = EbpfLoader::default();
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_shared_loader() {
        let loader = Arc::new(EbpfLoader::new());
        let _shared: SharedEbpfLoader = loader;
    }

    #[test]
    fn test_loader_debug() {
        let loader = EbpfLoader::new();
        let debug_str = format!("{:?}", loader);
        assert!(debug_str.contains("EbpfLoader"));
    }

    #[test]
    fn test_is_mock_after_load() {
        let loader = EbpfLoader::new();
        let mock_before = loader.is_mock();
        loader.load().unwrap();
        let mock_after = loader.is_mock();
        assert_eq!(mock_before, mock_after);
    }

    #[test]
    fn test_unload_without_load() {
        // Unloading without loading should be safe (no-op)
        let loader = EbpfLoader::new();
        assert!(!loader.is_loaded());
        let result = loader.unload();
        assert!(result.is_ok());
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_loader_full_cycle() {
        let loader = EbpfLoader::new();

        // Initial state
        assert!(!loader.is_loaded());

        // Load
        loader.load().unwrap();
        assert!(loader.is_loaded());

        // Unload
        loader.unload().unwrap();
        assert!(!loader.is_loaded());

        // Load again
        loader.load().unwrap();
        assert!(loader.is_loaded());
    }

    #[test]
    fn test_shared_loader_operations() {
        let shared: SharedEbpfLoader = Arc::new(EbpfLoader::new());
        let clone = Arc::clone(&shared);

        shared.load().unwrap();
        assert!(clone.is_loaded());
    }

    #[test]
    fn test_loader_no_ebpf_feature_logging() {
        let subscriber = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::INFO)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let mut loader = EbpfLoader::new();
        // Force mock_mode to false to simulate "attempting" to load real eBPF
        // This exercises the code path where the feature is disabled (if compiled without feature)
        // or where it tries to load and falls back.
        // Specifically targeting lines 53, 56-57
        #[cfg(not(feature = "ebpf"))]
        {
            loader.set_mock_mode(false);
            loader.load().unwrap();
            assert!(loader.is_loaded());
        }

        #[cfg(feature = "ebpf")]
        {
            // If feature is enabled, we just want to ensure we don't regress
            loader.load().unwrap();
        }
    }
}
