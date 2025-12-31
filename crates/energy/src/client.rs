//! Energy API clients for WattTime and Electricity Maps

use crate::types::{
    CarbonIntensity, ElectricityMapsResponse, EnergyApiError, Region, WattTimeIndexResponse,
    WattTimeRegionResponse,
};
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Trait for energy API clients
#[allow(async_fn_in_trait)]
pub trait EnergyApiClient: Send + Sync {
    /// Get current carbon intensity for a region
    async fn get_carbon_intensity(
        &self,
        region: &Region,
    ) -> Result<CarbonIntensity, EnergyApiError>;

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
/// API Documentation: <https://docs.watttime.org/>
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
    async fn get_carbon_intensity(
        &self,
        region: &Region,
    ) -> Result<CarbonIntensity, EnergyApiError> {
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
/// API Documentation: <https://static.electricitymaps.com/api/docs/index.html>
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
    async fn get_carbon_intensity(
        &self,
        region: &Region,
    ) -> Result<CarbonIntensity, EnergyApiError> {
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
            rating: Some(
                match data.carbon_intensity as u32 {
                    0..=50 => "very_low",
                    51..=150 => "low",
                    151..=300 => "medium",
                    301..=500 => "high",
                    _ => "very_high",
                }
                .to_string(),
            ),
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
        let intensity = self
            .get_carbon_intensity_by_location(latitude, longitude)
            .await?;
        Ok(intensity.region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

        let client =
            ElectricityMapsClient::new("test_key".to_string()).with_base_url(mock_server.uri());

        let region = Region::new("DE", "Germany");
        let intensity = client.get_carbon_intensity(&region).await.unwrap();

        assert_eq!(intensity.region.id, "DE");
    }

    #[tokio::test]
    async fn test_watttime_rate_limit() {
        let mock_server = MockServer::start().await;

        // Mock Login ok
        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "valid_token"
            })))
            .mount(&mock_server)
            .await;

        // Mock 429 on signal-index
        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await;

        match result {
            Err(EnergyApiError::RateLimitExceeded {
                retry_after_seconds,
            }) => {
                assert_eq!(retry_after_seconds, 120);
            }
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_watttime_region_not_found() {
        let mock_server = MockServer::start().await;

        // Mock Login ok
        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "valid_token"
            })))
            .mount(&mock_server)
            .await;

        // Mock 404
        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let region = Region::new("INVALID", "Invalid");
        let result = client.get_carbon_intensity(&region).await;

        match result {
            Err(EnergyApiError::RegionNotFound { region_id }) => {
                assert_eq!(region_id, "INVALID");
            }
            _ => panic!("Expected RegionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_electricity_maps_server_error() {
        let mock_server = MockServer::start().await;

        // Mock 500
        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());

        let region = Region::new("DE", "Germany");
        let result = client.get_carbon_intensity(&region).await;

        assert!(matches!(result, Err(EnergyApiError::HttpError(_))));
    }

    #[test]
    fn test_watttime_client_creation() {
        let client = WattTimeClient::new("user".to_string(), "pass".to_string());
        let _ = &client;
    }

    #[tokio::test]
    async fn test_watttime_auth_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let result = client.ensure_token().await;
        assert!(matches!(result, Err(EnergyApiError::AuthenticationError)));
    }

    #[tokio::test]
    async fn test_watttime_token_reuse() {
        let mock_server = MockServer::start().await;

        // Should only be called once
        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "reused_token"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let t1 = client.ensure_token().await.unwrap();
        let t2 = client.ensure_token().await.unwrap();
        assert_eq!(t1, "reused_token");
        assert_eq!(t2, "reused_token");
    }

    #[tokio::test]
    async fn test_electricity_maps_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("US", "USA");
        let result = client.get_carbon_intensity(&region).await;
        assert!(matches!(result, Err(EnergyApiError::AuthenticationError)));
    }

    #[tokio::test]
    async fn test_electricity_maps_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("FR", "France");
        let result = client.get_carbon_intensity(&region).await;

        match result {
            Err(EnergyApiError::RateLimitExceeded {
                retry_after_seconds,
            }) => {
                assert_eq!(retry_after_seconds, 60);
            }
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_watttime_get_region_for_location() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "test_token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/region-from-loc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "abbrev": "CAISO_NORTH",
                "name": "California ISO - Northern"
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let region = client
            .get_region_for_location(37.7749, -122.4194)
            .await
            .unwrap();
        assert_eq!(region.id, "CAISO_NORTH");
    }

    #[tokio::test]
    async fn test_watttime_get_carbon_intensity_full() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "test_token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 800.0,
                "percent": 75
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());

        let region = Region::new("CAISO", "California");
        let intensity = client.get_carbon_intensity(&region).await.unwrap();

        assert_eq!(intensity.region.id, "CAISO");
        assert!(intensity.value > 0.0);
        assert!(intensity.rating.is_some());
    }

    #[tokio::test]
    async fn test_electricity_maps_by_location() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "DE",
                "carbonIntensity": 200.0,
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let intensity = client
            .get_carbon_intensity_by_location(52.52, 13.405)
            .await
            .unwrap();

        assert_eq!(intensity.region.id, "DE");
        assert!(intensity.value > 0.0);
    }

    #[tokio::test]
    async fn test_electricity_maps_get_region_for_location() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "FR",
                "carbonIntensity": 50.0,
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = client
            .get_region_for_location(48.8566, 2.3522)
            .await
            .unwrap();

        assert_eq!(region.id, "FR");
    }

    #[test]
    fn test_electricity_maps_client_creation() {
        let client = ElectricityMapsClient::new("api_key".to_string());
        let _ = &client;
    }

    #[tokio::test]
    async fn test_watttime_percent_rating_very_low() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 100.0,
                "percent": 10  // 0-20 = very_low
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("very_low"));
    }

    #[tokio::test]
    async fn test_watttime_percent_rating_low() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 100.0,
                "percent": 30  // 21-40 = low
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("low"));
    }

    #[tokio::test]
    async fn test_watttime_percent_rating_medium() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 100.0,
                "percent": 50  // 41-60 = medium
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("medium"));
    }

    #[tokio::test]
    async fn test_watttime_percent_rating_high() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 100.0,
                "percent": 70  // 61-80 = high
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("high"));
    }

    #[tokio::test]
    async fn test_watttime_percent_rating_very_high() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-12-25T14:00:00Z",
                "moer": 100.0,
                "percent": 90  // 81+ = very_high
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("user".to_string(), "pass".to_string())
            .with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "California");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("very_high"));
    }

    #[tokio::test]
    async fn test_electricity_maps_rating_very_low() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "NO",
                "carbonIntensity": 25.0,  // 0-50 = very_low
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("NO", "Norway");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("very_low"));
    }

    #[tokio::test]
    async fn test_electricity_maps_rating_low() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "SE",
                "carbonIntensity": 100.0,  // 51-150 = low
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("SE", "Sweden");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("low"));
    }

    #[tokio::test]
    async fn test_electricity_maps_rating_medium() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "DE",
                "carbonIntensity": 200.0,  // 151-300 = medium
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("DE", "Germany");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("medium"));
    }

    #[tokio::test]
    async fn test_electricity_maps_rating_high() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "PL",
                "carbonIntensity": 400.0,  // 301-500 = high
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("PL", "Poland");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("high"));
    }

    #[tokio::test]
    async fn test_electricity_maps_rating_very_high() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "CN",
                "carbonIntensity": 600.0,  // 501+ = very_high
                "datetime": "2025-12-25T14:00:00Z",
                "updatedAt": "2025-12-25T14:05:00Z"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("key".to_string()).with_base_url(mock_server.uri());
        let region = Region::new("CN", "China");
        let result = client.get_carbon_intensity(&region).await.unwrap();
        assert_eq!(result.rating.as_deref(), Some("very_high"));
    }

    #[tokio::test]
    async fn test_watttime_by_location() {
        // Line 167: get_carbon_intensity_by_location
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/region-from-loc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "abbrev": "CAISO",
                "name": "California"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "2025-01-01T12:00:00Z",
                "moer": 500.0,
                "percent": 50
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("u".into(), "p".into()).with_base_url(mock_server.uri());
        let result = client.get_carbon_intensity_by_location(10.0, 20.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watttime_date_parse_error() {
        // Line 145: Date parse error
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/signal-index"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ba": "CAISO",
                "point_time": "INVALID_DATE",
                "moer": 500.0
            })))
            .mount(&mock_server)
            .await;

        let client = WattTimeClient::new("u".into(), "p".into()).with_base_url(mock_server.uri());
        let region = Region::new("CAISO", "Cal");
        let result = client.get_carbon_intensity(&region).await;

        match result {
            Err(EnergyApiError::ParseError(_)) => {}
            _ => panic!("Expected ParseError"),
        }
    }

    #[tokio::test]
    async fn test_electricity_maps_date_parse_error_region() {
        // Line 259: Date parse error in get_carbon_intensity
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "DE",
                "carbonIntensity": 250.0,
                "datetime": "INVALID_DATE",
                "updatedAt": "INVALID_DATE"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("k".into()).with_base_url(mock_server.uri());
        let region = Region::new("DE", "Germany");
        let result = client.get_carbon_intensity(&region).await;

        match result {
            Err(EnergyApiError::ParseError(_)) => {}
            _ => panic!("Expected ParseError"),
        }
    }

    #[tokio::test]
    async fn test_electricity_maps_date_parse_error_location() {
        // Line 304: Date parse error in get_carbon_intensity_by_location
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/carbon-intensity/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone": "DE",
                "carbonIntensity": 250.0,
                "datetime": "INVALID_DATE",
                "updatedAt": "INVALID_DATE"
            })))
            .mount(&mock_server)
            .await;

        let client = ElectricityMapsClient::new("k".into()).with_base_url(mock_server.uri());
        let result = client.get_carbon_intensity_by_location(50.0, 10.0).await;

        match result {
            Err(EnergyApiError::ParseError(_)) => {}
            _ => panic!("Expected ParseError"),
        }
    }

    #[tokio::test]
    async fn test_watttime_token_race_stress() {
        // Line 78: Race condition check.

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "race_token"
            })))
            .mount(&mock_server)
            .await;

        let client =
            Arc::new(WattTimeClient::new("u".into(), "p".into()).with_base_url(mock_server.uri()));

        let mut handles = vec![];
        for _ in 0..10 {
            let c = client.clone();
            handles.push(tokio::spawn(async move { c.ensure_token().await }));
        }

        for h in handles {
            let token = h.await.unwrap().unwrap();
            assert_eq!(token, "race_token");
        }
    }
}
