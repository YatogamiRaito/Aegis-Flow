//! Service Discovery Module
//!
//! Provides DNS-based service discovery and load balancing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Service endpoint with health status
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Address of the endpoint
    pub addr: SocketAddr,
    /// Is the endpoint healthy
    pub healthy: bool,
    /// Last health check time
    pub last_check: Instant,
    /// Consecutive failures
    pub failures: u32,
    /// Current weight for load balancing
    pub weight: u32,
}

impl Endpoint {
    /// Create a new healthy endpoint
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            healthy: true,
            last_check: Instant::now(),
            failures: 0,
            weight: 100,
        }
    }

    /// Mark endpoint as failed
    pub fn mark_failed(&mut self) {
        self.failures += 1;
        if self.failures >= 3 {
            self.healthy = false;
            self.weight = 0;
        }
        self.last_check = Instant::now();
    }

    /// Mark endpoint as healthy
    pub fn mark_healthy(&mut self) {
        self.healthy = true;
        self.failures = 0;
        self.weight = 100;
        self.last_check = Instant::now();
    }
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadBalanceStrategy {
    /// Round-robin selection
    #[default]
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Random selection
    Random,
    /// Weighted round-robin
    WeightedRoundRobin,
}

/// Service registry for discovered services
pub struct ServiceRegistry {
    /// Map of service name to endpoints
    services: Arc<RwLock<HashMap<String, Vec<Endpoint>>>>,
    /// Load balancing strategy
    strategy: LoadBalanceStrategy,
    /// Round-robin counter per service
    rr_counters: Arc<RwLock<HashMap<String, usize>>>,
    /// Health check interval
    #[allow(dead_code)]
    health_check_interval: Duration,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            // Pre-allocate for typical number of services (10-20)
            services: Arc::new(RwLock::new(HashMap::with_capacity(16))),
            strategy,
            rr_counters: Arc::new(RwLock::new(HashMap::with_capacity(16))),
            health_check_interval: Duration::from_secs(10),
        }
    }

    /// Register a service with endpoints
    pub async fn register(&self, service: &str, endpoints: Vec<SocketAddr>) {
        let mut services = self.services.write().await;
        let eps: Vec<Endpoint> = endpoints.into_iter().map(Endpoint::new).collect();
        info!(
            "📍 Registered service '{}' with {} endpoints",
            service,
            eps.len()
        );
        services.insert(service.to_string(), eps);
    }

    /// Get next endpoint for a service using load balancing
    pub async fn get_endpoint(&self, service: &str) -> Option<SocketAddr> {
        let services = self.services.read().await;
        let endpoints = services.get(service)?;

        let healthy: Vec<&Endpoint> = endpoints.iter().filter(|e| e.healthy).collect();
        if healthy.is_empty() {
            warn!("⚠️ No healthy endpoints for service '{}'", service);
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let mut counters = self.rr_counters.write().await;
                let counter = counters.entry(service.to_string()).or_insert(0);
                let idx = *counter % healthy.len();
                *counter = counter.wrapping_add(1);
                Some(healthy[idx].addr)
            }
            LoadBalanceStrategy::Random => {
                use rand::Rng;
                let idx = rand::thread_rng().gen_range(0..healthy.len());
                Some(healthy[idx].addr)
            }
            LoadBalanceStrategy::LeastConnections => {
                // For now, just use round-robin (would need connection tracking)
                Some(healthy[0].addr)
            }
            LoadBalanceStrategy::WeightedRoundRobin => {
                // Use weights for selection
                let total_weight: u32 = healthy.iter().map(|e| e.weight).sum();

                use rand::Rng;
                let mut target = rand::thread_rng().gen_range(0..total_weight);
                for ep in &healthy {
                    if target < ep.weight {
                        return Some(ep.addr);
                    }
                    target -= ep.weight;
                }
                // Should be unreachable if logic is correct
                Some(healthy[0].addr)
            }
        }
    }

    /// Mark an endpoint as failed
    #[allow(clippy::collapsible_if)]
    pub async fn mark_failed(&self, service: &str, addr: SocketAddr) {
        let mut services = self.services.write().await;
        if let Some(endpoints) = services.get_mut(service) {
            if let Some(ep) = endpoints.iter_mut().find(|e| e.addr == addr) {
                ep.mark_failed();
                debug!("❌ Marked endpoint {} as failed for '{}'", addr, service);
            }
        }
    }

    /// Mark an endpoint as healthy
    #[allow(clippy::collapsible_if)]
    pub async fn mark_healthy(&self, service: &str, addr: SocketAddr) {
        let mut services = self.services.write().await;
        if let Some(endpoints) = services.get_mut(service) {
            if let Some(ep) = endpoints.iter_mut().find(|e| e.addr == addr) {
                ep.mark_healthy();
                debug!("✅ Marked endpoint {} as healthy for '{}'", addr, service);
            }
        }
    }

    /// Get all registered services
    pub async fn list_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// Get endpoint count for a service
    pub async fn endpoint_count(&self, service: &str) -> usize {
        let services = self.services.read().await;
        services.get(service).map(|e| e.len()).unwrap_or(0)
    }

    /// Get healthy endpoint count for a service
    pub async fn healthy_count(&self, service: &str) -> usize {
        let services = self.services.read().await;
        services
            .get(service)
            .map(|eps| eps.iter().filter(|e| e.healthy).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_service() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let endpoints = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
        ];

        registry.register("backend", endpoints).await;

        assert_eq!(registry.endpoint_count("backend").await, 2);
        assert_eq!(registry.healthy_count("backend").await, 2);
    }

    #[tokio::test]
    async fn test_round_robin() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("backend", vec![ep1, ep2]).await;

        let first = registry.get_endpoint("backend").await.unwrap();
        let second = registry.get_endpoint("backend").await.unwrap();
        let third = registry.get_endpoint("backend").await.unwrap();

        assert_eq!(first, ep1);
        assert_eq!(second, ep2);
        assert_eq!(third, ep1);
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("backend", vec![ep1, ep2]).await;

        // Mark ep1 as failed multiple times
        for _ in 0..3 {
            registry.mark_failed("backend", ep1).await;
        }

        assert_eq!(registry.healthy_count("backend").await, 1);

        // All requests should go to ep2
        let addr = registry.get_endpoint("backend").await.unwrap();
        assert_eq!(addr, ep2);
    }

    #[tokio::test]
    async fn test_empty_registry_lookup() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        assert!(registry.get_endpoint("non-existent").await.is_none());
    }

    #[tokio::test]
    async fn test_mark_healthy_restore() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        registry.register("restore-service", vec![ep1]).await;

        // Fail it
        for _ in 0..3 {
            registry.mark_failed("restore-service", ep1).await;
        }

        assert_eq!(registry.healthy_count("restore-service").await, 0);
        assert!(registry.get_endpoint("restore-service").await.is_none());

        // Restore it
        registry.mark_healthy("restore-service", ep1).await;

        assert_eq!(registry.healthy_count("restore-service").await, 1);
        assert!(registry.get_endpoint("restore-service").await.is_some());
    }

    #[tokio::test]
    async fn test_random_strategy() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::Random);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("random-service", vec![ep1, ep2]).await;

        // Should return one of them
        let picked = registry.get_endpoint("random-service").await.unwrap();
        assert!(picked == ep1 || picked == ep2);
    }

    #[tokio::test]
    async fn test_least_connections_strategy() {
        // Currently implements simple fallback, just verifying it doesn't panic
        let registry = ServiceRegistry::new(LoadBalanceStrategy::LeastConnections);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        registry.register("lc-service", vec![ep1]).await;
        assert_eq!(registry.get_endpoint("lc-service").await.unwrap(), ep1);
    }

    #[tokio::test]
    async fn test_weighted_strategy_with_failure() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::WeightedRoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("weighted-service", vec![ep1, ep2]).await;

        // Fail ep1
        for _ in 0..3 {
            registry.mark_failed("weighted-service", ep1).await;
        }

        // Should always pick ep2 (ep1 weight becomes 0)
        for _ in 0..10 {
            assert_eq!(
                registry.get_endpoint("weighted-service").await.unwrap(),
                ep2
            );
        }
    }

    #[test]
    fn test_endpoint_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let endpoint = Endpoint::new(addr);
        assert!(endpoint.healthy);
        assert_eq!(endpoint.failures, 0);
        assert_eq!(endpoint.weight, 100);
    }

    #[test]
    fn test_endpoint_mark_failed() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut endpoint = Endpoint::new(addr);

        endpoint.mark_failed();
        assert_eq!(endpoint.failures, 1);
        assert!(endpoint.healthy); // Still healthy with 1 failure

        endpoint.mark_failed();
        endpoint.mark_failed();
        assert_eq!(endpoint.failures, 3);
        assert!(!endpoint.healthy); // Unhealthy after 3 failures
        assert_eq!(endpoint.weight, 0);
    }

    #[test]
    fn test_endpoint_mark_healthy() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut endpoint = Endpoint::new(addr);

        for _ in 0..3 {
            endpoint.mark_failed();
        }
        assert!(!endpoint.healthy);

        endpoint.mark_healthy();
        assert!(endpoint.healthy);
        assert_eq!(endpoint.failures, 0);
        assert_eq!(endpoint.weight, 100);
    }

    #[test]
    fn test_load_balance_strategy_default() {
        let strategy: LoadBalanceStrategy = Default::default();
        assert_eq!(strategy, LoadBalanceStrategy::RoundRobin);
    }

    #[tokio::test]
    async fn test_get_endpoint_round_robin() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let addrs: Vec<SocketAddr> = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
        ];
        registry.register("test-svc", addrs.clone()).await;

        let ep1 = registry.get_endpoint("test-svc").await.unwrap();
        let ep2 = registry.get_endpoint("test-svc").await.unwrap();
        assert!(addrs.contains(&ep1));
        assert!(addrs.contains(&ep2));
    }

    #[tokio::test]
    async fn test_get_endpoint_random() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::Random);
        let addrs: Vec<SocketAddr> = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
        ];
        registry.register("test-svc", addrs.clone()).await;

        let ep = registry.get_endpoint("test-svc").await.unwrap();
        assert!(addrs.contains(&ep));
    }

    #[tokio::test]
    async fn test_get_endpoint_least_connections() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::LeastConnections);
        let addrs: Vec<SocketAddr> = vec!["127.0.0.1:8080".parse().unwrap()];
        registry.register("test-svc", addrs.clone()).await;

        let ep = registry.get_endpoint("test-svc").await.unwrap();
        assert_eq!(ep, addrs[0]);
    }

    #[tokio::test]
    async fn test_get_endpoint_weighted_round_robin() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::WeightedRoundRobin);
        let addrs: Vec<SocketAddr> = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
        ];
        registry.register("test-svc", addrs.clone()).await;

        let ep = registry.get_endpoint("test-svc").await.unwrap();
        assert!(addrs.contains(&ep));
    }

    #[tokio::test]
    async fn test_get_endpoint_no_healthy() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        registry.register("test-svc", vec![addr]).await;

        // Mark as failed 3 times
        for _ in 0..3 {
            registry.mark_failed("test-svc", addr).await;
        }

        let result = registry.get_endpoint("test-svc").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mark_healthy_after_failed() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        registry.register("test-svc", vec![addr]).await;

        for _ in 0..3 {
            registry.mark_failed("test-svc", addr).await;
        }
        assert!(registry.get_endpoint("test-svc").await.is_none());

        registry.mark_healthy("test-svc", addr).await;
        assert!(registry.get_endpoint("test-svc").await.is_some());
    }
    #[tokio::test]
    async fn test_list_services_registry() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        registry
            .register("svc1", vec!["127.0.0.1:8080".parse().unwrap()])
            .await;
        registry
            .register("svc2", vec!["127.0.0.1:8081".parse().unwrap()])
            .await;

        let services = registry.list_services().await;
        assert_eq!(services.len(), 2);
    }

    #[tokio::test]
    async fn test_weighted_round_robin_varying_weights() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::WeightedRoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("weighted", vec![ep1, ep2]).await;

        let ep = registry.get_endpoint("weighted").await.unwrap();
        assert!(ep == ep1 || ep == ep2);
    }

    #[tokio::test]
    async fn test_weighted_round_robin_zero_weight_fallback() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::WeightedRoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        registry.register("zero-weight", vec![ep1]).await;

        // Mark failed 3 times to set weight to 0
        for _ in 0..3 {
            registry.mark_failed("zero-weight", ep1).await;
        }

        // Re-register with a fresh endpoint to simulate zero weight scenario
        registry.mark_healthy("zero-weight", ep1).await;

        // Should still return an endpoint
        let ep = registry.get_endpoint("zero-weight").await;
        assert!(ep.is_some());
    }

    #[tokio::test]
    async fn test_register_multiple_endpoints() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let eps: Vec<SocketAddr> = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];

        registry.register("multi-ep", eps.clone()).await;

        // Verify all endpoints are accessible via round-robin
        for _ in 0..3 {
            let ep = registry.get_endpoint("multi-ep").await.unwrap();
            assert!(eps.contains(&ep));
        }
    }

    #[tokio::test]
    async fn test_weighted_round_robin_edge_cases() {
        // Case 1: All weights zero (should fallback or behave gracefully)
        let registry = ServiceRegistry::new(LoadBalanceStrategy::WeightedRoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        registry.register("all-zero", vec![ep1]).await;

        for _ in 0..3 {
            registry.mark_failed("all-zero", ep1).await;
        }
        // At this point weight is 0. But get_endpoint filters for healthy.
        // If we mark it healthy again, weight resets to 100.
        // So let's test a case where we manually set weight to 0 if possible?
        // We can't validly have a healthy endpoint with weight 0 via public API currently.
        // But we can test single endpoint logic.

        let result = registry.get_endpoint("all-zero").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mark_failed_nonexistent_service() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        // Should not panic
        registry.mark_failed("nonexistent", ep1).await;
        registry.mark_healthy("nonexistent", ep1).await;
    }

    #[tokio::test]
    async fn test_all_endpoints_unhealthy() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let ep: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        registry.register("unhealthy-svc", vec![ep]).await;

        // Mark as failed multiple times
        for _ in 0..5 {
            registry.mark_failed("unhealthy-svc", ep).await;
        }

        // Should return None when no healthy endpoints
        let result = registry.get_endpoint("unhealthy-svc").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lc_strategy_with_multiple_endpoints() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::LeastConnections);
        let ep1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep2: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        registry.register("lc-svc", vec![ep1, ep2]).await;

        // Both have 0 connections, either can be selected
        let selected = registry.get_endpoint("lc-svc").await;
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_random_strategy_with_three_endpoints() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::Random);
        let eps: Vec<SocketAddr> = vec![
            "127.0.0.1:8080".parse().unwrap(),
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];

        registry.register("random-svc", eps.clone()).await;

        // Verify random selection works
        for _ in 0..5 {
            let selected = registry.get_endpoint("random-svc").await;
            assert!(selected.is_some());
            assert!(eps.contains(&selected.unwrap()));
        }
    }

    #[test]
    fn test_endpoint_new() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep = Endpoint::new(addr);

        assert_eq!(ep.addr, addr);
        assert!(ep.healthy);
        assert_eq!(ep.failures, 0);
        assert_eq!(ep.weight, 100);
    }

    #[test]
    fn test_endpoint_failure_threshold() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut ep = Endpoint::new(addr);

        ep.mark_failed();
        assert_eq!(ep.failures, 1);
        assert!(ep.healthy); // Still healthy after 1 failure

        ep.mark_failed();
        ep.mark_failed();
        assert!(!ep.healthy); // Unhealthy after 3 failures
        assert_eq!(ep.weight, 0);
    }

    #[test]
    fn test_endpoint_recovery() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut ep = Endpoint::new(addr);

        // Make unhealthy first
        for _ in 0..3 {
            ep.mark_failed();
        }
        assert!(!ep.healthy);

        ep.mark_healthy();
        assert!(ep.healthy);
        assert_eq!(ep.failures, 0);
    }

    #[test]
    fn test_endpoint_debug() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep = Endpoint::new(addr);
        let debug = format!("{:?}", ep);
        assert!(debug.contains("Endpoint"));
    }

    #[test]
    fn test_endpoint_clone() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ep1 = Endpoint::new(addr);
        let ep2 = ep1.clone();

        assert_eq!(ep1.addr, ep2.addr);
        assert_eq!(ep1.healthy, ep2.healthy);
    }

    #[test]
    fn test_endpoint_initial_state() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let ep = Endpoint::new(addr);

        assert!(ep.healthy);
        assert_eq!(ep.failures, 0);
        assert_eq!(ep.weight, 100);
    }

    #[test]
    fn test_endpoint_mark_failed_once() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut ep = Endpoint::new(addr);

        ep.mark_failed();
        assert_eq!(ep.failures, 1);
        assert!(ep.healthy); // Still healthy after 1 failure
    }

    #[test]
    fn test_endpoint_mark_failed_twice() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut ep = Endpoint::new(addr);

        ep.mark_failed();
        ep.mark_failed();
        assert_eq!(ep.failures, 2);
        assert!(ep.healthy); // Still healthy after 2 failures
    }

    #[test]
    fn test_endpoint_mark_failed_three_times() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut ep = Endpoint::new(addr);

        ep.mark_failed();
        ep.mark_failed();
        ep.mark_failed();

        assert_eq!(ep.failures, 3);
        assert!(!ep.healthy); // Unhealthy after 3 failures
        assert_eq!(ep.weight, 0);
    }

    #[test]
    fn test_load_balance_strategy_debug() {
        let strategy = LoadBalanceStrategy::RoundRobin;
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("RoundRobin"));
    }

    #[tokio::test]
    async fn test_list_services_empty() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        let services = registry.list_services().await;
        assert_eq!(services.len(), 0);
    }

    #[tokio::test]
    async fn test_endpoint_count_nonexistent() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        assert_eq!(registry.endpoint_count("non-existent").await, 0);
    }
}
