//! Error types for Aegis-Flow

use thiserror::Error;

/// Main error type for Aegis-Flow operations
#[derive(Error, Debug)]
pub enum AegisError {
    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Network operation failed
    #[error("Network error: {0}")]
    Network(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// TEE/Enclave related error
    #[error("TEE error: {0}")]
    Tee(String),

    /// Attestation verification failed
    #[error("Attestation error: {0}")]
    Attestation(String),

    /// I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for Aegis-Flow operations
pub type Result<T> = std::result::Result<T, AegisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", AegisError::Crypto("fail".to_string())),
            "Cryptographic error: fail"
        );
        assert_eq!(
            format!("{}", AegisError::Network("fail".to_string())),
            "Network error: fail"
        );
        assert_eq!(
            format!("{}", AegisError::Config("fail".to_string())),
            "Configuration error: fail"
        );
        assert_eq!(
            format!("{}", AegisError::Tee("fail".to_string())),
            "TEE error: fail"
        );
        assert_eq!(
            format!("{}", AegisError::Attestation("fail".to_string())),
            "Attestation error: fail"
        );
        assert_eq!(
            format!("{}", AegisError::Internal("fail".to_string())),
            "Internal error: fail"
        );
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let aegis_err: AegisError = io_err.into();
        assert!(matches!(aegis_err, AegisError::Io(_)));
        // Verify IO error display as well
        assert!(format!("{}", aegis_err).contains("file not found"));
    }

    #[test]
    fn test_error_debug() {
        let err = AegisError::Crypto("test crypto error".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Crypto"));
        assert!(debug_str.contains("test crypto error"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors = [
            AegisError::Crypto("c".to_string()),
            AegisError::Network("n".to_string()),
            AegisError::Config("cfg".to_string()),
            AegisError::Tee("tee".to_string()),
            AegisError::Attestation("att".to_string()),
            AegisError::Internal("int".to_string()),
        ];

        for err in errors {
            let display = format!("{}", err);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_error_io_kind() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let aegis_err: AegisError = io_err.into();
        if let AegisError::Io(e) = aegis_err {
            assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        } else {
            panic!("Expected Io variant");
        }
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let aegis_err: AegisError = io_err.into();
        assert!(aegis_err.source().is_some());

        let crypto_err = AegisError::Crypto("test".to_string());
        assert!(crypto_err.source().is_none());
    }
}
