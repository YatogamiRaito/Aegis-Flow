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
    async fn test_server_entry_point() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        };

        // We can't easily wait for "ready" with run(), so we just spawn it
        // and loop connect until successful or timeout
        let _server_task = tokio::spawn(async move {
            super::run(config).await
        });

        // Give it a tiny bit to bind
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Since we don't know the port (it was 0), this test is tricky with run().
        // run() binds and doesn't return the address.
        // Wait, run() logs the address? No, it logs "ready".
        // Actually, if we pass port 0, we can't know what port it bound to without 
        // passing a listener or having a channel.
        // The current run() implementation is:
        // let listener = TcpListener::bind(&addr).await?;
        // run_with_listener...
        
        // If we can't determine the port, we can't connect.
        // So we should probably modify run() to strictly use the config port 
        // or just test run_with_listener properly if run() is just a wrapper.
        // coverage for run() might be low priority if it's just a bind wrapper.
        // However, I can test run() failure (invalid address).
        
        let bad_config = ProxyConfig {
            host: "999.999.999.999".to_string(), // Invalid IP
            port: 80,
            ..Default::default()
        };
        
        let result = super::run(bad_config).await;
        assert!(result.is_err());
    }
}
