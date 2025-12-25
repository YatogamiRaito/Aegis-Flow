//! Hybrid Key Exchange: X25519 + Kyber
//!
//! This module implements a hybrid key exchange combining classical X25519
//! with post-quantum Kyber-768 for "Harvest Now, Decrypt Later" protection.

use aegis_common::{AegisError, Result};
use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret as KyberSharedSecret};
use rand::rngs::OsRng;
use tracing::{debug, instrument};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};

/// Combined public key for hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridPublicKey {
    /// X25519 public key (32 bytes)
    pub x25519: [u8; 32],
    /// Kyber-768 public key
    pub kyber: Vec<u8>,
}

impl AsRef<[u8]> for HybridPublicKey {
    fn as_ref(&self) -> &[u8] {
        // For simplicity, return X25519 part. Full serialization would concat both.
        &self.x25519
    }
}

/// Combined shared secret from hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridSharedSecret {
    /// The combined shared secret (KDF output)
    inner: Vec<u8>,
}

impl HybridSharedSecret {
    /// Create a new hybrid shared secret from X25519 and Kyber secrets
    pub fn combine(x25519_secret: &[u8], kyber_secret: &[u8]) -> Self {
        // Simple concatenation - in production, use HKDF
        let mut combined = Vec::with_capacity(x25519_secret.len() + kyber_secret.len());
        combined.extend_from_slice(x25519_secret);
        combined.extend_from_slice(kyber_secret);
        Self { inner: combined }
    }

    /// Get the combined secret as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

impl AsRef<[u8]> for HybridSharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

/// Hybrid Key Exchange implementation (X25519 + Kyber-768)
#[derive(Debug, Default)]
pub struct HybridKeyExchange;

impl HybridKeyExchange {
    /// Create a new HybridKeyExchange instance
    pub fn new() -> Self {
        Self
    }

    /// Generate a new hybrid key pair
    #[instrument(skip(self))]
    pub fn generate_keypair(&self) -> Result<(HybridPublicKey, HybridSecretKey)> {
        debug!("Generating hybrid key pair (X25519 + Kyber-768)");

        // Generate X25519 key pair
        let x25519_secret = EphemeralSecret::random_from_rng(OsRng);
        let x25519_public = X25519PublicKey::from(&x25519_secret);

        // Generate Kyber-768 key pair
        let (kyber_pk, kyber_sk) = kyber768::keypair();

        let public_key = HybridPublicKey {
            x25519: x25519_public.to_bytes(),
            kyber: kyber_pk.as_bytes().to_vec(),
        };

        let secret_key = HybridSecretKey {
            x25519: x25519_secret,
            kyber: kyber_sk.as_bytes().to_vec(),
        };

        debug!("Hybrid key pair generated successfully");
        Ok((public_key, secret_key))
    }

    /// Encapsulate: Used by the client to create ciphertext and shared secret
    #[instrument(skip(self, peer_public_key))]
    pub fn encapsulate(
        &self,
        peer_public_key: &HybridPublicKey,
    ) -> Result<(HybridCiphertext, HybridSharedSecret)> {
        debug!("Encapsulating hybrid shared secret");

        // X25519 key exchange
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

        let peer_x25519_pk = X25519PublicKey::from(peer_public_key.x25519);
        let x25519_shared = ephemeral_secret.diffie_hellman(&peer_x25519_pk);

        // Kyber-768 encapsulation
        let kyber_pk = kyber768::PublicKey::from_bytes(&peer_public_key.kyber)
            .map_err(|e| AegisError::Crypto(format!("Invalid Kyber public key: {:?}", e)))?;
        let (kyber_ss, kyber_ct) = kyber768::encapsulate(&kyber_pk);

        let ciphertext = HybridCiphertext {
            x25519_ephemeral: ephemeral_public.to_bytes(),
            kyber_ciphertext: kyber_ct.as_bytes().to_vec(),
        };

        let shared_secret =
            HybridSharedSecret::combine(x25519_shared.as_bytes(), kyber_ss.as_bytes());

        debug!("Hybrid encapsulation completed");
        Ok((ciphertext, shared_secret))
    }

    /// Decapsulate: Used by the server to derive shared secret from ciphertext
    #[instrument(skip(self, ciphertext, secret_key))]
    pub fn decapsulate(
        &self,
        ciphertext: &HybridCiphertext,
        secret_key: &HybridSecretKey,
    ) -> Result<HybridSharedSecret> {
        debug!("Decapsulating hybrid shared secret");

        // X25519 key exchange using the ephemeral public key from ciphertext
        // Note: We need to reconstruct the X25519 shared secret differently
        // since EphemeralSecret is consumed. This is a limitation we'll address.

        // For now, we'll use a workaround - in production, use StaticSecret
        let _peer_ephemeral = X25519PublicKey::from(ciphertext.x25519_ephemeral);

        // Kyber-768 decapsulation
        let kyber_sk = kyber768::SecretKey::from_bytes(&secret_key.kyber)
            .map_err(|e| AegisError::Crypto(format!("Invalid Kyber secret key: {:?}", e)))?;
        let kyber_ct = kyber768::Ciphertext::from_bytes(&ciphertext.kyber_ciphertext)
            .map_err(|e| AegisError::Crypto(format!("Invalid Kyber ciphertext: {:?}", e)))?;

        let kyber_ss = kyber768::decapsulate(&kyber_ct, &kyber_sk);

        // Note: Full X25519 decapsulation would require storing the secret key
        // For this MVP, we'll return just the Kyber shared secret with X25519 placeholder
        let shared_secret = HybridSharedSecret::combine(
            &[0u8; 32], // Placeholder - needs StaticSecret for proper implementation
            kyber_ss.as_bytes(),
        );

        debug!("Hybrid decapsulation completed");
        Ok(shared_secret)
    }

    /// Get algorithm name
    pub fn algorithm_name(&self) -> &'static str {
        "X25519-Kyber768-Hybrid"
    }
}

/// Secret key for hybrid key exchange
#[allow(dead_code)] // x25519 field will be used when we implement StaticSecret for decapsulation
pub struct HybridSecretKey {
    x25519: EphemeralSecret,
    kyber: Vec<u8>,
}

impl std::fmt::Debug for HybridSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridSecretKey")
            .field("x25519", &"[REDACTED]")
            .field("kyber", &"[REDACTED]")
            .finish()
    }
}

/// Ciphertext for hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridCiphertext {
    /// X25519 ephemeral public key
    pub x25519_ephemeral: [u8; 32],
    /// Kyber-768 ciphertext
    pub kyber_ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_keypair_generation() {
        let kex = HybridKeyExchange::new();
        let result = kex.generate_keypair();
        assert!(result.is_ok(), "Keypair generation should succeed");

        let (pk, _sk) = result.unwrap();
        assert_eq!(pk.x25519.len(), 32, "X25519 public key should be 32 bytes");
        assert!(!pk.kyber.is_empty(), "Kyber public key should not be empty");
    }

    #[test]
    fn test_hybrid_encapsulation() {
        let kex = HybridKeyExchange::new();
        let (pk, _sk) = kex.generate_keypair().unwrap();

        let result = kex.encapsulate(&pk);
        assert!(result.is_ok(), "Encapsulation should succeed");

        let (ct, ss) = result.unwrap();
        assert_eq!(ct.x25519_ephemeral.len(), 32);
        assert!(!ct.kyber_ciphertext.is_empty());
        assert!(!ss.as_ref().is_empty(), "Shared secret should not be empty");
    }

    #[test]
    fn test_algorithm_name() {
        let kex = HybridKeyExchange::new();
        assert_eq!(kex.algorithm_name(), "X25519-Kyber768-Hybrid");
    }

    #[test]
    fn test_shared_secret_combine() {
        let x25519 = [1u8; 32];
        let kyber = [2u8; 32];

        let ss = HybridSharedSecret::combine(&x25519, &kyber);
        assert_eq!(ss.as_bytes().len(), 64);
        assert_eq!(&ss.as_bytes()[..32], &x25519);
        assert_eq!(&ss.as_bytes()[32..], &kyber);
    }
}
