use aegis_proxy::PqcProxyServer;
use aegis_proxy::ProxyConfig;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

async fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_pqc_server_immediate_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ProxyConfig::default();
    let server = PqcProxyServer::new(config);

    let result = server.run_with_listener(listener, async {}).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pqc_server_delayed_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = ProxyConfig::default();
    let server = PqcProxyServer::new(config);

    let result = server
        .run_with_listener(listener, async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pqc_server_with_client_connect() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    let config = ProxyConfig {
        host: "127.0.0.1".to_string(),
        port,
        ..Default::default()
    };
    let server = PqcProxyServer::new(config);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        server
            .run_with_listener(listener, async {
                shutdown_rx.await.ok();
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client - this triggers the accept() branch
    let stream = TcpStream::connect(&addr).await;
    assert!(stream.is_ok());

    // Give server a moment to handle connection
    tokio::time::sleep(Duration::from_millis(50)).await;

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_pqc_server_accept_multiple_connections() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    let config = ProxyConfig::default();
    let server = PqcProxyServer::new(config);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        server
            .run_with_listener(listener, async {
                shutdown_rx.await.ok();
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect multiple clients
    for _ in 0..3 {
        let _ = TcpStream::connect(&addr).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_pqc_server_with_pqc_enabled() {
    let port = get_free_port().await;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    let config = ProxyConfig {
        pqc_enabled: true,
        ..Default::default()
    };
    let server = PqcProxyServer::new(config);

    let result = server.run_with_listener(listener, async {}).await;
    assert!(result.is_ok());
}
