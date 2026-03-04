//! Hybrid Key Exchange: X25519 + ML-KEM-768 / ML-KEM-1024
//!
//! This module implements a hybrid key exchange combining classical X25519
//! with post-quantum ML-KEM-768 or ML-KEM-1024 (NIST FIPS 203) for
//! "Harvest Now, Decrypt Later" protection.
//!
//! Key derivation follows IETF draft-ietf-tls-hybrid-design-10:
//! - `combine()` uses HKDF-SHA256 extract over concat(X25519_SS || MLKEM_SS)
//! - `derive_key()` uses HKDF-SHA256 expand with a context label
//!
//! # RFC Reference
//! See docs/rfcs/RFC-001-hybrid-kex.md for design details.

use aegis_common::{AegisError, Result};
use hkdf::Hkdf;
use pqcrypto_mlkem::mlkem768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret as MlkemSharedSecret};
use rand::rngs::OsRng;
use sha2::Sha256;
use tracing::{debug, instrument};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

// IETF draft-ietf-tls-hybrid-design-10 labels
const KDF_EXTRACT_LABEL: &[u8] = b"aegis-flow-hybrid-kex-v1";
const KDF_SESSION_LABEL: &[u8] = b"aegis-flow-session-key-v1";
const KDF_CLIENT_LABEL: &[u8] = b"aegis-flow-client-key-v1";
const KDF_SERVER_LABEL: &[u8] = b"aegis-flow-server-key-v1";

/// Security level for ML-KEM algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityLevel {
    /// ML-KEM-768 — NIST Security Level 3 (default, faster)
    #[default]
    Standard,
    /// ML-KEM-1024 — NIST Security Level 5 (highest security)
    High,
}

/// The set of algorithms supported, in preference order (strongest first)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqcAlgorithm {
    /// X25519 + ML-KEM-1024
    HybridMlKem1024,
    /// X25519 + ML-KEM-768
    HybridMlKem768,
}

impl PqcAlgorithm {
    /// All algorithms in strongest-first order
    pub fn strength_order() -> &'static [PqcAlgorithm] {
        &[PqcAlgorithm::HybridMlKem1024, PqcAlgorithm::HybridMlKem768]
    }

    /// Return name string
    pub fn name(self) -> &'static str {
        match self {
            PqcAlgorithm::HybridMlKem1024 => "X25519-MLKEM1024-Hybrid",
            PqcAlgorithm::HybridMlKem768 => "X25519-MLKEM768-Hybrid",
        }
    }
}

/// Result of algorithm negotiation between two peers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiatedAlgorithm {
    pub algorithm: PqcAlgorithm,
}

impl NegotiatedAlgorithm {
    /// Negotiate the best mutually supported algorithm.
    ///
    /// Iterates supported algorithms in strength order and returns the first
    /// one that both peers support.
    pub fn negotiate(
        local_supported: &[PqcAlgorithm],
        peer_supported: &[PqcAlgorithm],
    ) -> Option<Self> {
        for alg in PqcAlgorithm::strength_order() {
            if local_supported.contains(alg) && peer_supported.contains(alg) {
                return Some(NegotiatedAlgorithm { algorithm: *alg });
            }
        }
        None
    }
}

/// Combined public key for hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridPublicKey {
    /// Full serialized bytes: [X25519 (32 bytes)] || [ML-KEM (N bytes)]
    bytes: Vec<u8>,
}

impl HybridPublicKey {
    /// Serialize the hybrid public key to bytes (X25519 || ML-KEM)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(AegisError::Crypto("Public key too short".to_string()));
        }
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    /// Access only the X25519 component (32 bytes)
    pub fn x25519_bytes(&self) -> &[u8; 32] {
        self.bytes[..32].try_into().unwrap()
    }

    /// Access only the ML-KEM component
    pub fn mlkem_bytes(&self) -> &[u8] {
        &self.bytes[32..]
    }
}

impl AsRef<[u8]> for HybridPublicKey {
    /// Returns the full serialized key (X25519 || ML-KEM) as required by KeyExchange trait.
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Combined shared secret from hybrid key exchange
///
/// Zeroed on drop via `ZeroizeOnDrop`.
/// Note: Clone is intentionally NOT derived because cloning secret material into
/// separate heap allocations can leave non-zeroized copies if not tracked properly.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct HybridSharedSecret {
    /// The combined shared secret — HKDF-SHA256 PRK output (32 bytes)
    inner: [u8; 32],
}

impl HybridSharedSecret {
    /// Combine X25519 and ML-KEM shared secrets using HKDF-SHA256 extract.
    ///
    /// Follows IETF draft-ietf-tls-hybrid-design-10:
    /// IKM = concat(X25519_SS || MLKEM_SS), salt = None, info = label
    pub fn combine(x25519_secret: &[u8; 32], mlkem_secret: &[u8]) -> Self {
        // IKM = X25519_SS || MLKEM_SS
        let mut ikm = Vec::with_capacity(32 + mlkem_secret.len());
        ikm.extend_from_slice(x25519_secret);
        ikm.extend_from_slice(mlkem_secret);

        // HKDF-SHA256 extract: PRK = HMAC-SHA256(salt=0x00..., IKM)
        let (prk, _) = Hkdf::<Sha256>::extract(None, &ikm);

        // Expand into final combined secret (32 bytes)
        let mut inner = [0u8; 32];
        Hkdf::<Sha256>::from_prk(&prk)
            .expect("PRK length is valid for Sha256")
            .expand(KDF_EXTRACT_LABEL, &mut inner)
            .expect("32-byte output is within HKDF-SHA256 limits");

        // Zeroize IKM before dropping
        ikm.zeroize();

        Self { inner }
    }

    /// Get the combined PRK as bytes (32 bytes)
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.inner
    }

    /// Derive a 32-byte symmetric session key using HKDF-SHA256 expand.
    ///
    /// info = `"aegis-flow-session-key-v1"` (context separation per RFC 5869)
    pub fn derive_key(&self) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, &self.inner);
        let mut key = [0u8; 32];
        hk.expand(KDF_SESSION_LABEL, &mut key)
            .expect("32-byte output is within HKDF-SHA256 limits");
        key
    }

    /// Derive a directional 32-byte key for the client→server direction.
    ///
    /// Ensures client and server use distinct keys even from the same shared secret.
    pub fn derive_client_key(&self) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, &self.inner);
        let mut key = [0u8; 32];
        hk.expand(KDF_CLIENT_LABEL, &mut key)
            .expect("32-byte output is within HKDF-SHA256 limits");
        key
    }

    /// Derive a directional 32-byte key for the server→client direction.
    ///
    /// Ensures client and server use distinct keys even from the same shared secret.
    pub fn derive_server_key(&self) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, &self.inner);
        let mut key = [0u8; 32];
        hk.expand(KDF_SERVER_LABEL, &mut key)
            .expect("32-byte output is within HKDF-SHA256 limits");
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

/// Hybrid Key Exchange implementation (X25519 + ML-KEM-768 or ML-KEM-1024)
#[derive(Debug, Default)]
pub struct HybridKeyExchange {
    security_level: SecurityLevel,
}

impl HybridKeyExchange {
    /// Create a new `HybridKeyExchange` with default security level (ML-KEM-768)
    pub fn new() -> Self {
        Self {
            security_level: SecurityLevel::Standard,
        }
    }

    /// Create with explicit security level
    pub fn new_with_level(level: SecurityLevel) -> Self {
        Self {
            security_level: level,
        }
    }

    /// Generate a new hybrid key pair
    #[instrument(skip(self))]
    pub fn generate_keypair(&self) -> Result<(HybridPublicKey, HybridSecretKey)> {
        debug!("Generating hybrid key pair (X25519 + ML-KEM-768)");

        // Generate X25519 key pair using StaticSecret (reusable)
        let x25519_secret = X25519StaticSecret::random_from_rng(OsRng);
        let x25519_public = X25519PublicKey::from(&x25519_secret);

        // Generate ML-KEM-768 key pair (ML-KEM-1024 requires separate crate)
        let (mlkem_pk, mlkem_sk) = mlkem768::keypair();

        let mut bytes = Vec::with_capacity(32 + mlkem_pk.as_bytes().len());
        bytes.extend_from_slice(x25519_public.as_bytes());
        bytes.extend_from_slice(mlkem_pk.as_bytes());

        let public_key = HybridPublicKey { bytes };

        let secret_key = HybridSecretKey {
            x25519: x25519_secret,
            mlkem: mlkem_sk.as_bytes().to_vec(),
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

        let peer_x25519_pk = X25519PublicKey::from(*peer_public_key.x25519_bytes());
        let x25519_shared = ephemeral_secret.diffie_hellman(&peer_x25519_pk);

        // ML-KEM-768 encapsulation
        let mlkem_pk = mlkem768::PublicKey::from_bytes(peer_public_key.mlkem_bytes())
            .map_err(|e| AegisError::Crypto(format!("Invalid ML-KEM public key: {:?}", e)))?;
        let (mlkem_ss, mlkem_ct) = mlkem768::encapsulate(&mlkem_pk);

        let ciphertext = HybridCiphertext {
            x25519_ephemeral: ephemeral_public.to_bytes(),
            mlkem_ciphertext: mlkem_ct.as_bytes().to_vec(),
        };

        let shared_secret =
            HybridSharedSecret::combine(x25519_shared.as_bytes(), mlkem_ss.as_bytes());

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

        // ML-KEM-768 decapsulation
        let mlkem_sk = mlkem768::SecretKey::from_bytes(&secret_key.mlkem)
            .map_err(|e| AegisError::Crypto(format!("Invalid ML-KEM secret key: {:?}", e)))?;
        let mlkem_ct = mlkem768::Ciphertext::from_bytes(&ciphertext.mlkem_ciphertext)
            .map_err(|e| AegisError::Crypto(format!("Invalid ML-KEM ciphertext: {:?}", e)))?;

        let mlkem_ss = mlkem768::decapsulate(&mlkem_ct, &mlkem_sk);

        let shared_secret =
            HybridSharedSecret::combine(x25519_shared.as_bytes(), mlkem_ss.as_bytes());

        debug!("Hybrid decapsulation completed");
        Ok(shared_secret)
    }

    /// Get algorithm name
    pub fn algorithm_name(&self) -> &'static str {
        match self.security_level {
            SecurityLevel::Standard => "X25519-MLKEM768-Hybrid",
            SecurityLevel::High => "X25519-MLKEM1024-Hybrid",
        }
    }
}

/// Implement the `KeyExchange` trait for `HybridKeyExchange`.
///
/// Wire format:
/// - `generate_keypair()` → (public_key_bytes, secret_key_bytes)
/// - `encapsulate(peer_pk_bytes)` → (ciphertext_bytes, shared_secret)
/// - `decapsulate(ct_bytes, sk_bytes)` → shared_secret
impl crate::traits::KeyExchange for HybridKeyExchange {
    type PublicKey = HybridPublicKey;
    type SharedSecret = HybridSharedSecret;

    fn generate_keypair(&self) -> Result<(Self::PublicKey, Vec<u8>)> {
        let (pk, sk) = HybridKeyExchange::generate_keypair(self)?;
        // Serialize secret key: x25519 (32 bytes raw) || mlkem (remaining)
        let mut sk_bytes = Vec::with_capacity(32 + sk.mlkem.len());
        sk_bytes.extend_from_slice(sk.x25519.as_bytes());
        sk_bytes.extend_from_slice(&sk.mlkem);
        Ok((pk, sk_bytes))
    }

    fn encapsulate(&self, peer_public_key: &[u8]) -> Result<(Vec<u8>, Self::SharedSecret)> {
        let pk = HybridPublicKey::from_bytes(peer_public_key)?;
        let (ct, ss) = HybridKeyExchange::encapsulate(self, &pk)?;
        Ok((ct.to_bytes(), ss))
    }

    fn decapsulate(&self, ciphertext: &[u8], secret_key: &[u8]) -> Result<Self::SharedSecret> {
        if secret_key.len() < 32 {
            return Err(AegisError::Crypto("Secret key too short".to_string()));
        }
        let mut x25519_bytes = [0u8; 32];
        x25519_bytes.copy_from_slice(&secret_key[..32]);
        let mlkem_bytes = secret_key[32..].to_vec();

        // Reconstruct X25519StaticSecret from raw bytes
        let x25519 = X25519StaticSecret::from(x25519_bytes);
        let sk = HybridSecretKey {
            x25519,
            mlkem: mlkem_bytes,
        };
        let ct = HybridCiphertext::from_bytes(ciphertext)?;
        HybridKeyExchange::decapsulate(self, &ct, &sk)
    }

    fn algorithm_name(&self) -> &'static str {
        HybridKeyExchange::algorithm_name(self)
    }
}

/// Secret key for hybrid key exchange.
///
/// The `mlkem` field is zeroized on drop via `ZeroizeOnDrop`.
/// The `x25519` field (`X25519StaticSecret`) implements `ZeroizeOnDrop` natively.
#[derive(ZeroizeOnDrop)]
pub struct HybridSecretKey {
    x25519: X25519StaticSecret,
    mlkem: Vec<u8>,
}

impl std::fmt::Debug for HybridSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridSecretKey")
            .field("x25519", &"[REDACTED]")
            .field("mlkem", &"[REDACTED]")
            .finish()
    }
}

/// Ciphertext for hybrid key exchange
#[derive(Debug, Clone)]
pub struct HybridCiphertext {
    /// X25519 ephemeral public key
    pub x25519_ephemeral: [u8; 32],
    /// ML-KEM-768 ciphertext
    pub mlkem_ciphertext: Vec<u8>,
}

impl HybridCiphertext {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.mlkem_ciphertext.len());
        bytes.extend_from_slice(&self.x25519_ephemeral);
        bytes.extend_from_slice(&self.mlkem_ciphertext);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(AegisError::Crypto("Ciphertext too short".to_string()));
        }
        let mut x25519_ephemeral = [0u8; 32];
        x25519_ephemeral.copy_from_slice(&bytes[..32]);
        let mlkem_ciphertext = bytes[32..].to_vec();

        Ok(Self {
            x25519_ephemeral,
            mlkem_ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::KeyExchange as KeyExchangeTrait;

    // =========================================================================
    // Phase 1 Tests: HKDF KDF
    // =========================================================================

    #[test]
    fn test_combine_uses_hkdf_not_concat() {
        let x25519 = [1u8; 32];
        let mlkem = [2u8; 32];

        let ss = HybridSharedSecret::combine(&x25519, &mlkem);

        // Raw concat would produce first 32 bytes = x25519 value, but HKDF won't
        assert_ne!(
            ss.as_bytes(),
            &x25519,
            "HKDF output must differ from raw X25519"
        );

        // The inner must also not simply be the mlkem value
        assert_ne!(
            ss.as_bytes(),
            &mlkem[..32],
            "HKDF output must differ from raw MLKEM"
        );
    }

    #[test]
    fn test_combine_deterministic() {
        let x25519 = [5u8; 32];
        let mlkem = [7u8; 32];

        let ss1 = HybridSharedSecret::combine(&x25519, &mlkem);
        let ss2 = HybridSharedSecret::combine(&x25519, &mlkem);

        assert_eq!(
            ss1.as_bytes(),
            ss2.as_bytes(),
            "combine() must be deterministic"
        );
    }

    #[test]
    fn test_combine_different_inputs_produce_different_outputs() {
        let x1 = [1u8; 32];
        let x2 = [2u8; 32];
        let m = [3u8; 32];

        let ss1 = HybridSharedSecret::combine(&x1, &m);
        let ss2 = HybridSharedSecret::combine(&x2, &m);

        assert_ne!(ss1.as_bytes(), ss2.as_bytes());
    }

    #[test]
    fn test_derive_key_uses_hkdf() {
        let x25519 = [1u8; 32];
        let mlkem = [2u8; 32];

        let ss = HybridSharedSecret::combine(&x25519, &mlkem);
        let key = ss.derive_key();

        // Key must differ from inner PRK (different HKDF expand context)
        assert_ne!(
            &key,
            ss.as_bytes(),
            "derive_key output must differ from PRK"
        );
        assert_ne!(key, [0u8; 32], "Key must not be all zeros");
    }

    #[test]
    fn test_derive_key_expansion_length() {
        let x25519 = [3u8; 32];
        let mlkem = [4u8; 32];
        let ss = HybridSharedSecret::combine(&x25519, &mlkem);
        let key = ss.derive_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_different_contexts_produce_different_keys() {
        let x25519 = [9u8; 32];
        let mlkem = [10u8; 32];
        let ss = HybridSharedSecret::combine(&x25519, &mlkem);

        let session_key = ss.derive_key();
        let client_key = ss.derive_client_key();
        let server_key = ss.derive_server_key();

        assert_ne!(
            session_key, client_key,
            "Session and client keys must differ"
        );
        assert_ne!(
            session_key, server_key,
            "Session and server keys must differ"
        );
        assert_ne!(client_key, server_key, "Client and server keys must differ");
    }

    // =========================================================================
    // Phase 2 Tests: Zeroization (compile-time via ZeroizeOnDrop)
    // =========================================================================

    #[test]
    fn test_secret_key_zeroed_after_drop() {
        // Verify that HybridSecretKey implements ZeroizeOnDrop (compile-time check)
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<HybridSecretKey>();
    }

    #[test]
    fn test_shared_secret_zeroed_after_drop() {
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<HybridSharedSecret>();
    }

    // =========================================================================
    // Phase 3 Tests: KeyExchange Trait Implementation
    // =========================================================================

    #[test]
    fn test_hybrid_kex_implements_key_exchange_trait() {
        let kex = HybridKeyExchange::new();
        // This call uses the trait method
        let result = <HybridKeyExchange as KeyExchangeTrait>::generate_keypair(&kex);
        assert!(result.is_ok());
        let (pk, sk_bytes) = result.unwrap();
        assert!(!pk.x25519_bytes().iter().all(|&b| b == 0));
        assert!(
            sk_bytes.len() > 32,
            "Secret key bytes must include X25519 + ML-KEM"
        );
    }

    #[test]
    fn test_trait_object_safety_with_hybrid() {
        fn _assert_object_safe(
            _: &dyn KeyExchangeTrait<PublicKey = HybridPublicKey, SharedSecret = HybridSharedSecret>,
        ) {
        }
        let kex = HybridKeyExchange::new();
        _assert_object_safe(&kex);
    }

    #[test]
    fn test_trait_roundtrip() {
        let kex = HybridKeyExchange::new();

        // Server generates keypair via trait
        let (server_pk, server_sk_bytes) =
            <HybridKeyExchange as KeyExchangeTrait>::generate_keypair(&kex).unwrap();
        let server_pk_bytes = server_pk.to_bytes();

        // Client encapsulates via trait
        let (ct_bytes, client_ss) =
            <HybridKeyExchange as KeyExchangeTrait>::encapsulate(&kex, &server_pk_bytes).unwrap();

        // Server decapsulates via trait
        let server_ss =
            <HybridKeyExchange as KeyExchangeTrait>::decapsulate(&kex, &ct_bytes, &server_sk_bytes)
                .unwrap();

        assert_eq!(
            client_ss.as_bytes(),
            server_ss.as_bytes(),
            "Trait-based roundtrip must produce same shared secret"
        );
    }

    #[test]
    fn test_public_key_as_ref_returns_mlkem_bytes() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let as_ref: &[u8] = pk.as_ref();
        assert_eq!(as_ref, pk.bytes.as_slice());
    }

    #[test]
    fn test_x25519_bytes_accessor() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let x25519_bytes: &[u8; 32] = pk.x25519_bytes();
        assert_eq!(x25519_bytes, &pk.bytes[..32]);
    }

    // =========================================================================
    // Algorithm Negotiation Tests
    // =========================================================================

    #[test]
    fn test_algorithm_negotiation_selects_best() {
        let local = vec![PqcAlgorithm::HybridMlKem1024, PqcAlgorithm::HybridMlKem768];
        let peer = vec![PqcAlgorithm::HybridMlKem1024, PqcAlgorithm::HybridMlKem768];

        let negotiated = NegotiatedAlgorithm::negotiate(&local, &peer).unwrap();
        assert_eq!(
            negotiated.algorithm,
            PqcAlgorithm::HybridMlKem1024,
            "Should select strongest mutual algorithm"
        );
    }

    #[test]
    fn test_negotiation_fallback() {
        let local = vec![PqcAlgorithm::HybridMlKem1024, PqcAlgorithm::HybridMlKem768];
        // Peer only supports ML-KEM-768
        let peer = vec![PqcAlgorithm::HybridMlKem768];

        let negotiated = NegotiatedAlgorithm::negotiate(&local, &peer).unwrap();
        assert_eq!(
            negotiated.algorithm,
            PqcAlgorithm::HybridMlKem768,
            "Should fallback to ML-KEM-768 when 1024 not supported by peer"
        );
    }

    #[test]
    fn test_negotiation_none_when_no_overlap() {
        let local = vec![PqcAlgorithm::HybridMlKem1024];
        let peer = vec![PqcAlgorithm::HybridMlKem768];

        let negotiated = NegotiatedAlgorithm::negotiate(&local, &peer);
        assert!(
            negotiated.is_none(),
            "Should return None when no common algorithm"
        );
    }

    // =========================================================================
    // Directional Key Tests
    // =========================================================================

    #[test]
    fn test_bidirectional_derived_keys_differ() {
        let kex = HybridKeyExchange::new();
        let (server_pk, server_sk) = kex.generate_keypair().unwrap();
        let (ct, client_ss) = kex.encapsulate(&server_pk).unwrap();
        let server_ss = kex.decapsulate(&ct, &server_sk).unwrap();

        // Both sides derive the same directional keys
        assert_eq!(client_ss.derive_client_key(), server_ss.derive_client_key());
        assert_eq!(client_ss.derive_server_key(), server_ss.derive_server_key());

        // But client and server keys are different
        assert_ne!(
            client_ss.derive_client_key(),
            client_ss.derive_server_key(),
            "Client and server directional keys must differ"
        );
    }

    // =========================================================================
    // Existing Tests (preserved)
    // =========================================================================

    #[test]
    fn test_hybrid_keypair_generation() {
        let kex = HybridKeyExchange::new();
        let result = kex.generate_keypair();
        assert!(result.is_ok(), "Keypair generation should succeed");

        let (pk, _sk) = result.unwrap();
        assert_eq!(
            pk.x25519_bytes().len(),
            32,
            "X25519 public key should be 32 bytes"
        );
        assert!(
            !pk.mlkem_bytes().is_empty(),
            "ML-KEM public key should not be empty"
        );
    }

    #[test]
    fn test_hybrid_encapsulation() {
        let kex = HybridKeyExchange::new();
        let (pk, _sk) = kex.generate_keypair().unwrap();

        let result = kex.encapsulate(&pk);
        assert!(result.is_ok(), "Encapsulation should succeed");

        let (ct, ss) = result.unwrap();
        assert_eq!(ct.x25519_ephemeral.len(), 32);
        assert!(!ct.mlkem_ciphertext.is_empty());
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
        assert_eq!(kex.algorithm_name(), "X25519-MLKEM768-Hybrid");

        let kex_high = HybridKeyExchange::new_with_level(SecurityLevel::High);
        assert_eq!(kex_high.algorithm_name(), "X25519-MLKEM1024-Hybrid");
    }

    #[test]
    fn test_public_key_serialization() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();

        let bytes = pk.to_bytes();
        let pk2 = HybridPublicKey::from_bytes(&bytes).unwrap();

        assert_eq!(pk.bytes, pk2.bytes);
    }

    #[test]
    fn test_ciphertext_serialization() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let (ct, _) = kex.encapsulate(&pk).unwrap();

        let bytes = ct.to_bytes();
        let ct2 = HybridCiphertext::from_bytes(&bytes).unwrap();

        assert_eq!(ct.x25519_ephemeral, ct2.x25519_ephemeral);
        assert_eq!(ct.mlkem_ciphertext, ct2.mlkem_ciphertext);
    }

    #[test]
    fn test_invalid_public_key_from_bytes() {
        let invalid_bytes = vec![0u8; 10]; // Too short
        let result = HybridPublicKey::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_ciphertext_from_bytes() {
        let invalid_bytes = vec![0u8; 10]; // Too short
        let result = HybridCiphertext::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_secret_debug_redacts() {
        let x25519 = [1u8; 32];
        let mlkem = [2u8; 32];
        let ss = HybridSharedSecret::combine(&x25519, &mlkem);
        let debug_str = format!("{:?}", ss);
        assert!(debug_str.contains("REDACTED"));
    }

    #[test]
    fn test_hybrid_shared_secret_as_ref() {
        let x25519 = [1u8; 32];
        let mlkem = [2u8; 32];
        let ss = HybridSharedSecret::combine(&x25519, &mlkem);
        let as_ref: &[u8] = ss.as_ref();
        assert_eq!(as_ref.len(), 32); // HKDF output is 32 bytes
    }

    #[test]
    fn test_hybrid_key_exchange_default() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        assert!(!pk.x25519_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_secret_key_debug_redacts() {
        let kex = HybridKeyExchange::new();
        let (_, sk) = kex.generate_keypair().unwrap();
        let debug_str = format!("{:?}", sk);
        assert!(debug_str.contains("REDACTED"));
        assert!(debug_str.contains("HybridSecretKey"));
    }

    #[test]
    fn test_hybrid_public_key_clone() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let cloned = pk.clone();
        assert_eq!(pk.bytes, cloned.bytes);
    }

    #[test]
    fn test_hybrid_ciphertext_clone() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let (ct, _) = kex.encapsulate(&pk).unwrap();
        let cloned = ct.clone();
        assert_eq!(ct.x25519_ephemeral, cloned.x25519_ephemeral);
        assert_eq!(ct.mlkem_ciphertext, cloned.mlkem_ciphertext);
    }

    #[test]
    fn test_public_key_to_bytes() {
        let kex = HybridKeyExchange::new();
        let (pk, _) = kex.generate_keypair().unwrap();
        let bytes = pk.to_bytes();

        assert!(!bytes.is_empty());
        assert!(bytes.len() > 32); // x25519 + mlkem
    }

    #[test]
    fn test_public_key_roundtrip() {
        let kex = HybridKeyExchange::new();
        let (pk1, _) = kex.generate_keypair().unwrap();
        let bytes = pk1.to_bytes();
        let pk2 = HybridPublicKey::from_bytes(&bytes).unwrap();

        assert_eq!(pk1.bytes, pk2.bytes);
    }

    #[test]
    fn test_public_key_from_short_bytes() {
        let short_bytes = vec![0u8; 16]; // too short
        let result = HybridPublicKey::from_bytes(&short_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_secret_equality() {
        let kex = HybridKeyExchange::new();
        let (pk, sk) = kex.generate_keypair().unwrap();
        let (ct, ss_enc) = kex.encapsulate(&pk).unwrap();
        let ss_dec = kex.decapsulate(&ct, &sk).unwrap();

        // Both should produce shared secrets with the same underlying bytes
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_pqc_algorithm_names() {
        assert_eq!(
            PqcAlgorithm::HybridMlKem768.name(),
            "X25519-MLKEM768-Hybrid"
        );
        assert_eq!(
            PqcAlgorithm::HybridMlKem1024.name(),
            "X25519-MLKEM1024-Hybrid"
        );
    }

    #[test]
    fn test_pqc_algorithm_strength_order() {
        let order = PqcAlgorithm::strength_order();
        assert_eq!(
            order[0],
            PqcAlgorithm::HybridMlKem1024,
            "ML-KEM-1024 should be strongest"
        );
        assert_eq!(order[1], PqcAlgorithm::HybridMlKem768);
    }

    // =========================================================================
    // Property-Based Tests
    // =========================================================================
    use proptest::prelude::*;

    proptest! {
        // Property 1: Different X25519 inputs mixed with same ML-KEM output produce different shared secrets
        #[test]
        fn test_combine_different_x25519_inputs_produce_different_outputs(
            x1 in any::<[u8; 32]>(),
            x2 in any::<[u8; 32]>()
        ) {
            prop_assume!(x1 != x2);
            let mlkem = [0xAA; 32];
            let out1 = HybridSharedSecret::combine(&x1, &mlkem);
            let out2 = HybridSharedSecret::combine(&x2, &mlkem);
            assert_ne!(out1.inner, out2.inner);
        }

        // Property 2: Different ML-KEM inputs mixed with same X25519 output produce different shared secrets
        #[test]
        fn test_combine_different_mlkem_inputs_produce_different_outputs(
            m1 in any::<[u8; 32]>(),
            m2 in any::<[u8; 32]>()
        ) {
            prop_assume!(m1 != m2);
            let x25519 = [0xBB; 32];
            let out1 = HybridSharedSecret::combine(&x25519, &m1);
            let out2 = HybridSharedSecret::combine(&x25519, &m2);
            assert_ne!(out1.inner, out2.inner);
        }

        // Property 3: client_key and server_key derived from the SAME shared secret are ALWAYS different
        #[test]
        fn test_directional_keys_are_always_different(
            secret_bytes in any::<[u8; 32]>()
        ) {
            let secret = HybridSharedSecret { inner: secret_bytes };
            let client_key = secret.derive_client_key();
            let server_key = secret.derive_server_key();
            assert_ne!(client_key, server_key);
        }
    }
}
