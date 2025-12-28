//! PQC-enabled proxy server implementation

use crate::config::ProxyConfig;
use aegis_crypto::stream::EncryptedStream;
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
        info!("🔒 Using algorithm: X25519-MLKEM768-Hybrid");

        self.run_with_listener(listener, std::future::pending())
            .await
    }

    /// Run with provided listener and shutdown signal
    pub async fn run_with_listener(
        &self,
        listener: TcpListener,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> Result<()> {
        // Create a pinned box for the shutdown future since we need to pin it for select!
        let mut shutdown = Box::pin(shutdown);

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((mut socket, peer_addr)) => {
                            info!("📥 New connection from: {}", peer_addr);
                            let handshake = Arc::clone(&self.handshake);
                            let config = self.config.clone();

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

                                // Secure echo server (Encrypted Data Plane)
                                let key = secure_channel.encryption_key().as_bytes();
                                let encrypted_socket = EncryptedStream::new(socket, key);
                                let io = get_tokio_io(encrypted_socket);
                                let upstream = config.upstream_addr.clone();

                                let service = hyper::service::service_fn(move |req| {
                                    let upstream = upstream.clone();
                                    async move { crate::http_proxy::handle_request(req, &upstream).await }
                                });

                                if let Err(e) = hyper::server::conn::http2::Builder::new(
                                    crate::http_proxy::TokioExecutor,
                                )
                                .max_frame_size(65535)
                                .serve_connection(io, service)
                                .await
                                {
                                    error!("❌ HTTP/2 connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            warn!("⚠️ Accept error: {}", e);
                        }
                    }
                }
                _ = &mut shutdown => {
                    info!("🛑 Shutting down PQC proxy server");
                    break;
                }
            }
        }
        Ok(())
    }
}

// Helper to wrap EncryptedStream in TokioIo
fn get_tokio_io<T>(stream: T) -> hyper_util::rt::TokioIo<T>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    hyper_util::rt::TokioIo::new(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_crypto::tls::{PqcHandshake, PqcTlsConfig};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_pqc_server_graceful_shutdown() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };

        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();

        let server = PqcProxyServer::new(config);

        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        // Trigger shutdown
        tx.send(()).unwrap();

        let result = timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "Server shutdown timed out");
        assert!(
            result.unwrap().unwrap().is_ok(),
            "Server finished with error"
        );
    }

    #[tokio::test]
    async fn test_pqc_server_handshake() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            upstream_addr: "127.0.0.1:8080".to_string(),
            ..Default::default()
        };

        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        let server = Arc::new(PqcProxyServer::new(config.clone()));
        let (tx, rx) = tokio::sync::oneshot::channel();

        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            server_clone
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
        });

        // Give server time to start accepting
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let client_handshake = PqcHandshake::new(PqcTlsConfig::default());

        // Receive server public key
        let mut pk_len_bytes = [0u8; 4];
        client.read_exact(&mut pk_len_bytes).await.unwrap();
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;

        let mut pk_bytes = vec![0u8; pk_len];
        client.read_exact(&mut pk_bytes).await.unwrap();

        let server_pk = aegis_crypto::HybridPublicKey::from_bytes(&pk_bytes).unwrap();
        let (ciphertext, client_channel) = client_handshake.client_complete(&server_pk).unwrap();

        let ct_bytes = ciphertext.to_bytes();
        client
            .write_all(&(ct_bytes.len() as u32).to_be_bytes())
            .await
            .unwrap();
        client.write_all(&ct_bytes).await.unwrap();

        // 🔒 Data Plane
        let key = client_channel.encryption_key().as_bytes();
        let mut encrypted_client = EncryptedStream::new(client, key);

        // Just verify we can write to the encrypted stream without error
        let frame1 = b"Frame 1";
        encrypted_client.write_all(frame1).await.unwrap();

        // Cleanup
        tx.send(()).unwrap();
    }

    #[tokio::test]
    async fn test_pqc_server_run_and_shutdown() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();

        let server_handle = tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut len_buf = [0u8; 4];
        let n = stream.read_exact(&mut len_buf).await.unwrap();
        assert_eq!(n, 4);

        tx.send(()).unwrap();
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(2), server_handle).await;
        assert!(result.is_ok());
    }
}
