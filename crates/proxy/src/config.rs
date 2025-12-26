//! Proxy configuration module
//!
//! Provides configuration loading from YAML/TOML files with environment variable overrides
//! and hot reload support.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Configuration file format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| match ext.to_str()? {
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "json" => Some(Self::Json),
            _ => None,
        })
    }
}

/// TLS/mTLS specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to server certificate
    #[serde(default = "default_cert_path")]
    pub cert_path: String,
    /// Path to private key
    #[serde(default = "default_key_path")]
    pub key_path: String,
    /// Path to CA certificate for client verification
    pub ca_path: Option<String>,
    /// Require client certificates (mTLS)
    #[serde(default)]
    pub require_client_cert: bool,
}

fn default_true() -> bool {
    true
}
fn default_cert_path() -> String {
    "/etc/aegis/certs/server.crt".to_string()
}
fn default_key_path() -> String {
    "/etc/aegis/certs/server.key".to_string()
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            ca_path: None,
            require_client_cert: false,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Enable JSON format
    #[serde(default)]
    pub json_format: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: false,
        }
    }
}

/// Health endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health endpoints
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Health check port (separate from main port)
    #[serde(default = "default_health_port")]
    pub port: u16,
    /// Liveness endpoint path
    #[serde(default = "default_liveness_path")]
    pub liveness_path: String,
    /// Readiness endpoint path
    #[serde(default = "default_readiness_path")]
    pub readiness_path: String,
}

fn default_health_port() -> u16 {
    8080
}
fn default_liveness_path() -> String {
    "/healthz".to_string()
}
fn default_readiness_path() -> String {
    "/ready".to_string()
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_health_port(),
            liveness_path: default_liveness_path(),
            readiness_path: default_readiness_path(),
        }
    }
}

/// Proxy server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// Enable TLS/mTLS
    #[serde(default = "default_true")]
    pub tls_enabled: bool,
    /// Enable Post-Quantum Cryptography
    #[serde(default = "default_true")]
    pub pqc_enabled: bool,
    /// Worker thread count (0 = auto)
    #[serde(default)]
    pub worker_threads: usize,
    /// Upstream address to forward requests to
    #[serde(default = "default_upstream")]
    pub upstream_addr: String,
    /// TLS configuration
    #[serde(default)]
    pub tls: TlsConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LogConfig,
    /// Health endpoint configuration
    #[serde(default)]
    pub health: HealthConfig,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8443
}
fn default_upstream() -> String {
    "127.0.0.1:8080".to_string()
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            tls_enabled: true,
            pqc_enabled: true,
            worker_threads: 0,
            upstream_addr: default_upstream(),
            tls: TlsConfig::default(),
            logging: LogConfig::default(),
            health: HealthConfig::default(),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from a file
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConfigError::IoError(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let format = ConfigFormat::from_path(path)
            .ok_or_else(|| ConfigError::UnsupportedFormat(path.display().to_string()))?;

        Self::parse(&content, format)
    }

    /// Parse configuration from string
    pub fn parse(content: &str, format: ConfigFormat) -> Result<Self, ConfigError> {
        let config: Self = match format {
            ConfigFormat::Yaml => serde_yml::from_str(content)
                .map_err(|e| ConfigError::ParseError(format!("YAML parse error: {}", e)))?,
            ConfigFormat::Toml => toml::from_str(content)
                .map_err(|e| ConfigError::ParseError(format!("TOML parse error: {}", e)))?,
            ConfigFormat::Json => serde_json::from_str(content)
                .map_err(|e| ConfigError::ParseError(format!("JSON parse error: {}", e)))?,
        };

        Ok(config)
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("AEGIS_HOST") {
            debug!("Overriding host from AEGIS_HOST: {}", host);
            self.host = host;
        }
        if let Ok(port) = std::env::var("AEGIS_PORT")
            && let Ok(p) = port.parse()
        {
            debug!("Overriding port from AEGIS_PORT: {}", p);
            self.port = p;
        }
        if let Ok(upstream) = std::env::var("AEGIS_UPSTREAM") {
            debug!("Overriding upstream from AEGIS_UPSTREAM: {}", upstream);
            self.upstream_addr = upstream;
        }
        if let Ok(val) = std::env::var("AEGIS_TLS_ENABLED")
            && let Ok(enabled) = val.parse()
        {
            debug!("Overriding tls_enabled from AEGIS_TLS_ENABLED: {}", enabled);
            self.tls_enabled = enabled;
        }
        if let Ok(val) = std::env::var("AEGIS_PQC_ENABLED")
            && let Ok(enabled) = val.parse()
        {
            debug!("Overriding pqc_enabled from AEGIS_PQC_ENABLED: {}", enabled);
            self.pqc_enabled = enabled;
        }
        if let Ok(workers) = std::env::var("AEGIS_WORKER_THREADS")
            && let Ok(w) = workers.parse()
        {
            debug!("Overriding worker_threads from AEGIS_WORKER_THREADS: {}", w);
            self.worker_threads = w;
        }
        if let Ok(level) = std::env::var("AEGIS_LOG_LEVEL") {
            debug!("Overriding log level from AEGIS_LOG_LEVEL: {}", level);
            self.logging.level = level;
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::ValidationError("Port cannot be 0".to_string()));
        }
        if self.upstream_addr.is_empty() {
            return Err(ConfigError::ValidationError(
                "Upstream address is required".to_string(),
            ));
        }
        if self.tls_enabled && self.tls.enabled {
            // Check that cert paths exist when TLS is enabled
            if !Path::new(&self.tls.cert_path).exists() {
                warn!("TLS certificate not found at {}", self.tls.cert_path);
            }
            if !Path::new(&self.tls.key_path).exists() {
                warn!("TLS private key not found at {}", self.tls.key_path);
            }
        }
        Ok(())
    }

    /// Create from file with environment overrides and validation
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        info!("Loading configuration from {}", path.display());
        let mut config = Self::load_from_file(path)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let format = ConfigFormat::from_path(path)
            .ok_or_else(|| ConfigError::UnsupportedFormat(path.display().to_string()))?;

        let content = match format {
            ConfigFormat::Yaml => serde_yml::to_string(self)
                .map_err(|e| ConfigError::ParseError(format!("YAML serialize error: {}", e)))?,
            ConfigFormat::Toml => toml::to_string_pretty(self)
                .map_err(|e| ConfigError::ParseError(format!("TOML serialize error: {}", e)))?,
            ConfigFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| ConfigError::ParseError(format!("JSON serialize error: {}", e)))?,
        };

        std::fs::write(path, content).map_err(|e| {
            ConfigError::IoError(format!("Failed to write {}: {}", path.display(), e))
        })?;

        info!("Configuration saved to {}", path.display());
        Ok(())
    }
}

/// Configuration error types
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
    UnsupportedFormat(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::UnsupportedFormat(path) => write!(f, "Unsupported config format: {}", path),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Hot-reloadable configuration manager
pub struct ConfigManager {
    /// Current configuration
    config: Arc<RwLock<ProxyConfig>>,
    /// Configuration file path
    config_path: Option<PathBuf>,
    /// Last modified time
    last_modified: Arc<RwLock<Option<SystemTime>>>,
}

impl ConfigManager {
    /// Create a new config manager with default configuration
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ProxyConfig::default())),
            config_path: None,
            last_modified: Arc::new(RwLock::new(None)),
        }
    }

    /// Create from file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let config = ProxyConfig::load(path)?;
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: Some(path.to_path_buf()),
            last_modified: Arc::new(RwLock::new(modified)),
        })
    }

    /// Get current configuration (clone)
    pub fn get(&self) -> ProxyConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Get configuration reference
    pub fn config(&self) -> Arc<RwLock<ProxyConfig>> {
        Arc::clone(&self.config)
    }

    /// Check if configuration file has been modified
    pub fn check_for_changes(&self) -> bool {
        let Some(ref path) = self.config_path else {
            return false;
        };

        let current_modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();

        let last = self.last_modified.read().ok().and_then(|l| *l);

        match (current_modified, last) {
            (Some(current), Some(last)) => current > last,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// Reload configuration from file
    pub fn reload(&self) -> Result<bool, ConfigError> {
        let Some(ref path) = self.config_path else {
            return Ok(false);
        };

        if !self.check_for_changes() {
            return Ok(false);
        }

        info!(
            "Configuration change detected, reloading from {}",
            path.display()
        );
        let new_config = ProxyConfig::load(path)?;

        {
            let mut config = self
                .config
                .write()
                .map_err(|_| ConfigError::IoError("Lock poisoned".to_string()))?;
            *config = new_config;
        }

        {
            let mut last_modified = self
                .last_modified
                .write()
                .map_err(|_| ConfigError::IoError("Lock poisoned".to_string()))?;
            *last_modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        }

        info!("Configuration reloaded successfully");
        Ok(true)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.port, 8443);
        assert!(config.tls_enabled);
        assert!(config.pqc_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ProxyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.port, parsed.port);
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
host: "127.0.0.1"
port: 9443
pqc_enabled: false
upstream_addr: "backend:8080"
"#;
        let config = ProxyConfig::parse(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9443);
        assert!(!config.pqc_enabled);
        assert_eq!(config.upstream_addr, "backend:8080");
    }

    #[test]
    fn test_toml_parsing() {
        let toml_content = r#"
host = "0.0.0.0"
port = 8443
pqc_enabled = true
upstream_addr = "localhost:3000"

[tls]
enabled = true
require_client_cert = true
"#;
        let config = ProxyConfig::parse(toml_content, ConfigFormat::Toml).unwrap();
        assert_eq!(config.port, 8443);
        assert!(config.tls.require_client_cert);
    }

    #[test]
    fn test_validation() {
        let mut config = ProxyConfig::default();
        assert!(config.validate().is_ok());

        config.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_format_detection() {
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.yml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.toml")),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.json")),
            Some(ConfigFormat::Json)
        );
        assert_eq!(ConfigFormat::from_path(Path::new("config.txt")), None);
    }

    #[test]
    fn test_config_manager() {
        let manager = ConfigManager::new();
        let config = manager.get();
        assert_eq!(config.port, 8443);
    }

    #[test]
    fn test_load_from_yaml_file() {
        let yaml = r#"
host: "0.0.0.0"
port: 7443
upstream_addr: "test:8080"
"#;
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = ProxyConfig::load_from_file(file.path()).unwrap();
        assert_eq!(config.port, 7443);
    }

    #[test]
    fn test_config_manager_from_file() {
        let yaml = "port: 6443\nupstream_addr: \"backend:80\"\n";
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let manager = ConfigManager::from_file(file.path()).unwrap();
        let config = manager.get();
        assert_eq!(config.port, 6443);
    }

    #[test]
    fn test_nested_config() {
        let config = ProxyConfig::default();
        assert!(config.health.enabled);
        assert_eq!(config.health.liveness_path, "/healthz");
        assert_eq!(config.logging.level, "info");
    }
}
