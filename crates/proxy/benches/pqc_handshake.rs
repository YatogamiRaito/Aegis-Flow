//! PQC Handshake Benchmark
//!
//! Measures the performance of Kyber-768 + X25519 hybrid key exchange.

use aegis_crypto::HybridKeyExchange;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

/// Benchmark Kyber-768 key generation
fn bench_keypair_generation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();

    c.bench_function("pqc/keypair_generation", |b| {
        b.iter(|| {
            let (pk, _sk) = kex.generate_keypair().unwrap();
            black_box(pk)
        })
    });
}

/// Benchmark encapsulation (client-side)
fn bench_encapsulation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, _sk) = kex.generate_keypair().unwrap();

    c.bench_function("pqc/encapsulation", |b| {
        b.iter(|| {
            let (ciphertext, shared_secret) = kex.encapsulate(&pk).unwrap();
            black_box((ciphertext, shared_secret))
        })
    });
}

/// Benchmark decapsulation (server-side)
fn bench_decapsulation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, sk) = kex.generate_keypair().unwrap();
    let (ciphertext, _) = kex.encapsulate(&pk).unwrap();

    c.bench_function("pqc/decapsulation", |b| {
        b.iter(|| {
            let shared_secret = kex.decapsulate(&ciphertext, &sk).unwrap();
            black_box(shared_secret)
        })
    });
}

/// Benchmark full handshake (keypair + encap + decap)
fn bench_full_handshake(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();

    let mut group = c.benchmark_group("pqc/full_handshake");
    group.throughput(Throughput::Elements(1));

    group.bench_function("complete", |b| {
        b.iter(|| {
            // Server generates keypair
            let (pk, sk) = kex.generate_keypair().unwrap();

            // Client encapsulates
            let (ciphertext, client_secret) = kex.encapsulate(&pk).unwrap();

            // Server decapsulates
            let server_secret = kex.decapsulate(&ciphertext, &sk).unwrap();

            // Verify shared secrets match
            assert_eq!(client_secret.derive_key(), server_secret.derive_key());

            black_box((client_secret, server_secret))
        })
    });

    group.finish();
}

/// Benchmark key derivation
fn bench_key_derivation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, _) = kex.generate_keypair().unwrap();
    let (_, shared_secret) = kex.encapsulate(&pk).unwrap();

    c.bench_function("pqc/key_derivation", |b| {
        b.iter(|| {
            let key = shared_secret.derive_key();
            black_box(key)
        })
    });
}

criterion_group!(
    benches,
    bench_keypair_generation,
    bench_encapsulation,
    bench_decapsulation,
    bench_full_handshake,
    bench_key_derivation,
);

criterion_main!(benches);
