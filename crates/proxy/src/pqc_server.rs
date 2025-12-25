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
        info!("🔒 Using algorithm: X25519-Kyber768-Hybrid");

        loop {
            match listener.accept().await {
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
        let (ciphertext, client_channel) = client_handshake.client_complete(&server_pk).unwrap();

        // Send ciphertext to server (Handshake Finalization)
        let ct_bytes = ciphertext.to_bytes();
        client
            .write_all(&(ct_bytes.len() as u32).to_be_bytes())
            .await
            .unwrap();
        client.write_all(&ct_bytes).await.unwrap();

        // 🔒 Upgrade to Encrypted Data Plane
        let key = client_channel.encryption_key().as_bytes();
        let mut encrypted_client = aegis_crypto::stream::EncryptedStream::new(client, key);
        // let io = get_tokio_io(encrypted_client);

        // Test RAW multi-frame echo (bypass Hyper to verify stream integrity)
        let frame1 = b"Frame 1: reliable transport";
        encrypted_client.write_all(frame1).await.unwrap();
        encrypted_client.flush().await.unwrap();

        let frame2 = b"Frame 2: multiple chunks check";
        encrypted_client.write_all(frame2).await.unwrap();
        encrypted_client.flush().await.unwrap();

        let mut buf = vec![0u8; frame1.len() + frame2.len()];
        encrypted_client.read_exact(&mut buf).await.unwrap();

        println!("Received echo: {:?}", String::from_utf8_lossy(&buf));
        assert_eq!(&buf[..frame1.len()], frame1);
        assert_eq!(&buf[frame1.len()..], frame2);
        /*
        // Send HTTP Request
        // Configure explicit frame size to match EncryptedStream capabilities
        let (mut sender, conn) = hyper::client::conn::http2::Builder::new(crate::http_proxy::TokioExecutor)
            .max_frame_size(65535)
            .handshake(io)
            .await
            .unwrap();

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                error!("Connection failed: {:?}", e);
            }
        });

        let req = hyper::Request::builder()
            .uri("http://localhost/ready")
            .body(http_body_util::Full::new(bytes::Bytes::new()))
            .unwrap();

        let res = sender.send_request(req).await.unwrap();

        assert_eq!(res.status(), hyper::StatusCode::OK);

        // Read response body
        use http_body_util::BodyExt;
        let body = res.collect().await.unwrap().to_bytes();
        assert_eq!(body, "{\"status\":\"ready\"}");
        */
    }
}
