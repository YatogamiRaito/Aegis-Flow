use crate::config::SplitClientsConfig;
use hyper::Request;
use std::hash::Hasher;
use murmur3::murmur3_32;
use std::io::Cursor;
use crate::variables::{RequestContext, VariableResolver};

/// Evaluates a `split_clients` rule against an incoming HTTP request.
/// Deterministically returns the assigned bucket value based on exact percentage distributions.
pub fn evaluate_split_client<B>(
    config: &SplitClientsConfig,
    req: &Request<B>,
    client_ip: &str,
) -> String {
    let ctx = RequestContext {
        uri: req.uri(),
        method: req.method(),
        headers: req.headers(),
        remote_addr: client_ip,
        server_name: "", // Not strictly needed for split_client evaluating $remote_addr
        server_port: 0,
        request_uri: req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or(req.uri().path()),
        scheme: req.uri().scheme_str().unwrap_or("http"),
    };
    
    // We pass `None` for config here to prevent infinite recursion,
    // as evaluate_split_client is invoked from inside VariableResolver searching for split_clients.
    let resolver = VariableResolver::new(ctx, None);
    let resolved_key = resolver.interpolate(&config.key);

    // MurmurHash3 the resultant string to obtain a uniformly distributed u32 bucket ID
    let mut cursor = Cursor::new(resolved_key.as_bytes());
    let hash = murmur3_32(&mut cursor, 0).unwrap_or(0);

    // Map the 32-bit hash into a 0.0 -> 100.0 percentage range
    let hash_percent = (hash as f64 / u32::MAX as f64) * 100.0;

    let mut cumulative: f64 = 0.0;

    for bucket in &config.buckets {
        cumulative += bucket.percent;
        if hash_percent <= cumulative {
            return bucket.value.clone();
        }
    }

    // Fallback: If distributions math wasn't perfectly 100%, return the last bucket natively
    config.buckets.last().map(|b| b.value.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SplitClientsConfig, SplitClientsBucket};
    use hyper::Request;

    #[test]
    fn test_split_clients_deterministic_distribution() {
        let config = SplitClientsConfig {
            key: "$remote_addr".to_string(),
            variable: "$variant".to_string(),
            buckets: vec![
                SplitClientsBucket { percent: 10.0, value: "canary".to_string() },
                SplitClientsBucket { percent: 90.0, value: "stable".to_string() },
            ],
        };

        // For the same IP, the output bucket must be perfectly constant without RNG
        let req1 = Request::builder().uri("/").body(()).unwrap();
        let bucket1 = evaluate_split_client(&config, &req1, "192.168.1.100");
        let bucket2 = evaluate_split_client(&config, &req1, "192.168.1.100");
        assert_eq!(bucket1, bucket2);

        // Given enough IP distributions, canary will flag appropriately.
        // E.g., verifying a separate IP resolves deterministically as stable or canary.
        let bucket3 = evaluate_split_client(&config, &req1, "10.0.0.5");
        assert!(!bucket3.is_empty());
    }
}
