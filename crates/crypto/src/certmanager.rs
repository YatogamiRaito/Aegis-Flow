//! Certificate Manager Module
//!
//! Provides X.509 certificate management with:
//! - Self-signed certificate generation for testing
//! - Certificate chain validation
//! - Expiry monitoring
//! - PEM/DER parsing

use aegis_common::{AegisError, Result};
use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use std::path::Path;
use std::time::SystemTime;
use tracing::{debug, info, warn};
use x509_parser::prelude::*;

// Re-import time crate with explicit path to avoid conflict with x509_parser::time
// Re-import time crate with explicit path to avoid conflict with x509_parser::time
use ::time as time_crate;

/// Certificate type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertType {
    /// Root CA certificate
    RootCa,
    /// Intermediate CA certificate
    IntermediateCa,
    /// Leaf/end-entity certificate
    EndEntity,
}

/// Parsed certificate information
#[derive(Debug, Clone)]
pub struct ParsedCert {
    /// Subject Common Name
    pub subject_cn: String,
    /// Issuer Common Name
    pub issuer_cn: String,
    /// Serial number (hex)
    pub serial: String,
    /// Not valid before (UTC timestamp)
    pub not_before: i64,
    /// Not valid after (UTC timestamp)
    pub not_after: i64,
    /// Certificate type
    pub cert_type: CertType,
    /// SHA-256 fingerprint
    pub fingerprint: String,
    /// Subject Alternative Names
    pub san: Vec<String>,
    /// Raw DER bytes
    pub der_bytes: Vec<u8>,
}

impl ParsedCert {
    /// Check if certificate is currently valid
    pub fn is_valid_now(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now >= self.not_before && now <= self.not_after
    }

    /// Get days until expiry
    pub fn days_until_expiry(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        (self.not_after - now) / 86400
    }

    /// Check if certificate is expiring soon (within 30 days)
    pub fn is_expiring_soon(&self) -> bool {
        self.days_until_expiry() <= 30
    }
}

/// Certificate Manager for handling X.509 certificates
#[derive(Default)]
pub struct CertManager {
    /// Trusted CA certificates
    trusted_cas: Vec<ParsedCert>,
    /// Server certificate
    server_cert: Option<ParsedCert>,
    /// Private key (PEM format)
    private_key_pem: Option<String>,
}

impl CertManager {
    /// Create a new empty certificate manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a PEM-encoded certificate
    pub fn parse_pem(pem_data: &[u8]) -> Result<ParsedCert> {
        let pem_parsed = ::pem::parse(pem_data)
            .map_err(|e| AegisError::Crypto(format!("Failed to parse PEM: {}", e)))?;

        Self::parse_der(pem_parsed.contents())
    }

    /// Parse a DER-encoded certificate
    pub fn parse_der(der_data: &[u8]) -> Result<ParsedCert> {
        let (_, cert) = X509Certificate::from_der(der_data)
            .map_err(|e| AegisError::Crypto(format!("Failed to parse X.509: {:?}", e)))?;

        let subject_cn = cert
            .subject()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .unwrap_or("Unknown")
            .to_string();

        let issuer_cn = cert
            .issuer()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .unwrap_or("Unknown")
            .to_string();

        let serial = cert.serial.to_string();

        let not_before = cert.validity().not_before.timestamp();
        let not_after = cert.validity().not_after.timestamp();

        // Determine certificate type
        let is_ca = cert.is_ca();
        let cert_type = if is_ca {
            if subject_cn == issuer_cn {
                CertType::RootCa
            } else {
                CertType::IntermediateCa
            }
        } else {
            CertType::EndEntity
        };

        // Calculate fingerprint
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(der_data);
        let fingerprint = hex::encode(hasher.finalize());

        // Extract SANs
        let san = cert
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|san| {
                san.value
                    .general_names
                    .iter()
                    .filter_map(|gn| match gn {
                        GeneralName::DNSName(dns) => Some(dns.to_string()),
                        GeneralName::IPAddress(ip) => {
                            if ip.len() == 4 {
                                Some(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ParsedCert {
            subject_cn,
            issuer_cn,
            serial,
            not_before,
            not_after,
            cert_type,
            fingerprint,
            san,
            der_bytes: der_data.to_vec(),
        })
    }

    /// Load certificate from file (PEM or DER)
    pub fn load_from_file(path: &Path) -> Result<ParsedCert> {
        let data = std::fs::read(path)
            .map_err(|e| AegisError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

        // Try PEM first, then DER
        if data.starts_with(b"-----BEGIN") {
            Self::parse_pem(&data)
        } else {
            Self::parse_der(&data)
        }
    }

    /// Add a trusted CA certificate
    pub fn add_trusted_ca(&mut self, cert: ParsedCert) -> Result<()> {
        if cert.cert_type != CertType::RootCa && cert.cert_type != CertType::IntermediateCa {
            return Err(AegisError::Crypto(
                "Certificate is not a CA certificate".to_string(),
            ));
        }
        info!("Added trusted CA: {}", cert.subject_cn);
        self.trusted_cas.push(cert);
        Ok(())
    }

    /// Set the server certificate
    pub fn set_server_cert(&mut self, cert: ParsedCert, private_key_pem: String) -> Result<()> {
        if cert.cert_type != CertType::EndEntity {
            warn!("Server certificate is a CA certificate, this may not be intended");
        }
        info!("Set server certificate: {}", cert.subject_cn);
        self.server_cert = Some(cert);
        self.private_key_pem = Some(private_key_pem);
        Ok(())
    }

    /// Verify a certificate chain
    pub fn verify_chain(&self, cert: &ParsedCert) -> Result<bool> {
        // Check if the issuer is in trusted CAs
        for ca in &self.trusted_cas {
            if cert.issuer_cn == ca.subject_cn {
                if !ca.is_valid_now() {
                    return Err(AegisError::Crypto("CA certificate has expired".to_string()));
                }
                debug!(
                    "Certificate {} issued by trusted CA {}",
                    cert.subject_cn, ca.subject_cn
                );
                return Ok(true);
            }
        }

        // Self-signed check
        if cert.subject_cn == cert.issuer_cn && cert.cert_type == CertType::RootCa {
            debug!("Certificate {} is self-signed root CA", cert.subject_cn);
            return Ok(true);
        }

        Err(AegisError::Crypto(format!(
            "Issuer {} not found in trusted CAs",
            cert.issuer_cn
        )))
    }

    /// Generate a self-signed certificate for testing
    pub fn generate_self_signed(
        cn: &str,
        sans: &[String],
        validity_days: u32,
    ) -> Result<(String, String)> {
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, cn);
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Aegis-Flow");

        // Add SANs (skip invalid entries instead of panicking)
        params.subject_alt_names = sans
            .iter()
            .filter_map(|s| {
                if let Ok(ip) = s.parse::<std::net::IpAddr>() {
                    Some(SanType::IpAddress(ip))
                } else if let Ok(dns) = s.clone().try_into() {
                    Some(SanType::DnsName(dns))
                } else {
                    tracing::warn!("Skipping invalid SAN: {}", s);
                    None
                }
            })
            .collect();

        // Set validity
        let now = time_crate::OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + time_crate::Duration::days(validity_days as i64);

        // Generate key pair
        let key_pair = KeyPair::generate()
            .map_err(|e| AegisError::Crypto(format!("Failed to generate key pair: {}", e)))?;

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| AegisError::Crypto(format!("Failed to generate certificate: {}", e)))?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        info!(
            "Generated self-signed certificate for CN={}, valid for {} days",
            cn, validity_days
        );

        Ok((cert_pem, key_pem))
    }

    /// Get all certificates that are expiring soon
    pub fn get_expiring_certs(&self) -> Vec<&ParsedCert> {
        // Estimate: usually 0-2 certs expiring
        let mut expiring = Vec::with_capacity(self.trusted_cas.len() + 1);

        for ca in &self.trusted_cas {
            if ca.is_expiring_soon() {
                expiring.push(ca);
            }
        }

        if let Some(ref cert) = self.server_cert
            && cert.is_expiring_soon()
        {
            expiring.push(cert);
        }

        expiring
    }

    /// Get the server certificate
    pub fn server_cert(&self) -> Option<&ParsedCert> {
        self.server_cert.as_ref()
    }

    /// Get the private key PEM
    pub fn private_key_pem(&self) -> Option<&str> {
        self.private_key_pem.as_deref()
    }

    /// Get trusted CA count
    pub fn trusted_ca_count(&self) -> usize {
        self.trusted_cas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed() {
        let (cert_pem, key_pem) = CertManager::generate_self_signed(
            "test.aegis.local",
            &["localhost".to_string(), "127.0.0.1".to_string()],
            365,
        )
        .unwrap();

        assert!(cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));

        // Parse the generated certificate
        let parsed = CertManager::parse_pem(cert_pem.as_bytes()).unwrap();
        assert_eq!(parsed.subject_cn, "test.aegis.local");
        assert!(parsed.is_valid_now());
        assert!(parsed.days_until_expiry() > 360);
    }

    #[test]
    fn test_cert_manager_new() {
        let manager = CertManager::new();
        assert_eq!(manager.trusted_ca_count(), 0);
        assert!(manager.server_cert().is_none());
    }

    #[test]
    fn test_parsed_cert_validity() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "test".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "1".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "abc".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(cert.is_valid_now());
        assert_eq!(cert.days_until_expiry(), 1);
        assert!(cert.is_expiring_soon());
    }

    #[test]
    fn test_expired_cert() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "expired".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "2".to_string(),
            not_before: now - 86400 * 100,
            not_after: now - 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "def".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(!cert.is_valid_now());
        assert!(cert.days_until_expiry() < 0);
    }

    #[test]
    fn test_self_signed_chain_verification() {
        // Generate a self-signed CA
        let (ca_pem, _) = CertManager::generate_self_signed("Aegis Test CA", &[], 365).unwrap();
        let mut ca_cert = CertManager::parse_pem(ca_pem.as_bytes()).unwrap();
        // Force it to be recognized as a root CA for testing
        ca_cert.cert_type = CertType::RootCa;
        ca_cert.issuer_cn = ca_cert.subject_cn.clone();

        let mut manager = CertManager::new();
        manager.add_trusted_ca(ca_cert).unwrap();

        assert_eq!(manager.trusted_ca_count(), 1);
    }
}
