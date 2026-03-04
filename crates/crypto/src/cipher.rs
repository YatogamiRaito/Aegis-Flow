//! Symmetric Encryption Module
//!
//! Provides AES-256-GCM and ChaCha20-Poly1305 encryption for secure data transfer.
//!
//! Security properties:
//! - Keys are zeroized on drop via `ZeroizeOnDrop`
//! - Nonce counter is monotonically increasing; exhaustion returns `Err`

use aegis_common::{AegisError, Result};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use chacha20poly1305::ChaCha20Poly1305;
use hkdf::Hkdf;
use sha2::Sha256;
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Maximum safe nonce value. One below u64::MAX to leave a sentinel.
const NONCE_EXHAUSTION_THRESHOLD: u64 = u64::MAX - 1;

/// Cipher algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CipherAlgorithm {
    /// AES-256-GCM (recommended for hardware with AES-NI)
    #[default]
    Aes256Gcm,
    /// ChaCha20-Poly1305 (recommended for software-only)
    ChaCha20Poly1305,
}

/// Encryption key derived from shared secret.
///
/// The key material is zeroized on drop via [`ZeroizeOnDrop`].
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    key: [u8; 32],
    #[zeroize(skip)]
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

    /// Get raw key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
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

/// Internal engine holding initialized cipher states
enum CipherEngine {
    Aes(Box<Aes256Gcm>),
    ChaCha(ChaCha20Poly1305),
}

/// Cipher for encrypting/decrypting data
pub struct Cipher {
    key: EncryptionKey,
    engine: CipherEngine,
    nonce_counter: AtomicU64,
}

impl Cipher {
    /// Create a new cipher with the given key
    pub fn new(key: EncryptionKey) -> Self {
        let engine = match key.algorithm() {
            CipherAlgorithm::Aes256Gcm => CipherEngine::Aes(Box::new(
                Aes256Gcm::new_from_slice(&key.key)
                    .map_err(|_| {
                        AegisError::Crypto("Invalid key length for AES-256-GCM".to_string())
                    })
                    .expect("Invalid AES key length (should be caught by map_err)"),
            )),
            CipherAlgorithm::ChaCha20Poly1305 => CipherEngine::ChaCha(
                ChaCha20Poly1305::new_from_slice(&key.key).expect("Invalid ChaCha key length"),
            ),
        };

        Self {
            key,
            engine,
            nonce_counter: AtomicU64::new(1),
        }
    }

    /// Encrypt plaintext data.
    ///
    /// Returns `Err(AegisError::Crypto("Nonce space exhausted"))` when the nonce
    /// counter approaches `u64::MAX` to prevent nonce reuse.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // Guard against nonce exhaustion *before* incrementing
        let nonce_value = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        if nonce_value >= NONCE_EXHAUSTION_THRESHOLD {
            return Err(AegisError::Crypto(
                "Nonce space exhausted — rotate encryption key immediately".to_string(),
            ));
        }
        let nonce = self.create_nonce(nonce_value);

        let ciphertext = match &self.engine {
            CipherEngine::Aes(cipher) => cipher
                .encrypt(Nonce::from_slice(&nonce), plaintext)
                .map_err(|e| AegisError::Crypto(format!("AES encryption failed: {}", e)))?,
            CipherEngine::ChaCha(cipher) => cipher
                .encrypt(chacha20poly1305::Nonce::from_slice(&nonce), plaintext)
                .map_err(|e| AegisError::Crypto(format!("ChaCha encryption failed: {}", e)))?,
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

        let plaintext = match &self.engine {
            CipherEngine::Aes(cipher) => cipher
                .decrypt(Nonce::from_slice(nonce), data)
                .map_err(|e| AegisError::Crypto(format!("AES decryption failed: {}", e)))?,
            CipherEngine::ChaCha(cipher) => cipher
                .decrypt(chacha20poly1305::Nonce::from_slice(nonce), data)
                .map_err(|e| AegisError::Crypto(format!("ChaCha decryption failed: {}", e)))?,
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

    /// Number of encryptions remaining before nonce exhaustion.
    pub fn nonce_remaining(&self) -> u64 {
        let current = self.nonce_counter.load(Ordering::SeqCst);
        NONCE_EXHAUSTION_THRESHOLD.saturating_sub(current)
    }

    /// Rotate the encryption key without changing the counter position.
    ///
    /// Resets the nonce counter to 1 so the new key starts from a known nonce.
    /// # Policy
    /// Callers should rotate when `nonce_remaining()` is low or on a schedule.
    pub fn rotate_key(&mut self, new_key: EncryptionKey) {
        self.engine = match new_key.algorithm() {
            CipherAlgorithm::Aes256Gcm => CipherEngine::Aes(Box::new(
                Aes256Gcm::new_from_slice(&new_key.key).expect("Invalid AES key length"),
            )),
            CipherAlgorithm::ChaCha20Poly1305 => CipherEngine::ChaCha(
                ChaCha20Poly1305::new_from_slice(&new_key.key).expect("Invalid ChaCha key length"),
            ),
        };
        self.key = new_key;
        self.nonce_counter.store(1, Ordering::SeqCst);
    }

    /// Get the encryption key
    pub fn key(&self) -> &EncryptionKey {
        &self.key
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

    #[test]
    fn test_cipher_debug_and_accessors() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key.clone());

        // Test debug
        let debug_str = format!("{:?}", cipher);
        assert!(debug_str.contains("Cipher"));
        assert!(debug_str.contains("Aes256Gcm"));

        let key_debug = format!("{:?}", key);
        assert!(key_debug.contains("REDACTED"));

        // Test accessors
        assert_eq!(key.algorithm(), CipherAlgorithm::Aes256Gcm);
        assert_eq!(key.as_bytes(), &[0x42; 32]);
        assert_eq!(cipher.key().algorithm(), CipherAlgorithm::Aes256Gcm);
    }

    #[test]
    fn test_decrypt_too_short() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let short_ct = vec![0u8; 11]; // Less than nonce size (12)
        let result = cipher.decrypt(&short_ct);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cryptographic error: Ciphertext too short"
        );
    }

    #[test]
    fn test_cipher_algorithm_variants() {
        assert_ne!(
            CipherAlgorithm::Aes256Gcm,
            CipherAlgorithm::ChaCha20Poly1305
        );
    }

    #[test]
    fn test_encryption_key_clone() {
        let key = EncryptionKey::from_raw([0xAB; 32], CipherAlgorithm::Aes256Gcm);
        let cloned = key.clone();
        assert_eq!(key.as_bytes(), cloned.as_bytes());
        assert_eq!(key.algorithm(), cloned.algorithm());
    }

    #[test]
    fn test_chacha20_cipher() {
        let key = EncryptionKey::from_raw([0xCC; 32], CipherAlgorithm::ChaCha20Poly1305);
        let cipher = Cipher::new(key);

        let plaintext = b"ChaCha20 test message";
        let ciphertext = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&ciphertext).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_cipher_different_keys_different_ciphertexts() {
        let key1 = EncryptionKey::from_raw([0x11; 32], CipherAlgorithm::Aes256Gcm);
        let key2 = EncryptionKey::from_raw([0x22; 32], CipherAlgorithm::Aes256Gcm);

        let cipher1 = Cipher::new(key1);
        let cipher2 = Cipher::new(key2);

        let plaintext = b"Same message";
        let ct1 = cipher1.encrypt(plaintext).unwrap();
        let ct2 = cipher2.encrypt(plaintext).unwrap();

        // Ciphertexts should be different due to different keys
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_cipher_nonce_uniqueness() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        let plaintext = b"test message";
        let ct1 = cipher.encrypt(plaintext).unwrap();
        let ct2 = cipher.encrypt(plaintext).unwrap();

        // Same message encrypted twice should produce different ciphertexts (due to random nonce)
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_cipher_algorithm_display() {
        assert_eq!(format!("{:?}", CipherAlgorithm::Aes256Gcm), "Aes256Gcm");
        assert_eq!(
            format!("{:?}", CipherAlgorithm::ChaCha20Poly1305),
            "ChaCha20Poly1305"
        );
    }

    #[test]
    fn test_cross_algorithm_decryption_fails() {
        let aes_key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let chacha_key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::ChaCha20Poly1305);

        let aes_cipher = Cipher::new(aes_key);
        let chacha_cipher = Cipher::new(chacha_key);

        let plaintext = b"test";
        let aes_ct = aes_cipher.encrypt(plaintext).unwrap();

        // Trying to decrypt AES ciphertext with ChaCha should fail
        let result = chacha_cipher.decrypt(&aes_ct);
        assert!(result.is_err());
    }

    // =========================================================================
    // Track 30: New Hardening Tests (FR-2, FR-7)
    // =========================================================================

    #[test]
    fn test_encryption_key_zeroed_after_drop() {
        use zeroize::ZeroizeOnDrop;
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<EncryptionKey>();
    }

    #[test]
    fn test_nonce_overflow_returns_error() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        // Manually advance counter to just before the threshold
        cipher
            .nonce_counter
            .store(NONCE_EXHAUSTION_THRESHOLD, Ordering::SeqCst);

        // Now encrypt should return an error
        let result = cipher.encrypt(b"test");
        assert!(
            result.is_err(),
            "Encrypt must fail when nonce counter is at threshold"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Nonce space exhausted"),
            "Error message must mention nonce exhaustion, got: {err_msg}"
        );
    }

    #[test]
    fn test_nonce_remaining_accessor() {
        let key = EncryptionKey::from_raw([0x42; 32], CipherAlgorithm::Aes256Gcm);
        let cipher = Cipher::new(key);

        // Initially should have a very large remaining count
        let initial_remaining = cipher.nonce_remaining();
        assert!(
            initial_remaining > 1_000_000,
            "Initially nonces remaining should be huge"
        );

        // After one encryption it decrements by one
        let _ = cipher.encrypt(b"x").unwrap();
        assert_eq!(cipher.nonce_remaining(), initial_remaining - 1);
    }

    #[test]
    fn test_key_rotation_resets_nonce_counter() {
        let key = EncryptionKey::from_raw([0x11; 32], CipherAlgorithm::Aes256Gcm);
        let mut cipher = Cipher::new(key);

        // Advance counter a bit
        for _ in 0..100 {
            let _ = cipher.encrypt(b"x").unwrap();
        }
        assert_eq!(cipher.nonce_counter(), 101);

        // Rotate key
        let new_key = EncryptionKey::from_raw([0x22; 32], CipherAlgorithm::Aes256Gcm);
        cipher.rotate_key(new_key);

        // Counter should reset to 1
        assert_eq!(
            cipher.nonce_counter(),
            1,
            "rotate_key() must reset nonce counter to 1"
        );

        // Encryption with new key must still work
        let ct = cipher.encrypt(b"after rotation").unwrap();
        let pt = cipher.decrypt(&ct).unwrap();
        assert_eq!(&pt, b"after rotation");
    }

    #[test]
    fn test_key_rotation_produces_different_ciphertexts() {
        let key1 = EncryptionKey::from_raw([0x11; 32], CipherAlgorithm::Aes256Gcm);
        let key2 = EncryptionKey::from_raw([0x22; 32], CipherAlgorithm::Aes256Gcm);

        let mut cipher = Cipher::new(key1);
        let ct_before = cipher.encrypt(b"same").unwrap();

        // The nonce counter starts at 1 for the ciphertext above
        cipher.rotate_key(key2);
        let ct_after = cipher.encrypt(b"same").unwrap();

        // Same nonce counter value (both 1), but different keys → different ciphertexts
        assert_ne!(
            ct_before, ct_after,
            "Different keys must produce different ciphertexts"
        );
    }
}
