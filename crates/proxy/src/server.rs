//! TCP/UDP server implementation

use crate::ProxyConfig;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, instrument, warn};

use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

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
    run_accept_loop(listener, shutdown).await
}

/// Trait to abstract connection accepting for testing
pub trait ConnectionAcceptor {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;
    fn accept(
        &mut self,
    ) -> impl std::future::Future<Output = std::io::Result<(Self::Stream, SocketAddr)>> + Send;
}

impl ConnectionAcceptor for TcpListener {
    type Stream = tokio::net::TcpStream;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, SocketAddr)> {
        TcpListener::accept(self).await
    }
}

/// Generic accept loop that works with any ConnectionAcceptor
pub async fn run_accept_loop<A>(
    mut acceptor: A,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<()>
where
    A: ConnectionAcceptor + Send,
{
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accept_result = acceptor.accept() => {
                match accept_result {
                    Ok((socket, peer_addr)) => {
                        info!("📥 New connection from: {}", peer_addr);
                        tokio::spawn(handle_connection(socket, peer_addr));
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

/// Handle a single client connection
pub async fn handle_connection<S>(mut socket: S, peer_addr: SocketAddr)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
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

    #[tokio::test]
    async fn test_run_with_listener_immediate_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let result = run_with_listener(listener, async {}).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_listener_dynamic_port() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        assert!(addr.port() > 0);
    }

    #[tokio::test]
    async fn test_run_with_delayed_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let result = run_with_listener(listener, async {
            tokio::time::sleep(Duration::from_millis(5)).await;
        })
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_socket_write_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            run_with_listener(listener, std::future::pending())
                .await
                .ok();
        });

        // Connect, send data, but close connection before server can echo back?
        // Server reads, then tries to write.
        // If client closes read side, server write might fail (ECONNRESET or Broken Pipe)?
        // Depends on OS TCP stack.
        let mut client = TcpStream::connect(addr).await.unwrap();

        // Use shutdown(Read) to signal we don't want response?
        // Or shutdown(Receive) on socket?
        // TcpStream::shutdown(Shutdown::Read) closes input.
        // If server writes to it, it sends RST?
        // Let's try.
        client.write_all(b"trigger_write").await.unwrap();
        // Immediately close the read end of client, so server write fails
        // Actually, we need to close the socket entirely usually to trigger broken pipe on server write.
        // But if we close socket, server loop read might error out first?
        // Race condition.
        // We'll just rely on this increasing probability of hitting error path.
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    struct MockAcceptor<F> {
        accept_fn: F,
    }

    impl<F, Fut> ConnectionAcceptor for MockAcceptor<F>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = std::io::Result<(tokio_test::io::Mock, SocketAddr)>>
            + Send,
    {
        type Stream = tokio_test::io::Mock;
        fn accept(
            &mut self,
        ) -> impl std::future::Future<Output = std::io::Result<(Self::Stream, SocketAddr)>> + Send
        {
            (self.accept_fn)()
        }
    }

    #[tokio::test]
    async fn test_accept_loop_error() {
        // Test that accept errors are logged but don't crash the server loop immediately
        let mut attempts = 0;
        let acceptor = MockAcceptor {
            accept_fn: move || {
                attempts += 1;
                async move {
                    if attempts == 1 {
                        // First attempt fails
                        Err(std::io::Error::other("Accept failed"))
                    } else {
                        // Second attempt hangs (simulation of waiting for next connection)
                        std::future::pending().await
                    }
                }
            },
        };

        // Run for a short time to process the first error
        let result = tokio::select! {
             res = run_accept_loop(acceptor, std::future::pending()) => res,
             _ = tokio::time::sleep(Duration::from_millis(50)) => Ok(()),
        };

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_connection_read_error() {
        // Mock a stream that fails on first read
        let mock = tokio_test::io::Builder::new()
            .read_error(std::io::Error::other("Read failed"))
            .build();

        let addr = "127.0.0.1:1234".parse().unwrap();

        // Should handle error gracefully (log and exit)
        handle_connection(mock, addr).await;
    }

    #[tokio::test]
    async fn test_handle_connection_write_error() {
        // Mock a stream that reads ok but fails on write
        let mock = tokio_test::io::Builder::new()
            .read(b"test data")
            .write_error(std::io::Error::other("Write failed"))
            .build();

        let addr = "127.0.0.1:1234".parse().unwrap();

        handle_connection(mock, addr).await;
    }

    #[tokio::test]
    async fn test_handle_connection_closed_by_peer() {
        // Mock a stream that returns 0 bytes (EOF) immediately
        let mock = tokio_test::io::Builder::new()
            .read(b"") // 0 bytes read
            .build();

        let addr = "127.0.0.1:1234".parse().unwrap();
        handle_connection(mock, addr).await;
    }

    #[test]
    fn test_proxy_config_defaults() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ..Default::default()
        };
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);

        let default_config = ProxyConfig::default();
        assert_eq!(default_config.host, "0.0.0.0");
        assert_eq!(default_config.port, 8443);
    }

    #[tokio::test]
    async fn test_accept_loop_mixed_results() {
        let mut attempts = 0;
        let acceptor = MockAcceptor {
            accept_fn: move || {
                attempts += 1;
                async move {
                    match attempts {
                        1 => {
                            // First: Success
                            let mock = tokio_test::io::Builder::new().build();
                            let addr = "127.0.0.1:1001".parse().unwrap();
                            Ok((mock, addr))
                        }
                        2 => {
                            // Second: Error
                            Err(std::io::Error::other("Simulated accept error"))
                        }
                        3 => {
                            // Third: Success
                            let mock = tokio_test::io::Builder::new().build();
                            let addr = "127.0.0.1:1002".parse().unwrap();
                            Ok((mock, addr))
                        }
                        _ => {
                            // Then hang
                            std::future::pending().await
                        }
                    }
                }
            },
        };

        // Run loop for a short time
        let result = tokio::select! {
             res = run_accept_loop(acceptor, std::future::pending()) => res,
             // Give enough time for the mock steps to run
             _ = tokio::time::sleep(Duration::from_millis(100)) => Ok(()),
        };
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_conversation() {
        // Conversational test: Read -> Write -> Read -> Write -> EOF
        // This exercises the `loop` and `match` arms multiple times in one go.
        let mock = tokio_test::io::Builder::new()
            .read(b"hello")
            .write(b"hello")
            .read(b"world")
            .write(b"world")
            // Implicit EOF at end matches Ok(0)
            .build();

        let addr = "127.0.0.1:1234".parse().unwrap();
        handle_connection(mock, addr).await;
    }
}
