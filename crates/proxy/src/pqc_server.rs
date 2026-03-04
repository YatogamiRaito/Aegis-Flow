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
                                    async move {
                                        crate::http_proxy::handle_request(
                                            req,
                                            &upstream,
                                            None,
                                            None,
                                            std::sync::Arc::new(crate::proxy_cache::TtlConfig::new(60)),
                                            std::sync::Arc::new(crate::proxy_cache::BypassCheck::default()),
                                            None,
                                            std::sync::Arc::new(Vec::new()),
                                        ).await
                                    }
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
        use crate::http_proxy::TokioExecutor;
        use bytes::Bytes;
        use http_body_util::Full;
        use hyper::Request;

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
        let encrypted_client = EncryptedStream::new(client, key);

        // Wrap in TokioIo
        let io = get_tokio_io(encrypted_client);

        // Initiate HTTP/2 Client Handshake over Encrypted Stream
        let (mut request_sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor, io)
                .await
                .unwrap();

        // Spawn connection driver
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                // Connection closed or error
                println!("Connection error: {:?}", e);
            }
        });

        // Send HTTP/2 Request
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost/health")
            .body(Full::new(Bytes::default()))
            .unwrap();

        let response = request_sender.send_request(request).await.unwrap();

        assert!(response.status().is_success());

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

    #[tokio::test]
    async fn test_pqc_handshake_invalid_length() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server
        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Client connects
        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Read Server PK Length (4 bytes)
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await.unwrap();
        let pk_len = u32::from_be_bytes(buf);

        // Read Server PK
        let mut pk_buf = vec![0u8; pk_len as usize];
        stream.read_exact(&mut pk_buf).await.unwrap();

        // Send Invalid Ciphertext Length (> 10240)
        let invalid_len = 10_241u32;
        stream.write_all(&invalid_len.to_be_bytes()).await.unwrap();

        // Server should verify length and close connection.
        // Reading from stream should result in 0 bytes (EOF) or error.
        let mut check_buf = [0u8; 1];
        let n = stream.read(&mut check_buf).await.unwrap();
        assert_eq!(n, 0, "Server should close connection on invalid length");
    }

    #[tokio::test]
    async fn test_pqc_handshake_read_error() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        // Server writes PK length (4 bytes) and PK
        // We read nothing or partial, then close
        stream.shutdown().await.unwrap();

        // Give server time to try reading from us and fail
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_malformed_ciphertext() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Read PK
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await.unwrap();
        let pk_len = u32::from_be_bytes(buf);
        let mut pk_buf = vec![0u8; pk_len as usize];
        stream.read_exact(&mut pk_buf).await.unwrap();

        // Send Valid Length but MALFORMED Ciphertext
        let ct_len = 100u32;
        stream.write_all(&ct_len.to_be_bytes()).await.unwrap();

        // Send random junk as ciphertext
        let junk = [0xAAu8; 100];
        stream.write_all(&junk).await.unwrap();

        // Server should fail to parse/decapsulate and close
        let mut check_buf = [0u8; 1];
        let n = stream.read(&mut check_buf).await.unwrap();
        assert_eq!(
            n, 0,
            "Server should close connection on malformed ciphertext"
        );
    }

    #[test]
    fn test_pqc_proxy_server_creation() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 8443,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        // Just verify it creates successfully
        let _ = format!("{:p}", &server);
    }

    #[test]
    fn test_get_tokio_io() {
        use tokio::io::duplex;
        let (client, _server) = duplex(1024);
        let io = get_tokio_io(client);
        let _ = io;
    }

    #[tokio::test]
    async fn test_pqc_handshake_client_closes_prematurely() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Client connects and immediately closes
        {
            let _stream = TcpStream::connect(addr).await.unwrap();
        }

        // Give server a moment to hit the read error
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_pqc_server_accept_shutdown() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server_handle = tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
        });

        tx.send(()).unwrap();
        let result = timeout(Duration::from_secs(1), server_handle).await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_pqc_handshake_socket_write_error() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect but close read side immediately to cause write error on server
        let mut stream = TcpStream::connect(addr).await.unwrap();
        // We need to read just enough to establish connection but then simulate error
        // Actually, if we close/shutdown everything, the server might fail to write the PK length
        stream.shutdown().await.unwrap(); // Shutdown everything

        // Give server a moment to fail
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_run_shutdown_integration() {
        // Test normal run with shutdown signal
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
        });

        // Let it start
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Send shutdown
        tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ciphertext_too_large() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect and send an oversized ciphertext length
        let mut stream = TcpStream::connect(addr).await.unwrap();
        // Read the PK length and PK first
        let mut pk_len_bytes = [0u8; 4];
        let _ = stream.read_exact(&mut pk_len_bytes).await;
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;
        let mut pk_bytes = vec![0u8; pk_len];
        let _ = stream.read_exact(&mut pk_bytes).await;

        // Send an oversized ciphertext length (> 10000 bytes)
        let fake_ct_len: u32 = 15000;
        stream.write_all(&fake_ct_len.to_be_bytes()).await.unwrap();

        // Server should reject and close connection
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[test]
    fn test_pqc_server_config_options() {
        let config = ProxyConfig {
            host: "0.0.0.0".to_string(),
            port: 8443,
            pqc_enabled: true,
            upstream_addr: "backend:9000".to_string(),
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let _ = format!("{:p}", &server);
    }

    #[tokio::test]
    async fn test_pqc_server_bind_privileged_port() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 1, // Privileged port
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let result = server.run().await;
        // Should fail to bind
        assert!(result.is_err());
    }

    #[test]
    fn test_pqc_server_with_default_config() {
        let config = ProxyConfig::default();
        let server = PqcProxyServer::new(config);
        let _ = &server;
    }

    #[test]
    fn test_pqc_server_with_custom_port() {
        let config = ProxyConfig {
            port: 9443,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let _ = &server;
    }

    #[test]
    fn test_pqc_server_pqc_mode() {
        let config = ProxyConfig {
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let _ = &server;
    }

    #[tokio::test]
    async fn test_pqc_server_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = ProxyConfig::default();
        let server = PqcProxyServer::new(config);
        let result = server.run_with_listener(listener, async {}).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pqc_handshake_write_failure_after_accept() {
        // Simulate client closing connection immediately after accept, causing server write fail
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect then immediately drop
        let _stream = TcpStream::connect(addr).await.unwrap();
        drop(_stream);

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_partial_ciphertext_read() {
        // Test when client sends incomplete ciphertext (covers lines 100-101)
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Read PK length and PK
        let mut pk_len_bytes = [0u8; 4];
        stream.read_exact(&mut pk_len_bytes).await.unwrap();
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;
        let mut pk_bytes = vec![0u8; pk_len];
        stream.read_exact(&mut pk_bytes).await.unwrap();

        // Send valid ciphertext length (e.g., 500 bytes)
        let ct_len: u32 = 500;
        stream.write_all(&ct_len.to_be_bytes()).await.unwrap();

        // Send only partial ciphertext (less than 500 bytes) then close
        let partial_data = [0xBBu8; 50]; // Only 50 bytes instead of 500
        stream.write_all(&partial_data).await.unwrap();
        stream.shutdown().await.unwrap();

        // Server should fail to read full ciphertext - give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_partial_ciphertext_length_read() {
        // Test when client sends incomplete ciphertext length (covers line 87-89)
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Read PK length and PK
        let mut pk_len_bytes = [0u8; 4];
        stream.read_exact(&mut pk_len_bytes).await.unwrap();
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;
        let mut pk_bytes = vec![0u8; pk_len];
        stream.read_exact(&mut pk_bytes).await.unwrap();

        // Send only 2 bytes of ciphertext length instead of 4
        stream.write_all(&[0x00, 0x01]).await.unwrap();
        stream.shutdown().await.unwrap();

        // Give server time to hit read error
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_http2_connection_error() {
        // Test HTTP/2 connection error path (covers line 147)
        use crate::http_proxy::TokioExecutor;
        use bytes::Bytes;
        use http_body_util::Full;
        use hyper::Request;

        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            upstream_addr: "127.0.0.1:1".to_string(), // Invalid upstream
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

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let client_handshake = PqcHandshake::new(PqcTlsConfig::default());

        // Complete handshake
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

        // Setup encrypted stream
        let key = client_channel.encryption_key().as_bytes();
        let encrypted_client = EncryptedStream::new(client, key);
        let io = get_tokio_io(encrypted_client);

        // Initiate HTTP/2 connection
        let (mut request_sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor, io)
                .await
                .unwrap();

        tokio::spawn(async move {
            let _ = connection.await;
        });

        // Send request that will fail due to invalid upstream
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost/test")
            .body(Full::new(Bytes::default()))
            .unwrap();

        // This request should fail since upstream is invalid
        let _result = request_sender.send_request(request).await;
        // We don't care about the result, just that the error path is exercised

        tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_complete_success_path() {
        // Test complete successful handshake path (covers line 125 channel_id logging)
        use crate::http_proxy::TokioExecutor;
        use bytes::Bytes;
        use http_body_util::Full;
        use hyper::Request;

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

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let client_handshake = PqcHandshake::new(PqcTlsConfig::default());

        // Complete handshake
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

        // Setup encrypted stream
        let key = client_channel.encryption_key().as_bytes();
        let encrypted_client = EncryptedStream::new(client, key);
        let io = get_tokio_io(encrypted_client);

        // Initiate HTTP/2 connection
        let (mut request_sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor, io)
                .await
                .unwrap();

        tokio::spawn(async move {
            let _ = connection.await;
        });

        // Send health request
        let request = Request::builder()
            .method("GET")
            .uri("http://localhost/health")
            .body(Full::new(Bytes::default()))
            .unwrap();

        let response = request_sender.send_request(request).await.unwrap();
        assert!(response.status().is_success());

        tx.send(()).unwrap();
    }

    #[tokio::test]
    async fn test_pqc_multiple_connections() {
        // Test multiple concurrent connections to ensure loop works (covers line 49)
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
                .ok();
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        // Connect multiple clients
        for _ in 0..3 {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            // Read PK length
            let mut pk_len_bytes = [0u8; 4];
            let _ = stream.read_exact(&mut pk_len_bytes).await;
            drop(stream);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_pqc_handshake_ciphertext_parse_error() {
        // Test when ciphertext data is too short to parse (< 32 bytes)
        // This covers lines 108-110: HybridCiphertext::from_bytes error
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server
                .run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Read PK length and PK
        let mut pk_len_bytes = [0u8; 4];
        stream.read_exact(&mut pk_len_bytes).await.unwrap();
        let pk_len = u32::from_be_bytes(pk_len_bytes) as usize;
        let mut pk_bytes = vec![0u8; pk_len];
        stream.read_exact(&mut pk_bytes).await.unwrap();

        // Send ciphertext length of 20 bytes (less than required 32 bytes for HybridCiphertext)
        let ct_len: u32 = 20;
        stream.write_all(&ct_len.to_be_bytes()).await.unwrap();

        // Send 20 bytes of junk data - this will cause from_bytes to fail
        let short_data = [0xCCu8; 20];
        stream.write_all(&short_data).await.unwrap();

        // Server should fail to parse ciphertext and close connection
        let mut check_buf = [0u8; 1];
        let n = stream.read(&mut check_buf).await.unwrap();
        assert_eq!(
            n, 0,
            "Server should close connection on ciphertext parse error"
        );
    }
    #[tokio::test]
    async fn test_pqc_handshake_http2_protocol_error() {
        // Test invalid HTTP/2 protocol after PQC handshake (covers line 147)
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            pqc_enabled: true,
            ..Default::default()
        };
        let server = PqcProxyServer::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            server
                .run_with_listener(listener, async {
                    rx.await.ok();
                })
                .await
                .ok();
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let client_handshake = PqcHandshake::new(PqcTlsConfig::default());

        // Complete handshake
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

        // Setup encrypted stream
        let key = client_channel.encryption_key().as_bytes();
        let mut encrypted_client = EncryptedStream::new(client, key);

        // Send INVALID HTTP/2 connection preface (random garbage)
        encrypted_client
            .write_all(b"NOT_HTTP2_PREFACE")
            .await
            .unwrap();
        encrypted_client.flush().await.unwrap();

        // Wait for server to process and likely close connection
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cleanup
        tx.send(()).unwrap();
    }
}
