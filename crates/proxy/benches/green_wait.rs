use aegis_energy::{CarbonIntensity, CarbonIntensityCache, EnergyApiClient, EnergyApiError, ForecastPoint, Region};
use aegis_proxy::green_wait::{DeferredJob, GreenWaitConfig, GreenWaitScheduler, JobPriority};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tokio::runtime::Runtime;

#[derive(Clone)]
struct MockBenchClient;

impl EnergyApiClient for MockBenchClient {
    async fn get_carbon_intensity(&self, _region: &Region) -> Result<CarbonIntensity, EnergyApiError> {
        unimplemented!()
    }
    async fn get_carbon_intensity_by_location(&self, _lat: f64, _lon: f64) -> Result<CarbonIntensity, EnergyApiError> {
        unimplemented!()
    }
    async fn get_region_for_location(&self, _lat: f64, _lon: f64) -> Result<Region, EnergyApiError> {
        unimplemented!()
    }
    async fn get_carbon_forecast(&self, _region: &Region, _hours: u32) -> Result<Vec<ForecastPoint>, EnergyApiError> {
        unimplemented!()
    }
}

fn bench_process_ready_jobs_1000(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("green_wait/process_ready_jobs");
    // Disable sample collection validation because setup is slow
    group.sample_size(10);
    group.throughput(Throughput::Elements(1000));

    group.bench_function("1000_jobs", |b| {
        b.iter_with_setup(
            || {
                let cache = CarbonIntensityCache::new(300);
                let client = MockBenchClient;
                let config = GreenWaitConfig {
                    enabled: true,
                    default_threshold: 150.0,
                    max_queue_size: 2000,
                    ..Default::default()
                };
                
                let temp_file = tempfile::NamedTempFile::new().unwrap();
                let scheduler = GreenWaitScheduler::new(config, client, cache, temp_file.path()).unwrap();
                
                rt.block_on(async {
                    for i in 0..1000 {
                        let job = DeferredJob::new(
                            format!("job-{}", i),
                            JobPriority::Normal, 
                            Region::new("mock", "Mock"),
                            150.0,
                            vec![],
                        );
                        scheduler.submit(job).await;
                    }
                    scheduler.update_region_intensity("mock", 10.0).await;
                });
                
                (scheduler, temp_file) // keep TempFile alive
            },
            |(scheduler, _temp_file)| {
                rt.block_on(async move {
                    let jobs = scheduler.process_ready_jobs().await;
                    std::hint::black_box(jobs);
                });
            },
        );
    });

    group.finish();
}

criterion_group!(benches, bench_process_ready_jobs_1000);
criterion_main!(benches);
