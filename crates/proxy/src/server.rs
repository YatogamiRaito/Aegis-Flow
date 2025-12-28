//! TCP/UDP server implementation

use crate::ProxyConfig;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, instrument, warn};

/// Run the proxy server with the given configuration
#[instrument(skip(config))]
pub async fn run(config: ProxyConfig) -> Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("🎯 Aegis-Flow proxy is ready to accept connections");

    run_with_listener(listener, std::future::pending()).await
}

/// Run with provided listener and shutdown signal
pub async fn run_with_listener(
    listener: TcpListener,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<()> {
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut socket, peer_addr)) => {
                        info!("📥 New connection from: {}", peer_addr);

                        tokio::spawn(async move {
                            let mut buf = [0u8; 4096];

                            loop {
                                match socket.read(&mut buf).await {
                                    Ok(0) => {
                                        info!("📤 Connection closed: {}", peer_addr);
                                        break;
                                    }
                                    Ok(n) => {
                                        // Echo server for MVP
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
            _ = &mut shutdown => {
                info!("🛑 Shutting down proxy server");
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_echo_server() {
        // Start server on a random port
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Random port
            ..Default::default()
        };

        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();

        // Spawn server task with graceful shutdown
        let handle = tokio::spawn(async move {
            run_with_listener(listener, async {
                rx.await.ok();
            })
            .await
        });

        // Connect client
        let mut client = TcpStream::connect(addr).await.unwrap();
        let test_data = b"Hello, Aegis-Flow!";

        client.write_all(test_data).await.unwrap();

        let mut response = vec![0u8; test_data.len()];
        let result = timeout(Duration::from_secs(1), client.read_exact(&mut response)).await;

        assert!(result.is_ok());
        assert_eq!(&response, test_data);

        tx.send(()).unwrap();
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_echo_server_client_disconnects() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect and immediately drop
        {
            let _client = TcpStream::connect(addr).await.unwrap();
        }

        // Give server time to hit Ok(0)
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_echo_server_partial_read() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"partial").await.unwrap();
        // Shutdown write side to signal EOF after data
        client.shutdown().await.unwrap();

        let mut buf = vec![0u8; 7];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"partial");
    }
    #[tokio::test]
    async fn test_server_entry_point() {
        // Just verify the run() wrapper function returns the expected future
        // We can't easily run it since it binds to a port and loops forever
        // But we can check it returns a future
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };
        let future = run(config);
        // It should be a future that resolves to Result<()>
        drop(future);
    }

    #[tokio::test]
    async fn test_run_accepts_multiple_clients() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            run_with_listener(listener, async {
                rx.await.ok();
            })
            .await
            .ok();
        });

        // Connect multiple clients
        for _ in 0..3 {
            let mut client = TcpStream::connect(addr).await.unwrap();
            client.write_all(b"test").await.unwrap();
            let mut buf = [0u8; 4];
            client.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"test");
        }

        tx.send(()).unwrap();
    }

    #[tokio::test]
    async fn test_run_bind_failure() {
        // Try to bind to privileged port (should fail without root)
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 1, // Privileged port
            ..Default::default()
        };
        let result = run(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_connection_zero_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect and immediately close
        let _client = TcpStream::connect(addr).await.unwrap();
        // Client closes without sending data
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
    }

    #[tokio::test]
    async fn test_run_with_listener_immediate_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let result = run_with_listener(listener, async {}).await;
        assert!(result.is_ok());
    }
}
