pub mod snapshot;
pub mod server;

pub use snapshot::Snapshot;
pub use server::run_xds_server;
pub mod lds;
pub mod cds;
pub mod rds;
pub mod ads;

pub mod envoy {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("envoy.api.v3");
        }
    }
}
