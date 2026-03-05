use tonic::{Request, Response, Status};
use futures_util::Stream;
use std::pin::Pin;
use crate::xds::envoy::api::v3::{
    cluster_discovery_service_server::ClusterDiscoveryService,
    DiscoveryRequest, DiscoveryResponse,
};

pub struct CdsService;

#[tonic::async_trait]
impl ClusterDiscoveryService for CdsService {
    type StreamClustersStream = Pin<Box<dyn Stream<Item = Result<DiscoveryResponse, Status>> + Send>>;

    async fn stream_clusters(
        &self,
        _request: Request<tonic::Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamClustersStream>, Status> {
        Err(Status::unimplemented("CDS not fully implemented yet"))
    }
}
