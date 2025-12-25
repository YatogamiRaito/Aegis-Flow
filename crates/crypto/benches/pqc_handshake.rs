//! Benchmark for PQC hybrid key exchange

use aegis_crypto::HybridKeyExchange;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

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

criterion_group!(
    benches,
    benchmark_keypair_generation,
    benchmark_encapsulation,
    benchmark_full_handshake,
);

criterion_main!(benches);
