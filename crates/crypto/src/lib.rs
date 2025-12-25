//! Aegis-Crypto: Post-Quantum Cryptography module for Aegis-Flow
//!
//! This crate provides hybrid key exchange (Kyber + X25519) and
//! digital signature capabilities using NIST-standardized PQC algorithms.
//!
//! # Features
//!
//! - **Hybrid Key Exchange**: X25519 + Kyber-768 for quantum resistance
//! - **TLS Integration**: Custom crypto provider for rustls (coming soon)
//! - **Formal Verification**: Designed for Kani/Verus verification
//!
//! # Example
//!
//! ```rust
//! use aegis_crypto::HybridKeyExchange;
//!
//! let kex = HybridKeyExchange::new();
//!
//! // Server generates keypair
//! let (server_pk, server_sk) = kex.generate_keypair().unwrap();
//!
//! // Client encapsulates
//! let (ciphertext, client_secret) = kex.encapsulate(&server_pk).unwrap();
//!
//! // Server decapsulates
//! let server_secret = kex.decapsulate(&ciphertext, &server_sk).unwrap();
//!
//! // Both have the same shared secret
//! assert_eq!(client_secret.as_bytes(), server_secret.as_bytes());
//! ```

pub mod cipher;
pub mod hybrid_kex;
pub mod mtls;
pub mod stream;
pub mod tls;
pub mod traits;

pub use cipher::{Cipher, CipherAlgorithm, EncryptionKey};
pub use hybrid_kex::{HybridCiphertext, HybridKeyExchange, HybridPublicKey, HybridSharedSecret};
pub use mtls::{CertInfo, MtlsConfig, MtlsHandler};
pub use traits::KeyExchange;
