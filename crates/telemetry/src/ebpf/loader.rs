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
#[allow(dead_code)]
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
        // In test environment, likely mock mode
        // Just verify it doesn't panic
        let _ = loader.is_mock();
    }
}
