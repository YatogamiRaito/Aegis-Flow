//! Key Exchange trait definitions

use aegis_common::Result;

/// Trait for key exchange algorithms
pub trait KeyExchange: Send + Sync {
    /// The type representing the public key
    type PublicKey: AsRef<[u8]>;

    /// The type representing the shared secret
    type SharedSecret: AsRef<[u8]>;

    /// Generate a new key pair
    fn generate_keypair(&self) -> Result<(Self::PublicKey, Vec<u8>)>;

    /// Encapsulate a shared secret using the peer's public key
    fn encapsulate(&self, peer_public_key: &[u8]) -> Result<(Vec<u8>, Self::SharedSecret)>;

    /// Decapsulate a shared secret using the ciphertext and secret key
    fn decapsulate(&self, ciphertext: &[u8], secret_key: &[u8]) -> Result<Self::SharedSecret>;

    /// Return the algorithm name
    fn algorithm_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Placeholder test - actual tests in hybrid_kex module
    #[test]
    fn test_trait_is_object_safe() {
        // This test ensures KeyExchange trait is object-safe
        fn _assert_object_safe(_: &dyn KeyExchange<PublicKey = Vec<u8>, SharedSecret = Vec<u8>>) {}
    }
}
