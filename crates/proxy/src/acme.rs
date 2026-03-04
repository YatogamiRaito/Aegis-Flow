use anyhow::{Context, Result};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, NewAccount, NewOrder, OrderStatus,
};
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use rustls::sign::CertifiedKey;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, Notify};
use tracing::{debug, error, info, warn};

pub mod crypto_helpers {
    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, KeyInit},
    };
    use rand::RngCore;

    pub fn encrypt_pem(pem: &str, hex_key: &str) -> anyhow::Result<Vec<u8>> {
        let key = hex::decode(hex_key).map_err(|e| anyhow::anyhow!("Key parse err: {}", e))?;
        if key.len() != 32 {
            anyhow::bail!("Encryption key must be exactly 32 bytes (64 hex characters)");
        }
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key));
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let mut output = nonce_bytes.to_vec();
        output.extend(ciphertext);
        Ok(output)
    }

    pub fn decrypt_pem(data: &[u8], hex_key: &str) -> anyhow::Result<String> {
        let key = hex::decode(hex_key).map_err(|e| anyhow::anyhow!("Key parse err: {}", e))?;
        if key.len() != 32 {
            anyhow::bail!("Encryption key must be exactly 32 bytes (64 hex characters)");
        }
        if data.len() < 12 {
            anyhow::bail!("Encrypted data too short");
        }
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&data[0..12]);
        let plaintext = cipher
            .decrypt(nonce, &data[12..])
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        Ok(String::from_utf8(plaintext)?)
    }

    pub async fn get_cloudflare_zone(
        client: &reqwest::Client,
        api_token: &str,
        domain: &str,
    ) -> anyhow::Result<String> {
        let trimmed_domain = domain.trim_start_matches("*.");
        let mut parts: Vec<&str> = trimmed_domain.split('.').collect();
        while parts.len() >= 2 {
            let test_name = parts.join(".");
            let url = format!(
                "https://api.cloudflare.com/client/v4/zones?name={}",
                test_name
            );

            let res = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let json: serde_json::Value = res.json().await?;
            if let Some(arr) = json["result"].as_array() {
                if let Some(zone) = arr.first() {
                    if let Some(id) = zone["id"].as_str() {
                        return Ok(id.to_string());
                    }
                }
            }
            parts.remove(0);
        }
        anyhow::bail!(
            "Could not find Cloudflare zone for domain: {}",
            trimmed_domain
        );
    }
}

/// ACME Manager handles certificate ordering, renewals, and answering HTTP-01 challenges.
#[derive(Clone)]
pub struct AcmeManager {
    pub config: crate::config::AutoHttpsConfig,
    /// Pending HTTP-01 challenge tokens and their expected key authorizations
    /// Maps `token` -> `key_auth`
    http_challenges: Arc<RwLock<HashMap<String, String>>>,
    /// Prevent multiple simultaneous issuance requests for the same domain
    in_flight: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    /// Pending TLS-ALPN-01 validation certificates mapped by domain
    alpn_challenges: Arc<RwLock<HashMap<String, Arc<CertifiedKey>>>>,
}

impl std::fmt::Debug for AcmeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcmeManager")
            .field("config", &self.config)
            .finish()
    }
}

impl AcmeManager {
    pub fn new(config: crate::config::AutoHttpsConfig) -> Self {
        Self {
            config,
            http_challenges: Arc::new(RwLock::new(HashMap::new())),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            alpn_challenges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retrieve the ALPN validation certificate if one is actively staged for the domain.
    pub fn get_alpn_challenge_cert(&self, domain: &str) -> Option<Arc<CertifiedKey>> {
        if let Ok(map) = self.alpn_challenges.read() {
            map.get(domain).cloned()
        } else {
            None
        }
    }

    /// On-Demand TLS hook: guarantees a certificate is valid for the domain.
    /// Deduplicates simultaneous requests via `tokio::sync::Notify`.
    pub async fn ensure_cert(&self, domain: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let cert_dir = Self::expand_home(PathBuf::from(&self.config.cert_storage));
        let domain_dir = cert_dir.join(domain);
        let cert_file = domain_dir.join("fullchain.pem");
        let key_file = domain_dir.join("privkey.pem");

        // Localhost Local Dev Fallback
        if domain == "localhost" || domain == "127.0.0.1" {
            if !cert_file.exists() || !key_file.exists() {
                let subject_alt_names = vec![domain.to_string()];
                let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
                tokio::fs::create_dir_all(&domain_dir).await.ok();
                tokio::fs::write(&cert_file, cert.cert.pem()).await.ok();

                if let Some(enc_key) = &self.config.encryption_key {
                    if let Ok(enc_data) =
                        crypto_helpers::encrypt_pem(&cert.key_pair.serialize_pem(), enc_key)
                    {
                        tokio::fs::write(&key_file, enc_data).await.ok();
                    } else {
                        tokio::fs::write(&key_file, cert.key_pair.serialize_pem())
                            .await
                            .ok();
                    }
                } else {
                    tokio::fs::write(&key_file, cert.key_pair.serialize_pem())
                        .await
                        .ok();
                }
                info!("Generated self-signed certificate for {}", domain);
            }
            return Ok(());
        }

        // Fast path: check if cert exists and is valid
        if cert_file.exists() {
            let res: Result<Vec<u8>, std::io::Error> = tokio::fs::read(cert_file.as_path()).await;
            if let Ok(cert_bytes) = res {
                let cert_slice: &[u8] = &cert_bytes;
                if let Ok((_, pem)) = x509_parser::pem::parse_x509_pem(cert_slice) {
                    if let Ok((_, cert)) = x509_parser::parse_x509_certificate(&pem.contents) {
                        let not_after = cert.validity().not_after;
                        let expiry = std::time::SystemTime::UNIX_EPOCH
                            + std::time::Duration::from_secs(not_after.timestamp() as u64);
                        if let Ok(duration) = expiry.duration_since(std::time::SystemTime::now()) {
                            let days_left = duration.as_secs() / 86400;
                            if days_left > self.config.renew_before_days as u64 {
                                return Ok(()); // Valid, fast return
                            }
                        }
                    }
                }
            }
        }

        // Lock in-flight map
        let notify = {
            let mut flights = self.in_flight.lock().await;
            if let Some(notifier) = flights.get(domain) {
                Some(notifier.clone())
            } else {
                let notifier = Arc::new(Notify::new());
                flights.insert(domain.to_string(), notifier.clone());
                None
            }
        };

        if let Some(notifier) = notify {
            info!(
                "⏳ Waiting for in-flight ACME order to complete for: {}",
                domain
            );
            notifier.notified().await;
            return Ok(());
        }

        // We are the leader, issue the certificate
        info!("🚀 On-Demand TLS triggered for {}", domain);
        let res = self.issue_cert(vec![domain.to_string()]).await;

        // Cleanup and notify waiters
        let notifier = {
            let mut flights = self.in_flight.lock().await;
            flights.remove(domain)
        };
        if let Some(notifier) = notifier {
            notifier.notify_waiters();
        }

        res
    }

    /// Try to fulfill a let's encrypt certificate for the given domains.
    pub async fn issue_cert(&self, domains: Vec<String>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        info!(
            "Starting ACME auto-https workflow for domains: {:?}",
            domains
        );
        let contact_owned = if self.config.email.is_empty() {
            vec![]
        } else {
            vec![format!("mailto:{}", self.config.email)]
        };
        let contact: Vec<&str> = contact_owned.iter().map(|s| s.as_str()).collect();

        let ca_url = if self.config.acme_ca.is_empty() {
            instant_acme::LetsEncrypt::Production.url()
        } else {
            &self.config.acme_ca
        };

        // Initialize ACME Account
        let builder = Account::builder()
            .map_err(|e| anyhow::anyhow!("Failed to build ACME client: {}", e))?;
        let (mut account, _) = builder
            .create(
                &NewAccount {
                    contact: &contact,
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                ca_url.to_string(),
                None,
            )
            .await
            .context("Failed to create ACME account")?;

        let identifiers: Vec<Identifier> = domains
            .iter()
            .map(|name| Identifier::Dns(name.clone()))
            .collect();

        // Create an order
        let mut order = account
            .new_order(&NewOrder::new(&identifiers))
            .await
            .context("Failed to create ACME order")?;

        let state = order.state();
        if !matches!(state.status, OrderStatus::Pending | OrderStatus::Ready) {
            anyhow::bail!("Order status is {:?}", state.status);
        }

        let mut auths = order.authorizations();
        while let Some(authz_res) = auths.next().await {
            let mut authz = authz_res?;
            if authz.status == AuthorizationStatus::Valid {
                continue;
            }

            let domain = domains[0].clone();

            let has_dns01 = authz
                .challenges
                .iter()
                .any(|c| c.r#type == ChallengeType::Dns01);
            let has_alpn = authz
                .challenges
                .iter()
                .any(|c| c.r#type == ChallengeType::TlsAlpn01);
            let provider = self.config.dns_challenge.provider.to_lowercase();
            let is_cloudflare =
                provider == "cloudflare" && !self.config.dns_challenge.api_token.is_empty();

            if has_dns01 && is_cloudflare {
                let mut dns01 = authz.challenge(ChallengeType::Dns01).unwrap();
                let key_auth = dns01.key_authorization();
                let mut hasher = Sha256::new();
                hasher.update(key_auth.as_str().as_bytes());
                let hash = hasher.finalize();
                use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
                let txt_value = URL_SAFE_NO_PAD.encode(hash);

                let challenge_domain = domain.trim_start_matches("*.");
                info!(
                    "Setting Cloudflare TXT record for _acme-challenge.{} to {}",
                    challenge_domain, txt_value
                );

                let client = reqwest::Client::new();
                let zone_id = crypto_helpers::get_cloudflare_zone(
                    &client,
                    &self.config.dns_challenge.api_token,
                    challenge_domain,
                )
                .await?;

                let record_url = format!(
                    "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
                    zone_id
                );
                let payload = serde_json::json!({
                    "type": "TXT",
                    "name": format!("_acme-challenge.{}", challenge_domain),
                    "content": txt_value,
                    "ttl": 120
                });

                client
                    .post(&record_url)
                    .header(
                        "Authorization",
                        format!("Bearer {}", self.config.dns_challenge.api_token),
                    )
                    .json(&payload)
                    .send()
                    .await?;

                info!("Waiting 15s for Cloudflare DNS propagation...");
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;

                dns01.set_ready().await?;
            } else if has_alpn {
                let mut alpn = authz.challenge(ChallengeType::TlsAlpn01).unwrap();
                let key_auth = alpn.key_authorization();
                let mut hasher = Sha256::new();
                hasher.update(key_auth.as_str().as_bytes());
                let hash = hasher.finalize();

                let mut params = CertificateParams::new(vec![domain.clone()]).unwrap();
                let mut ext = rcgen::CustomExtension::from_oid_content(
                    &[1, 3, 6, 1, 5, 5, 7, 1, 31],
                    hash.to_vec(),
                );
                ext.set_criticality(true);
                params.custom_extensions.push(ext);

                let key_pair = KeyPair::generate().unwrap();
                let cert = params.self_signed(&key_pair).unwrap();

                let cert_der = rustls::pki_types::CertificateDer::from(cert.der().to_vec());
                let priv_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
                    rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der()),
                );
                let sign_key = rustls::crypto::ring::sign::any_supported_type(&priv_der).unwrap();
                let certified_key = Arc::new(CertifiedKey::new(vec![cert_der], sign_key));

                if let Ok(mut map) = self.alpn_challenges.write() {
                    map.insert(domain.clone(), certified_key);
                }
                info!("Registered TLS-ALPN-01 challenge for domain {}", domain);

                alpn.set_ready().await?;
            } else {
                let has_http01 = authz
                    .challenges
                    .iter()
                    .any(|c| c.r#type == ChallengeType::Http01);
                if has_http01 {
                    let mut http01 = authz.challenge(ChallengeType::Http01).unwrap();
                    let key_auth = http01.key_authorization();
                    let token = http01.token.clone();

                    self.http_challenges
                        .write()
                        .unwrap()
                        .insert(token.clone(), key_auth.as_str().to_string());
                    info!("Registered HTTP-01 challenge for token {}", token);

                    http01.set_ready().await?;
                } else {
                    anyhow::bail!(
                        "No supported challenge (HTTP-01 or TLS-ALPN-01) found for domain {}",
                        domain
                    );
                }
            }
        }

        use instant_acme::RetryPolicy;
        let mut policy = RetryPolicy::new().timeout(std::time::Duration::from_secs(60));
        let status = order.poll_ready(&policy).await?;

        // After polling, clear all registered challenges since they are completed
        if let Ok(mut map) = self.http_challenges.write() {
            map.clear();
        }
        if let Ok(mut map) = self.alpn_challenges.write() {
            map.clear();
        }

        if status == OrderStatus::Invalid {
            anyhow::bail!("Order became invalid during readiness check");
        }

        if status != OrderStatus::Ready && status != OrderStatus::Valid {
            anyhow::bail!("Order did not reach Ready/Valid state: {:?}", status);
        }

        // 3. Generate keypair for the certificate
        let params = CertificateParams::new(domains.clone()).unwrap();
        let key_pair = KeyPair::generate()?;

        // Finalize order with CSR
        let csr = params.serialize_request(&key_pair)?;
        order.finalize_csr(csr.der()).await?;

        // Wait for cert issuance
        let final_cert_pem = order.poll_certificate(&policy).await?;

        // Download certificates
        let cert_chain_pem = final_cert_pem;
        let private_key_pem = key_pair.serialize_pem();

        // 4. Persistence
        let domain_dir = PathBuf::from(&self.config.cert_storage).join(&domains[0]);
        // handle `~` expansion
        let domain_dir = Self::expand_home(domain_dir);

        if !domain_dir.exists() {
            tokio::fs::create_dir_all(&domain_dir).await?;
        }

        let cert_path = domain_dir.join("fullchain.pem");
        let key_path = domain_dir.join("privkey.pem");

        tokio::fs::write(&cert_path, &cert_chain_pem).await?;

        if let Some(enc_key) = &self.config.encryption_key {
            let enc_data = crypto_helpers::encrypt_pem(&private_key_pem, enc_key)?;
            tokio::fs::write(&key_path, enc_data).await?;
        } else {
            tokio::fs::write(&key_path, private_key_pem).await?;
        }

        // Fetch OCSP Staple right after issuance
        if self.config.enabled {
            if let Ok(ocsp_bytes) = self.fetch_ocsp_staple(&cert_chain_pem).await {
                let ocsp_path = domain_dir.join("ocsp.der");
                tokio::fs::write(&ocsp_path, ocsp_bytes).await.ok();
            }
        }

        info!(
            "Successfully issued and persisted certificate for {:?}",
            domains
        );

        Ok(())
    }

    /// Check if a request path matches an active HTTP-01 challenge.
    pub fn check_http_challenge(&self, path: &str) -> Option<String> {
        let prefix = "/.well-known/acme-challenge/";
        if path.starts_with(prefix) {
            let token = &path[prefix.len()..];
            if let Ok(guards) = self.http_challenges.read() {
                return guards.get(token).cloned();
            }
        }
        None
    }

    pub fn expand_home(path: PathBuf) -> PathBuf {
        if let Some(path_str) = path.to_str() {
            let p_str: &str = path_str;
            if p_str.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    return PathBuf::from(home).join(&path_str[2..]);
                }
            }
        }
        path
    }

    /// Fetches an OCSP staple for a given PEM certificate chain (leaf + issuer)
    pub async fn fetch_ocsp_staple(
        &self,
        cert_chain_pem: &str,
    ) -> std::result::Result<Vec<u8>, anyhow::Error> {
        use sha1::Sha1;
        use x509_cert::Certificate;
        use x509_cert::der::{Decode, Encode};
        use x509_ocsp::Request;
        use x509_ocsp::builder::*;
        use x509_parser::prelude::*;

        let mut pems = Vec::new();
        let mut rem = cert_chain_pem.as_bytes();
        while let Ok((next_rem, pem)) = x509_parser::pem::parse_x509_pem(rem) {
            pems.push(pem);
            rem = next_rem;
            if rem.is_empty() {
                break;
            }
        }

        if pems.len() < 2 {
            anyhow::bail!("Certificate chain must contain at least leaf and issuer to fetch OCSP");
        }

        let leaf_der = &pems[0].contents;
        let issuer_der = &pems[1].contents;

        let (_, leaf_cert) = X509Certificate::from_der(leaf_der)
            .map_err(|e| anyhow::anyhow!("X509 leaf parsing failed: {:?}", e))?;

        // Extract AIA extension for OCSP responder URL
        let mut ocsp_url = None;
        if let Ok(Some(ext)) =
            leaf_cert.get_extension_unique(&oid_registry::OID_PKIX_AUTHORITY_INFO_ACCESS)
        {
            if let ParsedExtension::AuthorityInfoAccess(aia) = ext.parsed_extension() {
                for access in &aia.accessdescs {
                    if access.access_method.to_id_string() == "1.3.6.1.5.5.7.48.1" {
                        if let x509_parser::prelude::GeneralName::URI(uri) = &access.access_location
                        {
                            ocsp_url = Some(uri.to_string());
                        }
                    }
                }
            }
        }

        let responder_url = ocsp_url
            .ok_or_else(|| anyhow::anyhow!("No OCSP Responder URL found in AIA extension"))?;

        // Build the actual OCSP Request using x509-ocsp
        let issuer_cert = Certificate::from_der(issuer_der)
            .map_err(|e| anyhow::anyhow!("Failed to parse issuer der for x509-ocsp: {}", e))?;

        let serial_bytes = leaf_cert.raw_serial();
        let serial = x509_cert::serial_number::SerialNumber::new(serial_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid serial: {}", e))?;

        let ocsp_req = OcspRequestBuilder::default()
            .with_request(
                Request::from_issuer::<Sha1>(&issuer_cert, serial)
                    .map_err(|e| anyhow::anyhow!("Failed to construct CertID: {}", e))?,
            )
            .build();

        let req_der = ocsp_req
            .to_der()
            .map_err(|e| anyhow::anyhow!("Failed to encode OcspRequest: {}", e))?;

        // Send HTTP POST
        let client = reqwest::Client::new();
        let res = client
            .post(&responder_url)
            .header("Content-Type", "application/ocsp-request")
            .body(req_der)
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("OCSP Responder returned HTTP {}", res.status());
        }

        let bytes = res.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Spawns a background task that checks and renews certificates automatically daily
    pub fn start_background_renewal(self: Arc<Self>) {
        if !self.config.enabled {
            return;
        }

        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600 * 24)); // Run once a day
            loop {
                interval.tick().await;
                if let Err(e) = manager.check_renewals().await {
                    error!("ACME Background renewal check failed: {}", e);
                }
            }
        });
    }

    /// Checks the `cert_storage` for expiring certificates and renews them
    async fn check_renewals(&self) -> Result<()> {
        let storage = Self::expand_home(PathBuf::from(&self.config.cert_storage));
        if !storage.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(storage.as_path()).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let domain = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let cert_file = path.join("fullchain.pem");
                if cert_file.exists() {
                    let cert_bytes: Vec<u8> = tokio::fs::read(cert_file.as_path()).await?;
                    let cert_slice: &[u8] = &cert_bytes;
                    if let Ok((_, pem)) = x509_parser::pem::parse_x509_pem(cert_slice) {
                        if let Ok((_, cert)) = x509_parser::parse_x509_certificate(&pem.contents) {
                            let not_after = cert.validity().not_after;
                            let expiry_system_time = std::time::SystemTime::UNIX_EPOCH
                                + std::time::Duration::from_secs(not_after.timestamp() as u64);
                            let now = std::time::SystemTime::now();
                            if let Ok(duration) = expiry_system_time.duration_since(now) {
                                let days_left = duration.as_secs() / 86400;
                                if days_left <= self.config.renew_before_days as u64 {
                                    info!(
                                        "Certificate for {} expires in {} days, renewing...",
                                        domain, days_left
                                    );
                                    if let Err(e) = self.issue_cert(vec![domain.clone()]).await {
                                        error!("Failed to renew certificate for {}: {}", domain, e);
                                    }
                                } else {
                                    debug!(
                                        "Certificate for {} is valid for {} more days",
                                        domain, days_left
                                    );
                                }
                            } else {
                                // Already expired
                                info!("Certificate for {} is already expired, renewing...", domain);
                                if let Err(e) = self.issue_cert(vec![domain.clone()]).await {
                                    error!(
                                        "Failed to renew expired certificate for {}: {}",
                                        domain, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rcgen_alpn() {
        let domain = "example.com";
        let hash = vec![1, 2, 3, 4];
        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()]).unwrap();
        let mut ext = rcgen::CustomExtension::from_oid_content(&[1, 3, 6, 1, 5, 5, 7, 1, 31], hash);
        ext.set_criticality(true);
        params.custom_extensions.push(ext);

        let key_pair = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        assert!(!cert.pem().is_empty());
    }
}
