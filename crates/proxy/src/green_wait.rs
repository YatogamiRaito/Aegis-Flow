//! Green-Wait: Temporal Shifting for Carbon-Aware Job Scheduling
//!
//! Defers non-urgent jobs to time periods with lower carbon intensity.
//! Uses energy forecasts to schedule jobs during "green" windows.

use aegis_energy::{CarbonIntensityCache, EnergyApiClient, Region};
use crate::metrics;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Priority level for deferred jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum JobPriority {
    /// Must execute immediately regardless of carbon intensity
    Critical = 0,
    /// Can wait up to 5 minutes for green window
    High = 1,
    /// Can wait up to 30 minutes for green window
    #[default]
    Normal = 2,
    /// Can wait up to 2 hours for green window
    Low = 3,
    /// Can wait indefinitely for optimal green window
    Background = 4,
}

impl JobPriority {
    /// Maximum wait time for this priority level
    pub fn max_wait_duration(&self) -> Duration {
        match self {
            JobPriority::Critical => Duration::ZERO,
            JobPriority::High => Duration::from_secs(5 * 60), // 5 minutes
            JobPriority::Normal => Duration::from_secs(30 * 60), // 30 minutes
            JobPriority::Low => Duration::from_secs(2 * 60 * 60), // 2 hours
            JobPriority::Background => Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }
}

/// A deferred job waiting for a green window
#[derive(Debug)]
pub struct DeferredJob {
    /// Unique job identifier
    pub id: String,
    /// Job priority
    pub priority: JobPriority,
    /// Target region for execution
    pub region: Region,
    /// When the job was submitted
    pub submitted_at: Instant,
    /// Maximum carbon intensity threshold for execution
    pub carbon_threshold: f64,
    /// Job payload (opaque bytes)
    pub payload: Vec<u8>,
}

impl DeferredJob {
    /// Create a new deferred job
    pub fn new(
        id: impl Into<String>,
        priority: JobPriority,
        region: Region,
        carbon_threshold: f64,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id: id.into(),
            priority,
            region,
            submitted_at: Instant::now(),
            carbon_threshold,
            payload,
        }
    }

    /// Check if this job has exceeded its maximum wait time
    pub fn is_expired(&self) -> bool {
        self.submitted_at.elapsed() > self.priority.max_wait_duration()
    }

    /// Time remaining before expiration
    pub fn time_remaining(&self) -> Duration {
        let max_wait = self.priority.max_wait_duration();
        let elapsed = self.submitted_at.elapsed();
        if elapsed >= max_wait {
            Duration::ZERO
        } else {
            max_wait - elapsed
        }
    }
}

/// Configuration for the Green-Wait scheduler
#[derive(Debug, Clone)]
pub struct GreenWaitConfig {
    /// Enable green-wait scheduling
    pub enabled: bool,
    /// Default carbon intensity threshold (gCO2/kWh)
    pub default_threshold: f64,
    /// How often to check for green windows (seconds)
    pub check_interval_secs: u64,
    /// Maximum queue size
    pub max_queue_size: usize,
}

impl Default for GreenWaitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_threshold: 150.0,
            check_interval_secs: 60,
            max_queue_size: 1000,
        }
    }
}

/// Result of attempting to schedule a job
#[derive(Debug)]
pub enum ScheduleResult {
    /// Job executed immediately (carbon is low or priority is critical)
    ExecutedImmediately,
    /// Job was queued for later execution
    Queued { position: usize },
    /// Queue is full, job rejected
    QueueFull,
    /// Scheduler is disabled
    Disabled,
}

/// Green-Wait Scheduler for temporal shifting
pub struct GreenWaitScheduler<C: EnergyApiClient> {
    config: GreenWaitConfig,
    client: Arc<C>,
    cache: Arc<CarbonIntensityCache>,
    /// Queue of deferred jobs
    queue: Arc<Mutex<VecDeque<DeferredJob>>>,
    /// Current carbon intensity per region
    region_intensity: Arc<RwLock<std::collections::HashMap<String, f64>>>,
}

impl<C: EnergyApiClient + Send + Sync + 'static> GreenWaitScheduler<C> {
    /// Create a new Green-Wait scheduler
    pub fn new(config: GreenWaitConfig, client: C, cache: CarbonIntensityCache) -> Self {
        Self {
            config,
            client: Arc::new(client),
            cache: Arc::new(cache),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            region_intensity: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Check if the scheduler is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get current queue length
    pub async fn queue_length(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Submit a job for green-wait scheduling
    pub async fn submit(&self, job: DeferredJob) -> ScheduleResult {
        if !self.config.enabled {
            return ScheduleResult::Disabled;
        }

        // Critical priority jobs execute immediately
        if job.priority == JobPriority::Critical {
            info!(job_id = %job.id, "Critical job executing immediately");
            return ScheduleResult::ExecutedImmediately;
        }

        // Check current carbon intensity
        if let Some(intensity) = self.get_region_intensity(&job.region.id).await
            && intensity <= job.carbon_threshold
        {
            info!(
                job_id = %job.id,
                intensity = intensity,
                threshold = job.carbon_threshold,
                "Carbon intensity is low, executing immediately"
            );
            return ScheduleResult::ExecutedImmediately;
        }

        // Queue the job
        let mut queue = self.queue.lock().await;
        if queue.len() >= self.config.max_queue_size {
            warn!(job_id = %job.id, "Queue is full, rejecting job");
            return ScheduleResult::QueueFull;
        }

        let position = queue.len();
        debug!(job_id = %job.id, position = position, "Job queued for green window");
        queue.push_back(job);
        metrics::update_deferred_jobs(queue.len());

        ScheduleResult::Queued { position }
    }

    /// Get carbon intensity for a region
    async fn get_region_intensity(&self, region_id: &str) -> Option<f64> {
        let intensities = self.region_intensity.read().await;
        intensities.get(region_id).copied()
    }

    /// Update carbon intensity for a region
    pub async fn update_region_intensity(&self, region_id: &str, intensity: f64) {
        let mut intensities = self.region_intensity.write().await;
        intensities.insert(region_id.to_string(), intensity);
    }

    /// Process ready jobs from the queue
    pub async fn process_ready_jobs(&self) -> Vec<DeferredJob> {
        let mut queue = self.queue.lock().await;
        let intensities = self.region_intensity.read().await;

        let mut ready_jobs = Vec::new();
        let mut remaining_jobs = VecDeque::new();

        while let Some(job) = queue.pop_front() {
            // Check if job is expired (must execute now)
            if job.is_expired() {
                info!(
                    job_id = %job.id,
                    "Job expired, executing regardless of carbon intensity"
                );
                ready_jobs.push(job);
                continue;
            }

            // Check if carbon intensity is acceptable
            if let Some(&intensity) = intensities.get(&job.region.id)
                && intensity <= job.carbon_threshold
            {
                info!(
                    job_id = %job.id,
                    intensity = intensity,
                    threshold = job.carbon_threshold,
                    "Green window detected, executing job"
                );
                ready_jobs.push(job);
                continue;
            }

            // Job not ready, keep in queue
            remaining_jobs.push_back(job);
        }

        *queue = remaining_jobs;
        metrics::update_deferred_jobs(queue.len());
        ready_jobs
    }

    /// Refresh carbon intensity data for all queued regions
    /// Call this periodically from your main loop
    pub async fn refresh_intensities(&self) {
        // Get unique regions from queued jobs
        let regions: Vec<Region> = {
            let q = self.queue.lock().await;
            let mut seen = std::collections::HashMap::new();
            for job in q.iter() {
                seen.entry(job.region.id.clone())
                    .or_insert(job.region.clone());
            }
            seen.into_values().collect()
        };

        // Update carbon intensity for each region
        for region in regions {
            if let Some(cached) = self.cache.get(&region).await {
                self.update_region_intensity(&region.id, cached.value).await;
                metrics::update_carbon_intensity(&region.id, cached.value);
            } else if let Ok(intensity) = self.client.get_carbon_intensity(&region).await {
                self.cache.put(intensity.clone()).await;
                self.update_region_intensity(&region.id, intensity.value)
                    .await;
                metrics::update_carbon_intensity(&region.id, intensity.value);
            }
        }
    }

    /// Check if scheduler is running (always true for non-background mode)
    pub fn is_running(&self) -> bool {
        true
    }

    /// Get queue statistics
    pub async fn stats(&self) -> GreenWaitStats {
        let queue = self.queue.lock().await;
        let total = queue.len();
        let expired = queue.iter().filter(|j| j.is_expired()).count();

        let by_priority = [
            queue
                .iter()
                .filter(|j| j.priority == JobPriority::Critical)
                .count(),
            queue
                .iter()
                .filter(|j| j.priority == JobPriority::High)
                .count(),
            queue
                .iter()
                .filter(|j| j.priority == JobPriority::Normal)
                .count(),
            queue
                .iter()
                .filter(|j| j.priority == JobPriority::Low)
                .count(),
            queue
                .iter()
                .filter(|j| j.priority == JobPriority::Background)
                .count(),
        ];

        GreenWaitStats {
            total_queued: total,
            expired_count: expired,
            by_priority,
        }
    }
}

/// Statistics for the Green-Wait queue
#[derive(Debug, Clone)]
pub struct GreenWaitStats {
    /// Total jobs in queue
    pub total_queued: usize,
    /// Jobs that have exceeded their wait time
    pub expired_count: usize,
    /// Jobs by priority level [Critical, High, Normal, Low, Background]
    pub by_priority: [usize; 5],
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_energy::{CarbonIntensity, EnergyApiError};

    struct MockClient {
        intensity: f64,
    }

    impl EnergyApiClient for MockClient {
        async fn get_carbon_intensity(
            &self,
            region: &Region,
        ) -> Result<CarbonIntensity, EnergyApiError> {
            Ok(CarbonIntensity {
                region: region.clone(),
                value: self.intensity,
                timestamp: chrono::Utc::now(),
                valid_for_seconds: 300,
                rating: None,
            })
        }

        async fn get_carbon_intensity_by_location(
            &self,
            lat: f64,
            lon: f64,
        ) -> Result<CarbonIntensity, EnergyApiError> {
            let region = Region {
                id: "mock".to_string(),
                name: "Mock".to_string(),
                latitude: Some(lat),
                longitude: Some(lon),
            };
            self.get_carbon_intensity(&region).await
        }

        async fn get_region_for_location(
            &self,
            lat: f64,
            lon: f64,
        ) -> Result<Region, EnergyApiError> {
            Ok(Region {
                id: "mock".to_string(),
                name: "Mock".to_string(),
                latitude: Some(lat),
                longitude: Some(lon),
            })
        }
    }

    #[test]
    fn test_job_priority_wait_times() {
        assert_eq!(JobPriority::Critical.max_wait_duration(), Duration::ZERO);
        assert_eq!(
            JobPriority::High.max_wait_duration(),
            Duration::from_secs(300)
        );
        assert_eq!(
            JobPriority::Normal.max_wait_duration(),
            Duration::from_secs(1800)
        );
        assert_eq!(
            JobPriority::Low.max_wait_duration(),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn test_default_config() {
        let config = GreenWaitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_threshold, 150.0);
        assert_eq!(config.max_queue_size, 1000);
    }

    #[tokio::test]
    async fn test_critical_job_executes_immediately() {
        let client = MockClient { intensity: 500.0 }; // High carbon
        let cache = CarbonIntensityCache::new(300);
        let scheduler = GreenWaitScheduler::new(GreenWaitConfig::default(), client, cache);

        let job = DeferredJob::new(
            "critical-1",
            JobPriority::Critical,
            Region::new("us-west", "US West"),
            100.0,
            vec![],
        );

        let result = scheduler.submit(job).await;
        assert!(matches!(result, ScheduleResult::ExecutedImmediately));
    }

    #[tokio::test]
    async fn test_job_queued_when_carbon_high() {
        let client = MockClient { intensity: 500.0 }; // High carbon
        let cache = CarbonIntensityCache::new(300);
        let scheduler = GreenWaitScheduler::new(GreenWaitConfig::default(), client, cache);

        // Set current intensity
        scheduler.update_region_intensity("us-west", 500.0).await;

        let job = DeferredJob::new(
            "normal-1",
            JobPriority::Normal,
            Region::new("us-west", "US West"),
            100.0, // Threshold lower than current intensity
            vec![],
        );

        let result = scheduler.submit(job).await;
        assert!(matches!(result, ScheduleResult::Queued { position: 0 }));
        assert_eq!(scheduler.queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_job_executes_when_carbon_low() {
        let client = MockClient { intensity: 50.0 }; // Low carbon
        let cache = CarbonIntensityCache::new(300);
        let scheduler = GreenWaitScheduler::new(GreenWaitConfig::default(), client, cache);

        // Set current intensity to low
        scheduler.update_region_intensity("us-west", 50.0).await;

        let job = DeferredJob::new(
            "normal-1",
            JobPriority::Normal,
            Region::new("us-west", "US West"),
            100.0, // Threshold higher than current intensity
            vec![],
        );

        let result = scheduler.submit(job).await;
        assert!(matches!(result, ScheduleResult::ExecutedImmediately));
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let client = MockClient { intensity: 500.0 };
        let cache = CarbonIntensityCache::new(300);
        let scheduler = GreenWaitScheduler::new(GreenWaitConfig::default(), client, cache);

        scheduler.update_region_intensity("us-west", 500.0).await;

        // Add jobs of different priorities
        for i in 0..3 {
            let job = DeferredJob::new(
                format!("job-{}", i),
                JobPriority::Normal,
                Region::new("us-west", "US West"),
                100.0,
                vec![],
            );
            scheduler.submit(job).await;
        }

        let stats = scheduler.stats().await;
        assert_eq!(stats.total_queued, 3);
        assert_eq!(stats.by_priority[2], 3); // Normal priority
    }

    #[tokio::test]
    async fn test_process_ready_jobs() {
        let client = MockClient { intensity: 50.0 };
        let cache = CarbonIntensityCache::new(300);
        let scheduler = GreenWaitScheduler::new(GreenWaitConfig::default(), client, cache);

        // Initially high carbon
        scheduler.update_region_intensity("us-west", 500.0).await;

        let job = DeferredJob::new(
            "job-1",
            JobPriority::Normal,
            Region::new("us-west", "US West"),
            100.0,
            vec![],
        );
        scheduler.submit(job).await;
        assert_eq!(scheduler.queue_length().await, 1);

        // Now carbon drops
        scheduler.update_region_intensity("us-west", 50.0).await;

        let ready = scheduler.process_ready_jobs().await;
        assert_eq!(ready.len(), 1);
        assert_eq!(scheduler.queue_length().await, 0);
    }
}
