//! ML-DSA Signing Benchmark
//!
//! Measures performance of ML-DSA key generation, signing, and verification

use aegis_crypto::signing::{MlDsa44Signer, MlDsa65Signer, MlDsa87Signer, SigningKeyPair};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn bench_mldsa44(c: &mut Criterion) {
    let mut group = c.benchmark_group("ML-DSA-44");
    let message = b"Benchmark message for ML-DSA-44 signature testing";

    group.bench_function("keygen", |b| b.iter(|| MlDsa44Signer::generate().unwrap()));

    let signer = MlDsa44Signer::generate().unwrap();
    let signature = signer.sign(message).unwrap();

    group.bench_function("sign", |b| b.iter(|| signer.sign(message).unwrap()));

    group.bench_function("verify", |b| {
        b.iter(|| signer.verify(message, &signature).unwrap())
    });

    group.finish();
}

fn bench_mldsa65(c: &mut Criterion) {
    let mut group = c.benchmark_group("ML-DSA-65");
    let message = b"Benchmark message for ML-DSA-65 signature testing";

    group.bench_function("keygen", |b| b.iter(|| MlDsa65Signer::generate().unwrap()));

    let signer = MlDsa65Signer::generate().unwrap();
    let signature = signer.sign(message).unwrap();

    group.bench_function("sign", |b| b.iter(|| signer.sign(message).unwrap()));

    group.bench_function("verify", |b| {
        b.iter(|| signer.verify(message, &signature).unwrap())
    });

    group.finish();
}

fn bench_mldsa87(c: &mut Criterion) {
    let mut group = c.benchmark_group("ML-DSA-87");
    let message = b"Benchmark message for ML-DSA-87 signature testing";

    group.bench_function("keygen", |b| b.iter(|| MlDsa87Signer::generate().unwrap()));

    let signer = MlDsa87Signer::generate().unwrap();
    let signature = signer.sign(message).unwrap();

    group.bench_function("sign", |b| b.iter(|| signer.sign(message).unwrap()));

    group.bench_function("verify", |b| {
        b.iter(|| signer.verify(message, &signature).unwrap())
    });

    group.finish();
}

fn bench_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ML-DSA-65-message-sizes");
    let signer = MlDsa65Signer::generate().unwrap();

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let message = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("sign", size), size, |b, _| {
            b.iter(|| signer.sign(&message).unwrap())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_mldsa44,
    bench_mldsa65,
    bench_mldsa87,
    bench_message_sizes
);
criterion_main!(benches);
