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

    loop {
        match listener.accept().await {
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
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
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
}
