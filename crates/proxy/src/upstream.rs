use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastConnections,
    IpHash,
    GenericHash(String), // The hash key, e.g., "$request_uri"
    PowerOfTwoChoices,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        LoadBalanceStrategy::RoundRobin
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamServer {
    pub addr: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default)]
    pub max_connections: Option<u32>,
    #[serde(default)]
    pub backup: bool,
    #[serde(default)]
    pub down: bool,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_healthy_thresh")]
    pub healthy_threshold: u32,
    #[serde(default = "default_unhealthy_thresh")]
    pub unhealthy_threshold: u32,
    #[serde(default = "default_hc_path")]
    pub path: String,
}

fn default_interval() -> u64 {
    5000
}
fn default_timeout() -> u64 {
    2000
}
fn default_healthy_thresh() -> u32 {
    2
}
fn default_unhealthy_thresh() -> u32 {
    3
}
fn default_hc_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyConfig {
    pub cookie_name: String,
    // Add more sticky session configuration if needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_cb_error_thresh")]
    pub error_threshold_percent: u8,
    #[serde(default = "default_cb_window")]
    pub window_size_ms: u64,
    #[serde(default = "default_cb_open_time")]
    pub open_time_ms: u64,
}

fn default_cb_error_thresh() -> u8 {
    50
}
fn default_cb_window() -> u64 {
    10000
}
fn default_cb_open_time() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamGroup {
    pub name: String,
    pub servers: Vec<UpstreamServer>,
    #[serde(default)]
    pub strategy: LoadBalanceStrategy,
    pub health_check: Option<HealthCheckConfig>,
    pub sticky: Option<StickyConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upstream_server_defaults() {
        let toml_str = r#"
        addr = "127.0.0.1:8080"
        "#;
        let server: UpstreamServer = toml::from_str(toml_str).unwrap();
        assert_eq!(server.addr, "127.0.0.1:8080");
        assert_eq!(server.weight, 1);
        assert_eq!(server.max_connections, None);
        assert!(!server.backup);
        assert!(!server.down);
    }

    #[test]
    fn test_upstream_group_parsing() {
        let toml_str = r#"
        name = "backend_pool"
        strategy = "leastconnections"

        [[servers]]
        addr = "192.168.1.10:80"
        weight = 3

        [[servers]]
        addr = "192.168.1.11:80"
        backup = true

        [health_check]
        interval_ms = 10000
        path = "/health"
        "#;

        let group: UpstreamGroup = toml::from_str(toml_str).unwrap();
        assert_eq!(group.name, "backend_pool");
        assert_eq!(group.strategy, LoadBalanceStrategy::LeastConnections);
        assert_eq!(group.servers.len(), 2);
        assert_eq!(group.servers[0].weight, 3);
        assert!(group.servers[1].backup);

        let hc = group.health_check.unwrap();
        assert_eq!(hc.interval_ms, 10000);
        assert_eq!(hc.path, "/health");
        // Check structural default mapping
        assert_eq!(hc.timeout_ms, 2000);
    }
}
