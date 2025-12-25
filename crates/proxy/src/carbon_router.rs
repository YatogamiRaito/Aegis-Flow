//! Carbon-Aware Routing Module
//!
//! Routes traffic based on carbon intensity data from energy APIs.

#![allow(dead_code)]

use std::collections::HashMap;

/// Carbon-aware router configuration
#[derive(Debug, Clone, Default)]
pub struct CarbonRouterConfig {
    /// Enable carbon-aware routing
    pub enabled: bool,
    /// Carbon intensity threshold for routing decisions
    pub threshold: f64,
    /// Region preferences
    pub preferred_regions: Vec<String>,
}

/// Carbon-aware router
pub struct CarbonRouter {
    config: CarbonRouterConfig,
    #[allow(dead_code)]
    region_scores: HashMap<String, f64>,
}

impl CarbonRouter {
    /// Create a new carbon router
    pub fn new(config: CarbonRouterConfig) -> Self {
        Self {
            config,
            region_scores: HashMap::new(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CarbonRouterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.threshold, 0.0);
        assert!(config.preferred_regions.is_empty());
    }

    #[test]
    fn test_router_creation() {
        let config = CarbonRouterConfig {
            enabled: true,
            threshold: 100.0,
            preferred_regions: vec!["us-west".to_string()],
        };
        let router = CarbonRouter::new(config);
        assert!(router.is_enabled());
        assert_eq!(router.threshold(), 100.0);
    }

    #[test]
    fn test_router_disabled_by_default() {
        let router = CarbonRouter::new(CarbonRouterConfig::default());
        assert!(!router.is_enabled());
    }
}
