//! Benchmark for PQC hybrid key exchange

use aegis_crypto::HybridKeyExchange;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn benchmark_keypair_generation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();

    c.bench_function("hybrid_keypair_generation", |b| {
        b.iter(|| black_box(kex.generate_keypair().unwrap()))
    });
}

fn benchmark_encapsulation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, _sk) = kex.generate_keypair().unwrap();

    c.bench_function("hybrid_encapsulation", |b| {
        b.iter(|| black_box(kex.encapsulate(&pk).unwrap()))
    });
}

fn benchmark_decapsulation(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, sk) = kex.generate_keypair().unwrap();
    let (ct, _) = kex.encapsulate(&pk).unwrap();

    c.bench_function("hybrid_decapsulation", |b| {
        b.iter(|| black_box(kex.decapsulate(&ct, &sk).unwrap()))
    });
}

fn benchmark_full_handshake(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();

    c.bench_function("hybrid_full_handshake", |b| {
        b.iter(|| {
            let (pk, sk) = kex.generate_keypair().unwrap();
            let (ct, _client_ss) = kex.encapsulate(&pk).unwrap();
            let _server_ss = kex.decapsulate(&ct, &sk).unwrap();
        })
    });
}

fn benchmark_derive_key(c: &mut Criterion) {
    let kex = HybridKeyExchange::new();
    let (pk, sk) = kex.generate_keypair().unwrap();
    let (ct, _) = kex.encapsulate(&pk).unwrap();
    let ss = kex.decapsulate(&ct, &sk).unwrap();

    c.bench_function("hybrid_derive_key", |b| {
        b.iter(|| black_box(ss.derive_key()))
    });
}

criterion_group!(
    benches,
    benchmark_keypair_generation,
    benchmark_encapsulation,
    benchmark_decapsulation,
    benchmark_full_handshake,
    benchmark_derive_key,
);

criterion_main!(benches);
