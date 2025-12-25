//! mTLS (Mutual TLS) Module with PQC Support
//!
//! Provides certificate-based authentication with Post-Quantum cryptography.

use aegis_common::{AegisError, Result};
use std::path::Path;
use tracing::{debug, info};

/// mTLS Configuration
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Path to server certificate
    pub cert_path: String,
    /// Path to server private key
    pub key_path: String,
    /// Path to CA certificate for client verification
    pub ca_path: Option<String>,
    /// Require client certificates
    pub require_client_cert: bool,
    /// Enable PQC key exchange
    pub pqc_enabled: bool,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "/etc/aegis/certs/server.crt".to_string(),
            key_path: "/etc/aegis/certs/server.key".to_string(),
            ca_path: Some("/etc/aegis/certs/ca.crt".to_string()),
            require_client_cert: false,
            pqc_enabled: true,
        }
    }
}

/// Certificate verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether verification succeeded
    pub verified: bool,
    /// Subject common name
    pub subject_cn: Option<String>,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Expiration time (Unix timestamp)
    pub expires_at: u64,
}

/// mTLS Handler for certificate operations
pub struct MtlsHandler {
    config: MtlsConfig,
}

impl MtlsHandler {
    /// Create a new mTLS handler
    pub fn new(config: MtlsConfig) -> Self {
        Self { config }
    }

    /// Check if certificate files exist
    pub fn validate_paths(&self) -> Result<()> {
        if !Path::new(&self.config.cert_path).exists() {
            return Err(AegisError::Config(format!(
                "Certificate not found: {}",
                self.config.cert_path
            )));
        }

        if !Path::new(&self.config.key_path).exists() {
            return Err(AegisError::Config(format!(
                "Private key not found: {}",
                self.config.key_path
            )));
        }

        if self.config.ca_path.as_ref().is_some_and(|p| !Path::new(p).exists()) {
            return Err(AegisError::Config(format!(
                "CA certificate not found: {}",
                self.config.ca_path.as_ref().unwrap()
            )));
        }

        debug!("✅ Certificate paths validated");
        Ok(())
    }

    /// Get certificate info (placeholder for actual implementation)
    pub fn get_cert_info(&self) -> Result<CertInfo> {
        info!("📜 Loading certificate from {}", self.config.cert_path);

        // In production, this would parse the actual certificate
        Ok(CertInfo {
            subject: "CN=aegis-flow,O=Aegis".to_string(),
            issuer: "CN=Aegis CA,O=Aegis".to_string(),
            serial: "0001".to_string(),
            not_before: 0,
            not_after: u64::MAX,
            is_ca: false,
        })
    }

    /// Check if PQC is enabled
    pub fn is_pqc_enabled(&self) -> bool {
        self.config.pqc_enabled
    }
}

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertInfo {
    /// Subject distinguished name
    pub subject: String,
    /// Issuer distinguished name
    pub issuer: String,
    /// Serial number (hex)
    pub serial: String,
    /// Not valid before (Unix timestamp)
    pub not_before: u64,
    /// Not valid after (Unix timestamp)
    pub not_after: u64,
    /// Is this a CA certificate
    pub is_ca: bool,
}

impl CertInfo {
    /// Check if certificate is currently valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        current_time >= self.not_before && current_time <= self.not_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MtlsConfig::default();
        assert!(config.pqc_enabled);
        assert!(!config.require_client_cert);
        assert!(config.ca_path.is_some());
    }

    #[test]
    fn test_mtls_handler_creation() {
        let config = MtlsConfig::default();
        let handler = MtlsHandler::new(config);
        assert!(handler.is_pqc_enabled());
    }

    #[test]
    fn test_cert_validity() {
        let cert = CertInfo {
            subject: "CN=test".to_string(),
            issuer: "CN=ca".to_string(),
            serial: "0001".to_string(),
            not_before: 1000,
            not_after: 2000,
            is_ca: false,
        };

        assert!(cert.is_valid(1500));
        assert!(!cert.is_valid(500));
        assert!(!cert.is_valid(2500));
    }

    #[test]
    fn test_path_validation_error() {
        let config = MtlsConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            ..Default::default()
        };
        let handler = MtlsHandler::new(config);
        assert!(handler.validate_paths().is_err());
    }
}
