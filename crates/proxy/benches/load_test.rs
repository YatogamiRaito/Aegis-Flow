//! Load Test Benchmark
//!
//! High-concurrency stress test for measuring RPS (Requests Per Second).

use aegis_proxy::{Http3Config, Http3Handler, Http3Request, Http3Response};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Benchmark sequential request handling
fn bench_sequential_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = Arc::new(Http3Handler::new(
        Http3Config::default(),
        "127.0.0.1:8080".to_string(),
    ));

    let mut group = c.benchmark_group("load/sequential");
    group.throughput(Throughput::Elements(1000));
    group.sample_size(10);

    group.bench_function("1k_requests", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..1000 {
                    let req = Http3Request::new("GET", "/health");
                    let resp = handler.handle_request(req).await;
                    black_box(resp);
                }
            })
        })
    });

    group.finish();
}

/// Benchmark concurrent request handling
fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = Arc::new(Http3Handler::new(
        Http3Config::default(),
        "127.0.0.1:8080".to_string(),
    ));

    let mut group = c.benchmark_group("load/concurrent");
    group.sample_size(10);

    for concurrency in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64 * 100));

        group.bench_with_input(
            BenchmarkId::new("requests", concurrency),
            concurrency,
            |b, &conc| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(conc);

                        for _ in 0..conc {
                            let h = handler.clone();
                            handles.push(tokio::spawn(async move {
                                for _ in 0..100 {
                                    let req = Http3Request::new("GET", "/health");
                                    let resp = h.handle_request(req).await;
                                    black_box(resp);
                                }
                            }));
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

/// Measure raw RPS capability
fn bench_rps_measurement(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = Arc::new(Http3Handler::new(
        Http3Config::default(),
        "127.0.0.1:8080".to_string(),
    ));

    let mut group = c.benchmark_group("load/rps");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("sustained_load", |b| {
        b.iter_custom(|iters| {
            let handler = handler.clone();
            let start = Instant::now();

            rt.block_on(async {
                let counter = Arc::new(AtomicU64::new(0));
                let mut handles = Vec::new();

                // Spawn worker tasks
                for _ in 0..num_cpus::get() {
                    let h = handler.clone();
                    let c = counter.clone();
                    let target = iters;

                    handles.push(tokio::spawn(async move {
                        while c.fetch_add(1, Ordering::Relaxed) < target {
                            let req = Http3Request::new("GET", "/health");
                            let resp = h.handle_request(req).await;
                            black_box(resp);
                        }
                    }));
                }

                for handle in handles {
                    let _ = handle.await;
                }
            });

            start.elapsed()
        })
    });

    group.finish();
}

/// Memory allocation benchmark under load
fn bench_memory_under_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = Arc::new(Http3Handler::new(
        Http3Config::default(),
        "127.0.0.1:8080".to_string(),
    ));

    let mut group = c.benchmark_group("load/memory");
    group.sample_size(10);

    group.bench_function("allocation_pressure", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate high allocation pressure
                let mut responses: Vec<Http3Response> = Vec::with_capacity(1000);

                for _ in 0..1000 {
                    let req = Http3Request::new("POST", "/api/data")
                        .with_header("content-type", "application/json")
                        .with_body(bytes::Bytes::from(r#"{"key":"value","data":"test"}"#));
                    responses.push(handler.handle_request(req).await);
                }

                black_box(responses.len())
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_requests,
    bench_concurrent_requests,
    bench_rps_measurement,
    bench_memory_under_load,
);

criterion_main!(benches);
