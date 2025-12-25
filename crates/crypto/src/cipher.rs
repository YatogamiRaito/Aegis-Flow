//! Symmetric Encryption Module
//!
//! Provides AES-256-GCM and ChaCha20-Poly1305 encryption for secure data transfer.

use aegis_common::{AegisError, Result};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use chacha20poly1305::ChaCha20Poly1305;
use hkdf::Hkdf;
use sha2::Sha256;
use std::sync::atomic::{AtomicU64, Ordering};

/// Cipher algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherAlgorithm {
    /// AES-256-GCM (recommended for hardware with AES-NI)
    Aes256Gcm,
    /// ChaCha20-Poly1305 (recommended for software-only)
    ChaCha20Poly1305,
}

impl Default for CipherAlgorithm {
    fn default() -> Self {
        Self::Aes256Gcm
    }
}

/// Encryption key derived from shared secret
#[derive(Clone)]
pub struct EncryptionKey {
    key: [u8; 32],
    algorithm: CipherAlgorithm,
}

impl EncryptionKey {
    /// Derive encryption key from shared secret using HKDF-SHA256
    pub fn derive(shared_secret: &[u8], info: &[u8], algorithm: CipherAlgorithm) -> Result<Self> {
        let hk = Hkdf::<Sha256>::new(None, shared_secret);
        let mut key = [0u8; 32];
        hk.expand(info, &mut key)
            .map_err(|_| AegisError::Crypto("HKDF expansion failed".to_string()))?;

        Ok(Self { key, algorithm })
    }

    /// Create from raw key bytes (for testing)
    pub fn from_raw(key: [u8; 32], algorithm: CipherAlgorithm) -> Self {
        Self { key, algorithm }
    }

    /// Get the algorithm
    pub fn algorithm(&self) -> CipherAlgorithm {
        self.algorithm
    }
}

impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionKey")
            .field("key", &"[REDACTED]")
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.key.iter_mut().for_each(|b| *b = 0);
    }
}

/// Cipher for encrypting/decrypting data
pub struct Cipher {
    key: EncryptionKey,
    nonce_counter: AtomicU64,
}

impl Cipher {
    /// Create a new cipher with the given key
    pub fn new(key: EncryptionKey) -> Self {
        Self {
            key,
            nonce_counter: AtomicU64::new(1),
        }
    }

    /// Encrypt plaintext data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce_value = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let nonce = self.create_nonce(nonce_value);

        let ciphertext = match self.key.algorithm {
            CipherAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(&self.key.key)
                    .map_err(|e| AegisError::Crypto(format!("AES key error: {}", e)))?;
                cipher
                    .encrypt(Nonce::from_slice(&nonce), plaintext)
                    .map_err(|e| AegisError::Crypto(format!("AES encryption failed: {}", e)))?
            }
            CipherAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(&self.key.key)
                    .map_err(|e| AegisError::Crypto(format!("ChaCha key error: {}", e)))?;
                cipher
                    .encrypt(chacha20poly1305::Nonce::from_slice(&nonce), plaintext)
                    .map_err(|e| AegisError::Crypto(format!("ChaCha encryption failed: {}", e)))?
            }
        };

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt ciphertext data
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(AegisError::Crypto("Ciphertext too short".to_string()));
        }

        let (nonce, data) = ciphertext.split_at(12);

        let plaintext = match self.key.algorithm {
            CipherAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(&self.key.key)
                    .map_err(|e| AegisError::Crypto(format!("AES key error: {}", e)))?;
                cipher
                    .decrypt(Nonce::from_slice(nonce), data)
                    .map_err(|e| AegisError::Crypto(format!("AES decryption failed: {}", e)))?
            }
            CipherAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(&self.key.key)
                    .map_err(|e| AegisError::Crypto(format!("ChaCha key error: {}", e)))?;
                cipher
                    .decrypt(chacha20poly1305::Nonce::from_slice(nonce), data)
                    .map_err(|e| AegisError::Crypto(format!("ChaCha decryption failed: {}", e)))?
            }
        };

        Ok(plaintext)
    }

    /// Create a 12-byte nonce from counter value
    fn create_nonce(&self, counter: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&counter.to_be_bytes());
        nonce
    }

    /// Get current nonce counter (for debugging)
    pub fn nonce_counter(&self) -> u64 {
        self.nonce_counter.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cipher")
            .field("algorithm", &self.key.algorithm)
            .field("nonce_counter", &self.nonce_counter.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_gcm_encrypt_decrypt() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = b"Hello, Aegis-Flow!";
        let ciphertext = cipher.encrypt(plaintext).unwrap();

        assert_ne!(&ciphertext[12..], plaintext);
        assert!(ciphertext.len() > plaintext.len()); // ciphertext includes nonce + tag

        let decrypted = cipher.decrypt(&ciphertext).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_chacha20_encrypt_decrypt() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::ChaCha20Poly1305);
        let cipher = Cipher::new(key);

        let plaintext = b"Hello, ChaCha20!";
        let ciphertext = cipher.encrypt(plaintext).unwrap();

        let decrypted = cipher.decrypt(&ciphertext).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_key_derivation() {
        let shared_secret = [0xAB; 64];
        let key =
            EncryptionKey::derive(&shared_secret, b"aegis-flow-v1", CipherAlgorithm::Aes256Gcm)
                .unwrap();

        let cipher = Cipher::new(key);
        let plaintext = b"HKDF derived key works!";
        let ciphertext = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&ciphertext).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_nonce_counter_increments() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        assert_eq!(cipher.nonce_counter(), 1);

        let _ = cipher.encrypt(b"first").unwrap();
        assert_eq!(cipher.nonce_counter(), 2);

        let _ = cipher.encrypt(b"second").unwrap();
        assert_eq!(cipher.nonce_counter(), 3);
    }

    #[test]
    fn test_different_ciphertexts_for_same_plaintext() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = b"same message";
        let ct1 = cipher.encrypt(plaintext).unwrap();
        let ct2 = cipher.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertext due to different nonces
        assert_ne!(ct1, ct2);

        // But both should decrypt to the same plaintext
        assert_eq!(cipher.decrypt(&ct1).unwrap(), plaintext);
        assert_eq!(cipher.decrypt(&ct2).unwrap(), plaintext);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = b"authentic message";
        let mut ciphertext = cipher.encrypt(plaintext).unwrap();

        // Tamper with the ciphertext
        ciphertext[15] ^= 0xFF;

        let result = cipher.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = b"";
        let ciphertext = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&ciphertext).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_large_plaintext() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = vec![0xAB; 1_000_000]; // 1MB
        let ciphertext = cipher.encrypt(&plaintext).unwrap();
        let decrypted = cipher.decrypt(&ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
