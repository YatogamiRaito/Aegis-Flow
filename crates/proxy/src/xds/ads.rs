use tonic::{Request, Response, Status};
use futures_util::Stream;
use std::pin::Pin;
use crate::xds::envoy::api::v3::{
    aggregated_discovery_service_server::AggregatedDiscoveryService,
    DiscoveryRequest, DiscoveryResponse,
};

pub struct AdsService;

#[tonic::async_trait]
impl AggregatedDiscoveryService for AdsService {
    type StreamAggregatedResourcesStream = Pin<Box<dyn Stream<Item = Result<DiscoveryResponse, Status>> + Send>>;

    async fn stream_aggregated_resources(
        &self,
        _request: Request<tonic::Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamAggregatedResourcesStream>, Status> {
        Err(Status::unimplemented("ADS not fully implemented yet"))
    }
}
