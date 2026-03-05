use tonic::{Request, Response, Status};
use futures_util::Stream;
use std::pin::Pin;
use crate::xds::envoy::api::v3::{
    route_discovery_service_server::RouteDiscoveryService,
    DiscoveryRequest, DiscoveryResponse,
};

pub struct RdsService;

#[tonic::async_trait]
impl RouteDiscoveryService for RdsService {
    type StreamRoutesStream = Pin<Box<dyn Stream<Item = Result<DiscoveryResponse, Status>> + Send>>;

    async fn stream_routes(
        &self,
        _request: Request<tonic::Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamRoutesStream>, Status> {
        Err(Status::unimplemented("RDS not fully implemented yet"))
    }
}
