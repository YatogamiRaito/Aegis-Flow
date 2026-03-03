use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// HTTP-01 challenge: serve a key authorization at /.well-known/acme-challenge/<token>
pub struct Http01Challenge {
    pub token: String,
    pub key_authorization: String,
}

impl Http01Challenge {
    pub fn new(token: &str, jwk_thumbprint: &str) -> Self {
        let key_authorization = format!("{}.{}", token, jwk_thumbprint);
        Self {
            token: token.to_string(),
            key_authorization,
        }
    }

    pub fn serve_path(&self) -> String {
        format!("/.well-known/acme-challenge/{}", self.token)
    }
}

/// DNS-01 challenge: add a TXT record at _acme-challenge.<domain>
pub struct Dns01Challenge {
    pub domain: String,
    pub record_name: String,
    pub txt_value: String,
}

impl Dns01Challenge {
    pub fn new(domain: &str, key_authorization: &str) -> Self {
        // TXT value = base64url(sha256(key_authorization))
        let mut hasher = Sha256::new();
        hasher.update(key_authorization.as_bytes());
        let digest = hasher.finalize();
        let txt_value = URL_SAFE_NO_PAD.encode(digest);

        let record_name = format!("_acme-challenge.{}", domain);

        Self {
            domain: domain.to_string(),
            record_name,
            txt_value,
        }
    }
}

/// Compute JWK thumbprint from a JWK (for HTTP-01 key authorization)
pub fn compute_jwk_thumbprint(jwk: &crate::acme::Jwk) -> String {
    // Canonical JSON with sorted keys: {"crv":"...","kty":"...","x":"...","y":"..."}
    let canonical = format!(
        r#"{{"crv":"{}","kty":"{}","x":"{}","y":"{}"}}"#,
        jwk.crv, jwk.kty, jwk.x, jwk.y
    );

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acme::{EcdsaKey, Jwk};

    #[test]
    fn test_http01_challenge_path() {
        let ch = Http01Challenge::new("my_token", "my_thumbprint");
        assert_eq!(ch.serve_path(), "/.well-known/acme-challenge/my_token");
        assert_eq!(ch.key_authorization, "my_token.my_thumbprint");
    }

    #[test]
    fn test_dns01_challenge_record() {
        let ch = Dns01Challenge::new("example.com", "some_key_auth");
        assert_eq!(ch.record_name, "_acme-challenge.example.com");
        assert!(!ch.txt_value.is_empty());
    }

    #[test]
    fn test_jwk_thumbprint() {
        let key = EcdsaKey::generate();
        let jwk = key.jwk();
        let thumbprint = compute_jwk_thumbprint(&jwk);
        assert!(!thumbprint.is_empty());
    }
}
