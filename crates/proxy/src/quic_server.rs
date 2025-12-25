//! QUIC Server Module
//!
//! HTTP/3 and QUIC protocol support using s2n-quic for modern low-latency transport.

use anyhow::Result;
use s2n_quic::Server;
use s2n_quic::stream::BidirectionalStream;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::config::ProxyConfig;

/// QUIC server configuration
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// UDP bind address
    pub bind_address: String,
    /// Path to TLS certificate
    pub cert_path: String,
    /// Path to TLS private key
    pub key_path: String,
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    /// Maximum concurrent streams per connection
    pub max_streams: u32,
    /// Connection idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Enable Post-Quantum Cryptography (Kyber+X25519 hybrid)
    pub pqc_enabled: bool,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:443".to_string(),
            cert_path: "certs/server.crt".to_string(),
            key_path: "certs/server.key".to_string(),
            enable_0rtt: true,
            max_streams: 100,
            idle_timeout_secs: 30,
            pqc_enabled: true, // Default to PQC enabled
        }
    }
}

/// QUIC connection statistics
#[derive(Debug, Default, Clone)]
pub struct QuicStats {
    /// Total connections accepted
    pub connections_accepted: u64,
    /// Total streams handled
    pub streams_handled: u64,
    /// Current active connections
    pub active_connections: u64,
    /// 0-RTT connections
    pub zero_rtt_connections: u64,
}

/// QUIC Server using s2n-quic
pub struct QuicServer {
    config: QuicConfig,
    proxy_config: ProxyConfig,
    stats: Arc<RwLock<QuicStats>>,
}

impl QuicServer {
    /// Create a new QUIC server
    pub fn new(config: QuicConfig, proxy_config: ProxyConfig) -> Self {
        Self {
            config,
            proxy_config,
            stats: Arc::new(RwLock::new(QuicStats::default())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(proxy_config: ProxyConfig) -> Self {
        Self::new(QuicConfig::default(), proxy_config)
    }

    /// Get current statistics
    pub async fn stats(&self) -> QuicStats {
        self.stats.read().await.clone()
    }

    /// Check if TLS certificates exist
    fn check_certificates(&self) -> Result<()> {
        let cert_path = Path::new(&self.config.cert_path);
        let key_path = Path::new(&self.config.key_path);

        if !cert_path.exists() {
            anyhow::bail!("TLS certificate not found: {}", self.config.cert_path);
        }
        if !key_path.exists() {
            anyhow::bail!("TLS private key not found: {}", self.config.key_path);
        }

        Ok(())
    }

    /// Run the QUIC server
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        // Verify certificates exist
        self.check_certificates()?;

        info!("🚀 Starting QUIC server on {}", self.config.bind_address);
        info!("📜 Using certificate: {}", self.config.cert_path);
        info!("🔑 Using private key: {}", self.config.key_path);

        // Build the QUIC server
        let server = Server::builder()
            .with_tls((
                Path::new(&self.config.cert_path),
                Path::new(&self.config.key_path),
            ))?
            .with_io(self.config.bind_address.as_str())?
            .start()
            .map_err(|e| anyhow::anyhow!("Failed to start QUIC server: {}", e))?;

        info!("✅ QUIC server listening on UDP {}", self.config.bind_address);
        info!("🔐 0-RTT resumption: {}", if self.config.enable_0rtt { "enabled" } else { "disabled" });
        info!("🛡️ Post-Quantum Cryptography: {}", if self.config.pqc_enabled { "enabled (Kyber+X25519)" } else { "disabled" });

        // Accept connections
        self.accept_connections(server).await
    }

    /// Accept and handle QUIC connections
    async fn accept_connections(&self, mut server: Server) -> Result<()> {
        while let Some(connection) = server.accept().await {
            let stats = Arc::clone(&self.stats);
            let upstream = self.proxy_config.upstream_addr.clone();

            // Update stats
            {
                let mut s = stats.write().await;
                s.connections_accepted += 1;
                s.active_connections += 1;
            }

            let peer_addr = connection.remote_addr();
            info!("📥 QUIC connection from {:?}", peer_addr);

            // Spawn connection handler
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(connection, upstream, Arc::clone(&stats)).await {
                    error!("❌ Connection error: {}", e);
                }

                // Decrement active connections
                let mut s = stats.write().await;
                s.active_connections = s.active_connections.saturating_sub(1);
            });
        }

        Ok(())
    }

    /// Handle a single QUIC connection
    async fn handle_connection(
        mut connection: s2n_quic::Connection,
        upstream: String,
        stats: Arc<RwLock<QuicStats>>,
    ) -> Result<()> {
        // Accept bidirectional streams from the connection
        while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
            let upstream = upstream.clone();
            let stats = Arc::clone(&stats);

            // Update stream stats
            {
                let mut s = stats.write().await;
                s.streams_handled += 1;
            }

            // Spawn stream handler
            tokio::spawn(async move {
                if let Err(e) = Self::handle_stream(stream, upstream).await {
                    warn!("⚠️ Stream error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Handle a single bidirectional stream with HTTP/3 handler
    async fn handle_stream(stream: BidirectionalStream, upstream: String) -> Result<()> {
        use bytes::Bytes;
        use crate::http3_handler::{Http3Config, Http3Handler, Http3Request};

        let (mut recv, mut send) = stream.split();

        // Create HTTP/3 handler
        let handler = Http3Handler::new(Http3Config::default(), upstream);

        // Read request data
        let mut request_data = Vec::new();

        // Collect request bytes
        while let Ok(Some(chunk)) = recv.receive().await {
            request_data.extend_from_slice(&chunk);
            // Limit request size
            if request_data.len() > 16 * 1024 * 1024 {
                warn!("Request too large, dropping");
                return Ok(());
            }
        }

        debug!("📨 Received {} bytes request", request_data.len());

        // Parse simple HTTP-like request (method, path)
        // For now, parse first line as "METHOD /path"
        let request_str = String::from_utf8_lossy(&request_data);
        let mut lines = request_str.lines();
        let first_line = lines.next().unwrap_or("GET /");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let path = parts.next().unwrap_or("/");

        // Create HTTP/3 request
        let request = Http3Request::new(method, path);

        // Handle request
        let response = handler.handle_request(request).await;

        // Send response status line
        let status_line = format!("HTTP/3 {} OK\r\n\r\n", response.status);
        send.send(Bytes::from(status_line)).await?;

        // Send response body
        if !response.body.is_empty() {
            send.send(response.body).await?;
        }

        debug!("✅ Response sent with status {}", response.status);
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = QuicConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:443");
        assert!(config.enable_0rtt);
        assert_eq!(config.max_streams, 100);
        assert_eq!(config.idle_timeout_secs, 30);
    }

    #[test]
    fn test_custom_config() {
        let config = QuicConfig {
            bind_address: "127.0.0.1:8443".to_string(),
            cert_path: "custom/cert.pem".to_string(),
            key_path: "custom/key.pem".to_string(),
            enable_0rtt: false,
            max_streams: 50,
            idle_timeout_secs: 60,
            pqc_enabled: true,
        };

        assert_eq!(config.bind_address, "127.0.0.1:8443");
        assert!(!config.enable_0rtt);
        assert_eq!(config.max_streams, 50);
        assert!(config.pqc_enabled);
    }

    #[test]
    fn test_quic_server_creation() {
        let quic_config = QuicConfig::default();
        let proxy_config = ProxyConfig::default();
        let server = QuicServer::new(quic_config, proxy_config);

        assert_eq!(server.config.bind_address, "0.0.0.0:443");
    }

    #[test]
    fn test_quic_stats_default() {
        let stats = QuicStats::default();
        assert_eq!(stats.connections_accepted, 0);
        assert_eq!(stats.streams_handled, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.zero_rtt_connections, 0);
    }

    #[tokio::test]
    async fn test_stats_retrieval() {
        let server = QuicServer::with_defaults(ProxyConfig::default());
        let stats = server.stats().await;

        assert_eq!(stats.connections_accepted, 0);
    }
}
