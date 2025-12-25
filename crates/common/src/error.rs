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
        let err = AegisError::Crypto("key exchange failed".to_string());
        assert_eq!(
            format!("{}", err),
            "Cryptographic error: key exchange failed"
        );
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let aegis_err: AegisError = io_err.into();
        assert!(matches!(aegis_err, AegisError::Io(_)));
    }
}
