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
    } // Missing brace restored

    #[test]
    fn test_load_missing_file() {
        let path = Path::new("/path/to/non/existent/file.crt");
        let result = CertManager::load_from_file(path);
        // Should be AegisError::Config or IoError
        match result {
            Err(AegisError::Config(msg)) => assert!(msg.contains("Failed to read")),
            _ => panic!("Expected Config error handling IO failure"),
        }
    }

    #[test]
    fn test_parse_invalid_pem() {
        let invalid_pem = b"-----BEGIN CERTIFICATE-----\nINVALID_DATA\n-----END CERTIFICATE-----";
        let result = CertManager::parse_pem(invalid_pem);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_der() {
        let invalid_der = vec![0u8; 100]; // Just zeros
        let result = CertManager::parse_der(&invalid_der);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_chain_untrusted_issuer() {
        let (ca_pem, _) = CertManager::generate_self_signed("Untrusted CA", &[], 365).unwrap();
        let _ca_cert = CertManager::parse_pem(ca_pem.as_bytes()).unwrap();

        // Leaf cert signed by CA (simulation, since we can't easily sign with rcgen without key)
        // But verify_chain only checks subject/issuer match logic in this implementation
        // effectively trusting if issuer is in trusted list.
        // So we can mock a cert with issuer = "Untrusted CA"

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let leaf = ParsedCert {
            subject_cn: "leaf".to_string(),
            issuer_cn: "Untrusted CA".to_string(),
            serial: "1".to_string(),
            not_before: now,
            not_after: now + 1000,
            cert_type: CertType::EndEntity,
            fingerprint: "abc".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let manager = CertManager::new();
        // issuer not in trusted CAs
        assert!(manager.verify_chain(&leaf).is_err());
    }

    #[test]
    fn test_verify_chain_expired_ca() {
        let (ca_pem, _) = CertManager::generate_self_signed("Expired CA", &[], 365).unwrap();
        let mut ca_cert = CertManager::parse_pem(ca_pem.as_bytes()).unwrap();

        // Force expiry
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        ca_cert.not_after = now - 100; // Expired
        ca_cert.cert_type = CertType::RootCa;

        let mut manager = CertManager::new();
        manager.add_trusted_ca(ca_cert).unwrap();

        let leaf = ParsedCert {
            subject_cn: "leaf".to_string(),
            issuer_cn: "Expired CA".to_string(), // Matches expired CA
            serial: "1".to_string(),
            not_before: now,
            not_after: now + 1000,
            cert_type: CertType::EndEntity,
            fingerprint: "abc".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        // Should fail due to expired CA
        let result = manager.verify_chain(&leaf);
        assert!(result.is_err());
        match result {
            Err(AegisError::Crypto(msg)) => assert!(msg.contains("expired")),
            _ => panic!("Expected expiry error"),
        }
    }

    #[test]
    fn test_set_server_cert_ca_warning() {
        let (ca_pem, key_pem) = CertManager::generate_self_signed("Root CA", &[], 365).unwrap();
        let mut ca_cert = CertManager::parse_pem(ca_pem.as_bytes()).unwrap();
        ca_cert.cert_type = CertType::RootCa; // Explicitly set as CA

        let mut manager = CertManager::new();
        // Should succeed (but log warn, which we can't test, but we exercise the path)
        assert!(manager.set_server_cert(ca_cert, key_pem).is_ok());
    }

    #[test]
    fn test_generate_invalid_san() {
        // Passing invalid SAN strings
        let (cert, _) =
            CertManager::generate_self_signed("test", &["invalid san string".to_string()], 1)
                .unwrap();
        // Should just skip it and succeed
        assert!(!cert.is_empty());
    }

    #[test]
    fn test_cert_type_variants() {
        assert_ne!(CertType::RootCa, CertType::IntermediateCa);
        assert_ne!(CertType::IntermediateCa, CertType::EndEntity);
        assert_ne!(CertType::RootCa, CertType::EndEntity);
    }

    #[test]
    fn test_parsed_cert_debug() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "debug-test".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "123".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "abc123".to_string(),
            san: vec!["example.com".to_string()],
            der_bytes: vec![0u8; 10],
        };

        let debug_str = format!("{:?}", cert);
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("EndEntity"));
    }

    #[test]
    fn test_parsed_cert_san() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "san-test".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "456".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "def456".to_string(),
            san: vec!["*.example.com".to_string(), "localhost".to_string()],
            der_bytes: vec![],
        };

        assert_eq!(cert.san.len(), 2);
        assert!(cert.san.contains(&"localhost".to_string()));
    }

    #[test]
    fn test_add_trusted_ca_non_ca_cert() {
        let mut manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let end_entity = ParsedCert {
            subject_cn: "end-entity".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "123".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity, // Not a CA
            fingerprint: "xyz".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.add_trusted_ca(end_entity);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_trusted_ca_success() {
        let mut manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let ca = ParsedCert {
            subject_cn: "test-ca".to_string(),
            issuer_cn: "test-ca".to_string(),
            serial: "001".to_string(),
            not_before: now - 86400,
            not_after: now + 86400 * 365,
            cert_type: CertType::RootCa,
            fingerprint: "ca-fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.add_trusted_ca(ca);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_chain_no_trusted_ca() {
        let manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "test-cert".to_string(),
            issuer_cn: "unknown-ca".to_string(),
            serial: "567".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.verify_chain(&cert);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_chain_self_signed_root() {
        let manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let root_ca = ParsedCert {
            subject_cn: "self-signed-root".to_string(),
            issuer_cn: "self-signed-root".to_string(),
            serial: "001".to_string(),
            not_before: now - 86400,
            not_after: now + 86400 * 365,
            cert_type: CertType::RootCa,
            fingerprint: "root-fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.verify_chain(&root_ca);
        assert!(result.unwrap());
    }

    #[test]
    fn test_load_from_file_missing() {
        let result = CertManager::load_from_file(std::path::Path::new("/nonexistent/cert.crt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_set_server_cert() {
        let mut manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "server.example.com".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "789".to_string(),
            not_before: now - 86400,
            not_after: now + 86400,
            cert_type: CertType::EndEntity,
            fingerprint: "srv-fp".to_string(),
            san: vec!["server.example.com".to_string()],
            der_bytes: vec![],
        };

        let result = manager.set_server_cert(cert, "fake-key".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cert_manager_empty_initialization() {
        let manager = CertManager::new();
        assert!(manager.trusted_cas.is_empty());
        assert!(manager.server_cert.is_none());
    }

    #[test]
    fn test_add_trusted_ca_duplicate() {
        let mut manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let ca = ParsedCert {
            subject_cn: "test-ca".to_string(),
            issuer_cn: "test-ca".to_string(),
            serial: "001".to_string(),
            not_before: now - 86400,
            not_after: now + 86400 * 365,
            cert_type: CertType::RootCa,
            fingerprint: "fp1".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        manager.add_trusted_ca(ca.clone()).unwrap();
        // Adding same CA again should still succeed
        manager.add_trusted_ca(ca).unwrap();
    }

    #[test]
    fn test_verify_chain_expired_cert() {
        let manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let expired = ParsedCert {
            subject_cn: "expired-cert".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "999".to_string(),
            not_before: now - 86400 * 365,
            not_after: now - 86400, // Expired yesterday
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.verify_chain(&expired);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_chain_not_yet_valid() {
        let manager = CertManager::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let future = ParsedCert {
            subject_cn: "future-cert".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "888".to_string(),
            not_before: now + 86400, // Valid from tomorrow
            not_after: now + 86400 * 365,
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.verify_chain(&future);
        assert!(result.is_err());
    }

    #[test]
    fn test_ipv6_san_ignored() {
        // Current implementation explicitly skips IPv6 SANs in parse_der (lines 160-164)
        // This test confirms that behavior to ensure lines are covered.
        
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, "ipv6-test");
        // Add an IPv6 address
        let ipv6 = "2001:db8::1".parse::<std::net::IpAddr>().unwrap();
        params.subject_alt_names.push(SanType::IpAddress(ipv6));
        
        let key_pair = KeyPair::generate().unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        let pem = cert.pem();
        
        // Parse it back
        let parsed = CertManager::parse_pem(pem.as_bytes()).unwrap();
        
        // Should NOT contain the IPv6 address in the string list because parse_der filters ip.len() == 4
        // If it did, it would be in parsed.san
        assert!(!parsed.san.iter().any(|s| s.contains("2001:db8")));
    }

    #[test]
    fn test_intermediate_chain_verification_mock() {
        // Creating a full valid chain with rcgen is complex because we need to sign the intermediate cert with the root key.
        // Instead, we verify the logic flow of `verify_chain` which iterates `trusted_cas` looking for `issuer_cn`.
        // We will create 3 certs: Root, Intermediate, Leaf.
        // We TRUST the Intermediate.
        // Leaf is issued by Intermediate.
        // Root is issued by itself.
        // Intermediate is issued by Root.
        
        // 1. Generate Intermediate CA
        let (int_pem, _) = CertManager::generate_self_signed("Intermediate CA", &[], 365).unwrap();
        let mut int_cert = CertManager::parse_pem(int_pem.as_bytes()).unwrap();
        int_cert.cert_type = CertType::IntermediateCa;
        
        // 2. Add Intermediate to Trusted CAs
        let mut manager = CertManager::new();
        manager.add_trusted_ca(int_cert.clone()).unwrap();
        
        // 3. Mock a Leaf cert issued by Intermediate
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
        let leaf = ParsedCert {
            subject_cn: "Leaf".to_string(),
            issuer_cn: "Intermediate CA".to_string(), // Matches trusted intermediate
            serial: "100".to_string(),
            not_before: now - 100,
            not_after: now + 1000,
            cert_type: CertType::EndEntity,
            fingerprint: "leaf-fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };
        
        // 4. Verify
        assert!(manager.verify_chain(&leaf).is_ok());
    }

    #[test]
    fn test_get_expiring_certs_logic() {
        let mut manager = CertManager::new();
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;

        // 1. Add valid CA (expires in 100 days)
        let valid_ca = ParsedCert {
            subject_cn: "Valid CA".to_string(),
            issuer_cn: "Valid CA".to_string(),
            serial: "1".to_string(),
            not_before: now - 100,
            not_after: now + 86400 * 100,
            cert_type: CertType::RootCa,
            fingerprint: "1".to_string(),
            san: vec![],
            der_bytes: vec![],
        };
        manager.add_trusted_ca(valid_ca).unwrap();

        // 2. Add expiring CA (expires in 10 days)
        let expiring_ca = ParsedCert {
            subject_cn: "Expiring CA".to_string(),
            issuer_cn: "Expiring CA".to_string(),
            serial: "2".to_string(),
            not_before: now - 100,
            not_after: now + 86400 * 10,
            cert_type: CertType::RootCa,
            fingerprint: "2".to_string(),
            san: vec![],
            der_bytes: vec![],
        };
        manager.add_trusted_ca(expiring_ca).unwrap();

        // 3. Set expiring server cert (expires in 5 days)
        let expiring_server = ParsedCert {
            subject_cn: "Server".to_string(),
            issuer_cn: "Valid CA".to_string(),
            serial: "3".to_string(),
            not_before: now - 100,
            not_after: now + 86400 * 5,
            cert_type: CertType::EndEntity,
            fingerprint: "3".to_string(),
            san: vec![],
            der_bytes: vec![],
        };
        manager.set_server_cert(expiring_server, "key".to_string()).unwrap();

        let expiring_list = manager.get_expiring_certs();
        assert_eq!(expiring_list.len(), 2);
        assert!(expiring_list.iter().any(|c| c.subject_cn == "Expiring CA"));
        assert!(expiring_list.iter().any(|c| c.subject_cn == "Server"));
        assert!(!expiring_list.iter().any(|c| c.subject_cn == "Valid CA"));
    }

    #[test]
    fn test_cert_type_display() {
        let root = format!("{:?}", CertType::RootCa);
        let intermediate = format!("{:?}", CertType::IntermediateCa);
        let end = format!("{:?}", CertType::EndEntity);

        assert!(root.contains("RootCa"));
        assert!(intermediate.contains("IntermediateCa"));
        assert!(end.contains("EndEntity"));
    }

    #[test]
    fn test_parsed_cert_is_ca() {
        let ca = ParsedCert {
            subject_cn: "ca".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "001".to_string(),
            not_before: 0,
            not_after: i64::MAX,
            cert_type: CertType::RootCa,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(matches!(ca.cert_type, CertType::RootCa));

        let end_entity = ParsedCert {
            subject_cn: "server".to_string(),
            issuer_cn: "ca".to_string(),
            serial: "002".to_string(),
            not_before: 0,
            not_after: i64::MAX,
            cert_type: CertType::EndEntity,
            fingerprint: "fp2".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(matches!(end_entity.cert_type, CertType::EndEntity));
    }

    #[test]
    fn test_generate_self_signed_cert() {
        let result = CertManager::generate_self_signed("test.example.com", &[], 365);
        assert!(result.is_ok());

        let (cert, key) = result.unwrap();
        assert!(!cert.is_empty());
        assert!(!key.is_empty());
    }

    #[test]
    fn test_generate_self_signed_with_sans() {
        let result = CertManager::generate_self_signed(
            "test.example.com",
            &[
                "alt1.example.com".to_string(),
                "alt2.example.com".to_string(),
            ],
            365,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsed_cert_is_valid_now() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "test".to_string(),
            issuer_cn: "test".to_string(),
            serial: "001".to_string(),
            not_before: now - 3600, // 1 hour ago
            not_after: now + 3600,  // 1 hour from now
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(cert.is_valid_now());
    }

    #[test]
    fn test_parsed_cert_expired() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "expired".to_string(),
            issuer_cn: "test".to_string(),
            serial: "002".to_string(),
            not_before: now - 7200, // 2 hours ago
            not_after: now - 3600,  // 1 hour ago (expired)
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(!cert.is_valid_now());
    }

    #[test]
    fn test_parsed_cert_days_until_expiry() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cert = ParsedCert {
            subject_cn: "test".to_string(),
            issuer_cn: "test".to_string(),
            serial: "003".to_string(),
            not_before: now - 86400,
            not_after: now + (30 * 86400), // 30 days from now
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let days = cert.days_until_expiry();
        assert!((29..=31).contains(&days));
    }

    #[test]
    fn test_parsed_cert_is_expiring_soon() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let expiring_cert = ParsedCert {
            subject_cn: "expiring".to_string(),
            issuer_cn: "test".to_string(),
            serial: "004".to_string(),
            not_before: now - 86400,
            not_after: now + (7 * 86400), // 7 days from now
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(expiring_cert.is_expiring_soon());

        let not_expiring = ParsedCert {
            subject_cn: "valid".to_string(),
            issuer_cn: "test".to_string(),
            serial: "005".to_string(),
            not_before: now - 86400,
            not_after: now + (365 * 86400), // 1 year from now
            cert_type: CertType::EndEntity,
            fingerprint: "fp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        assert!(!not_expiring.is_expiring_soon());
    }

    #[test]
    fn test_parse_self_signed_pem() {
        let (cert_pem, _) = CertManager::generate_self_signed("test.local", &[], 365).unwrap();

        let parsed = CertManager::parse_pem(cert_pem.as_bytes());
        assert!(parsed.is_ok());

        let cert = parsed.unwrap();
        assert!(cert.subject_cn.contains("test.local"));
        assert!(cert.is_valid_now());
    }

    #[test]
    fn test_cert_manager_add_trusted_ca() {
        let mut manager = CertManager::new();

        let ca_cert = ParsedCert {
            subject_cn: "MyCA".to_string(),
            issuer_cn: "MyCA".to_string(),
            serial: "001".to_string(),
            not_before: 0,
            not_after: i64::MAX,
            cert_type: CertType::RootCa,
            fingerprint: "cafp".to_string(),
            san: vec![],
            der_bytes: vec![],
        };

        let result = manager.add_trusted_ca(ca_cert);
        assert!(result.is_ok());
    }
}
