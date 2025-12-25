//! PQC-enabled proxy server implementation

use crate::config::ProxyConfig;
use aegis_crypto::tls::{PqcHandshake, PqcTlsConfig};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, error, info, instrument, warn};

/// PQC-enabled proxy server
pub struct PqcProxyServer {
    config: ProxyConfig,
    handshake: Arc<PqcHandshake>,
}

impl PqcProxyServer {
    /// Create a new PQC proxy server
    pub fn new(config: ProxyConfig) -> Self {
        let tls_config = PqcTlsConfig::default();
        let handshake = Arc::new(PqcHandshake::new(tls_config));

        Self { config, handshake }
    }

    /// Run the PQC proxy server
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("🎯 Aegis-Flow PQC proxy is ready to accept connections");
        info!("🔒 Using algorithm: X25519-Kyber768-Hybrid");

        loop {
            match listener.accept().await {
                Ok((mut socket, peer_addr)) => {
                    info!("📥 New connection from: {}", peer_addr);
                    let handshake = Arc::clone(&self.handshake);

                    tokio::spawn(async move {
                        // PQC Handshake Phase
                        debug!("🤝 Initiating PQC handshake with {}", peer_addr);

                        // Generate server keypair
                        let (server_pk, server_state) = match handshake.server_init() {
                            Ok(result) => result,
                            Err(e) => {
                                error!("❌ Failed to initialize handshake: {}", e);
                                return;
                            }
                        };

                        // Send public key to client
                        let pk_bytes = server_pk.to_bytes();
                        let pk_len = pk_bytes.len() as u32;

                        if let Err(e) = socket.write_all(&pk_len.to_be_bytes()).await {
                            error!("❌ Failed to send public key length: {}", e);
                            return;
                        }

                        if let Err(e) = socket.write_all(&pk_bytes).await {
                            error!("❌ Failed to send public key: {}", e);
                            return;
                        }

                        // Receive ciphertext from client
                        let mut ct_len_bytes = [0u8; 4];
                        if let Err(e) = socket.read_exact(&mut ct_len_bytes).await {
                            error!("❌ Failed to read ciphertext length: {}", e);
                            return;
                        }
                        let ct_len = u32::from_be_bytes(ct_len_bytes) as usize;

                        if ct_len > 10_000 {
                            error!("❌ Ciphertext too large: {} bytes", ct_len);
                            return;
                        }

                        let mut ct_bytes = vec![0u8; ct_len];
                        if let Err(e) = socket.read_exact(&mut ct_bytes).await {
                            error!("❌ Failed to read ciphertext: {}", e);
                            return;
                        }

                        // Parse ciphertext and complete handshake
                        let ciphertext = match aegis_crypto::HybridCiphertext::from_bytes(&ct_bytes)
                        {
                            Ok(ct) => ct,
                            Err(e) => {
                                error!("❌ Failed to parse ciphertext: {}", e);
                                return;
                            }
                        };

                        let secure_channel =
                            match handshake.server_complete(&ciphertext, server_state) {
                                Ok(channel) => channel,
                                Err(e) => {
                                    error!("❌ Failed to complete handshake: {}", e);
                                    return;
                                }
                            };

                        info!(
                            "✅ PQC handshake complete with {}, channel_id={}",
                            peer_addr,
                            secure_channel.channel_id()
                        );

                        // Secure echo server (for MVP - data is not encrypted in transit yet)
                        let mut buf = [0u8; 4096];

                        loop {
                            match socket.read(&mut buf).await {
                                Ok(0) => {
                                    info!("📤 Connection closed: {}", peer_addr);
                                    break;
                                }
                                Ok(n) => {
                                    debug!("📨 Received {} bytes from {}", n, peer_addr);
                                    // Echo back (in production, encrypt with secure_channel.encryption_key())
                                    if let Err(e) = socket.write_all(&buf[..n]).await {
                                        error!("❌ Write error: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("❌ Read error: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("⚠️ Accept error: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_crypto::tls::{PqcHandshake, PqcTlsConfig};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_pqc_server_handshake() {
        // Start server on a random port
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };

        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let handshake = Arc::new(PqcHandshake::new(PqcTlsConfig::default()));

        // Spawn server task
        let server_handshake = Arc::clone(&handshake);
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let (server_pk, server_state) = server_handshake.server_init().unwrap();

                // Send public key
                let pk_bytes = server_pk.to_bytes();
                let _ = socket
                    .write_all(&(pk_bytes.len() as u32).to_be_bytes())
                    .await;
                let _ = socket.write_all(&pk_bytes).await;

                // Receive ciphertext
                let mut ct_len_bytes = [0u8; 4];
                let _ = socket.read_exact(&mut ct_len_bytes).await;
                let ct_len = u32::from_be_bytes(ct_len_bytes) as usize;

                let mut ct_bytes = vec![0u8; ct_len];
                let _ = socket.read_exact(&mut ct_bytes).await;

                let ciphertext = aegis_crypto::HybridCiphertext::from_bytes(&ct_bytes).unwrap();
                let _channel = server_handshake
                    .server_complete(&ciphertext, server_state)
                    .unwrap();

                // Echo
                let mut buf = [0u8; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    let _ = socket.write_all(&buf[..n]).await;
                }
            }
        });

        // Client connects and performs handshake
        let mut client = TcpStream::connect(addr).await.unwrap();
        let client_handshake = PqcHandshake::new(PqcTlsConfig::default());

        // Receive server public key
        let mut pk_len_bytes = [0u8; 4];
        timeout(Duration::from_secs(1), client.read_exact(&mut pk_len_bytes))
            .await
            .unwrap()
            .unwrap();
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;

        let mut pk_bytes = vec![0u8; pk_len];
        timeout(Duration::from_secs(1), client.read_exact(&mut pk_bytes))
            .await
            .unwrap()
            .unwrap();

        let server_pk = aegis_crypto::HybridPublicKey::from_bytes(&pk_bytes).unwrap();

        // Complete handshake
        let (ciphertext, _client_channel) = client_handshake.client_complete(&server_pk).unwrap();

        // Send ciphertext
        let ct_bytes = ciphertext.to_bytes();
        client
            .write_all(&(ct_bytes.len() as u32).to_be_bytes())
            .await
            .unwrap();
        client.write_all(&ct_bytes).await.unwrap();

        // Test echo
        let test_data = b"Hello PQC World!";
        client.write_all(test_data).await.unwrap();

        let mut response = vec![0u8; test_data.len()];
        let result = timeout(Duration::from_secs(1), client.read_exact(&mut response)).await;

        assert!(result.is_ok());
        assert_eq!(&response, test_data);
    }
}
