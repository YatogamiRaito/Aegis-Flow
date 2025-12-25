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
    health_check_interval: Duration,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            rr_counters: Arc::new(RwLock::new(HashMap::new())),
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
                if total_weight == 0 {
                    return Some(healthy[0].addr);
                }
                use rand::Rng;
                let mut target = rand::thread_rng().gen_range(0..total_weight);
                for ep in &healthy {
                    if target < ep.weight {
                        return Some(ep.addr);
                    }
                    target -= ep.weight;
                }
                Some(healthy[0].addr)
            }
        }
    }

    /// Mark an endpoint as failed
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
    async fn test_list_services() {
        let registry = ServiceRegistry::new(LoadBalanceStrategy::RoundRobin);
        registry
            .register("service-a", vec!["127.0.0.1:8080".parse().unwrap()])
            .await;
        registry
            .register("service-b", vec!["127.0.0.1:9090".parse().unwrap()])
            .await;

        let services = registry.list_services().await;
        assert_eq!(services.len(), 2);
    }
}
