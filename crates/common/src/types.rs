//! Common types for Aegis-Flow

use serde::{Deserialize, Serialize};

/// Represents the type of key exchange used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KeyExchangeType {
    /// Classical X25519 key exchange
    X25519,
    /// Post-Quantum ML-KEM-768 (NIST FIPS 203)
    MlKem768,
    /// Post-Quantum ML-KEM-1024 (NIST FIPS 203)
    MlKem1024,
    /// Hybrid: X25519 + ML-KEM-768 (recommended, NIST compliant)
    #[default]
    HybridX25519MlKem768,
    /// Hybrid: X25519 + ML-KEM-1024 (highest security)
    HybridX25519MlKem1024,
    /// Legacy: Kyber-768 (deprecated, use MlKem768)
    #[deprecated(since = "0.10.0", note = "Use MlKem768 instead")]
    Kyber768,
    /// Legacy: Kyber-1024 (deprecated, use MlKem1024)
    #[deprecated(since = "0.10.0", note = "Use MlKem1024 instead")]
    Kyber1024,
    /// Legacy: Hybrid X25519 + Kyber-768 (deprecated)
    #[deprecated(since = "0.10.0", note = "Use HybridX25519MlKem768 instead")]
    HybridX25519Kyber768,
    /// Legacy: Hybrid X25519 + Kyber-1024 (deprecated)
    #[deprecated(since = "0.10.0", note = "Use HybridX25519MlKem1024 instead")]
    HybridX25519Kyber1024,
}

/// Connection security level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SecurityLevel {
    /// Standard TLS (classical crypto only)
    Standard,
    /// Post-Quantum ready (hybrid mode)
    #[default]
    PostQuantum,
    /// TEE-backed with attestation
    Confidential,
}

/// Attestation token structure (placeholder for future TEE integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationToken {
    /// Token format version
    pub version: u8,
    /// Enclave measurement/hash
    pub measurement: Vec<u8>,
    /// Signature over the attestation data
    pub signature: Vec<u8>,
    /// Optional additional claims
    pub claims: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_key_exchange() {
        assert_eq!(
            KeyExchangeType::default(),
            KeyExchangeType::HybridX25519MlKem768
        );
    }

    #[test]
    fn test_default_security_level() {
        assert_eq!(SecurityLevel::default(), SecurityLevel::PostQuantum);
    }

    #[test]
    fn test_key_exchange_serialization() {
        let ke = KeyExchangeType::HybridX25519MlKem1024;
        let json = serde_json::to_string(&ke).unwrap();
        let parsed: KeyExchangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(ke, parsed);
    }
}
