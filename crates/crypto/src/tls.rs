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
            algorithm: PqcAlgorithm::HybridKyber768,
        }
    }
}

/// Available PQC algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqcAlgorithm {
    /// X25519 only (classical)
    X25519Only,
    /// Kyber-768 only (PQC)
    Kyber768Only,
    /// Hybrid X25519 + Kyber-768 (recommended)
    HybridKyber768,
    /// Hybrid X25519 + Kyber-1024 (highest security)
    HybridKyber1024,
}

/// A secure channel established after PQC handshake
#[derive(Debug)]
pub struct SecureChannel {
    /// Derived encryption key (32 bytes)
    encryption_key: [u8; 32],
    /// Channel identifier
    channel_id: u64,
    /// Algorithm used
    algorithm: PqcAlgorithm,
}

impl SecureChannel {
    /// Get the encryption key for this channel
    pub fn encryption_key(&self) -> &[u8; 32] {
        &self.encryption_key
    }

    /// Get the channel identifier
    pub fn channel_id(&self) -> u64 {
        self.channel_id
    }

    /// Get the algorithm used
    pub fn algorithm(&self) -> PqcAlgorithm {
        self.algorithm
    }
}

impl Drop for SecureChannel {
    fn drop(&mut self) {
        // Zeroize encryption key on drop
        self.encryption_key.iter_mut().for_each(|b| *b = 0);
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

        let channel = SecureChannel {
            encryption_key,
            channel_id,
            algorithm: self.config.algorithm,
        };

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

        let channel = SecureChannel {
            encryption_key,
            channel_id,
            algorithm: state.algorithm,
        };

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

        // Both should have the same encryption key
        assert_eq!(
            client_channel.encryption_key(),
            server_channel.encryption_key()
        );
        assert_eq!(client_channel.algorithm(), server_channel.algorithm());
    }

    #[test]
    fn test_default_config() {
        let config = PqcTlsConfig::default();
        assert!(config.pqc_enabled);
        assert!(!config.mtls_required);
        assert_eq!(config.algorithm, PqcAlgorithm::HybridKyber768);
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
        let ids = vec![
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
}
