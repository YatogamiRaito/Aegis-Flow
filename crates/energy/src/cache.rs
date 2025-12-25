//! TTL-based cache for carbon intensity data

use crate::types::{CarbonIntensity, Region};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, instrument};

/// Cache for carbon intensity lookups
pub struct CarbonIntensityCache {
    cache: Cache<String, Arc<CarbonIntensity>>,
    default_ttl: Duration,
}

impl CarbonIntensityCache {
    /// Create a new cache with the specified TTL
    pub fn new(ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .max_capacity(1000) // Max 1000 regions cached
            .build();

        Self {
            cache,
            default_ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached carbon intensity for a region
    #[instrument(skip(self))]
    pub async fn get(&self, region: &Region) -> Option<Arc<CarbonIntensity>> {
        let key = Self::cache_key(region);
        let result = self.cache.get(&key).await;
        
        if let Some(ref intensity) = result {
            if !intensity.is_valid() {
                debug!(region_id = %region.id, "Cached intensity expired");
                self.cache.invalidate(&key).await;
                return None;
            }
            debug!(region_id = %region.id, "Cache hit");
        } else {
            debug!(region_id = %region.id, "Cache miss");
        }
        
        result
    }

    /// Store carbon intensity in cache
    #[instrument(skip(self, intensity))]
    pub async fn put(&self, intensity: CarbonIntensity) {
        let key = Self::cache_key(&intensity.region);
        debug!(region_id = %intensity.region.id, value = %intensity.value, "Caching intensity");
        self.cache.insert(key, Arc::new(intensity)).await;
    }

    /// Get cached intensity or fetch from provider
    pub async fn get_or_fetch<F, Fut>(
        &self,
        region: &Region,
        fetch_fn: F,
    ) -> Result<Arc<CarbonIntensity>, crate::types::EnergyApiError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<CarbonIntensity, crate::types::EnergyApiError>>,
    {
        // Try cache first
        if let Some(cached) = self.get(region).await {
            return Ok(cached);
        }

        // Fetch from API
        let intensity = fetch_fn().await?;
        let arc_intensity = Arc::new(intensity.clone());
        self.put(intensity).await;
        
        Ok(arc_intensity)
    }

    /// Invalidate cache for a specific region
    pub async fn invalidate(&self, region: &Region) {
        let key = Self::cache_key(region);
        self.cache.invalidate(&key).await;
    }

    /// Clear all cached entries
    pub async fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get default TTL
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    fn cache_key(region: &Region) -> String {
        region.id.clone()
    }
}

impl Default for CarbonIntensityCache {
    fn default() -> Self {
        Self::new(300) // 5 minutes default TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_intensity(region_id: &str, value: f64) -> CarbonIntensity {
        CarbonIntensity {
            region: Region::new(region_id, format!("Test {}", region_id)),
            value,
            timestamp: chrono::Utc::now(),
            valid_for_seconds: 300,
            rating: None,
        }
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = CarbonIntensityCache::new(60);
        let intensity = create_test_intensity("TEST_REGION", 150.0);
        let region = intensity.region.clone();

        cache.put(intensity).await;
        
        let cached = cache.get(&region).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().value, 150.0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = CarbonIntensityCache::new(60);
        let region = Region::new("NONEXISTENT", "Does Not Exist");

        let cached = cache.get(&region).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = CarbonIntensityCache::new(60);
        let intensity = create_test_intensity("TO_INVALIDATE", 200.0);
        let region = intensity.region.clone();

        cache.put(intensity).await;
        assert!(cache.get(&region).await.is_some());

        cache.invalidate(&region).await;
        assert!(cache.get(&region).await.is_none());
    }

    #[tokio::test]
    async fn test_get_or_fetch() {
        let cache = CarbonIntensityCache::new(60);
        let region = Region::new("FETCH_TEST", "Fetch Test Region");

        // First call should fetch
        let result = cache
            .get_or_fetch(&region, || async {
                Ok(create_test_intensity("FETCH_TEST", 100.0))
            })
            .await
            .unwrap();
        
        assert_eq!(result.value, 100.0);

        // Second call should use cache (different value in fetch proves it)
        let result2 = cache
            .get_or_fetch(&region, || async {
                Ok(create_test_intensity("FETCH_TEST", 999.0))
            })
            .await
            .unwrap();
        
        assert_eq!(result2.value, 100.0); // Still 100, not 999
    }
}
