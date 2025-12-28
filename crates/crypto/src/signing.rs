//! ML-DSA Digital Signatures (NIST FIPS 204)
//!
//! This module implements post-quantum digital signatures using ML-DSA
//! (formerly Dilithium) for quantum-resistant signing operations.
//!
//! # Security Levels
//!
//! - **ML-DSA-44**: Security Level 2 (~128-bit classical, ~64-bit quantum)
//! - **ML-DSA-65**: Security Level 3 (~192-bit classical, ~128-bit quantum) [Default]
//! - **ML-DSA-87**: Security Level 5 (~256-bit classical, ~192-bit quantum)
//!
//! # Example
//!
//! ```ignore
//! use aegis_crypto::signing::{MlDsa65Signer, SigningKeyPair};
//!
//! let signer = MlDsa65Signer::generate()?;
//! let message = b"Hello, quantum world!";
//!
//! // Sign
//! let signature = signer.sign(message)?;
//!
//! // Verify
//! assert!(signer.verify(message, &signature)?);
//! ```

use aegis_common::{AegisError, Result};
use pqcrypto_mldsa::{mldsa44, mldsa65, mldsa87};
use pqcrypto_traits::sign::{
    DetachedSignature, PublicKey as PqcPublicKey, SecretKey as PqcSecretKey,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// Algorithm identifier for ML-DSA variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MlDsaAlgorithm {
    /// ML-DSA-44 (Security Level 2)
    MlDsa44,
    /// ML-DSA-65 (Security Level 3) - Default
    #[default]
    MlDsa65,
    /// ML-DSA-87 (Security Level 5)
    MlDsa87,
}

impl MlDsaAlgorithm {
    /// Get the algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            Self::MlDsa44 => "ML-DSA-44",
            Self::MlDsa65 => "ML-DSA-65",
            Self::MlDsa87 => "ML-DSA-87",
        }
    }

    /// Get the NIST security level
    pub fn security_level(&self) -> u8 {
        match self {
            Self::MlDsa44 => 2,
            Self::MlDsa65 => 3,
            Self::MlDsa87 => 5,
        }
    }

    /// Get public key size in bytes
    pub fn public_key_size(&self) -> usize {
        match self {
            Self::MlDsa44 => 1312,
            Self::MlDsa65 => 1952,
            Self::MlDsa87 => 2592,
        }
    }

    /// Get approximate signature size in bytes (may vary slightly by implementation)
    pub fn signature_size_approx(&self) -> usize {
        match self {
            Self::MlDsa44 => 2420,
            Self::MlDsa65 => 3309, // Actual from pqcrypto-mldsa
            Self::MlDsa87 => 4627, // Actual from pqcrypto-mldsa
        }
    }
}

/// Trait for digital signature key pairs
pub trait SigningKeyPair: Send + Sync {
    /// Generate a new key pair
    fn generate() -> Result<Self>
    where
        Self: Sized;

    /// Sign a message
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;

    /// Verify a signature
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool>;

    /// Get the public key bytes
    fn public_key(&self) -> &[u8];

    /// Get the algorithm name
    fn algorithm(&self) -> MlDsaAlgorithm;
}

/// Signature with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaSignature {
    /// The raw signature bytes
    pub bytes: Vec<u8>,
    /// Algorithm used for signing
    pub algorithm: MlDsaAlgorithm,
}

impl MlDsaSignature {
    /// Create a new signature
    pub fn new(bytes: Vec<u8>, algorithm: MlDsaAlgorithm) -> Self {
        Self { bytes, algorithm }
    }

    /// Get signature as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

// ============================================================================
// ML-DSA-44 Implementation
// ============================================================================

/// ML-DSA-44 signer (Security Level 2)
pub struct MlDsa44Signer {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl MlDsa44Signer {
    /// Create from existing keys
    pub fn from_keys(public_key: Vec<u8>, secret_key: Vec<u8>) -> Result<Self> {
        if public_key.len() != MlDsaAlgorithm::MlDsa44.public_key_size() {
            return Err(AegisError::Crypto(format!(
                "Invalid ML-DSA-44 public key size: expected {}, got {}",
                MlDsaAlgorithm::MlDsa44.public_key_size(),
                public_key.len()
            )));
        }
        Ok(Self {
            public_key,
            secret_key,
        })
    }
}

impl SigningKeyPair for MlDsa44Signer {
    #[instrument(skip_all)]
    fn generate() -> Result<Self> {
        debug!("Generating ML-DSA-44 key pair");
        let (pk, sk) = mldsa44::keypair();

        Ok(Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        })
    }

    #[instrument(skip_all)]
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        debug!(message_len = message.len(), "Signing with ML-DSA-44");

        let sk = mldsa44::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid secret key: {:?}", e)))?;

        let sig = mldsa44::detached_sign(message, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    #[instrument(skip_all)]
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        debug!(
            message_len = message.len(),
            sig_len = signature.len(),
            "Verifying ML-DSA-44 signature"
        );

        let pk = mldsa44::PublicKey::from_bytes(&self.public_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;

        let sig = mldsa44::DetachedSignature::from_bytes(signature)
            .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;

        match mldsa44::verify_detached_signature(&sig, message, &pk) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    fn algorithm(&self) -> MlDsaAlgorithm {
        MlDsaAlgorithm::MlDsa44
    }
}

impl std::fmt::Debug for MlDsa44Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlDsa44Signer")
            .field("public_key", &format!("[{} bytes]", self.public_key.len()))
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

// ============================================================================
// ML-DSA-65 Implementation (Default)
// ============================================================================

/// ML-DSA-65 signer (Security Level 3) - Recommended default
pub struct MlDsa65Signer {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl MlDsa65Signer {
    /// Create from existing keys
    pub fn from_keys(public_key: Vec<u8>, secret_key: Vec<u8>) -> Result<Self> {
        if public_key.len() != MlDsaAlgorithm::MlDsa65.public_key_size() {
            return Err(AegisError::Crypto(format!(
                "Invalid ML-DSA-65 public key size: expected {}, got {}",
                MlDsaAlgorithm::MlDsa65.public_key_size(),
                public_key.len()
            )));
        }
        Ok(Self {
            public_key,
            secret_key,
        })
    }
}

impl SigningKeyPair for MlDsa65Signer {
    #[instrument(skip_all)]
    fn generate() -> Result<Self> {
        debug!("Generating ML-DSA-65 key pair");
        let (pk, sk) = mldsa65::keypair();

        Ok(Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        })
    }

    #[instrument(skip_all)]
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        debug!(message_len = message.len(), "Signing with ML-DSA-65");

        let sk = mldsa65::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid secret key: {:?}", e)))?;

        let sig = mldsa65::detached_sign(message, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    #[instrument(skip_all)]
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        debug!(
            message_len = message.len(),
            sig_len = signature.len(),
            "Verifying ML-DSA-65 signature"
        );

        let pk = mldsa65::PublicKey::from_bytes(&self.public_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;

        let sig = mldsa65::DetachedSignature::from_bytes(signature)
            .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;

        match mldsa65::verify_detached_signature(&sig, message, &pk) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    fn algorithm(&self) -> MlDsaAlgorithm {
        MlDsaAlgorithm::MlDsa65
    }
}

impl std::fmt::Debug for MlDsa65Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlDsa65Signer")
            .field("public_key", &format!("[{} bytes]", self.public_key.len()))
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

// ============================================================================
// ML-DSA-87 Implementation
// ============================================================================

/// ML-DSA-87 signer (Security Level 5)
pub struct MlDsa87Signer {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl MlDsa87Signer {
    /// Create from existing keys
    pub fn from_keys(public_key: Vec<u8>, secret_key: Vec<u8>) -> Result<Self> {
        if public_key.len() != MlDsaAlgorithm::MlDsa87.public_key_size() {
            return Err(AegisError::Crypto(format!(
                "Invalid ML-DSA-87 public key size: expected {}, got {}",
                MlDsaAlgorithm::MlDsa87.public_key_size(),
                public_key.len()
            )));
        }
        Ok(Self {
            public_key,
            secret_key,
        })
    }
}

impl SigningKeyPair for MlDsa87Signer {
    #[instrument(skip_all)]
    fn generate() -> Result<Self> {
        debug!("Generating ML-DSA-87 key pair");
        let (pk, sk) = mldsa87::keypair();

        Ok(Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        })
    }

    #[instrument(skip_all)]
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        debug!(message_len = message.len(), "Signing with ML-DSA-87");

        let sk = mldsa87::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid secret key: {:?}", e)))?;

        let sig = mldsa87::detached_sign(message, &sk);
        Ok(sig.as_bytes().to_vec())
    }

    #[instrument(skip_all)]
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        debug!(
            message_len = message.len(),
            sig_len = signature.len(),
            "Verifying ML-DSA-87 signature"
        );

        let pk = mldsa87::PublicKey::from_bytes(&self.public_key)
            .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;

        let sig = mldsa87::DetachedSignature::from_bytes(signature)
            .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;

        match mldsa87::verify_detached_signature(&sig, message, &pk) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    fn algorithm(&self) -> MlDsaAlgorithm {
        MlDsaAlgorithm::MlDsa87
    }
}

impl std::fmt::Debug for MlDsa87Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlDsa87Signer")
            .field("public_key", &format!("[{} bytes]", self.public_key.len()))
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

// ============================================================================
// Signature Verifier (Public Key Only)
// ============================================================================

/// Verifier for ML-DSA signatures (public key only)
pub struct MlDsaVerifier {
    public_key: Vec<u8>,
    algorithm: MlDsaAlgorithm,
}

impl MlDsaVerifier {
    /// Create a verifier from a public key
    pub fn new(public_key: Vec<u8>, algorithm: MlDsaAlgorithm) -> Result<Self> {
        let expected_size = algorithm.public_key_size();
        if public_key.len() != expected_size {
            return Err(AegisError::Crypto(format!(
                "Invalid {} public key size: expected {}, got {}",
                algorithm.name(),
                expected_size,
                public_key.len()
            )));
        }
        Ok(Self {
            public_key,
            algorithm,
        })
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        match self.algorithm {
            MlDsaAlgorithm::MlDsa44 => {
                let pk = mldsa44::PublicKey::from_bytes(&self.public_key)
                    .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;
                let sig = mldsa44::DetachedSignature::from_bytes(signature)
                    .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;
                match mldsa44::verify_detached_signature(&sig, message, &pk) {
                    Ok(()) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            MlDsaAlgorithm::MlDsa65 => {
                let pk = mldsa65::PublicKey::from_bytes(&self.public_key)
                    .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;
                let sig = mldsa65::DetachedSignature::from_bytes(signature)
                    .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;
                match mldsa65::verify_detached_signature(&sig, message, &pk) {
                    Ok(()) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            MlDsaAlgorithm::MlDsa87 => {
                let pk = mldsa87::PublicKey::from_bytes(&self.public_key)
                    .map_err(|e| AegisError::Crypto(format!("Invalid public key: {:?}", e)))?;
                let sig = mldsa87::DetachedSignature::from_bytes(signature)
                    .map_err(|e| AegisError::Crypto(format!("Invalid signature: {:?}", e)))?;
                match mldsa87::verify_detached_signature(&sig, message, &pk) {
                    Ok(()) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Get the algorithm
    pub fn algorithm(&self) -> MlDsaAlgorithm {
        self.algorithm
    }

    /// Get the public key
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl std::fmt::Debug for MlDsaVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlDsaVerifier")
            .field("algorithm", &self.algorithm)
            .field("public_key", &format!("[{} bytes]", self.public_key.len()))
            .finish()
    }
}

// ============================================================================
// Hybrid Signer (ML-DSA + Ed25519)
// ============================================================================

/// Hybrid signature format tag
const HYBRID_SIGNATURE_TAG: u8 = 0x01;
/// Pure ML-DSA signature tag (no Ed25519) - reserved for future use
#[allow(dead_code)]
const MLDSA_ONLY_TAG: u8 = 0x00;

/// Hybrid signature combining ML-DSA-65 and Ed25519
///
/// This provides both:
/// - Quantum resistance via ML-DSA-65
/// - Backwards compatibility for classical verification via Ed25519
///
/// Signature format: [tag: 1 byte][ed25519_sig: 64 bytes][mldsa_sig: variable]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSignature {
    /// Ed25519 signature (64 bytes)
    pub ed25519_sig: Vec<u8>,
    /// ML-DSA-65 signature
    pub mldsa_sig: Vec<u8>,
}

impl HybridSignature {
    /// Serialize to bytes with tag prefix
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + 64 + self.mldsa_sig.len());
        bytes.push(HYBRID_SIGNATURE_TAG);
        bytes.extend_from_slice(&self.ed25519_sig);
        bytes.extend_from_slice(&self.mldsa_sig);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(AegisError::Crypto("Empty signature".to_string()));
        }

        if bytes[0] != HYBRID_SIGNATURE_TAG {
            return Err(AegisError::Crypto(
                "Not a hybrid signature (wrong tag)".to_string(),
            ));
        }

        if bytes.len() < 1 + 64 {
            return Err(AegisError::Crypto(
                "Hybrid signature too short for Ed25519 component".to_string(),
            ));
        }

        Ok(Self {
            ed25519_sig: bytes[1..65].to_vec(),
            mldsa_sig: bytes[65..].to_vec(),
        })
    }
}

/// Hybrid public key (Ed25519 + ML-DSA-65)
#[derive(Debug, Clone)]
pub struct HybridSigningPublicKey {
    /// Ed25519 public key (32 bytes)
    pub ed25519_pk: Vec<u8>,
    /// ML-DSA-65 public key
    pub mldsa_pk: Vec<u8>,
}

impl HybridSigningPublicKey {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.mldsa_pk.len());
        bytes.extend_from_slice(&self.ed25519_pk);
        bytes.extend_from_slice(&self.mldsa_pk);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 + MlDsaAlgorithm::MlDsa65.public_key_size() {
            return Err(AegisError::Crypto(
                "Hybrid public key too short".to_string(),
            ));
        }

        Ok(Self {
            ed25519_pk: bytes[..32].to_vec(),
            mldsa_pk: bytes[32..].to_vec(),
        })
    }
}

/// Hybrid signer combining ML-DSA-65 and Ed25519
///
/// Provides quantum-resistant signatures with classical fallback.
/// Both signatures are generated for every sign operation.
pub struct HybridSigner {
    /// Ed25519 signing key
    ed25519_signing_key: ed25519_dalek::SigningKey,
    /// ML-DSA-65 signer
    mldsa_signer: MlDsa65Signer,
}

impl HybridSigner {
    /// Generate a new hybrid key pair
    #[instrument(skip_all)]
    pub fn generate() -> Result<Self> {
        debug!("Generating hybrid key pair (Ed25519 + ML-DSA-65)");

        // Generate Ed25519 key pair
        let mut csprng = rand::rngs::OsRng;
        let ed25519_signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);

        // Generate ML-DSA-65 key pair
        let mldsa_signer = MlDsa65Signer::generate()?;

        debug!("Hybrid key pair generated successfully");
        Ok(Self {
            ed25519_signing_key,
            mldsa_signer,
        })
    }

    /// Get the hybrid public key
    pub fn public_key(&self) -> HybridSigningPublicKey {
        HybridSigningPublicKey {
            ed25519_pk: self.ed25519_signing_key.verifying_key().to_bytes().to_vec(),
            mldsa_pk: self.mldsa_signer.public_key().to_vec(),
        }
    }

    /// Sign a message with both algorithms
    #[instrument(skip_all)]
    pub fn sign(&self, message: &[u8]) -> Result<HybridSignature> {
        debug!(
            message_len = message.len(),
            "Signing with hybrid (Ed25519 + ML-DSA-65)"
        );

        // Ed25519 signature
        use ed25519_dalek::Signer;
        let ed25519_sig = self.ed25519_signing_key.sign(message);

        // ML-DSA-65 signature
        let mldsa_sig = self.mldsa_signer.sign(message)?;

        Ok(HybridSignature {
            ed25519_sig: ed25519_sig.to_bytes().to_vec(),
            mldsa_sig,
        })
    }

    /// Verify a hybrid signature (both components must be valid)
    #[instrument(skip_all)]
    pub fn verify(&self, message: &[u8], signature: &HybridSignature) -> Result<bool> {
        debug!("Verifying hybrid signature");

        // Verify Ed25519
        let ed25519_valid = self.verify_ed25519(message, &signature.ed25519_sig)?;
        if !ed25519_valid {
            debug!("Ed25519 signature verification failed");
            return Ok(false);
        }

        // Verify ML-DSA-65
        let mldsa_valid = self.mldsa_signer.verify(message, &signature.mldsa_sig)?;
        if !mldsa_valid {
            debug!("ML-DSA-65 signature verification failed");
            return Ok(false);
        }

        debug!("Hybrid signature verified successfully");
        Ok(true)
    }

    /// Verify only the Ed25519 component (for classical compatibility)
    pub fn verify_ed25519(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        if signature.len() != 64 {
            return Err(AegisError::Crypto(format!(
                "Invalid Ed25519 signature length: expected 64, got {}",
                signature.len()
            )));
        }

        use ed25519_dalek::Verifier;
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| AegisError::Crypto("Invalid Ed25519 signature length".to_string()))?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        let verifying_key = self.ed25519_signing_key.verifying_key();

        match verifying_key.verify(message, &sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify only the ML-DSA component (quantum-resistant only)
    pub fn verify_mldsa(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        self.mldsa_signer.verify(message, signature)
    }
}

impl std::fmt::Debug for HybridSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridSigner")
            .field("ed25519_pk", &"[32 bytes]")
            .field("mldsa_pk", &"[variable bytes]")
            .finish()
    }
}

/// Hybrid signature verifier (public keys only)
pub struct HybridVerifier {
    /// Ed25519 verifying key
    ed25519_verifying_key: ed25519_dalek::VerifyingKey,
    /// ML-DSA verifier
    mldsa_verifier: MlDsaVerifier,
}

impl HybridVerifier {
    /// Create from hybrid public key
    pub fn new(public_key: &HybridSigningPublicKey) -> Result<Self> {
        // Parse Ed25519 public key
        let ed25519_bytes: [u8; 32] = public_key.ed25519_pk[..]
            .try_into()
            .map_err(|_| AegisError::Crypto("Invalid Ed25519 public key length".to_string()))?;
        let ed25519_verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&ed25519_bytes)
            .map_err(|e| AegisError::Crypto(format!("Invalid Ed25519 public key: {:?}", e)))?;

        // Create ML-DSA verifier
        let mldsa_verifier =
            MlDsaVerifier::new(public_key.mldsa_pk.clone(), MlDsaAlgorithm::MlDsa65)?;

        Ok(Self {
            ed25519_verifying_key,
            mldsa_verifier,
        })
    }

    /// Verify a hybrid signature (both components must be valid)
    pub fn verify(&self, message: &[u8], signature: &HybridSignature) -> Result<bool> {
        // Verify Ed25519
        if signature.ed25519_sig.len() != 64 {
            return Ok(false);
        }

        use ed25519_dalek::Verifier;
        let sig_bytes: [u8; 64] = signature.ed25519_sig[..]
            .try_into()
            .map_err(|_| AegisError::Crypto("Invalid Ed25519 signature length".to_string()))?;
        let ed25519_sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        if self
            .ed25519_verifying_key
            .verify(message, &ed25519_sig)
            .is_err()
        {
            return Ok(false);
        }

        // Verify ML-DSA
        self.mldsa_verifier.verify(message, &signature.mldsa_sig)
    }

    /// Verify only the ML-DSA component
    pub fn verify_mldsa_only(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        self.mldsa_verifier.verify(message, signature)
    }
}

impl std::fmt::Debug for HybridVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridVerifier")
            .field("ed25519_pk", &"[32 bytes]")
            .field("mldsa_algorithm", &self.mldsa_verifier.algorithm())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mldsa44_keygen() {
        let signer = MlDsa44Signer::generate().unwrap();
        assert_eq!(
            signer.public_key().len(),
            MlDsaAlgorithm::MlDsa44.public_key_size()
        );
        assert_eq!(signer.algorithm(), MlDsaAlgorithm::MlDsa44);
    }

    #[test]
    fn test_mldsa44_sign_verify() {
        let signer = MlDsa44Signer::generate().unwrap();
        let message = b"Test message for ML-DSA-44";

        let signature = signer.sign(message).unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");

        let valid = signer.verify(message, &signature).unwrap();
        assert!(valid, "Signature should be valid");

        // Verify with wrong message
        let wrong_message = b"Wrong message";
        let invalid = signer.verify(wrong_message, &signature).unwrap();
        assert!(!invalid, "Signature should be invalid for wrong message");
    }

    #[test]
    fn test_mldsa65_keygen() {
        let signer = MlDsa65Signer::generate().unwrap();
        assert_eq!(
            signer.public_key().len(),
            MlDsaAlgorithm::MlDsa65.public_key_size()
        );
        assert_eq!(signer.algorithm(), MlDsaAlgorithm::MlDsa65);
    }

    #[test]
    fn test_mldsa65_sign_verify() {
        let signer = MlDsa65Signer::generate().unwrap();
        let message = b"Test message for ML-DSA-65";

        let signature = signer.sign(message).unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");

        let valid = signer.verify(message, &signature).unwrap();
        assert!(valid, "Signature should be valid");
    }

    #[test]
    fn test_mldsa87_keygen() {
        let signer = MlDsa87Signer::generate().unwrap();
        assert_eq!(
            signer.public_key().len(),
            MlDsaAlgorithm::MlDsa87.public_key_size()
        );
        assert_eq!(signer.algorithm(), MlDsaAlgorithm::MlDsa87);
    }

    #[test]
    fn test_mldsa87_sign_verify() {
        let signer = MlDsa87Signer::generate().unwrap();
        let message = b"Test message for ML-DSA-87";

        let signature = signer.sign(message).unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");

        let valid = signer.verify(message, &signature).unwrap();
        assert!(valid, "Signature should be valid");
    }

    #[test]
    fn test_verifier_standalone() {
        let signer = MlDsa65Signer::generate().unwrap();
        let message = b"Message to verify";
        let signature = signer.sign(message).unwrap();

        // Create verifier from public key only
        let verifier =
            MlDsaVerifier::new(signer.public_key().to_vec(), MlDsaAlgorithm::MlDsa65).unwrap();

        let valid = verifier.verify(message, &signature).unwrap();
        assert!(valid, "Verifier should validate the signature");
    }

    #[test]
    fn test_algorithm_properties() {
        assert_eq!(MlDsaAlgorithm::MlDsa44.security_level(), 2);
        assert_eq!(MlDsaAlgorithm::MlDsa65.security_level(), 3);
        assert_eq!(MlDsaAlgorithm::MlDsa87.security_level(), 5);

        assert_eq!(MlDsaAlgorithm::default(), MlDsaAlgorithm::MlDsa65);
    }

    #[test]
    fn test_tampered_signature_fails() {
        let signer = MlDsa65Signer::generate().unwrap();
        let message = b"Original message";
        let mut signature = signer.sign(message).unwrap();

        // Tamper with signature
        signature[0] ^= 0xFF;

        let valid = signer.verify(message, &signature).unwrap();
        assert!(!valid, "Tampered signature should be invalid");
    }

    #[test]
    fn test_mldsa_signature_struct() {
        let signer = MlDsa65Signer::generate().unwrap();
        let message = b"Test message";
        let sig_bytes = signer.sign(message).unwrap();

        let sig = MlDsaSignature::new(sig_bytes.clone(), MlDsaAlgorithm::MlDsa65);
        assert_eq!(sig.as_bytes(), &sig_bytes);
        assert_eq!(sig.algorithm, MlDsaAlgorithm::MlDsa65);
    }

    // ========================================================================
    // Hybrid Signer Tests
    // ========================================================================

    #[test]
    fn test_hybrid_signer_generate() {
        let signer = HybridSigner::generate().unwrap();
        let pk = signer.public_key();

        assert_eq!(
            pk.ed25519_pk.len(),
            32,
            "Ed25519 public key should be 32 bytes"
        );
        assert_eq!(
            pk.mldsa_pk.len(),
            MlDsaAlgorithm::MlDsa65.public_key_size(),
            "ML-DSA-65 public key size"
        );
    }

    #[test]
    fn test_hybrid_sign_verify() {
        let signer = HybridSigner::generate().unwrap();
        let message = b"Hybrid signature test message";

        let signature = signer.sign(message).unwrap();

        assert_eq!(signature.ed25519_sig.len(), 64);
        assert!(!signature.mldsa_sig.is_empty());

        let valid = signer.verify(message, &signature).unwrap();
        assert!(valid, "Hybrid signature should be valid");
    }

    #[test]
    fn test_hybrid_verify_wrong_message() {
        let signer = HybridSigner::generate().unwrap();
        let message = b"Original message";
        let signature = signer.sign(message).unwrap();

        let wrong_message = b"Wrong message";
        let valid = signer.verify(wrong_message, &signature).unwrap();
        assert!(!valid, "Verification should fail for wrong message");
    }

    #[test]
    fn test_hybrid_verify_individual_components() {
        let signer = HybridSigner::generate().unwrap();
        let message = b"Component verification test";
        let signature = signer.sign(message).unwrap();

        // Verify Ed25519 only
        let ed25519_valid = signer
            .verify_ed25519(message, &signature.ed25519_sig)
            .unwrap();
        assert!(ed25519_valid, "Ed25519 component should be valid");

        // Verify ML-DSA only
        let mldsa_valid = signer.verify_mldsa(message, &signature.mldsa_sig).unwrap();
        assert!(mldsa_valid, "ML-DSA component should be valid");
    }

    #[test]
    fn test_hybrid_signature_serialization() {
        let signer = HybridSigner::generate().unwrap();
        let message = b"Serialization test";
        let signature = signer.sign(message).unwrap();

        // Serialize
        let bytes = signature.to_bytes();

        // Deserialize
        let recovered = HybridSignature::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.ed25519_sig, signature.ed25519_sig);
        assert_eq!(recovered.mldsa_sig, signature.mldsa_sig);
    }

    #[test]
    fn test_hybrid_verifier_standalone() {
        let signer = HybridSigner::generate().unwrap();
        let public_key = signer.public_key();
        let message = b"Verifier test message";
        let signature = signer.sign(message).unwrap();

        // Create verifier from public key only
        let verifier = HybridVerifier::new(&public_key).unwrap();

        let valid = verifier.verify(message, &signature).unwrap();
        assert!(valid, "Verifier should validate the signature");
    }

    #[test]
    fn test_hybrid_public_key_serialization() {
        let signer = HybridSigner::generate().unwrap();
        let pk = signer.public_key();

        // Serialize
        let bytes = pk.to_bytes();

        // Deserialize
        let recovered = HybridSigningPublicKey::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.ed25519_pk, pk.ed25519_pk);
        assert_eq!(recovered.mldsa_pk, pk.mldsa_pk);
    }

    #[test]
    fn test_algorithm_name() {
        assert_eq!(MlDsaAlgorithm::MlDsa44.name(), "ML-DSA-44");
        assert_eq!(MlDsaAlgorithm::MlDsa65.name(), "ML-DSA-65");
        assert_eq!(MlDsaAlgorithm::MlDsa87.name(), "ML-DSA-87");
    }

    #[test]
    fn test_algorithm_signature_size() {
        assert!(MlDsaAlgorithm::MlDsa44.signature_size_approx() > 0);
        assert!(MlDsaAlgorithm::MlDsa65.signature_size_approx() > 0);
        assert!(MlDsaAlgorithm::MlDsa87.signature_size_approx() > 0);
    }

    #[test]
    fn test_mldsa44_from_keys_invalid_size() {
        let result = MlDsa44Signer::from_keys(vec![0u8; 10], vec![0u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mldsa87_from_keys_invalid_size() {
        let result = MlDsa87Signer::from_keys(vec![0u8; 10], vec![0u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verifier_44_and_87() {
        // ML-DSA-44
        let signer44 = MlDsa44Signer::generate().unwrap();
        let msg = b"Test";
        let sig = signer44.sign(msg).unwrap();
        let verifier44 =
            MlDsaVerifier::new(signer44.public_key().to_vec(), MlDsaAlgorithm::MlDsa44).unwrap();
        assert!(verifier44.verify(msg, &sig).unwrap());

        // ML-DSA-87
        let signer87 = MlDsa87Signer::generate().unwrap();
        let sig87 = signer87.sign(msg).unwrap();
        let verifier87 =
            MlDsaVerifier::new(signer87.public_key().to_vec(), MlDsaAlgorithm::MlDsa87).unwrap();
        assert!(verifier87.verify(msg, &sig87).unwrap());
    }

    #[test]
    fn test_signer_debug_redacts_secret() {
        let signer = MlDsa65Signer::generate().unwrap();
        let debug_str = format!("{:?}", signer);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains(&format!("{:?}", signer.secret_key)));
    }

    #[test]
    fn test_signing_error_paths() {
        // Test invalid public key size for MlDsa44Signer
        let pks = MlDsaAlgorithm::MlDsa44.public_key_size();
        let invalid_pk = vec![0u8; pks - 1];
        let sk = vec![0u8; 100]; // Dummy SK size doesn't matter for this check
        assert!(MlDsa44Signer::from_keys(invalid_pk, sk).is_err());

        // Test invalid public key size for MlDsa65Signer
        let pks = MlDsaAlgorithm::MlDsa65.public_key_size();
        let invalid_pk = vec![0u8; pks + 1];
        assert!(MlDsa65Signer::from_keys(invalid_pk, vec![]).is_err());

        // Test invalid public key size for MlDsa87Signer
        let _pks = MlDsaAlgorithm::MlDsa87.public_key_size();
        let invalid_pk = vec![0u8; 10];
        assert!(MlDsa87Signer::from_keys(invalid_pk, vec![]).is_err());

        // Test HybridSignature::from_bytes errors
        assert!(HybridSignature::from_bytes(&[]).is_err()); // Empty
        assert!(HybridSignature::from_bytes(&[0x00, 0x01]).is_err()); // Wrong tag (0x01 is valid, but length short)
        assert!(HybridSignature::from_bytes(&[0x02, 0x00]).is_err()); // Wrong tag (0x02 invalid)

        let mut short_sig = vec![HYBRID_SIGNATURE_TAG];
        short_sig.extend_from_slice(&[0u8; 63]); // 1 byte short for Ed25519
        assert!(HybridSignature::from_bytes(&short_sig).is_err());

        // Test HybridSigningPublicKey::from_bytes errors
        assert!(HybridSigningPublicKey::from_bytes(&[]).is_err());
        assert!(HybridSigningPublicKey::from_bytes(&[0u8; 31]).is_err());
    }

    #[test]
    fn test_verifier_construction_errors() {
        // Test invalid public key size for MlDsaVerifier
        let alg = MlDsaAlgorithm::MlDsa44;
        let invalid_pk = vec![0u8; alg.public_key_size() - 1];
        assert!(MlDsaVerifier::new(invalid_pk, alg).is_err());
    }

    #[test]
    fn test_mldsa_algorithm_properties() {
        // Test name
        assert_eq!(MlDsaAlgorithm::MlDsa44.name(), "ML-DSA-44");
        assert_eq!(MlDsaAlgorithm::MlDsa65.name(), "ML-DSA-65");
        assert_eq!(MlDsaAlgorithm::MlDsa87.name(), "ML-DSA-87");

        // Test security level
        assert_eq!(MlDsaAlgorithm::MlDsa44.security_level(), 2);
        assert_eq!(MlDsaAlgorithm::MlDsa65.security_level(), 3);
        assert_eq!(MlDsaAlgorithm::MlDsa87.security_level(), 5);

        // Test public key sizes
        assert_eq!(MlDsaAlgorithm::MlDsa44.public_key_size(), 1312);
        assert_eq!(MlDsaAlgorithm::MlDsa65.public_key_size(), 1952);
        assert_eq!(MlDsaAlgorithm::MlDsa87.public_key_size(), 2592);

        // Test signature sizes
        assert_eq!(MlDsaAlgorithm::MlDsa44.signature_size_approx(), 2420);
        assert_eq!(MlDsaAlgorithm::MlDsa65.signature_size_approx(), 3309);
        assert_eq!(MlDsaAlgorithm::MlDsa87.signature_size_approx(), 4627);
    }

    #[test]
    fn test_mldsa_algorithm_default() {
        let default = MlDsaAlgorithm::default();
        assert_eq!(default, MlDsaAlgorithm::MlDsa65);
    }

    #[test]
    fn test_mldsa_algorithm_clone() {
        let alg = MlDsaAlgorithm::MlDsa87;
        let copied = alg; // MlDsaAlgorithm implements Copy
        assert_eq!(alg, copied);
    }

    #[test]
    fn test_signer_algorithm_accessor() {
        let signer44 = MlDsa44Signer::generate().unwrap();
        assert_eq!(signer44.algorithm(), MlDsaAlgorithm::MlDsa44);

        let signer65 = MlDsa65Signer::generate().unwrap();
        assert_eq!(signer65.algorithm(), MlDsaAlgorithm::MlDsa65);

        let signer87 = MlDsa87Signer::generate().unwrap();
        assert_eq!(signer87.algorithm(), MlDsaAlgorithm::MlDsa87);
    }

    #[test]
    fn test_verifier_algorithm_accessor() {
        let signer = MlDsa65Signer::generate().unwrap();
        let verifier =
            MlDsaVerifier::new(signer.public_key().to_vec(), MlDsaAlgorithm::MlDsa65).unwrap();
        assert_eq!(verifier.algorithm(), MlDsaAlgorithm::MlDsa65);
    }

    #[test]
    fn test_mldsa65_from_keys_valid() {
        let signer = MlDsa65Signer::generate().unwrap();
        let result =
            MlDsa65Signer::from_keys(signer.public_key().to_vec(), signer.secret_key.clone());
        assert!(result.is_ok());
    }

    #[test]
    fn test_mldsa_verifier_invalid_public_key() {
        let result = MlDsaVerifier::new(vec![0u8; 10], MlDsaAlgorithm::MlDsa65);
        assert!(result.is_err());
    }

    #[test]
    fn test_mldsa87_verify_with_wrong_message() {
        let signer = MlDsa87Signer::generate().unwrap();
        let message1 = b"original message";
        let message2 = b"different message";

        let signature = signer.sign(message1).unwrap();
        let result = signer.verify(message2, &signature).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_mldsa44_sign_empty_message() {
        let signer = MlDsa44Signer::generate().unwrap();
        let signature = signer.sign(b"").unwrap();
        assert!(!signature.is_empty());

        let valid = signer.verify(b"", &signature).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_verifier_cross_algorithm_fail() {
        let signer44 = MlDsa44Signer::generate().unwrap();
        let message = b"test";
        let sig44 = signer44.sign(message).unwrap();

        // Try to verify with wrong algorithm verifier
        let verifier65 =
            MlDsaVerifier::new(signer44.public_key().to_vec(), MlDsaAlgorithm::MlDsa65);
        assert!(verifier65.is_err()); // Should fail due to wrong key size
    }
}
