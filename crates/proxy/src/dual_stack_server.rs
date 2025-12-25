//! Dual-Stack Server Module
//!
//! Runs HTTP/2 over TCP and HTTP/3 over QUIC simultaneously.
//! Supports Alt-Svc header for HTTP/3 discovery.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::ProxyConfig;
use crate::http_proxy::{HttpProxy, HttpProxyConfig};
use crate::quic_server::{QuicConfig, QuicServer, QuicStats};

/// Dual-stack server configuration
#[derive(Debug, Clone)]
pub struct DualStackConfig {
    /// HTTP/2 proxy configuration
    pub http2_config: HttpProxyConfig,
    /// QUIC/HTTP3 configuration
    pub quic_config: QuicConfig,
    /// Enable HTTP/3 Alt-Svc advertisement
    pub advertise_h3: bool,
    /// QUIC port for Alt-Svc header
    pub quic_port: u16,
}

impl Default for DualStackConfig {
    fn default() -> Self {
        Self {
            http2_config: HttpProxyConfig::default(),
            quic_config: QuicConfig::default(),
            advertise_h3: true,
            quic_port: 443,
        }
    }
}

impl DualStackConfig {
    /// Generate Alt-Svc header value for HTTP/3 discovery
    pub fn alt_svc_header(&self) -> String {
        if self.advertise_h3 {
            format!("h3=\":{}\"; ma=86400", self.quic_port)
        } else {
            String::new()
        }
    }
}

/// Statistics for dual-stack server
#[derive(Debug, Default, Clone)]
pub struct DualStackStats {
    /// HTTP/2 statistics
    pub http2_requests: u64,
    /// HTTP/3 statistics  
    pub http3_requests: u64,
    /// QUIC connection statistics
    pub quic_stats: QuicStats,
}

/// Dual-stack server running HTTP/2 and HTTP/3 simultaneously
pub struct DualStackServer {
    config: DualStackConfig,
    proxy_config: ProxyConfig,
    stats: Arc<RwLock<DualStackStats>>,
}

impl DualStackServer {
    /// Create a new dual-stack server
    pub fn new(config: DualStackConfig, proxy_config: ProxyConfig) -> Self {
        Self {
            config,
            proxy_config,
            stats: Arc::new(RwLock::new(DualStackStats::default())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(proxy_config: ProxyConfig) -> Self {
        Self::new(DualStackConfig::default(), proxy_config)
    }

    /// Get current statistics
    pub async fn stats(&self) -> DualStackStats {
        self.stats.read().await.clone()
    }

    /// Get Alt-Svc header value
    pub fn alt_svc_header(&self) -> String {
        self.config.alt_svc_header()
    }

    /// Run both HTTP/2 and HTTP/3 servers
    pub async fn run(&self) -> Result<()> {
        info!("🚀 Starting Dual-Stack Server (HTTP/2 + HTTP/3)");

        let alt_svc = self.alt_svc_header();
        if !alt_svc.is_empty() {
            info!("📢 Alt-Svc: {}", alt_svc);
        }

        // Clone configurations for spawned tasks
        let http2_config = self.config.http2_config.clone();
        let quic_config = self.config.quic_config.clone();
        let _proxy_config = self.proxy_config.clone();
        let proxy_config2 = self.proxy_config.clone();

        // Spawn HTTP/2 server
        let http2_handle = tokio::spawn(async move {
            info!("🌐 Starting HTTP/2 server on {}", 
                http2_config.listen_addr);
            
            let proxy = HttpProxy::new(http2_config);
            if let Err(e) = proxy.run().await {
                error!("❌ HTTP/2 server error: {}", e);
            }
        });

        // Spawn HTTP/3 server
        let http3_handle = tokio::spawn(async move {
            info!("🚀 Starting HTTP/3 server on UDP {}", 
                quic_config.bind_address);
            
            let quic_server = QuicServer::new(quic_config, proxy_config2);
            if let Err(e) = quic_server.run().await {
                error!("❌ HTTP/3 server error: {}", e);
            }
        });

        // Wait for both servers
        tokio::select! {
            result = http2_handle => {
                if let Err(e) = result {
                    error!("HTTP/2 task failed: {}", e);
                }
            }
            result = http3_handle => {
                if let Err(e) = result {
                    error!("HTTP/3 task failed: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Check if HTTP/3 is enabled
    pub fn is_h3_enabled(&self) -> bool {
        self.config.advertise_h3
    }

    /// Get the QUIC bind address
    pub fn quic_bind_address(&self) -> &str {
        &self.config.quic_config.bind_address
    }

    /// Get the HTTP/2 listen address
    pub fn http2_listen_address(&self) -> String {
        self.config.http2_config.listen_addr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DualStackConfig::default();
        assert!(config.advertise_h3);
        assert_eq!(config.quic_port, 443);
    }

    #[test]
    fn test_alt_svc_header() {
        let config = DualStackConfig::default();
        let header = config.alt_svc_header();
        assert!(header.contains("h3="));
        assert!(header.contains("ma=86400"));
    }

    #[test]
    fn test_alt_svc_disabled() {
        let config = DualStackConfig {
            advertise_h3: false,
            ..Default::default()
        };
        assert!(config.alt_svc_header().is_empty());
    }

    #[test]
    fn test_dual_stack_server_creation() {
        let config = DualStackConfig::default();
        let proxy_config = ProxyConfig::default();
        let server = DualStackServer::new(config, proxy_config);
        
        assert!(server.is_h3_enabled());
    }

    #[test]
    fn test_dual_stack_addresses() {
        let server = DualStackServer::with_defaults(ProxyConfig::default());
        
        assert_eq!(server.quic_bind_address(), "0.0.0.0:443");
        assert!(server.http2_listen_address().contains("8080"));
    }

    #[tokio::test]
    async fn test_stats_retrieval() {
        let server = DualStackServer::with_defaults(ProxyConfig::default());
        let stats = server.stats().await;
        
        assert_eq!(stats.http2_requests, 0);
        assert_eq!(stats.http3_requests, 0);
    }
}
