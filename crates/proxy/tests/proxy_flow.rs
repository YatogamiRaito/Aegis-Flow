use aegis_proxy::{HttpProxy, HttpProxyConfig};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

/// Tokio executor for Hyper
#[derive(Clone, Copy)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

async fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn test_http_proxy_metrics_endpoint() {
    // 1. Initialize metrics (ignore errors if already initialized)
    let _ = std::panic::catch_unwind(aegis_proxy::metrics::init_metrics);

    // 2. Setup Proxy Config
    let proxy_port = get_free_port().await;
    let proxy_addr: SocketAddr = format!("127.0.0.1:{}", proxy_port).parse().unwrap();

    let config = HttpProxyConfig {
        listen_addr: proxy_addr,
        upstream_addr: "127.0.0.1:9090".to_string(),
        max_concurrent_streams: 100,
        initial_window_size: 65535,
    };

    let proxy = HttpProxy::new(config);

    // 3. Start Proxy in background
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let proxy_handle = tokio::spawn(async move {
        proxy
            .run_with_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
    });

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 4. Use hyper directly for HTTP/2 prior knowledge
    let stream = TcpStream::connect(proxy_addr)
        .await
        .expect("Failed to connect");
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http2::handshake(TokioExecutor, io)
        .await
        .expect("HTTP/2 handshake failed");

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Send request to /health
    let req = Request::builder()
        .uri("/health")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender
        .send_request(req)
        .await
        .expect("Failed to send request");
    assert_eq!(resp.status(), 200);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"OK");

    // Send request to /metrics
    let req = Request::builder()
        .uri("/metrics")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender
        .send_request(req)
        .await
        .expect("Failed to send metrics request");
    assert_eq!(resp.status(), 200);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&body_bytes);
    assert!(body.contains("aegis_requests_total"));

    // Cleanup
    shutdown_tx.send(()).ok();
    let _ = proxy_handle.await;
}

#[tokio::test]
async fn test_http_proxy_immediate_shutdown() {
    let proxy_port = get_free_port().await;
    let proxy_addr: SocketAddr = format!("127.0.0.1:{}", proxy_port).parse().unwrap();

    let config = HttpProxyConfig {
        listen_addr: proxy_addr,
        upstream_addr: "127.0.0.1:9091".to_string(),
        ..Default::default()
    };

    let proxy = HttpProxy::new(config);

    // Start and immediately shutdown
    let result = proxy.run_with_shutdown(async {}).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_http_proxy_multiple_requests() {
    let _ = std::panic::catch_unwind(aegis_proxy::metrics::init_metrics);

    let proxy_port = get_free_port().await;
    let proxy_addr: SocketAddr = format!("127.0.0.1:{}", proxy_port).parse().unwrap();

    let config = HttpProxyConfig {
        listen_addr: proxy_addr,
        upstream_addr: "127.0.0.1:9092".to_string(),
        max_concurrent_streams: 100,
        initial_window_size: 65535,
    };

    let proxy = HttpProxy::new(config);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let proxy_handle = tokio::spawn(async move {
        proxy
            .run_with_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client
    let stream = TcpStream::connect(proxy_addr)
        .await
        .expect("Failed to connect");
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http2::handshake(TokioExecutor, io)
        .await
        .expect("HTTP/2 handshake failed");

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Send multiple requests
    for path in ["/health", "/metrics", "/health", "/"] {
        let req = Request::builder()
            .uri(path)
            .body(Empty::<Bytes>::new())
            .unwrap();
        let resp = sender.send_request(req).await.expect("Request failed");
        assert!(resp.status().is_success());
    }

    shutdown_tx.send(()).ok();
    let _ = proxy_handle.await;
}

#[tokio::test]
async fn test_http_proxy_with_post_request() {
    let _ = std::panic::catch_unwind(aegis_proxy::metrics::init_metrics);

    let proxy_port = get_free_port().await;
    let proxy_addr: SocketAddr = format!("127.0.0.1:{}", proxy_port).parse().unwrap();

    let config = HttpProxyConfig {
        listen_addr: proxy_addr,
        upstream_addr: "127.0.0.1:9093".to_string(),
        ..Default::default()
    };

    let proxy = HttpProxy::new(config);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let proxy_handle = tokio::spawn(async move {
        proxy
            .run_with_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let stream = TcpStream::connect(proxy_addr)
        .await
        .expect("Failed to connect");
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http2::handshake(TokioExecutor, io)
        .await
        .expect("HTTP/2 handshake failed");

    tokio::spawn(async move {
        conn.await.ok();
    });

    // POST request
    let req = Request::builder()
        .method("POST")
        .uri("/api/data")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.expect("POST failed");
    assert!(resp.status().is_success());

    shutdown_tx.send(()).ok();
    let _ = proxy_handle.await;
}

#[tokio::test]
async fn test_http_proxy_delayed_shutdown() {
    let proxy_port = get_free_port().await;
    let proxy_addr: SocketAddr = format!("127.0.0.1:{}", proxy_port).parse().unwrap();

    let config = HttpProxyConfig {
        listen_addr: proxy_addr,
        upstream_addr: "127.0.0.1:9094".to_string(),
        ..Default::default()
    };

    let proxy = HttpProxy::new(config);

    // Shutdown after 100ms
    let result = proxy
        .run_with_shutdown(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

    assert!(result.is_ok());
}
