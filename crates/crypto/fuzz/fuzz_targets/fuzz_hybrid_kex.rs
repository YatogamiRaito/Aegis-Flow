#![no_main]
use libfuzzer_sys::fuzz_target;
use aegis_crypto::hybrid_kex::{HybridKeyExchange, HybridPublicKey, HybridSecretKey};
use aegis_crypto::traits::KeyExchange;

fuzz_target!(|data: &[u8]| {
    let kex = HybridKeyExchange::new();
    
    // Generate a valid keypair to use for decapsulation
    let (_pk, sk) = match kex.generate_keypair() {
        Ok(res) => res,
        Err(_) => return,
    };

    // Attempt to decapsulate the random data
    let _ = kex.decapsulate(data, &sk);
});
