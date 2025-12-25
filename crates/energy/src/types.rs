//! Types for carbon intensity and energy API responses

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Energy API provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EnergyApiProvider {
    #[default]
    WattTime,
    ElectricityMaps,
}

/// Represents a geographic region for carbon intensity lookup
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// Region identifier (e.g., "CAISO_NORTH", "DE", "US-CAL-CISO")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Latitude (optional, for reverse geocoding)
    pub latitude: Option<f64>,
    /// Longitude (optional, for reverse geocoding)
    pub longitude: Option<f64>,
}

impl Region {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            latitude: None,
            longitude: None,
        }
    }

    pub fn with_coordinates(mut self, lat: f64, lon: f64) -> Self {
        self.latitude = Some(lat);
        self.longitude = Some(lon);
        self
    }
}

/// Carbon intensity measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonIntensity {
    /// Region this measurement applies to
    pub region: Region,
    /// Carbon intensity in gCO2eq/kWh
    pub value: f64,
    /// Timestamp of the measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Data validity period in seconds
    pub valid_for_seconds: u64,
    /// Relative rating (if available): "very_low", "low", "medium", "high", "very_high"
    pub rating: Option<String>,
}

impl CarbonIntensity {
    /// Check if this measurement is still valid
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now();
        let valid_until = self.timestamp + chrono::Duration::seconds(self.valid_for_seconds as i64);
        now < valid_until
    }

    /// Get a normalized score (0.0 = cleanest, 1.0 = dirtiest)
    /// Based on typical ranges: 0-50 very low, 50-150 low, 150-300 medium, 300-500 high, 500+ very high
    pub fn normalized_score(&self) -> f64 {
        const MAX_INTENSITY: f64 = 800.0;
        (self.value / MAX_INTENSITY).clamp(0.0, 1.0)
    }
}

/// Errors that can occur when interacting with energy APIs
#[derive(Debug, Error)]
pub enum EnergyApiError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API authentication failed")]
    AuthenticationError,

    #[error("Rate limit exceeded, retry after {retry_after_seconds} seconds")]
    RateLimitExceeded { retry_after_seconds: u64 },

    #[error("Region not found: {region_id}")]
    RegionNotFound { region_id: String },

    #[error("API response parsing error: {0}")]
    ParseError(String),

    #[error("API returned error: {message}")]
    ApiError { message: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// WattTime API response for grid region
#[derive(Debug, Deserialize)]
pub struct WattTimeRegionResponse {
    pub abbrev: String,
    pub name: String,
}

/// WattTime API response for real-time carbon intensity
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WattTimeIndexResponse {
    pub freq: Option<String>,
    pub ba: String,
    pub percent: Option<f64>,
    pub moer: Option<f64>,
    pub point_time: String,
}

/// Electricity Maps API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct ElectricityMapsResponse {
    pub zone: String,
    pub carbon_intensity: f64,
    pub datetime: String,
    pub updated_at: String,
    #[serde(default)]
    pub fossil_fuel_percentage: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new("CAISO_NORTH", "California ISO - North")
            .with_coordinates(37.7749, -122.4194);

        assert_eq!(region.id, "CAISO_NORTH");
        assert_eq!(region.latitude, Some(37.7749));
    }

    #[test]
    fn test_carbon_intensity_normalized_score() {
        let region = Region::new("TEST", "Test Region");

        let low = CarbonIntensity {
            region: region.clone(),
            value: 50.0,
            timestamp: chrono::Utc::now(),
            valid_for_seconds: 300,
            rating: Some("low".to_string()),
        };

        let high = CarbonIntensity {
            region,
            value: 600.0,
            timestamp: chrono::Utc::now(),
            valid_for_seconds: 300,
            rating: Some("high".to_string()),
        };

        assert!(low.normalized_score() < 0.1);
        assert!(high.normalized_score() > 0.7);
    }

    #[test]
    fn test_carbon_intensity_validity() {
        let region = Region::new("TEST", "Test Region");

        let valid = CarbonIntensity {
            region: region.clone(),
            value: 100.0,
            timestamp: chrono::Utc::now(),
            valid_for_seconds: 300,
            rating: None,
        };

        let expired = CarbonIntensity {
            region,
            value: 100.0,
            timestamp: chrono::Utc::now() - chrono::Duration::seconds(600),
            valid_for_seconds: 300,
            rating: None,
        };

        assert!(valid.is_valid());
        assert!(!expired.is_valid());
    }
}
