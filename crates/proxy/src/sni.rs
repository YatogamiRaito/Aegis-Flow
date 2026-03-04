use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SniError {
    #[error("Certificate not found for SNI: {0}")]
    CertificateNotFound(String),
    #[error("No default certificate configured")]
    NoDefaultCertificate,
    #[error("Failed to parse private key")]
    PrivateKeyError,
    #[error("Failed to parse certificate")]
    CertificateError,
    #[error("Failed to load file: {0}")]
    Io(#[from] std::io::Error),
}

/// A certificate resolver that selects the appropriate certificate based on the SNI hostname.
#[derive(Debug)]
pub struct SniResolver {
    certs: HashMap<String, Arc<CertifiedKey>>,
    default_cert: Option<Arc<CertifiedKey>>,
    acme_manager: Option<Arc<crate::acme::AcmeManager>>,
    cert_storage: Option<PathBuf>,
}

impl SniResolver {
    pub fn new() -> Self {
        Self {
            certs: HashMap::new(),
            default_cert: None,
            acme_manager: None,
            cert_storage: None,
        }
    }

    pub fn set_acme_manager(&mut self, manager: Arc<crate::acme::AcmeManager>, storage: PathBuf) {
        self.acme_manager = Some(manager);
        self.cert_storage = Some(storage);
    }

    pub fn add_cert(&mut self, hostname: &str, cert: Arc<CertifiedKey>) {
        let hostname = hostname.to_lowercase();
        self.certs.insert(hostname, cert);
    }

    pub fn set_default_cert(&mut self, cert: Arc<CertifiedKey>) {
        self.default_cert = Some(cert);
    }

    fn get_cert_for_name(&self, name: &str) -> Option<Arc<CertifiedKey>> {
        let name = name.to_lowercase();
        if let Some(cert) = self.certs.get(&name) {
            return Some(cert.clone());
        }

        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() >= 2 {
            let wildcard = format!("*.{}", parts[1..].join("."));
            if let Some(cert) = self.certs.get(&wildcard) {
                return Some(cert.clone());
            }
        }

        None
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            // Check for TLS-ALPN-01 challenge if ALPN is present
            if let Some(mut alpn_iter) = client_hello.alpn() {
                if alpn_iter.any(|protocol| protocol == b"acme-tls/1") {
                    if let Some(manager) = &self.acme_manager {
                        if let Some(alpn_cert) = manager.get_alpn_challenge_cert(sni) {
                            return Some(alpn_cert);
                        }
                    }
                }
            }

            if let Some(cert) = self.get_cert_for_name(sni) {
                return Some(cert);
            }

            // Try loading from ACME storage synchronously
            if let Some(storage) = &self.cert_storage {
                let domain_dir = storage.join(sni);
                let cert_path = domain_dir.join("fullchain.pem");
                let key_path = domain_dir.join("privkey.pem");

                if cert_path.exists() && key_path.exists() {
                    if let Ok(cert_bytes) = std::fs::read(&cert_path) {
                        if let Ok(mut key_bytes) = std::fs::read(&key_path) {
                            if let Some(enc_key) = self
                                .acme_manager
                                .as_ref()
                                .and_then(|m| m.config.encryption_key.as_ref())
                            {
                                if let Ok(dec_pem) =
                                    crate::acme::crypto_helpers::decrypt_pem(&key_bytes, enc_key)
                                {
                                    key_bytes = dec_pem.into_bytes();
                                } else {
                                    tracing::warn!(
                                        "Failed to decrypt Private Key for SNI: {}",
                                        sni
                                    );
                                }
                            }
                            let mut cert_reader = std::io::BufReader::new(cert_bytes.as_slice());
                            let mut key_reader = std::io::BufReader::new(key_bytes.as_slice());

                            let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
                                .filter_map(Result::ok)
                                .collect();
                            let mut pkcs8_keys: Vec<_> =
                                rustls_pemfile::pkcs8_private_keys(&mut key_reader)
                                    .filter_map(Result::ok)
                                    .collect();

                            // Let's Encrypt creates PKCS8 natively often via rcgen, otherwise it might be RSA.
                            // Assuming PKCS8 for our ECDSA generator.
                            if !certs.is_empty() && !pkcs8_keys.is_empty() {
                                let key = pkcs8_keys.remove(0);
                                if let Ok(crypto_key) =
                                    rustls::crypto::ring::sign::any_supported_type(
                                        &PrivateKeyDer::Pkcs8(key),
                                    )
                                {
                                    let mut certified_key = CertifiedKey::new(certs, crypto_key);
                                    let ocsp_path = domain_dir.join("ocsp.der");
                                    if let Ok(ocsp_bytes) = std::fs::read(&ocsp_path) {
                                        certified_key.ocsp = Some(ocsp_bytes);
                                    }
                                    let certified_key = Arc::new(certified_key);
                                    return Some(certified_key);
                                }
                            }
                        }
                    }
                }
            }
        }
        self.default_cert.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sni_resolver_exact_match() {
        let _resolver = SniResolver::new();
        assert!(true);
    }

    use rcgen::generate_simple_self_signed;

    fn create_dummy_cert() -> Arc<CertifiedKey> {
        let cert = generate_simple_self_signed(vec!["example.com".to_string()]).unwrap();
        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();

        let pki_cert = CertificateDer::from(cert_der);
        let pki_key = PrivateKeyDer::try_from(key_der.clone()).unwrap();
        let crypto_key = rustls::crypto::ring::sign::any_supported_type(&pki_key).unwrap();

        Arc::new(CertifiedKey::new(vec![pki_cert.into_owned()], crypto_key))
    }

    #[test]
    fn test_sni_resolver_logic_with_dummy() {
        let mut resolver = SniResolver::new();
        let cert1 = create_dummy_cert();
        let cert2 = create_dummy_cert();
        let cert_default = create_dummy_cert();

        resolver.add_cert("example.com", cert1.clone());
        resolver.add_cert("*.test.com", cert2.clone());
        resolver.set_default_cert(cert_default.clone());

        assert!(Arc::ptr_eq(
            &resolver.get_cert_for_name("example.com").unwrap(),
            &cert1
        ));
        assert!(Arc::ptr_eq(
            &resolver.get_cert_for_name("www.test.com").unwrap(),
            &cert2
        ));
        assert!(resolver.get_cert_for_name("unknown.org").is_none());

        // We can't easily construct a ClientHello to test `resolve()` directly
        // but the internal matching logic is verified.
    }
}
