use crate::xds::snapshot::Snapshot;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

// Import generated types
pub mod envoy {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("envoy.api.v3");
        }
    }
}

use envoy::api::v3::*;

/// xDS Discovery Service implementation
pub struct DiscoveryService {
    snapshot: Arc<Snapshot>,
}

impl DiscoveryService {
    pub fn new(snapshot: Arc<Snapshot>) -> Self {
        Self { snapshot }
    }
}

#[tonic::async_trait]
impl listener_discovery_service_server::ListenerDiscoveryService for DiscoveryService {
    type StreamListenersStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    async fn stream_listeners(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamListenersStream>, Status> {
        info!("xDS: New LDS stream request");
        self.handle_stream("LDS", request).await
    }
}

#[tonic::async_trait]
impl cluster_discovery_service_server::ClusterDiscoveryService for DiscoveryService {
    type StreamClustersStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    async fn stream_clusters(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamClustersStream>, Status> {
        info!("xDS: New CDS stream request");
        self.handle_stream("CDS", request).await
    }
}

#[tonic::async_trait]
impl route_discovery_service_server::RouteDiscoveryService for DiscoveryService {
    type StreamRoutesStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    async fn stream_routes(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamRoutesStream>, Status> {
        info!("xDS: New RDS stream request");
        self.handle_stream("RDS", request).await
    }
}

impl DiscoveryService {
    async fn handle_stream(
        &self,
        svc: &str,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<ReceiverStream<Result<DiscoveryResponse, Status>>>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let snapshot = Arc::clone(&self.snapshot);
        let svc = svc.to_string();

        tokio::spawn(async move {
            let mut last_sent_version = String::new();

            while let Ok(Some(req)) = stream.message().await {
                let node_id = &req.node_id;

                if let Some(error) = &req.error_detail {
                    warn!(
                        "xDS: [{}] NACK received from node {}. Requested Version: {}, Nonce: {}, Error: {}",
                        svc, node_id, req.version_info, req.response_nonce, error.message
                    );
                    crate::metrics::record_error("xds_nack");
                } else if !req.response_nonce.is_empty() {
                    info!(
                        "xDS: [{}] ACK received from node {}. Version: {}, Nonce: {}",
                        svc, node_id, req.version_info, req.response_nonce
                    );
                } else {
                    info!(
                        "xDS: [{}] Initial DiscoveryRequest from node {}",
                        svc, node_id
                    );
                }

                // In xDS, if the client is ACK-ing the latest version and we have no new updates, we pause.
                if req.version_info == snapshot.version
                    && req.error_detail.is_none()
                    && last_sent_version == snapshot.version
                {
                    continue;
                }

                let nonce = format!(
                    "nonce-{}-{}",
                    snapshot.version,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_micros()
                );

                let response = DiscoveryResponse {
                    version_info: snapshot.version.clone(),
                    resources: snapshot.resources.clone(),
                    type_url: req.type_url.clone(),
                    nonce,
                };

                if let Err(e) = tx.send(Ok(response)).await {
                    warn!("xDS: [{}] Failed to send response: {}", svc, e);
                    break;
                }
                last_sent_version = snapshot.version.clone();
            }
            info!("xDS: [{}] Stream closed", svc);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl aggregated_discovery_service_server::AggregatedDiscoveryService for DiscoveryService {
    type StreamAggregatedResourcesStream = ReceiverStream<Result<DiscoveryResponse, Status>>;

    async fn stream_aggregated_resources(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamAggregatedResourcesStream>, Status> {
        info!("xDS: New ADS stream request");
        self.handle_stream("ADS", request).await
    }
}

/// Helper to start the xDS server
pub async fn run_xds_server(addr: &str, snapshot: Arc<Snapshot>) -> anyhow::Result<()> {
    let svc = DiscoveryService::new(snapshot);
    let lds = listener_discovery_service_server::ListenerDiscoveryServiceServer::new(
        DiscoveryService::new(svc.snapshot.clone()),
    );
    let cds = cluster_discovery_service_server::ClusterDiscoveryServiceServer::new(
        DiscoveryService::new(svc.snapshot.clone()),
    );
    let rds = route_discovery_service_server::RouteDiscoveryServiceServer::new(
        DiscoveryService::new(svc.snapshot.clone()),
    );
    let ads = aggregated_discovery_service_server::AggregatedDiscoveryServiceServer::new(
        DiscoveryService::new(svc.snapshot.clone()),
    );

    info!("🚀 xDS Server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(lds)
        .add_service(cds)
        .add_service(rds)
        .add_service(ads)
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
