use ring::signature::KeyPair;
use ring::{rand, signature};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Jwk {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub y: String,
}

pub struct EcdsaKey {
    key_pair: signature::EcdsaKeyPair,
}

impl EcdsaKey {
    pub fn generate() -> Self {
        let rng = rand::SystemRandom::new();
        let pkcs8 = signature::EcdsaKeyPair::generate_pkcs8(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &rng,
        )
        .unwrap();
        let key_pair = signature::EcdsaKeyPair::from_pkcs8(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();
        Self { key_pair }
    }

    pub fn jwk(&self) -> Jwk {
        let public_key = self.key_pair.public_key();
        let bytes = public_key.as_ref();

        // ECDSA P-256 public key uncompressed format: 0x04 || X (32 bytes) || Y (32 bytes)
        assert_eq!(bytes[0], 0x04);
        let x = &bytes[1..33];
        let y = &bytes[33..65];

        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        Jwk {
            kty: "EC".to_string(),
            crv: "P-256".to_string(),
            x: URL_SAFE_NO_PAD.encode(x),
            y: URL_SAFE_NO_PAD.encode(y),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let rng = rand::SystemRandom::new();
        self.key_pair.sign(&rng, message).unwrap().as_ref().to_vec()
    }
}

pub fn sign_jws_with_jwk(
    key: &EcdsaKey,
    payload: &str,
    nonce: &str,
    url: &str,
) -> serde_json::Value {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    let jwk = key.jwk();
    let protected_header = serde_json::json!({
        "alg": "ES256",
        "jwk": jwk,
        "nonce": nonce,
        "url": url,
    });

    let protected_str = serde_json::to_string(&protected_header).unwrap();
    let protected_b64 = URL_SAFE_NO_PAD.encode(protected_str);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);

    let signing_input = format!("{}.{}", protected_b64, payload_b64);
    let signature = key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature);

    serde_json::json!({
        "protected": protected_b64,
        "payload": payload_b64,
        "signature": sig_b64
    })
}

pub fn sign_jws_with_kid(
    key: &EcdsaKey,
    kid: &str,
    payload: &str,
    nonce: &str,
    url: &str,
) -> serde_json::Value {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    let protected_header = serde_json::json!({
        "alg": "ES256",
        "kid": kid,
        "nonce": nonce,
        "url": url,
    });

    let protected_str = serde_json::to_string(&protected_header).unwrap();
    let protected_b64 = URL_SAFE_NO_PAD.encode(protected_str);
    let payload_b64 = if payload.is_empty() {
        String::new()
    } else {
        URL_SAFE_NO_PAD.encode(payload)
    };

    let signing_input = format!("{}.{}", protected_b64, payload_b64);
    let signature = key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature);

    serde_json::json!({
        "protected": protected_b64,
        "payload": payload_b64,
        "signature": sig_b64
    })
}

#[derive(Debug, Deserialize)]
pub struct AcmeDirectory {
    #[serde(rename = "newNonce")]
    pub new_nonce: String,
    #[serde(rename = "newAccount")]
    pub new_account: String,
    #[serde(rename = "newOrder")]
    pub new_order: String,
    #[serde(rename = "revokeCert")]
    pub revoke_cert: Option<String>,
    #[serde(rename = "keyChange")]
    pub key_change: Option<String>,
}

pub struct AcmeClient {
    client: reqwest::Client,
    pub directory: Option<AcmeDirectory>,
    pub account_key: EcdsaKey,
    pub kid: Option<String>,
    pub directory_url: String, // E.g. https://acme-staging-v02.api.letsencrypt.org/directory
    cached_nonce: Option<String>,
}

impl AcmeClient {
    pub fn new(directory_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            directory: None,
            account_key: EcdsaKey::generate(),
            kid: None,
            directory_url: directory_url.to_string(),
            cached_nonce: None,
        }
    }

    pub async fn fetch_directory(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self.client.get(&self.directory_url).send().await?;
        let dir: AcmeDirectory = resp.json().await?;
        self.directory = Some(dir);
        Ok(())
    }

    pub async fn get_nonce(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(n) = self.cached_nonce.take() {
            return Ok(n);
        }

        // Fetch new
        let dir = self.directory.as_ref().ok_or("Directory not loaded")?;
        let resp = self.client.head(&dir.new_nonce).send().await?;

        let nonce = resp
            .headers()
            .get("replay-nonce")
            .ok_or("No replay-nonce header")?
            .to_str()?
            .to_string();

        Ok(nonce)
    }

    pub async fn register_account(
        &mut self,
        emails: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract strings we need before any mutable borrows
        let new_account_url = self
            .directory
            .as_ref()
            .ok_or("Directory not loaded")?
            .new_account
            .clone();

        let nonce = self.get_nonce().await?;

        let contacts: Vec<String> = emails.iter().map(|e| format!("mailto:{}", e)).collect();
        let payload = serde_json::json!({
            "termsOfServiceAgreed": true,
            "contact": contacts,
        })
        .to_string();

        let jws = sign_jws_with_jwk(&self.account_key, &payload, &nonce, &new_account_url);

        let resp: reqwest::Response = self
            .client
            .post(&new_account_url)
            .header("Content-Type", "application/jose+json")
            .json(&jws)
            .send()
            .await?;

        if let Some(new_nonce) = resp.headers().get("replay-nonce") {
            self.cached_nonce = Some(new_nonce.to_str()?.to_string());
        }

        // The URL of the account is returned in the Location header
        if let Some(loc) = resp.headers().get("location") {
            self.kid = Some(loc.to_str()?.to_string());
        } else {
            return Err("Account registration didn't return a Location header".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecdsa_key_gen() {
        let key = EcdsaKey::generate();
        let jwk = key.jwk();
        assert_eq!(jwk.kty, "EC");
        assert_eq!(jwk.crv, "P-256");
        assert!(!jwk.x.is_empty());
        assert!(!jwk.y.is_empty());
    }

    #[test]
    fn test_jws_signing() {
        let key = EcdsaKey::generate();
        let payload = r#"{"test":"data"}"#;
        let jws = sign_jws_with_jwk(
            &key,
            payload,
            "nonce123",
            "https://example.com/acme/new-acct",
        );

        assert!(jws.get("protected").is_some());
        assert!(jws.get("payload").is_some());
        assert!(jws.get("signature").is_some());
    }

    #[test]
    fn test_acme_client_init() {
        let client = AcmeClient::new("https://example.com/dir");
        assert_eq!(client.directory_url, "https://example.com/dir");
        assert!(client.directory.is_none());
    }

    // Note: Integration tests with a real/mocked HTTP server would be needed
    // to test fetch_directory, get_nonce, and register_account fully.
}
