//! Service Discovery and Load Balancing Module
//!
//! Provides mechanisms to discover upstream endpoints and distribute traffic.

use aegis_common::{AegisError, Result};
use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, error};

/// Service Discovery Interface
#[async_trait::async_trait]
pub trait ServiceDiscovery: Send + Sync {
    /// Discover available endpoints
    async fn discover(&self) -> Result<Vec<SocketAddr>>;
}

/// Static Service Discovery (from config)
pub struct StaticDiscovery {
    endpoints: Vec<SocketAddr>,
}

impl StaticDiscovery {
    pub fn new(endpoints: Vec<SocketAddr>) -> Self {
        Self { endpoints }
    }
}

#[async_trait::async_trait]
impl ServiceDiscovery for StaticDiscovery {
    async fn discover(&self) -> Result<Vec<SocketAddr>> {
        Ok(self.endpoints.clone())
    }
}

/// DNS-based Service Discovery
pub struct DnsDiscovery {
    hostname: String,
    port: u16,
    refresh_interval: Duration,
}

impl DnsDiscovery {
    pub fn new(hostname: String, port: u16, refresh_interval: Duration) -> Self {
        Self {
            hostname,
            port,
            refresh_interval,
        }
    }
}

#[async_trait::async_trait]
impl ServiceDiscovery for DnsDiscovery {
    async fn discover(&self) -> Result<Vec<SocketAddr>> {
        let addr_str = format!("{}:{}", self.hostname, self.port);
        // This is a blocking call wrapped in async_trait, ideally use tokio::net::lookup_host
        // But ToSocketAddrs is std::net. We'll use tokio::net::lookup_host in real implementation.
        // For now, let's use tokio's lookup_host
        match tokio::net::lookup_host(&addr_str).await {
            Ok(addrs) => Ok(addrs.collect()),
            Err(e) => Err(AegisError::Network(format!("DNS lookup failed: {}", e))),
        }
    }
}

/// Load Balancer Strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadBalanceStrategy {
    #[default]
    RoundRobin,
    Random,
    LeastConnections,
}

/// Load Balancer
pub struct LoadBalancer {
    discovery: Arc<dyn ServiceDiscovery>,
    endpoints: Arc<RwLock<VecDeque<SocketAddr>>>,
    strategy: LoadBalanceStrategy,
    last_refresh: Arc<RwLock<Instant>>,
    refresh_interval: Duration,
}

impl LoadBalancer {
    pub fn new(
        discovery: Arc<dyn ServiceDiscovery>,
        strategy: LoadBalanceStrategy,
        refresh_interval: Duration,
    ) -> Self {
        Self {
            discovery,
            endpoints: Arc::new(RwLock::new(VecDeque::new())),
            strategy,
            last_refresh: Arc::new(RwLock::new(Instant::now() - refresh_interval * 2)), // Force refresh
            refresh_interval,
        }
    }

    /// update endpoints from discovery source
    pub async fn refresh(&self) -> Result<()> {
        let mut last = self.last_refresh.write().unwrap();
        if last.elapsed() < self.refresh_interval {
            return Ok(());
        }

        match self.discovery.discover().await {
            Ok(new_endpoints) => {
                let mut endpoints = self.endpoints.write().unwrap();
                // Retain existing order if possible, or just replace?
                // For round-robin, replacing resets the order. 
                // Simple implementation: replace.
                *endpoints = VecDeque::from(new_endpoints);
                *last = Instant::now();
                debug!("🔄 Refreshed endpoints: {:?}", endpoints);
                Ok(())
            }
            Err(e) => {
                error!("❌ Service discovery failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get next endpoint
    pub async fn next_endpoint(&self) -> Result<SocketAddr> {
        // Try refresh if needed (best effort)
        let _ = self.refresh().await;

        let mut endpoints = self.endpoints.write().unwrap();
        if endpoints.is_empty() {
            return Err(AegisError::Network("No upstream endpoints available".to_string()));
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                if let Some(endpoint) = endpoints.pop_front() {
                    endpoints.push_back(endpoint);
                    Ok(endpoint)
                } else {
                    Err(AegisError::Network("No endpoints".to_string()))
                }
            }
            LoadBalanceStrategy::Random => {
                // simple random
                use rand::seq::SliceRandom;
                // VecDeque to Vec for random choice is expensive strictly speaking, 
                // but for small N it's fine.
                // Actually we can just pick random index.
                let idx = rand::random::<usize>() % endpoints.len();
                Ok(endpoints[idx])
            }
            LoadBalanceStrategy::LeastConnections => {
                // Not implemented yet (requires tracking connections)
                // Fallback to RR
                if let Some(endpoint) = endpoints.pop_front() {
                    endpoints.push_back(endpoint);
                    Ok(endpoint)
                } else {
                    Err(AegisError::Network("No endpoints".to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDiscovery {
        addrs: Vec<SocketAddr>,
    }

    #[async_trait::async_trait]
    impl ServiceDiscovery for MockDiscovery {
        async fn discover(&self) -> Result<Vec<SocketAddr>> {
            Ok(self.addrs.clone())
        }
    }

    #[tokio::test]
    async fn test_round_robin() {
        let addr1: SocketAddr = "127.0.0.1:8081".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:8082".parse().unwrap();
        
        let discovery = Arc::new(MockDiscovery {
            addrs: vec![addr1, addr2],
        });

        let lb = LoadBalancer::new(
            discovery,
            LoadBalanceStrategy::RoundRobin,
            Duration::from_secs(10),
        );

        // First call should trigger refresh
        let e1 = lb.next_endpoint().await.unwrap();
        let e2 = lb.next_endpoint().await.unwrap();
        let e3 = lb.next_endpoint().await.unwrap();

        assert!(e1 == addr1 || e1 == addr2);
        assert_ne!(e1, e2); // Should rotate
        assert_eq!(e3, e1); // Should wrap around
    }

    #[tokio::test]
    async fn test_empty_endpoints() {
         let discovery = Arc::new(MockDiscovery {
            addrs: vec![],
        });

        let lb = LoadBalancer::new(
            discovery,
            LoadBalanceStrategy::RoundRobin,
            Duration::from_secs(10),
        );

        assert!(lb.next_endpoint().await.is_err());
    }
}
