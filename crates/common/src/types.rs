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

    #[test]
    fn test_security_level_serialization() {
        let level = SecurityLevel::Confidential;
        let json = serde_json::to_string(&level).unwrap();
        let parsed: SecurityLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }

    #[test]
    fn test_all_security_levels() {
        assert_ne!(SecurityLevel::Standard, SecurityLevel::PostQuantum);
        assert_ne!(SecurityLevel::PostQuantum, SecurityLevel::Confidential);
        assert_ne!(SecurityLevel::Standard, SecurityLevel::Confidential);
    }

    #[test]
    fn test_attestation_token_creation() {
        let token = AttestationToken {
            version: 1,
            measurement: vec![0xAB; 32],
            signature: vec![0xCD; 64],
            claims: Some(serde_json::json!({"key": "value"})),
        };
        assert_eq!(token.version, 1);
        assert_eq!(token.measurement.len(), 32);
        assert_eq!(token.signature.len(), 64);
        assert!(token.claims.is_some());
    }

    #[test]
    fn test_attestation_token_without_claims() {
        let token = AttestationToken {
            version: 2,
            measurement: vec![0x00; 48],
            signature: vec![0xFF; 96],
            claims: None,
        };
        assert_eq!(token.version, 2);
        assert!(token.claims.is_none());
    }

    #[test]
    fn test_legacy_key_exchange_variants() {
        // Ensure legacy variants can be constructed and compared
        assert_eq!(KeyExchangeType::Kyber768, KeyExchangeType::Kyber768);
        assert_eq!(KeyExchangeType::Kyber1024, KeyExchangeType::Kyber1024);
        assert_eq!(
            KeyExchangeType::HybridX25519Kyber768,
            KeyExchangeType::HybridX25519Kyber768
        );
        assert_eq!(
            KeyExchangeType::HybridX25519Kyber1024,
            KeyExchangeType::HybridX25519Kyber1024
        );

        assert_ne!(KeyExchangeType::Kyber768, KeyExchangeType::Kyber1024);
    }

    #[test]
    fn test_attestation_token_debug() {
        let token = AttestationToken {
            version: 1,
            measurement: vec![0x11],
            signature: vec![0x22],
            claims: None,
        };
        let debug = format!("{:?}", token);
        assert!(debug.contains("AttestationToken"));
        assert!(debug.contains("version: 1"));
    }
}
