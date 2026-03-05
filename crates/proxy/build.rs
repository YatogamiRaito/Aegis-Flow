fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile protos if the xds feature is enabled
    #[cfg(feature = "xds")]
    {
        tonic_build::configure()
            .build_server(true)
            .compile(
                &[
                    "proto/envoy/api/v3/discovery.proto",
                    "proto/envoy/api/v3/lds.proto",
                    "proto/envoy/api/v3/cds.proto",
                    "proto/envoy/api/v3/rds.proto",
                ],
                &["proto"],
            )?;
    }
    Ok(())
}
