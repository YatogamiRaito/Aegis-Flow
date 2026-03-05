use tonic::{Request, Response, Status};
use futures_util::Stream;
use std::pin::Pin;
use crate::xds::envoy::api::v3::{
    listener_discovery_service_server::ListenerDiscoveryService,
    DiscoveryRequest, DiscoveryResponse,
};

pub struct LdsService;

#[tonic::async_trait]
impl ListenerDiscoveryService for LdsService {
    type StreamListenersStream = Pin<Box<dyn Stream<Item = Result<DiscoveryResponse, Status>> + Send>>;

    async fn stream_listeners(
        &self,
        _request: Request<tonic::Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamListenersStream>, Status> {
        Err(Status::unimplemented("LDS not fully implemented yet"))
    }
}
