//! Energy API clients for WattTime and Electricity Maps

use crate::types::{
    CarbonIntensity, EnergyApiError, ElectricityMapsResponse, Region,
    WattTimeIndexResponse, WattTimeRegionResponse,
};
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Trait for energy API clients
#[allow(async_fn_in_trait)]
pub trait EnergyApiClient: Send + Sync {
    /// Get current carbon intensity for a region
    async fn get_carbon_intensity(&self, region: &Region) -> Result<CarbonIntensity, EnergyApiError>;

    /// Get carbon intensity for coordinates (reverse geocoding)
    async fn get_carbon_intensity_by_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<CarbonIntensity, EnergyApiError>;

    /// Get the region for given coordinates
    async fn get_region_for_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Region, EnergyApiError>;
}

/// WattTime API client
/// API Documentation: https://docs.watttime.org/
pub struct WattTimeClient {
    client: Client,
    base_url: String,
    token: Arc<tokio::sync::RwLock<Option<String>>>,
    username: String,
    password: String,
}

impl WattTimeClient {
    const DEFAULT_BASE_URL: &'static str = "https://api.watttime.org/v3";

    pub fn new(username: String, password: String) -> Self {
        Self {
            client: Client::new(),
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            token: Arc::new(tokio::sync::RwLock::new(None)),
            username,
            password,
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    async fn ensure_token(&self) -> Result<String, EnergyApiError> {
        // Check if we have a valid token
        {
            let token_guard = self.token.read().await;
            if let Some(ref token) = *token_guard {
                return Ok(token.clone());
            }
        }

        // Need to authenticate
        let mut token_guard = self.token.write().await;
        
        // Double-check after acquiring write lock
        if let Some(ref token) = *token_guard {
            return Ok(token.clone());
        }

        debug!("Authenticating with WattTime API");
        let response = self
            .client
            .get(format!("{}/login", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(EnergyApiError::AuthenticationError);
        }

        let body: serde_json::Value = response.json().await?;
        let token = body["token"]
            .as_str()
            .ok_or_else(|| EnergyApiError::ParseError("Missing token in response".to_string()))?
            .to_string();

        *token_guard = Some(token.clone());
        Ok(token)
    }
}

impl EnergyApiClient for WattTimeClient {
    #[instrument(skip(self))]
    async fn get_carbon_intensity(&self, region: &Region) -> Result<CarbonIntensity, EnergyApiError> {
        let token = self.ensure_token().await?;

        let response = self
            .client
            .get(format!("{}/signal-index", self.base_url))
            .bearer_auth(&token)
            .query(&[("region", &region.id)])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(EnergyApiError::RateLimitExceeded {
                retry_after_seconds: retry_after,
            });
        }

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(EnergyApiError::RegionNotFound {
                region_id: region.id.clone(),
            });
        }

        let data: WattTimeIndexResponse = response.json().await?;

        // Convert WattTime MOER (Marginal Operating Emissions Rate) to gCO2/kWh
        // MOER is in lbs CO2/MWh, convert to grams CO2/kWh
        let carbon_value = data.moer.unwrap_or(0.0) * 0.453592; // lbs to kg, MWh to kWh

        let timestamp = chrono::DateTime::parse_from_rfc3339(&data.point_time)
            .map_err(|e| EnergyApiError::ParseError(e.to_string()))?
            .with_timezone(&chrono::Utc);

        Ok(CarbonIntensity {
            region: region.clone(),
            value: carbon_value,
            timestamp,
            valid_for_seconds: 300, // 5 minutes default
            rating: data.percent.map(|p| {
                match p as u32 {
                    0..=20 => "very_low",
                    21..=40 => "low",
                    41..=60 => "medium",
                    61..=80 => "high",
                    _ => "very_high",
                }
                .to_string()
            }),
        })
    }

    #[instrument(skip(self))]
    async fn get_carbon_intensity_by_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<CarbonIntensity, EnergyApiError> {
        let region = self.get_region_for_location(latitude, longitude).await?;
        self.get_carbon_intensity(&region).await
    }

    #[instrument(skip(self))]
    async fn get_region_for_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Region, EnergyApiError> {
        let token = self.ensure_token().await?;

        let response = self
            .client
            .get(format!("{}/region-from-loc", self.base_url))
            .bearer_auth(&token)
            .query(&[
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
            ])
            .send()
            .await?;

        let data: WattTimeRegionResponse = response.json().await?;

        Ok(Region {
            id: data.abbrev,
            name: data.name,
            latitude: Some(latitude),
            longitude: Some(longitude),
        })
    }
}

/// Electricity Maps API client
/// API Documentation: https://static.electricitymaps.com/api/docs/index.html
pub struct ElectricityMapsClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ElectricityMapsClient {
    const DEFAULT_BASE_URL: &'static str = "https://api.electricitymap.org/v3";

    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            api_key,
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

impl EnergyApiClient for ElectricityMapsClient {
    #[instrument(skip(self))]
    async fn get_carbon_intensity(&self, region: &Region) -> Result<CarbonIntensity, EnergyApiError> {
        let response = self
            .client
            .get(format!("{}/carbon-intensity/latest", self.base_url))
            .header("auth-token", &self.api_key)
            .query(&[("zone", &region.id)])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(EnergyApiError::AuthenticationError);
        }

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(EnergyApiError::RateLimitExceeded {
                retry_after_seconds: 60,
            });
        }

        let data: ElectricityMapsResponse = response.json().await?;

        let timestamp = chrono::DateTime::parse_from_rfc3339(&data.datetime)
            .map_err(|e| EnergyApiError::ParseError(e.to_string()))?
            .with_timezone(&chrono::Utc);

        Ok(CarbonIntensity {
            region: region.clone(),
            value: data.carbon_intensity,
            timestamp,
            valid_for_seconds: 3600, // 1 hour for Electricity Maps
            rating: Some(match data.carbon_intensity as u32 {
                0..=50 => "very_low",
                51..=150 => "low",
                151..=300 => "medium",
                301..=500 => "high",
                _ => "very_high",
            }.to_string()),
        })
    }

    #[instrument(skip(self))]
    async fn get_carbon_intensity_by_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<CarbonIntensity, EnergyApiError> {
        let response = self
            .client
            .get(format!("{}/carbon-intensity/latest", self.base_url))
            .header("auth-token", &self.api_key)
            .query(&[
                ("lat", latitude.to_string()),
                ("lon", longitude.to_string()),
            ])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(EnergyApiError::AuthenticationError);
        }

        let data: ElectricityMapsResponse = response.json().await?;

        let timestamp = chrono::DateTime::parse_from_rfc3339(&data.datetime)
            .map_err(|e| EnergyApiError::ParseError(e.to_string()))?
            .with_timezone(&chrono::Utc);

        let region = Region {
            id: data.zone.clone(),
            name: data.zone,
            latitude: Some(latitude),
            longitude: Some(longitude),
        };

        Ok(CarbonIntensity {
            region,
            value: data.carbon_intensity,
            timestamp,
            valid_for_seconds: 3600,
            rating: None,
        })
    }

    #[instrument(skip(self))]
    async fn get_region_for_location(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Region, EnergyApiError> {
        // Electricity Maps infers zone from coordinates in the carbon-intensity call
        // So we make a lightweight call to get the zone
        let intensity = self.get_carbon_intensity_by_location(latitude, longitude).await?;
        Ok(intensity.region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_watttime_authentication() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "test_token_12345"
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let token = client.ensure_token().await.unwrap();
        assert_eq!(token, "test_token_12345");
    }

    #[tokio::test]
    async fn test_electricity_maps_carbon_intensity() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "DE",
                "carbonIntensity": 250.5,
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("test_key".to_string())
            .with_base_url(mock_server.uri());

        let region = Region::new("DE", "Germany");
        let intensity = client.get_carbon_intensity(&region).await.unwrap();

        assert_eq!(intensity.value, 250.5);
        assert_eq!(intensity.region.id, "DE");
    }
}
