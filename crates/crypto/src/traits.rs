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

    // Mock implementation for testing
    struct MockKeyExchange;

    impl KeyExchange for MockKeyExchange {
        type PublicKey = Vec<u8>;
        type SharedSecret = Vec<u8>;

        fn generate_keypair(&self) -> Result<(Self::PublicKey, Vec<u8>)> {
            Ok((vec![1, 2, 3], vec![4, 5, 6]))
        }

        fn encapsulate(&self, _peer_public_key: &[u8]) -> Result<(Vec<u8>, Self::SharedSecret)> {
            Ok((vec![7, 8, 9], vec![10, 11, 12]))
        }

        fn decapsulate(
            &self,
            _ciphertext: &[u8],
            _secret_key: &[u8],
        ) -> Result<Self::SharedSecret> {
            Ok(vec![13, 14, 15])
        }

        fn algorithm_name(&self) -> &'static str {
            "MockKeyExchange"
        }
    }

    #[test]
    fn test_trait_is_object_safe() {
        // This test ensures KeyExchange trait is object-safe
        fn _assert_object_safe(_: &dyn KeyExchange<PublicKey = Vec<u8>, SharedSecret = Vec<u8>>) {}
    }

    #[test]
    fn test_mock_generate_keypair() {
        let kex = MockKeyExchange;
        let result = kex.generate_keypair();
        assert!(result.is_ok());

        let (public_key, secret_key) = result.unwrap();
        assert_eq!(public_key, vec![1, 2, 3]);
        assert_eq!(secret_key, vec![4, 5, 6]);
    }

    #[test]
    fn test_mock_encapsulate() {
        let kex = MockKeyExchange;
        let peer_key = vec![99, 88, 77];
        let result = kex.encapsulate(&peer_key);
        assert!(result.is_ok());

        let (ciphertext, shared_secret) = result.unwrap();
        assert_eq!(ciphertext, vec![7, 8, 9]);
        assert_eq!(shared_secret, vec![10, 11, 12]);
    }

    #[test]
    fn test_mock_decapsulate() {
        let kex = MockKeyExchange;
        let ciphertext = vec![1, 2];
        let secret_key = vec![3, 4];
        let result = kex.decapsulate(&ciphertext, &secret_key);
        assert!(result.is_ok());

        let shared_secret = result.unwrap();
        assert_eq!(shared_secret, vec![13, 14, 15]);
    }

    #[test]
    fn test_mock_algorithm_name() {
        let kex = MockKeyExchange;
        assert_eq!(kex.algorithm_name(), "MockKeyExchange");
    }

    #[test]
    fn test_public_key_as_ref() {
        let kex = MockKeyExchange;
        let (public_key, _) = kex.generate_keypair().unwrap();
        let as_bytes: &[u8] = public_key.as_ref();
        assert_eq!(as_bytes, &[1, 2, 3]);
    }

    #[test]
    fn test_shared_secret_as_ref() {
        let kex = MockKeyExchange;
        let (_, shared_secret) = kex.encapsulate(&[0]).unwrap();
        let as_bytes: &[u8] = shared_secret.as_ref();
        assert_eq!(as_bytes, &[10, 11, 12]);
    }

    #[test]
    fn test_encapsulate_with_empty_peer_key() {
        let kex = MockKeyExchange;
        let result = kex.encapsulate(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decapsulate_with_empty_inputs() {
        let kex = MockKeyExchange;
        let result = kex.decapsulate(&[], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_keypair_generation() {
        let kex = MockKeyExchange;
        let result1 = kex.generate_keypair();
        let result2 = kex.generate_keypair();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Mock implementation returns same values
        assert_eq!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockKeyExchange>();
    }
}
