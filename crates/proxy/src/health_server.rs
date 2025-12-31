use crate::config::HealthConfig;
use crate::lifecycle::LifecycleManager;
use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use metrics_exporter_prometheus::PrometheusHandle;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

/// Run the health/admin server
pub async fn run_health_server(
    config: HealthConfig,
    lifecycle: Arc<LifecycleManager>,
    metrics_handle: Option<PrometheusHandle>,
) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await?;

    info!("🏥 Health server listening on http://{}", addr);

    run_health_server_with_listener(listener, lifecycle, metrics_handle, std::future::pending())
        .await
}

pub async fn run_health_server_with_listener(
    listener: TcpListener,
    lifecycle: Arc<LifecycleManager>,
    metrics_handle: Option<PrometheusHandle>,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<()> {
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer_addr)) => {
                        let io = TokioIo::new(stream);
                        let lifecycle = lifecycle.clone();
                        let metrics_handle = metrics_handle.clone();

                        tokio::task::spawn(async move {
                            if let Err(err) = http1::Builder::new()
                                .serve_connection(
                                    io,
                                    service_fn(move |req| {
                                        handle_request(req, lifecycle.clone(), metrics_handle.clone())
                                    }),
                                )
                                .await
                            {
                                warn!(
                                    "Error serving health connection from {}: {}",
                                    peer_addr, err
                                );
                            }
                        });
                    }
                    Err(e) => {
                        error!("Health server accept error: {}", e);
                    }
                }
            }
            _ = &mut shutdown => {
                info!("🛑 Shutting down health server");
                break;
            }
        }
    }
    Ok(())
}

async fn handle_request<B>(
    req: Request<B>,
    lifecycle: Arc<LifecycleManager>,
    metrics_handle: Option<PrometheusHandle>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/health") => {
            let response = lifecycle.health_response().await;
            let json = serde_json::to_string(&response).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        }
        (&Method::GET, "/ready") => {
            let status = lifecycle.health_status().await;
            if status.is_ready() {
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::new(Bytes::from("Not Ready")))
                    .unwrap())
            }
        }
        (&Method::GET, "/metrics") => {
            if let Some(handle) = metrics_handle {
                let metrics = handle.render();
                Ok(Response::builder()
                    .header("Content-Type", "text/plain")
                    .body(Full::new(Bytes::from(metrics)))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(StatusCode::NOT_IMPLEMENTED)
                    .body(Full::new(Bytes::from("Metrics not enabled")))
                    .unwrap())
            }
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_lifecycle() -> Arc<LifecycleManager> {
        Arc::new(LifecycleManager::new())
    }

    #[tokio::test]
    async fn test_health_response_from_lifecycle() {
        let lifecycle = create_test_lifecycle();

        // Test health response generation
        let response = lifecycle.health_response().await;
        // Response should have required fields
        assert!(response.uptime_seconds.is_some());
    }

    #[tokio::test]
    async fn test_ready_endpoint_initial_state() {
        let lifecycle = create_test_lifecycle();

        // Initial state should not be ready
        let status = lifecycle.health_status().await;
        assert!(!status.is_ready());
    }

    #[tokio::test]
    async fn test_ready_endpoint_after_mark_ready() {
        let lifecycle = create_test_lifecycle();

        // Mark as ready
        lifecycle.mark_ready().await;

        // After marking ready, should be ready
        let status = lifecycle.health_status().await;
        assert!(status.is_ready());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_transitions() {
        let lifecycle = create_test_lifecycle();

        // Initial state
        let status = lifecycle.health_status().await;
        assert!(!status.is_ready());

        // After mark_ready
        lifecycle.mark_ready().await;
        let status = lifecycle.health_status().await;
        assert!(status.is_ready());

        // After mark_unhealthy
        lifecycle.mark_unhealthy().await;
        let status = lifecycle.health_status().await;
        assert!(!status.is_ready());
    }

    #[test]
    fn test_health_config_default() {
        let config = HealthConfig::default();
        assert!(config.port > 0);
    }

    #[test]
    fn test_health_config_clone() {
        let config = HealthConfig {
            port: 9090,
            enabled: true,
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.port, 9090);
        assert!(cloned.enabled);
    }

    #[test]
    fn test_connection_tracking() {
        let lifecycle = create_test_lifecycle();

        assert_eq!(lifecycle.active_connections(), 0);

        lifecycle.connection_started();
        assert_eq!(lifecycle.active_connections(), 1);

        lifecycle.connection_started();
        assert_eq!(lifecycle.active_connections(), 2);

        lifecycle.connection_finished();
        assert_eq!(lifecycle.active_connections(), 1);
    }

    #[test]
    fn test_uptime() {
        let lifecycle = create_test_lifecycle();

        // Uptime should be zero or positive
        let uptime = lifecycle.uptime();
        assert!(uptime.as_secs() < 5); // Should be nearly zero
    }

    #[tokio::test]
    async fn test_handle_request_health() {
        let lifecycle = create_test_lifecycle();
        let req = Request::builder()
            .uri("/health")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let resp = handle_request(req, lifecycle, None).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[tokio::test]
    async fn test_handle_request_ready() {
        let lifecycle = create_test_lifecycle();

        // Not ready initially
        let req = Request::builder()
            .uri("/ready")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let resp = handle_request(req, lifecycle.clone(), None).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Mark ready
        lifecycle.mark_ready().await;

        let req = Request::builder()
            .uri("/ready")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();
        let resp = handle_request(req, lifecycle, None).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_handle_request_404() {
        let lifecycle = create_test_lifecycle();
        let req = Request::builder()
            .uri("/unknown")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let resp = handle_request(req, lifecycle, None).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_request_metrics_disabled() {
        let lifecycle = create_test_lifecycle();
        let req = Request::builder()
            .uri("/metrics")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let resp = handle_request(req, lifecycle, None).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_health_server_run_and_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let lifecycle = Arc::new(LifecycleManager::new());
        let (tx, rx) = tokio::sync::oneshot::channel();

        let server_handle = tokio::spawn(async move {
            run_health_server_with_listener(listener, lifecycle, None, async {
                rx.await.ok();
            })
            .await
        });

        // Give it a moment to start accept loop
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Make a request to prove it's running
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http::<http_body_util::Full<bytes::Bytes>>();

        let uri = format!("http://{}/health", addr).parse().unwrap();
        let resp = client.get(uri).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Shutdown
        tx.send(()).unwrap();

        // Wait for server to finish
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(2), server_handle).await;
        assert!(result.is_ok(), "Server did not shut down in time");
        assert!(result.unwrap().is_ok()); // Task finished successfully
    }

    #[tokio::test]
    async fn test_health_server_bind_failure() {
        // Bind to a port first
        let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let config = HealthConfig {
            port,
            enabled: true,
            ..Default::default()
        };
        let lifecycle = Arc::new(LifecycleManager::new());

        let result = run_health_server(config, lifecycle, None).await;
        assert!(result.is_err(), "Should fail to bind to an occupied port");
    }

    #[tokio::test]
    async fn test_lifecycle_unhealthy_transition() {
        let lifecycle = create_test_lifecycle();
        lifecycle.mark_ready().await;
        assert!(lifecycle.health_status().await.is_ready());

        lifecycle.mark_unhealthy().await;
        assert!(!lifecycle.health_status().await.is_ready());
    }

    #[tokio::test]
    async fn test_run_health_server_not_bindable() {
        let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let config = HealthConfig {
            port,
            enabled: true,
            ..Default::default()
        };
        let lifecycle = Arc::new(LifecycleManager::new());

        let result = run_health_server(config, lifecycle, None).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_health_config_default_values() {
        let config = HealthConfig::default();
        assert_eq!(config.port, 8081); // Default health port (avoids conflict with frontend dev servers)
        assert!(config.enabled);
    }

    #[test]
    fn test_health_config_custom() {
        let config = HealthConfig {
            port: 8888,
            enabled: false,
            ..Default::default()
        };
        assert_eq!(config.port, 8888);
        assert!(!config.enabled);
    }

    #[tokio::test]
    async fn test_handle_request_metrics_with_handle() {
        let lifecycle = create_test_lifecycle();
        // Initialize metrics to get a handle
        let handle = crate::metrics::init_metrics();

        let req = Request::builder()
            .uri("/metrics")
            .method(Method::GET)
            .body(http_body_util::Empty::<Bytes>::new())
            .unwrap();

        let resp = handle_request(req, lifecycle, Some(handle)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("Content-Type").unwrap(), "text/plain");
    }

    #[tokio::test]
    async fn test_health_server_protocol_error() {
        // Line 59-61: Trigger error in serve_connection
        // We start the server, connect, and send garbage.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let lifecycle = Arc::new(LifecycleManager::new());
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Spawn server
        let server_handle = tokio::spawn(async move {
            run_health_server_with_listener(listener, lifecycle, None, async {
                rx.await.ok();
            })
            .await
        });

        // Connect and send garbage to trigger protocol error in http1::serve_connection
        use tokio::io::AsyncWriteExt;
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(b"NOT HTTP\r\n\r\n").await.unwrap();
        // Close write to force EOF or let server react
        // The server might log a warn. We can't verify the log easily but we hit the path.

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Clean shutdown
        tx.send(()).unwrap();
        let _ = server_handle.await;
    }
}
