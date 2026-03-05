use aegis_energy::{CarbonIntensity, CarbonIntensityCache, EnergyApiClient, EnergyApiError, ForecastPoint, Region};
use aegis_proxy::carbon_router::{CarbonRouter, CarbonRouterConfig};
use aegis_proxy::green_wait::{DeferredJob, GreenWaitConfig, GreenWaitScheduler, JobPriority, ScheduleResult};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct MockIntegrationClient {
    intensity: Arc<RwLock<f64>>,
    forecast: Arc<RwLock<Vec<ForecastPoint>>>,
}

impl MockIntegrationClient {
    fn new(intensity: f64, forecast: Vec<ForecastPoint>) -> Self {
        Self {
            intensity: Arc::new(RwLock::new(intensity)),
            forecast: Arc::new(RwLock::new(forecast)),
        }
    }

    async fn set_intensity(&self, i: f64) {
        *self.intensity.write().await = i;
    }
}

impl EnergyApiClient for MockIntegrationClient {
    async fn get_carbon_intensity(&self, region: &Region) -> Result<CarbonIntensity, EnergyApiError> {
        let val = *self.intensity.read().await;
        Ok(CarbonIntensity {
            region: region.clone(),
            value: val,
            timestamp: chrono::Utc::now(),
            valid_for_seconds: 300,
            rating: None,
        })
    }

    async fn get_carbon_intensity_by_location(&self, lat: f64, lon: f64) -> Result<CarbonIntensity, EnergyApiError> {
        let region = Region {
            id: "integration-region".to_string(),
            name: "Integration Region".to_string(),
            latitude: Some(lat),
            longitude: Some(lon),
        };
        self.get_carbon_intensity(&region).await
    }

    async fn get_region_for_location(&self, lat: f64, lon: f64) -> Result<Region, EnergyApiError> {
        Ok(Region {
            id: "integration-region".to_string(),
            name: "Integration Region".to_string(),
            latitude: Some(lat),
            longitude: Some(lon),
        })
    }

    async fn get_carbon_forecast(&self, _region: &Region, _hours: u32) -> Result<Vec<ForecastPoint>, EnergyApiError> {
        let points = self.forecast.read().await.clone();
        Ok(points)
    }
}

async fn setup_components(
    client: MockIntegrationClient,
    threshold: f64,
) -> (CarbonRouter<MockIntegrationClient>, GreenWaitScheduler<MockIntegrationClient>, CarbonIntensityCache) {
    let cache = CarbonIntensityCache::new(300);
    
    let router_config = CarbonRouterConfig {
        enabled: true,
        threshold,
        ..Default::default()
    };
    let router = CarbonRouter::new(router_config, client.clone(), cache.clone());
    router.register_region(Region::new("integration-region", "Integration Region")).await;
    
    let wait_config = GreenWaitConfig {
        enabled: true,
        default_threshold: threshold,
        max_queue_size: 100,
        ..Default::default()
    };
    let scheduler = GreenWaitScheduler::new(
        wait_config,
        client,
        cache.clone(),
        tempfile::NamedTempFile::new().unwrap().path(),
    ).unwrap();

    (router, scheduler, cache)
}

#[tokio::test]
async fn test_e2e_dirty_region_defers_job() {
    // High carbon intensity, above 150.0 threshold
    let client = MockIntegrationClient::new(300.0, vec![]);
    let (router, scheduler, _cache) = setup_components(client, 150.0).await;
    
    // Refresh to update router score
    router.refresh_carbon_data().await.unwrap();

    let region_id = "integration-region";
    assert!(!router.is_region_green(region_id).await);
    
    let job = DeferredJob::new(
        "job-1",
        JobPriority::Normal,
        Region::new(region_id, "Integration Region"),
        150.0, // threshold
        vec![], // payload
    );

    let result = scheduler.submit(job).await;
    match result {
        ScheduleResult::Queued { position } => assert_eq!(position, 0),
        _ => panic!("Expected job to be queued"),
    }
}

#[tokio::test]
async fn test_e2e_green_window_executes_deferred() {
    let client = MockIntegrationClient::new(300.0, vec![]);
    let (router, scheduler, cache) = setup_components(client.clone(), 150.0).await;
    
    router.refresh_carbon_data().await.unwrap();
    
    let region = Region::new("integration-region", "Integration Region");
    
    // Initially dirty, so it should queue
    let job = DeferredJob::new(
        "job-2",
        JobPriority::Normal,
        region.clone(),
        150.0,
        vec![],
    );
    scheduler.submit(job).await;
    assert_eq!(scheduler.queue_length().await, 1);
    
    // Carbon drops to 50.0
    client.set_intensity(50.0).await;
    
    // Clear cache to bypass TTL
    cache.clear().await;
    
    // Refresh components
    router.refresh_carbon_data().await.unwrap();
    scheduler.refresh_intensities().await;

    // Check if it's green now
    assert!(router.is_region_green("integration-region").await);
    
    let ready_jobs = scheduler.process_ready_jobs().await;
    assert_eq!(ready_jobs.len(), 1);
    assert_eq!(ready_jobs[0].id, "job-2");
    assert_eq!(scheduler.queue_length().await, 0);
}

#[tokio::test]
async fn test_e2e_forecast_driven_scheduling() {
    let now = chrono::Utc::now();
    let forecast = vec![
        ForecastPoint {
            timestamp: now + chrono::Duration::hours(1),
            predicted_intensity: 300.0,
            confidence: None,
        },
        ForecastPoint {
            timestamp: now + chrono::Duration::hours(2),
            predicted_intensity: 50.0,
            confidence: None,
        },
    ];
    let client = MockIntegrationClient::new(400.0, forecast);
    let (_router, scheduler, _cache) = setup_components(client, 150.0).await;
    
    let job = DeferredJob::new(
        "job-3",
        JobPriority::Background,
        Region::new("integration-region", "Integration Region"),
        150.0,
        vec![],
    );
    
    let optimal_time = scheduler.estimate_green_window(&job).await;
    assert!(optimal_time.is_some());
    assert_eq!(optimal_time.unwrap(), now + chrono::Duration::hours(2));
}
