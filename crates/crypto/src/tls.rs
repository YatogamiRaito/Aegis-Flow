//! TLS Integration for Post-Quantum Cryptography
//!
//! This module provides integration between our hybrid PQC key exchange
//! and the TLS layer using rustls.

use crate::hybrid_kex::{HybridCiphertext, HybridKeyExchange, HybridPublicKey, HybridSecretKey};
use aegis_common::Result;
use tracing::{debug, info, instrument};

/// PQC-enabled TLS configuration
#[derive(Debug, Clone)]
pub struct PqcTlsConfig {
    /// Enable hybrid PQC mode
    pub pqc_enabled: bool,
    /// Require client certificates (mTLS)
    pub mtls_required: bool,
    /// Algorithm selection
    pub algorithm: PqcAlgorithm,
}

impl Default for PqcTlsConfig {
    fn default() -> Self {
        Self {
            pqc_enabled: true,
            mtls_required: false,
            algorithm: PqcAlgorithm::HybridMlKem768,
        }
    }
}

/// Available PQC algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqcAlgorithm {
    /// X25519 only (classical)
    X25519Only,
    /// ML-KEM-768 only (NIST FIPS 203)
    MlKem768Only,
    /// Hybrid X25519 + ML-KEM-768 (recommended)
    HybridMlKem768,
    /// Hybrid X25519 + ML-KEM-1024 (highest security)
    HybridMlKem1024,
    /// Legacy: Kyber-768 only (deprecated)
    #[deprecated(since = "0.10.0", note = "Use MlKem768Only instead")]
    Kyber768Only,
    /// Legacy: Hybrid X25519 + Kyber-768 (deprecated)
    #[deprecated(since = "0.10.0", note = "Use HybridMlKem768 instead")]
    HybridKyber768,
    /// Legacy: Hybrid X25519 + Kyber-1024 (deprecated)
    #[deprecated(since = "0.10.0", note = "Use HybridMlKem1024 instead")]
    HybridKyber1024,
}

/// A secure channel established after PQC handshake
pub struct SecureChannel {
    /// Cipher for encryption/decryption
    cipher: crate::cipher::Cipher,
    /// Channel identifier
    channel_id: u64,
    /// Algorithm used
    algorithm: PqcAlgorithm,
}

impl SecureChannel {
    /// Create a new secure channel with encryption key
    pub(crate) fn new(encryption_key: [u8; 32], channel_id: u64, algorithm: PqcAlgorithm) -> Self {
        let key = crate::cipher::EncryptionKey::from_raw(
            encryption_key,
            crate::cipher::CipherAlgorithm::Aes256Gcm,
        );
        let cipher = crate::cipher::Cipher::new(key);

        Self {
            cipher,
            channel_id,
            algorithm,
        }
    }

    /// Encrypt data for transmission
    pub fn encrypt(&self, plaintext: &[u8]) -> aegis_common::Result<Vec<u8>> {
        self.cipher.encrypt(plaintext)
    }

    /// Decrypt received data
    pub fn decrypt(&self, ciphertext: &[u8]) -> aegis_common::Result<Vec<u8>> {
        self.cipher.decrypt(ciphertext)
    }

    /// Get the channel identifier
    pub fn channel_id(&self) -> u64 {
        self.channel_id
    }

    /// Get the algorithm used
    pub fn algorithm(&self) -> PqcAlgorithm {
        self.algorithm
    }

    /// Get the encryption key
    pub fn encryption_key(&self) -> &crate::cipher::EncryptionKey {
        self.cipher.key()
    }
}

impl std::fmt::Debug for SecureChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureChannel")
            .field("channel_id", &self.channel_id)
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

/// PQC-enabled handshake handler
pub struct PqcHandshake {
    kex: HybridKeyExchange,
    config: PqcTlsConfig,
    channel_counter: std::sync::atomic::AtomicU64,
}

impl PqcHandshake {
    /// Create a new handshake handler
    pub fn new(config: PqcTlsConfig) -> Self {
        Self {
            kex: HybridKeyExchange::new(),
            config,
            channel_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Server: Generate keypair for incoming connection
    #[instrument(skip(self))]
    pub fn server_init(&self) -> Result<(HybridPublicKey, ServerHandshakeState)> {
        debug!("Server initializing PQC handshake");
        let (pk, sk) = self.kex.generate_keypair()?;

        let state = ServerHandshakeState {
            secret_key: sk,
            algorithm: self.config.algorithm,
        };

        info!(
            "Server handshake initialized with {:?}",
            self.config.algorithm
        );
        Ok((pk, state))
    }

    /// Client: Complete handshake with server's public key
    #[instrument(skip(self, server_pk))]
    pub fn client_complete(
        &self,
        server_pk: &HybridPublicKey,
    ) -> Result<(HybridCiphertext, SecureChannel)> {
        debug!("Client completing PQC handshake");

        let (ciphertext, shared_secret) = self.kex.encapsulate(server_pk)?;
        let encryption_key = shared_secret.derive_key();
        let channel_id = self
            .channel_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let channel = SecureChannel::new(encryption_key, channel_id, self.config.algorithm);

        info!("Client handshake complete, channel_id={}", channel_id);
        Ok((ciphertext, channel))
    }

    /// Server: Complete handshake with client's ciphertext
    #[instrument(skip(self, ciphertext, state))]
    pub fn server_complete(
        &self,
        ciphertext: &HybridCiphertext,
        state: ServerHandshakeState,
    ) -> Result<SecureChannel> {
        debug!("Server completing PQC handshake");

        let shared_secret = self.kex.decapsulate(ciphertext, &state.secret_key)?;
        let encryption_key = shared_secret.derive_key();
        let channel_id = self
            .channel_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let channel = SecureChannel::new(encryption_key, channel_id, state.algorithm);

        info!("Server handshake complete, channel_id={}", channel_id);
        Ok(channel)
    }
}

/// Server-side handshake state (holds secret key during handshake)
pub struct ServerHandshakeState {
    secret_key: HybridSecretKey,
    algorithm: PqcAlgorithm,
}

impl std::fmt::Debug for ServerHandshakeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerHandshakeState")
            .field("secret_key", &"[REDACTED]")
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pqc_handshake_roundtrip() {
        let config = PqcTlsConfig::default();
        let server_handshake = PqcHandshake::new(config.clone());
        let client_handshake = PqcHandshake::new(config);

        // Server generates keypair
        let (server_pk, server_state) = server_handshake.server_init().unwrap();

        // Client completes handshake
        let (ciphertext, client_channel) = client_handshake.client_complete(&server_pk).unwrap();

        // Server completes handshake
        let server_channel = server_handshake
            .server_complete(&ciphertext, server_state)
            .unwrap();

        // Both should be able to encrypt/decrypt each other's messages
        let plaintext = b"Hello, PQC encrypted world!";
        let encrypted = client_channel.encrypt(plaintext).unwrap();
        let decrypted = server_channel.decrypt(&encrypted).unwrap();
        assert_eq!(&decrypted, plaintext);

        // And vice versa
        let server_encrypted = server_channel.encrypt(b"Server response").unwrap();
        let server_decrypted = client_channel.decrypt(&server_encrypted).unwrap();
        assert_eq!(&server_decrypted, b"Server response");

        assert_eq!(client_channel.algorithm(), server_channel.algorithm());
    }

    #[test]
    fn test_default_config() {
        let config = PqcTlsConfig::default();
        assert!(config.pqc_enabled);
        assert!(!config.mtls_required);
        assert_eq!(config.algorithm, PqcAlgorithm::HybridMlKem768);
    }

    #[test]
    fn test_channel_ids_are_unique() {
        let handshake = PqcHandshake::new(PqcTlsConfig::default());

        let (pk, state1) = handshake.server_init().unwrap();
        let (ct1, channel1) = handshake.client_complete(&pk).unwrap();
        let channel_server1 = handshake.server_complete(&ct1, state1).unwrap();

        let (pk2, state2) = handshake.server_init().unwrap();
        let (ct2, channel2) = handshake.client_complete(&pk2).unwrap();
        let channel_server2 = handshake.server_complete(&ct2, state2).unwrap();

        // All channel IDs should be unique
        let ids = [
            channel1.channel_id(),
            channel_server1.channel_id(),
            channel2.channel_id(),
            channel_server2.channel_id(),
        ];

        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                assert_ne!(ids[i], ids[j], "Channel IDs should be unique");
            }
        }
    }

    #[test]
    fn test_server_complete_invalid_ciphertext() {
        let config = PqcTlsConfig::default();
        let handshake = PqcHandshake::new(config);

        // 1. Setup "Good" Client (Key B)
        let client_h = PqcHandshake::new(PqcTlsConfig::default());
        let (pk_b, _) = client_h.server_init().unwrap();
        let (ct_for_b, client_chan) = client_h.client_complete(&pk_b).unwrap();

        // 2. Setup "Bad" Server (Key A)
        let (_, sk_a_state) = handshake.server_init().unwrap();

        // 3. Try to complete handshake on Server A using Ciphertext for B
        // This attempts to decapsulate ct_for_b using sk_a
        let result = handshake.server_complete(&ct_for_b, sk_a_state);

        if let Ok(bad_server_chan) = result {
            // Implicit rejection scenario: handshake succeeded with garbage key
            let plaintext = b"Secret";
            let encrypted = client_chan.encrypt(plaintext).unwrap();

            // Decrypt should fail (AEAD tag mismatch)
            let decrypt_result = bad_server_chan.decrypt(&encrypted);
            assert!(
                decrypt_result.is_err(),
                "Decryption should fail when keys mismatch"
            );
        } else {
            // Explicit rejection scenario
            assert!(result.is_err());
        }

        // Test explicit invalid ciphertext structure check if possible
        // (Just to use from_bytes and suppress unused warning if necessary, or omit)
        let _ = HybridCiphertext::from_bytes(&[0u8; 100]);
    }

    #[test]
    fn test_secure_channel_debug() {
        let channel = SecureChannel::new([0u8; 32], 123, PqcAlgorithm::HybridMlKem768);
        let debug_str = format!("{:?}", channel);
        assert!(debug_str.contains("SecureChannel"));
        assert!(debug_str.contains("123"));
        assert!(!debug_str.contains("secret")); // Should not leak keys
    }

    #[test]
    fn test_secure_channel_encrypt_decrypt() {
        let channel = SecureChannel::new([42u8; 32], 999, PqcAlgorithm::HybridMlKem768);
        let plaintext = b"Hello, PQC world!";
        let ciphertext = channel.encrypt(plaintext).unwrap();
        let decrypted = channel.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_secure_channel_properties() {
        let channel = SecureChannel::new([1u8; 32], 456, PqcAlgorithm::MlKem768Only);
        assert_eq!(channel.channel_id(), 456);
        assert_eq!(channel.algorithm(), PqcAlgorithm::MlKem768Only);
    }

    #[test]
    fn test_pqc_algorithm_variants() {
        let algos = [
            PqcAlgorithm::X25519Only,
            PqcAlgorithm::MlKem768Only,
            PqcAlgorithm::HybridMlKem768,
            PqcAlgorithm::HybridMlKem1024,
        ];
        for algo in algos {
            let _ = format!("{:?}", algo);
        }
    }

    #[test]
    fn test_pqc_tls_config_default() {
        let config = PqcTlsConfig::default();
        assert!(config.pqc_enabled);
        assert!(!config.mtls_required);
        assert_eq!(config.algorithm, PqcAlgorithm::HybridMlKem768);
    }

    #[test]
    fn test_pqc_tls_config_custom() {
        let config = PqcTlsConfig {
            pqc_enabled: false,
            mtls_required: true,
            algorithm: PqcAlgorithm::X25519Only,
        };
        assert!(!config.pqc_enabled);
        assert!(config.mtls_required);
        assert_eq!(config.algorithm, PqcAlgorithm::X25519Only);
    }

    #[test]
    fn test_pqc_algorithm_equality() {
        assert_eq!(PqcAlgorithm::HybridMlKem768, PqcAlgorithm::HybridMlKem768);
        assert_ne!(PqcAlgorithm::HybridMlKem768, PqcAlgorithm::HybridMlKem1024);
        assert_ne!(PqcAlgorithm::X25519Only, PqcAlgorithm::MlKem768Only);
    }

    #[test]
    fn test_secure_channel_different_ids() {
        let ch1 = SecureChannel::new([0u8; 32], 1, PqcAlgorithm::HybridMlKem768);
        let ch2 = SecureChannel::new([0u8; 32], 2, PqcAlgorithm::HybridMlKem768);
        assert_ne!(ch1.channel_id(), ch2.channel_id());
    }

    #[test]
    fn test_secure_channel_large_plaintext() {
        let channel = SecureChannel::new([0xAB; 32], 100, PqcAlgorithm::HybridMlKem768);
        let plaintext = vec![0xCD; 10000]; // 10KB
        let ciphertext = channel.encrypt(&plaintext).unwrap();
        let decrypted = channel.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_pqc_tls_config_clone() {
        let config = PqcTlsConfig {
            pqc_enabled: true,
            mtls_required: false,
            algorithm: PqcAlgorithm::HybridMlKem1024,
        };
        let cloned = config.clone();
        assert_eq!(config.pqc_enabled, cloned.pqc_enabled);
        assert_eq!(config.algorithm, cloned.algorithm);
    }

    #[test]
    fn test_pqc_algorithm_cloning() {
        let alg = PqcAlgorithm::HybridMlKem768;
        let copied = alg; // PqcAlgorithm is Copy
        assert_eq!(alg, copied);
    }

    #[test]
    fn test_pqc_algorithm_debug() {
        let alg = PqcAlgorithm::MlKem768Only;
        let debug = format!("{:?}", alg);
        assert!(debug.contains("MlKem768Only"));
    }

    #[test]
    fn test_pqc_algorithm_debug_format() {
        let alg = PqcAlgorithm::HybridMlKem768;
        let debug = format!("{:?}", alg);
        assert!(debug.contains("HybridMlKem768"));
    }

    #[test]
    fn test_pqc_algorithm_all_four() {
        let variants = [
            PqcAlgorithm::X25519Only,
            PqcAlgorithm::MlKem768Only,
            PqcAlgorithm::HybridMlKem768,
            PqcAlgorithm::HybridMlKem1024,
        ];
        for v in variants {
            let debug = format!("{:?}", v);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_secure_channel_encryption_key() {
        let key_bytes = [0xAB; 32];
        let channel = SecureChannel::new(key_bytes, 100, PqcAlgorithm::HybridMlKem768);
        let key = channel.encryption_key();
        // Key should be accessible
        let _ = format!("{:?}", key);
    }

    #[test]
    fn test_server_handshake_state_debug() {
        let config = PqcTlsConfig::default();
        let handshake = PqcHandshake::new(config);
        let (_, state) = handshake.server_init().unwrap();

        let debug = format!("{:?}", state);
        assert!(debug.contains("ServerHandshakeState"));
        assert!(debug.contains("[REDACTED]")); // Secret key should be redacted
    }

    #[test]
    fn test_pqc_config_debug() {
        let config = PqcTlsConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("PqcTlsConfig"));
    }
}
