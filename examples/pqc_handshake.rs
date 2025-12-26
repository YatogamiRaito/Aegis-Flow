//! Post-Quantum Key Exchange Example
//!
//! Demonstrates hybrid X25519 + ML-KEM-768 key exchange.
//!
//! Run with: cargo run --example pqc_handshake -p aegis-examples

use aegis_crypto::HybridKeyExchange;

fn main() {
    println!("🔐 Aegis-Flow Post-Quantum Key Exchange Demo\n");

    // Create key exchange instance
    let kex = HybridKeyExchange::new();

    // Step 1: Server generates keypair
    println!("1. Server generating keypair...");
    let (server_pk, server_sk) = kex.generate_keypair().expect("Failed to generate keypair");
    println!("   ✅ Public key generated (X25519 + ML-KEM-768 hybrid)");

    // Step 2: Client encapsulates (creates shared secret + ciphertext)
    println!("\n2. Client encapsulating...");
    let (ciphertext, client_secret) = kex.encapsulate(&server_pk).expect("Failed to encapsulate");
    println!("   ✅ Ciphertext generated (X25519 + ML-KEM-768 hybrid)");
    println!("   ✅ Shared secret size: {} bytes", client_secret.as_bytes().len());

    // Step 3: Server decapsulates (recovers shared secret)
    println!("\n3. Server decapsulating...");
    let server_secret = kex
        .decapsulate(&ciphertext, &server_sk)
        .expect("Failed to decapsulate");

    // Step 4: Verify both parties have the same secret
    println!("\n4. Verifying shared secrets match...");
    assert_eq!(
        client_secret.as_bytes(),
        server_secret.as_bytes(),
        "Shared secrets don't match!"
    );
    println!("   ✅ Secrets match! Secure channel established.");

    println!("\n🎉 Post-quantum key exchange completed successfully!");
    println!("   Algorithm: {}", kex.algorithm_name());
    println!("   This connection is protected against quantum computer attacks.");
}
