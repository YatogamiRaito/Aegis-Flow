//! eBPF Module
//!
//! Provides eBPF-based energy measurement (when `ebpf` feature is enabled).
//! Falls back to software estimation on systems without eBPF support.

mod loader;
mod metrics;

pub use loader::EbpfLoader;
pub use metrics::EbpfMetrics;

/// Check if eBPF is available on this system
pub fn is_ebpf_available() -> bool {
    #[cfg(feature = "ebpf")]
    {
        // Check for kernel version and BTF support
        check_kernel_support()
    }
    #[cfg(not(feature = "ebpf"))]
    {
        false
    }
}

#[cfg(feature = "ebpf")]
fn check_kernel_support() -> bool {
    use std::fs;

    // Check kernel version >= 5.8
    if let Ok(version) = fs::read_to_string("/proc/version") {
        if let Some(ver) = extract_kernel_version(&version) {
            if ver.0 > 5 || (ver.0 == 5 && ver.1 >= 8) {
                // Check for BTF support
                return std::path::Path::new("/sys/kernel/btf/vmlinux").exists();
            }
        }
    }
    false
}

#[cfg(feature = "ebpf")]
fn extract_kernel_version(version_str: &str) -> Option<(u32, u32)> {
    // Parse "Linux version X.Y.Z..."
    let parts: Vec<&str> = version_str.split_whitespace().collect();
    if parts.len() >= 3 && parts[0] == "Linux" && parts[1] == "version" {
        let version_parts: Vec<&str> = parts[2].split('.').collect();
        if version_parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (
                version_parts[0].parse::<u32>(),
                version_parts[1].parse::<u32>(),
            ) {
                return Some((major, minor));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_availability_check() {
        // Just verify it doesn't panic
        let _ = is_ebpf_available();
    }

    #[test]
    #[cfg(feature = "ebpf")]
    fn test_kernel_version_parsing() {
        let version = "Linux version 5.15.0-76-generic (buildd@lcy02-amd64-080)";
        let parsed = extract_kernel_version(version);
        assert_eq!(parsed, Some((5, 15)));
    }
}
