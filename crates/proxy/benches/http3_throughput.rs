//! HTTP/3 Throughput Benchmark
//!
//! Measures HTTP/3 request handling performance.

use aegis_proxy::{Http3Config, Http3Handler, Http3Request};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use tokio::runtime::Runtime;

/// Benchmark HTTP/3 request creation
fn bench_request_creation(c: &mut Criterion) {
    c.bench_function("http3/request_creation", |b| {
        b.iter(|| {
            let req = Http3Request::new("GET", "/api/data")
                .with_header("content-type", "application/json")
                .with_header("authorization", "Bearer token");
            black_box(req)
        })
    });
}

/// Benchmark HTTP/3 request handling
fn bench_request_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = Http3Handler::new(Http3Config::default(), "127.0.0.1:8080".to_string());

    let mut group = c.benchmark_group("http3/request_handling");
    group.throughput(Throughput::Elements(1));

    group.bench_function("health_endpoint", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Http3Request::new("GET", "/health");
                let resp = handler.handle_request(req).await;
                black_box(resp)
            })
        })
    });

    group.bench_function("ready_endpoint", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Http3Request::new("GET", "/ready");
                let resp = handler.handle_request(req).await;
                black_box(resp)
            })
        })
    });

    group.bench_function("unknown_endpoint", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = Http3Request::new("GET", "/api/unknown");
                let resp = handler.handle_request(req).await;
                black_box(resp)
            })
        })
    });

    group.finish();
}

/// Benchmark response creation
fn bench_response_creation(c: &mut Criterion) {
    use aegis_proxy::Http3Response;
    use bytes::Bytes;

    c.bench_function("http3/response_ok", |b| {
        b.iter(|| {
            let resp = Http3Response::ok(r#"{"status":"ok"}"#).with_header("x-request-id", "12345");
            black_box(resp)
        })
    });

    c.bench_function("http3/response_large", |b| {
        let large_body = Bytes::from(vec![b'x'; 65536]);
        b.iter(|| {
            let resp = Http3Response::ok(large_body.clone());
            black_box(resp)
        })
    });
}

criterion_group!(
    benches,
    bench_request_creation,
    bench_request_handling,
    bench_response_creation,
);

criterion_main!(benches);
