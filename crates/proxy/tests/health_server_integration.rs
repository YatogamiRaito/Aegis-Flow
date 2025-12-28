use aegis_proxy::health_server::run_health_server_with_listener;
use aegis_proxy::lifecycle::LifecycleManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;

async fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_health_server_immediate_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let lifecycle = Arc::new(LifecycleManager::new());

    let result = run_health_server_with_listener(
        listener,
        lifecycle,
        None,
        async {}, // immediate shutdown
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_health_server_with_client_connection() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    let lifecycle = Arc::new(LifecycleManager::new());

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_health_server_with_listener(listener, lifecycle, None, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect and send HTTP/1.1 request
    let mut stream = TcpStream::connect(&addr).await.expect("Failed to connect");
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200") || response.contains("OK"));

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_health_server_livez_endpoint() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    let lifecycle = Arc::new(LifecycleManager::new());
    lifecycle.mark_ready().await;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_health_server_with_listener(listener, lifecycle, None, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut stream = TcpStream::connect(&addr).await.expect("Failed to connect");
    stream
        .write_all(b"GET /livez HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    // Should get a response
    assert!(n > 0);
    assert!(response.contains("HTTP"));

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_health_server_readyz_endpoint() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    let lifecycle = Arc::new(LifecycleManager::new());
    lifecycle.mark_ready().await;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_health_server_with_listener(listener, lifecycle, None, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut stream = TcpStream::connect(&addr).await.expect("Failed to connect");
    stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(n > 0);
    assert!(response.contains("HTTP"));

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_health_server_delayed_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let lifecycle = Arc::new(LifecycleManager::new());

    let result = run_health_server_with_listener(listener, lifecycle, None, async {
        tokio::time::sleep(Duration::from_millis(100)).await;
    })
    .await;

    assert!(result.is_ok());
}
