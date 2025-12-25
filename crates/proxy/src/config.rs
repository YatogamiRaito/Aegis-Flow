//! Proxy configuration module

use serde::{Deserialize, Serialize};

/// Proxy server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Enable TLS/mTLS
    pub tls_enabled: bool,
    /// Enable Post-Quantum Cryptography
    pub pqc_enabled: bool,
    /// Worker thread count (0 = auto)
    pub worker_threads: usize,
    /// Upstream address to forward requests to
    pub upstream_addr: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8443,
            tls_enabled: true,
            pqc_enabled: true,
            worker_threads: 0,
            upstream_addr: "127.0.0.1:8080".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.port, 8443);
        assert!(config.tls_enabled);
        assert!(config.pqc_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ProxyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.port, parsed.port);
    }
}
