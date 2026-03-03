use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm, TokenData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    // Add other generic claims here as needed
}

pub struct JwtConfig {
    pub keys: Vec<DecodingKey>,
    pub validation: Validation,
}

impl JwtConfig {
    pub fn new(secret: &[u8], is_base64: bool) -> Self {
        let key = if is_base64 {
            DecodingKey::from_base64_secret(std::str::from_utf8(secret).unwrap()).unwrap_or_else(|_| DecodingKey::from_secret(secret))
        } else {
            DecodingKey::from_secret(secret)
        };
        
        Self {
            keys: vec![key],
            validation: Validation::new(Algorithm::HS256),
        }
    }

    pub fn set_algorithm(&mut self, alg: Algorithm) {
        self.validation = Validation::new(alg);
    }
    
    pub fn verify_token(&self, token: &str) -> Option<TokenData<Claims>> {
        for key in &self.keys {
            if let Ok(token_data) = decode::<Claims>(token, key, &self.validation) {
                return Some(token_data);
            }
        }
        None
    }

    pub fn extract_bearer<B>(req: &hyper::Request<B>) -> Option<String> {
        if let Some(auth_header) = req.headers().get(hyper::header::AUTHORIZATION) {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    return Some(auth_str[7..].to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_token(secret: &[u8], exp_offset: i64) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Handle negative offsets safely
        let exp = if exp_offset < 0 {
            now.saturating_sub(exp_offset.unsigned_abs() as u64)
        } else {
            now.saturating_add(exp_offset as u64)
        };

        let my_claims = Claims {
            sub: "b@b.com".to_owned(),
            exp: exp as usize,
        };

        encode(&Header::default(), &my_claims, &EncodingKey::from_secret(secret)).unwrap()
    }

    #[test]
    fn test_jwt_verify_success() {
        let secret = b"super_secret_key";
        let config = JwtConfig::new(secret, false);
        
        let valid_token = create_token(secret, 3600); // 1 hr future
        assert!(config.verify_token(&valid_token).is_some());
    }

    #[test]
    fn test_jwt_verify_expired() {
        let secret = b"super_secret_key";
        let config = JwtConfig::new(secret, false);
        
        let expired_token = create_token(secret, -3600); // 1 hr past
        assert!(config.verify_token(&expired_token).is_none());
    }

    #[test]
    fn test_jwt_verify_invalid_signature() {
        let secret1 = b"super_secret_key";
        let secret2 = b"wrong_key";
        
        let config = JwtConfig::new(secret2, false);
        
        let token = create_token(secret1, 3600); 
        assert!(config.verify_token(&token).is_none());
    }
}
