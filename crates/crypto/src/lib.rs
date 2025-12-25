//! Aegis-Crypto: Post-Quantum Cryptography module for Aegis-Flow
//!
//! This crate provides hybrid key exchange (Kyber + X25519) and
//! digital signature capabilities using NIST-standardized PQC algorithms.

pub mod hybrid_kex;
pub mod traits;

pub use hybrid_kex::HybridKeyExchange;
pub use traits::KeyExchange;
