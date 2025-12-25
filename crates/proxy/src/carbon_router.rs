//! Carbon-Aware Routing Module
//!
//! Routes traffic based on carbon intensity data from energy APIs.
//! Implements spatial arbitrage - selecting regions with lowest carbon footprint.

use aegis_energy::{CarbonIntensityCache, EnergyApiClient, Region};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Carbon-aware router configuration
#[derive(Debug, Clone)]
pub struct CarbonRouterConfig {
    /// Enable carbon-aware routing
    pub enabled: bool,
    /// Carbon intensity threshold (gCO2/kWh) - prefer regions below this
    pub threshold: f64,
    /// Maximum acceptable carbon intensity (hard limit)
    pub max_intensity: f64,
    /// Prefer renewable energy sources
    pub prefer_renewable: bool,
    /// Region preferences (fallback order)
    pub preferred_regions: Vec<String>,
    /// Weight factor for carbon intensity in routing decisions (0.0-1.0)
    pub carbon_weight: f64,
}

impl Default for CarbonRouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 200.0,     // 200 gCO2/kWh is considered "moderate"
            max_intensity: 500.0, // Above this is high-carbon
            prefer_renewable: true,
            preferred_regions: vec![],
            carbon_weight: 0.5, // Balance between latency and carbon
        }
    }
}

/// Represents a routable region with its carbon data
#[derive(Debug, Clone)]
pub struct RegionScore {
    /// Region identifier
    pub region_id: String,
    /// Current carbon intensity
    pub carbon_intensity: f64,
    /// Normalized score (0.0 = best, 1.0 = worst)
    pub score: f64,
    /// Is this region currently recommended
    pub recommended: bool,
}

/// Carbon-aware router for spatial arbitrage
pub struct CarbonRouter<C: EnergyApiClient> {
    config: CarbonRouterConfig,
    /// Energy API client
    client: Arc<C>,
    /// Carbon intensity cache
    cache: Arc<CarbonIntensityCache>,
    /// Region scores (cached for quick lookup)
    region_scores: Arc<RwLock<HashMap<String, RegionScore>>>,
    /// Registered regions
    regions: Arc<RwLock<Vec<Region>>>,
}

impl<C: EnergyApiClient + Send + Sync> CarbonRouter<C> {
    /// Create a new carbon router
    pub fn new(config: CarbonRouterConfig, client: C, cache: CarbonIntensityCache) -> Self {
        Self {
            config,
            client: Arc::new(client),
            cache: Arc::new(cache),
            region_scores: Arc::new(RwLock::new(HashMap::new())),
            regions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if carbon routing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the carbon threshold
    pub fn threshold(&self) -> f64 {
        self.config.threshold
    }

    /// Register a region for carbon-aware routing
    pub async fn register_region(&self, region: Region) {
        let mut regions = self.regions.write().await;
        info!("🌍 Registered region for carbon routing: {}", region.id);
        regions.push(region);
    }

    /// Get all registered regions
    pub async fn get_regions(&self) -> Vec<Region> {
        self.regions.read().await.clone()
    }

    /// Update carbon intensity for all registered regions
    pub async fn refresh_carbon_data(&self) -> Result<(), aegis_energy::EnergyApiError> {
        let regions = self.regions.read().await.clone();
        let mut scores = self.region_scores.write().await;

        for region in &regions {
            // Try cache first
            if let Some(cached) = self.cache.get(region).await {
                let score = self.calculate_score(cached.value);
                scores.insert(
                    region.id.clone(),
                    RegionScore {
                        region_id: region.id.clone(),
                        carbon_intensity: cached.value,
                        score,
                        recommended: cached.value < self.config.threshold,
                    },
                );
                continue;
            }

            // Fetch from API
            match self.client.get_carbon_intensity(region).await {
                Ok(intensity) => {
                    self.cache.put(intensity.clone()).await;
                    let score = self.calculate_score(intensity.value);
                    scores.insert(
                        region.id.clone(),
                        RegionScore {
                            region_id: region.id.clone(),
                            carbon_intensity: intensity.value,
                            score,
                            recommended: intensity.value < self.config.threshold,
                        },
                    );
                    debug!(
                        "📊 Updated carbon data for {}: {} gCO2/kWh",
                        region.id, intensity.value
                    );
                }
                Err(e) => {
                    warn!("⚠️ Failed to fetch carbon data for {}: {}", region.id, e);
                }
            }
        }

        Ok(())
    }

    /// Calculate normalized score (0.0 = greenest, 1.0 = highest carbon)
    fn calculate_score(&self, intensity: f64) -> f64 {
        // Normalize to 0-1 range based on max_intensity
        (intensity / self.config.max_intensity).min(1.0)
    }

    /// Select the best region based on carbon intensity
    pub async fn select_greenest_region(&self) -> Option<String> {
        let scores = self.region_scores.read().await;

        if scores.is_empty() {
            return None;
        }

        // Find region with lowest carbon intensity
        scores
            .iter()
            .filter(|(_, s)| s.carbon_intensity <= self.config.max_intensity)
            .min_by(|(_, a), (_, b)| {
                a.carbon_intensity
                    .partial_cmp(&b.carbon_intensity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id.clone())
    }

    /// Get regions sorted by carbon intensity (lowest first)
    pub async fn get_sorted_regions(&self) -> Vec<RegionScore> {
        let scores = self.region_scores.read().await;
        let mut sorted: Vec<RegionScore> = scores.values().cloned().collect();
        sorted.sort_by(|a, b| {
            a.carbon_intensity
                .partial_cmp(&b.carbon_intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    /// Check if a region is currently "green" (below threshold)
    pub async fn is_region_green(&self, region_id: &str) -> bool {
        let scores = self.region_scores.read().await;
        scores
            .get(region_id)
            .map(|s| s.recommended)
            .unwrap_or(false)
    }

    /// Get carbon intensity for a specific region
    pub async fn get_region_intensity(&self, region_id: &str) -> Option<f64> {
        let scores = self.region_scores.read().await;
        scores.get(region_id).map(|s| s.carbon_intensity)
    }

    /// Calculate routing weight for a region (for weighted load balancing)
    /// Higher weight = more traffic should be sent to this region
    pub async fn get_routing_weight(&self, region_id: &str) -> u32 {
        let scores = self.region_scores.read().await;

        if let Some(score) = scores.get(region_id) {
            if score.carbon_intensity > self.config.max_intensity {
                return 0; // No traffic to high-carbon regions
            }

            // Invert score: low carbon = high weight
            let inverted = 1.0 - score.score;
            let weight = (inverted * 100.0 * self.config.carbon_weight) as u32;
            weight.max(1) // Minimum weight of 1
        } else {
            50 // Default weight if no data
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_energy::{CarbonIntensity, EnergyApiError};

    /// Mock client for testing
    struct MockEnergyClient {
        intensities: HashMap<String, f64>,
    }

    impl MockEnergyClient {
        fn new() -> Self {
            let mut intensities = HashMap::new();
            intensities.insert("us-west".to_string(), 50.0); // Very green
            intensities.insert("us-east".to_string(), 350.0); // High carbon
            intensities.insert("eu-west".to_string(), 150.0); // Moderate
            Self { intensities }
        }
    }

    impl EnergyApiClient for MockEnergyClient {
        async fn get_carbon_intensity(
            &self,
            region: &Region,
        ) -> Result<CarbonIntensity, EnergyApiError> {
            let value = self.intensities.get(&region.id).copied().unwrap_or(200.0);
            Ok(CarbonIntensity {
                region: region.clone(),
                value,
                timestamp: chrono::Utc::now(),
                valid_for_seconds: 300,
                rating: None,
            })
        }

        async fn get_carbon_intensity_by_location(
            &self,
            latitude: f64,
            longitude: f64,
        ) -> Result<CarbonIntensity, EnergyApiError> {
            let region = Region {
                id: "mock-region".to_string(),
                name: "Mock Region".to_string(),
                latitude: Some(latitude),
                longitude: Some(longitude),
            };
            self.get_carbon_intensity(&region).await
        }

        async fn get_region_for_location(
            &self,
            latitude: f64,
            longitude: f64,
        ) -> Result<Region, EnergyApiError> {
            Ok(Region {
                id: "mock-region".to_string(),
                name: "Mock Region".to_string(),
                latitude: Some(latitude),
                longitude: Some(longitude),
            })
        }
    }

    #[test]
    fn test_default_config() {
        let config = CarbonRouterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.threshold, 200.0);
        assert_eq!(config.max_intensity, 500.0);
        assert!(config.prefer_renewable);
    }

    #[tokio::test]
    async fn test_router_creation() {
        let config = CarbonRouterConfig {
            enabled: true,
            threshold: 100.0,
            ..Default::default()
        };
        let client = MockEnergyClient::new();
        let cache = CarbonIntensityCache::new(300);
        let router = CarbonRouter::new(config, client, cache);

        assert!(router.is_enabled());
        assert_eq!(router.threshold(), 100.0);
    }

    #[tokio::test]
    async fn test_register_region() {
        let config = CarbonRouterConfig::default();
        let client = MockEnergyClient::new();
        let cache = CarbonIntensityCache::new(300);
        let router = CarbonRouter::new(config, client, cache);

        let region = Region {
            id: "us-west".to_string(),
            name: "US West".to_string(),
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
        };

        router.register_region(region).await;
        let regions = router.get_regions().await;
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].id, "us-west");
    }

    #[tokio::test]
    async fn test_select_greenest_region() {
        let config = CarbonRouterConfig {
            enabled: true,
            threshold: 200.0,
            max_intensity: 500.0,
            ..Default::default()
        };
        let client = MockEnergyClient::new();
        let cache = CarbonIntensityCache::new(300);
        let router = CarbonRouter::new(config, client, cache);

        // Register regions
        for region_id in ["us-west", "us-east", "eu-west"] {
            router
                .register_region(Region {
                    id: region_id.to_string(),
                    name: region_id.to_string(),
                    latitude: None,
                    longitude: None,
                })
                .await;
        }

        // Refresh data
        router.refresh_carbon_data().await.unwrap();

        // us-west should be selected (50 gCO2/kWh)
        let greenest = router.select_greenest_region().await;
        assert_eq!(greenest, Some("us-west".to_string()));
    }

    #[tokio::test]
    async fn test_routing_weights() {
        let config = CarbonRouterConfig {
            enabled: true,
            carbon_weight: 1.0,
            max_intensity: 500.0,
            ..Default::default()
        };
        let client = MockEnergyClient::new();
        let cache = CarbonIntensityCache::new(300);
        let router = CarbonRouter::new(config, client, cache);

        // Register and refresh
        for region_id in ["us-west", "us-east"] {
            router
                .register_region(Region {
                    id: region_id.to_string(),
                    name: region_id.to_string(),
                    latitude: None,
                    longitude: None,
                })
                .await;
        }
        router.refresh_carbon_data().await.unwrap();

        // us-west (50) should have higher weight than us-east (350)
        let west_weight = router.get_routing_weight("us-west").await;
        let east_weight = router.get_routing_weight("us-east").await;

        assert!(west_weight > east_weight);
    }

    #[tokio::test]
    async fn test_is_region_green() {
        let config = CarbonRouterConfig {
            enabled: true,
            threshold: 200.0,
            ..Default::default()
        };
        let client = MockEnergyClient::new();
        let cache = CarbonIntensityCache::new(300);
        let router = CarbonRouter::new(config, client, cache);

        for region_id in ["us-west", "us-east", "eu-west"] {
            router
                .register_region(Region {
                    id: region_id.to_string(),
                    name: region_id.to_string(),
                    latitude: None,
                    longitude: None,
                })
                .await;
        }
        router.refresh_carbon_data().await.unwrap();

        // us-west (50) and eu-west (150) are below threshold (200)
        assert!(router.is_region_green("us-west").await);
        assert!(router.is_region_green("eu-west").await);
        // us-east (350) is above threshold
        assert!(!router.is_region_green("us-east").await);
    }
}
