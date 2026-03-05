pub mod server;
pub mod snapshot;

pub use server::run_xds_server;
pub use snapshot::Snapshot;
pub mod ads;
pub mod cds;
pub mod lds;
pub mod rds;

pub mod envoy {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("envoy.api.v3");
        }
    }
}
