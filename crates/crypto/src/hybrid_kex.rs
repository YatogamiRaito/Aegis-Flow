//! Hybrid Key Exchange: X25519 + Kyber
//!
//! This module implements a hybrid key exchange combining classical X25519
//! with post-quantum Kyber-768 for "Harvest Now, Decrypt Later" protection.
//!
//! # RFC Reference
//! See docs/rfcs/RFC-001-hybrid-kex.md for design details.

use aegis_common::{AegisError, Result};
use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret as KyberSharedSecret};
use rand::rngs::OsRng;
use tracing::{debug, instrument};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

/// Combined public key for hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridPublicKey {
    /// X25519 public key (32 bytes)
    pub x25519: [u8; 32],
    /// Kyber-768 public key
    pub kyber: Vec<u8>,
}

impl HybridPublicKey {
    /// Serialize the hybrid public key to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.kyber.len());
        bytes.extend_from_slice(&self.x25519);
        bytes.extend_from_slice(&self.kyber);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(AegisError::Crypto("Public key too short".to_string()));
        }
        let mut x25519 = [0u8; 32];
        x25519.copy_from_slice(&bytes[..32]);
        let kyber = bytes[32..].to_vec();

        Ok(Self { x25519, kyber })
    }
}

impl AsRef<[u8]> for HybridPublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.x25519
    }
}

/// Combined shared secret from hybrid key exchange
#[derive(Clone)]
pub struct HybridSharedSecret {
    /// The combined shared secret (KDF output)
    inner: [u8; 64],
}

impl HybridSharedSecret {
    /// Create a new hybrid shared secret from X25519 and Kyber secrets
    pub fn combine(x25519_secret: &[u8; 32], kyber_secret: &[u8]) -> Self {
        let mut inner = [0u8; 64];
        inner[..32].copy_from_slice(x25519_secret);

        // Kyber shared secret is 32 bytes
        let kyber_len = kyber_secret.len().min(32);
        inner[32..32 + kyber_len].copy_from_slice(&kyber_secret[..kyber_len]);

        Self { inner }
    }

    /// Get the combined secret as bytes
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.inner
    }

    /// Derive a 32-byte key from the hybrid secret
    /// In production, use HKDF-SHA256
    pub fn derive_key(&self) -> [u8; 32] {
        // Simple XOR-based derivation for MVP
        // TODO: Replace with HKDF-SHA256
        let mut key = [0u8; 32];
        for (i, k) in key.iter_mut().enumerate() {
            *k = self.inner[i] ^ self.inner[i + 32];
        }
        key
    }
}

impl std::fmt::Debug for HybridSharedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridSharedSecret")
            .field("inner", &"[REDACTED]")
            .finish()
    }
}

impl AsRef<[u8]> for HybridSharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl Drop for HybridSharedSecret {
    fn drop(&mut self) {
        // Zeroize on drop for security
        self.inner.iter_mut().for_each(|b| *b = 0);
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

        // Generate X25519 key pair using StaticSecret (reusable)
        let x25519_secret = X25519StaticSecret::random_from_rng(OsRng);
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

        // X25519 key exchange - generate ephemeral keypair
        let ephemeral_secret = X25519StaticSecret::random_from_rng(OsRng);
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
        let peer_ephemeral = X25519PublicKey::from(ciphertext.x25519_ephemeral);
        let x25519_shared = secret_key.x25519.diffie_hellman(&peer_ephemeral);

        // Kyber-768 decapsulation
        let kyber_sk = kyber768::SecretKey::from_bytes(&secret_key.kyber)
            .map_err(|e| AegisError::Crypto(format!("Invalid Kyber secret key: {:?}", e)))?;
        let kyber_ct = kyber768::Ciphertext::from_bytes(&ciphertext.kyber_ciphertext)
            .map_err(|e| AegisError::Crypto(format!("Invalid Kyber ciphertext: {:?}", e)))?;

        let kyber_ss = kyber768::decapsulate(&kyber_ct, &kyber_sk);

        let shared_secret =
            HybridSharedSecret::combine(x25519_shared.as_bytes(), kyber_ss.as_bytes());

        debug!("Hybrid decapsulation completed");
        Ok(shared_secret)
    }

    /// Get algorithm name
    pub fn algorithm_name(&self) -> &'static str {
        "X25519-Kyber768-Hybrid"
    }
}

/// Secret key for hybrid key exchange
pub struct HybridSecretKey {
    x25519: X25519StaticSecret,
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

impl HybridCiphertext {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.kyber_ciphertext.len());
        bytes.extend_from_slice(&self.x25519_ephemeral);
        bytes.extend_from_slice(&self.kyber_ciphertext);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(AegisError::Crypto("Ciphertext too short".to_string()));
        }
        let mut x25519_ephemeral = [0u8; 32];
        x25519_ephemeral.copy_from_slice(&bytes[..32]);
        let kyber_ciphertext = bytes[32..].to_vec();

        Ok(Self {
            x25519_ephemeral,
            kyber_ciphertext,
        })
    }
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
    fn test_full_key_exchange_roundtrip() {
        let kex = HybridKeyExchange::new();

        // Server generates keypair
        let (server_pk, server_sk) = kex.generate_keypair().unwrap();

        // Client encapsulates
        let (ciphertext, client_ss) = kex.encapsulate(&server_pk).unwrap();

        // Server decapsulates
        let server_ss = kex.decapsulate(&ciphertext, &server_sk).unwrap();

        // Both should derive the same shared secret
        assert_eq!(
            client_ss.as_bytes(),
            server_ss.as_bytes(),
            "Client and server should derive the same shared secret"
        );
    }

    #[test]
    fn test_derive_key() {
        let kex = HybridKeyExchange::new();
        let (pk, sk) = kex.generate_keypair().unwrap();
        let (ct, client_ss) = kex.encapsulate(&pk).unwrap();
        let server_ss = kex.decapsulate(&ct, &sk).unwrap();

        let client_key = client_ss.derive_key();
        let server_key = server_ss.derive_key();

        assert_eq!(client_key, server_key, "Derived keys should match");
        assert_ne!(client_key, [0u8; 32], "Derived key should not be all zeros");
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

    #[test]
    fn test_public_key_serialization() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();

        let bytes = pk.to_bytes();
        let pk2 = HybridPublicKey::from_bytes(&bytes).unwrap();

        assert_eq!(pk.x25519, pk2.x25519);
        assert_eq!(pk.kyber, pk2.kyber);
    }

    #[test]
    fn test_ciphertext_serialization() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let (ct, _) = kex.encapsulate(&pk).unwrap();

        let bytes = ct.to_bytes();
        let ct2 = HybridCiphertext::from_bytes(&bytes).unwrap();

        assert_eq!(ct.x25519_ephemeral, ct2.x25519_ephemeral);
        assert_eq!(ct.kyber_ciphertext, ct2.kyber_ciphertext);
    }
}
