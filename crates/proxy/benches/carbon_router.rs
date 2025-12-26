//! Carbon Router Benchmark
//!
//! Measures carbon-aware routing decision performance.

use aegis_proxy::{CarbonRouterConfig, RegionScore};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

/// Benchmark router config creation
fn bench_router_config(c: &mut Criterion) {
    c.bench_function("carbon/config_creation", |b| {
        b.iter(|| {
            let config = CarbonRouterConfig::default();
            black_box(config)
        })
    });
}

/// Benchmark region score creation
fn bench_region_score(c: &mut Criterion) {
    c.bench_function("carbon/region_score_creation", |b| {
        b.iter(|| {
            let score = RegionScore {
                region_id: "us-east-1".to_string(),
                carbon_intensity: 150.0,
                score: 0.3,
                recommended: true,
            };
            black_box(score)
        })
    });
}

/// Benchmark routing decision with multiple regions
fn bench_routing_decision(c: &mut Criterion) {
    let regions: Vec<RegionScore> = vec![
        RegionScore {
            region_id: "us-east-1".to_string(),
            carbon_intensity: 350.0,
            score: 0.7,
            recommended: false,
        },
        RegionScore {
            region_id: "us-west-2".to_string(),
            carbon_intensity: 150.0,
            score: 0.3,
            recommended: true,
        },
        RegionScore {
            region_id: "eu-north-1".to_string(),
            carbon_intensity: 50.0,
            score: 0.1,
            recommended: true,
        },
    ];

    let mut group = c.benchmark_group("carbon/routing_decision");
    group.throughput(Throughput::Elements(1));

    group.bench_function("select_best_region", |b| {
        b.iter(|| {
            // Find region with lowest carbon intensity
            let best = regions
                .iter()
                .filter(|r| r.recommended)
                .min_by(|a, b| a.carbon_intensity.partial_cmp(&b.carbon_intensity).unwrap())
                .map(|r| r.region_id.clone());
            black_box(best)
        })
    });

    group.finish();
}

/// Benchmark spatial arbitrage calculation
fn bench_spatial_arbitrage(c: &mut Criterion) {
    let regions = [
        ("us-east-1", 350.0),
        ("us-west-2", 150.0),
        ("eu-north-1", 50.0),
        ("ap-south-1", 500.0),
        ("sa-east-1", 100.0),
    ];

    c.bench_function("carbon/spatial_arbitrage", |b| {
        b.iter(|| {
            // Find regions with carbon intensity below threshold
            let threshold = 200.0;
            let green_regions: Vec<_> = regions
                .iter()
                .filter(|(_, intensity)| *intensity < threshold)
                .map(|(region, _)| *region)
                .collect();
            black_box(green_regions)
        })
    });
}

/// Benchmark score normalization
fn bench_score_normalization(c: &mut Criterion) {
    let max_intensity: f64 = 500.0;
    let intensities: Vec<f64> = vec![50.0, 150.0, 250.0, 350.0, 450.0];

    c.bench_function("carbon/score_normalization", |b| {
        b.iter(|| {
            let scores: Vec<f64> = intensities
                .iter()
                .map(|i| (i / max_intensity).min(1.0))
                .collect();
            black_box(scores)
        })
    });
}

criterion_group!(
    benches,
    bench_router_config,
    bench_region_score,
    bench_routing_decision,
    bench_spatial_arbitrage,
    bench_score_normalization,
);

criterion_main!(benches);
