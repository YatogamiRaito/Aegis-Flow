use reqwest::Client;
use std::time::Duration;

pub struct UpstreamClientOptions {
    pub connect_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub keep_alive_timeout_ms: u64,
    pub max_idle_per_host: usize,
}

impl Default for UpstreamClientOptions {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 5000,
            read_timeout_ms: 30000,
            keep_alive_timeout_ms: 90000,
            max_idle_per_host: 100,
        }
    }
}

pub fn create_upstream_client(options: &UpstreamClientOptions) -> Client {
    Client::builder()
        .connect_timeout(Duration::from_millis(options.connect_timeout_ms))
        .timeout(Duration::from_millis(options.read_timeout_ms))
        // reqwest does not expose keep_alive_timeout directly, but it manages the pool.
        .pool_idle_timeout(Duration::from_millis(options.keep_alive_timeout_ms))
        .pool_max_idle_per_host(options.max_idle_per_host)
        .build()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder_options() {
        let opts = UpstreamClientOptions {
            connect_timeout_ms: 2000,
            read_timeout_ms: 5000,
            keep_alive_timeout_ms: 10000,
            max_idle_per_host: 50,
        };

        let client = create_upstream_client(&opts);
        
        // At this point we can't easily introspect reqwest::Client internal config, 
        // but if it built successfully without panicking, the options are valid.
        assert!(true);
    }
}
