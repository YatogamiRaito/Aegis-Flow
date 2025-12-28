use aegis_proxy::server::run_with_listener;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_server_immediate_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let result = run_with_listener(listener, async {}).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_delayed_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let result = run_with_listener(listener, async {
        tokio::time::sleep(Duration::from_millis(100)).await;
    })
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_server_echo_with_client() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_with_listener(listener, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client and test echo
    let mut stream = TcpStream::connect(&addr).await.expect("Failed to connect");

    // Send data
    let test_data = b"Hello, echo server!";
    stream.write_all(test_data).await.expect("Write failed");

    // Read echoed data
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.expect("Read failed");

    assert_eq!(&buf[..n], test_data);

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_server_multiple_clients() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_with_listener(listener, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect multiple clients
    for i in 0..3 {
        let mut stream = TcpStream::connect(&addr).await.expect("Failed to connect");
        let data = format!("Message {}", i);
        stream
            .write_all(data.as_bytes())
            .await
            .expect("Write failed");

        let mut buf = vec![0u8; 64];
        let n = stream.read(&mut buf).await.expect("Read failed");
        assert_eq!(&buf[..n], data.as_bytes());
    }

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_server_client_disconnect() {
    let port = get_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        run_with_listener(listener, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect and immediately close
    {
        let stream = TcpStream::connect(&addr).await.expect("Failed to connect");
        drop(stream); // Immediate disconnect
    }

    // Give server time to handle disconnect
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Server should still be running
    let stream = TcpStream::connect(&addr).await;
    assert!(stream.is_ok());

    shutdown_tx.send(()).ok();
    let _ = server_handle.await;
}
