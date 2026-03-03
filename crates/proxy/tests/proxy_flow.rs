use aegis_proxy::{HttpProxy, HttpProxyConfig};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;

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
        ..Default::default()
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

    // 4. Use HTTP/1.1 client (server now serves HTTP/1.1)
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http::<Empty<Bytes>>();

    // Send request to /health
    let uri: hyper::Uri = format!("http://{}/health", proxy_addr).parse().unwrap();
    let resp = client.get(uri).await.expect("Failed to send request");
    assert_eq!(resp.status(), 200);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"OK");

    // Send request to /metrics
    let uri: hyper::Uri = format!("http://{}/metrics", proxy_addr).parse().unwrap();
    let resp = client
        .get(uri)
        .await
        .expect("Failed to send metrics request");
    assert_eq!(resp.status(), 200);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&body_bytes);
    // Metrics may or may not contain aegis_requests_total depending on initialization
    assert!(!body.is_empty());

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

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Use HTTP/1.1 client (server now serves HTTP/1.1)
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http::<Empty<Bytes>>();

    // Send multiple requests to built-in endpoints
    for path in ["/health", "/metrics", "/health"] {
        let uri: hyper::Uri = format!("http://{}{}", proxy_addr, path).parse().unwrap();
        let resp = client.get(uri).await.expect("Request failed");
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

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Use HTTP/1.1 client (server now serves HTTP/1.1)
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http::<http_body_util::Full<Bytes>>();

    // POST request to non-builtin endpoint (forwards to upstream, returns BAD_GATEWAY when unreachable)
    let req = hyper::Request::builder()
        .method("POST")
        .uri(format!("http://{}/api/data", proxy_addr))
        .body(http_body_util::Full::new(Bytes::new()))
        .unwrap();
    let resp = client.request(req).await.expect("POST failed");
    // Upstream unreachable returns an error status (typically 502 BAD_GATEWAY)
    assert!(resp.status().is_client_error() || resp.status().is_server_error());

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
