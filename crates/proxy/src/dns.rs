//! Async DNS Resolver with TTL Support
//!
//! Wraps hickory-resolver to provide persistent, asynchronous DNS resolution
//! with respect for TTL values from upstream servers.

use std::sync::Arc;
use std::time::{Duration, Instant};
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use moka::future::Cache;
use tracing::{debug, error, warn};
use std::sync::atomic::{AtomicBool, Ordering};

/// A resolved IP address with its TTL
#[derive(Debug, Clone)]
pub struct ResolvedAddr {
    pub ips: Vec<std::net::IpAddr>,
    pub expires_at: Instant,
    pub stale_until: Instant,
    pub is_refreshing: Arc<AtomicBool>,
}

/// TTL-aware Async DNS Resolver
#[derive(Clone)]
pub struct AsyncResolver {
    resolver: TokioAsyncResolver,
    cache: Cache<String, ResolvedAddr>,
}

impl AsyncResolver {
    /// Create a new resolver with default system configuration
    pub fn new() -> Self {
        let (config, opts) = hickory_resolver::system_conf::read_system_conf()
            .unwrap_or_else(|_| (ResolverConfig::default(), ResolverOpts::default()));
        
        let resolver = TokioAsyncResolver::tokio(config, opts);
        let cache = Cache::builder()
            .max_capacity(1000)
            .build();

        Self { resolver, cache }
    }

    /// Resolve a hostname, using the cache if valid or returning a stale value while refreshing
    pub async fn resolve(&self, host: &str) -> anyhow::Result<Vec<std::net::IpAddr>> {
        let now = Instant::now();

        if let Some(resolved) = self.cache.get(host).await {
            if now < resolved.expires_at {
                debug!("DNS cache hit (fresh): {} -> {:?}", host, resolved.ips);
                return Ok(resolved.ips);
            } else if now < resolved.stale_until {
                debug!("DNS cache hit (stale): {} -> {:?}", host, resolved.ips);
                
                // Only trigger ONE background refresh for this host
                if !resolved.is_refreshing.swap(true, Ordering::AcqRel) {
                    let host_clone = host.to_string();
                    let resolver = self.resolver.clone();
                    let cache = self.cache.clone();

                    tokio::spawn(async move {
                        debug!("Background DNS refresh triggered for: {}", host_clone);
                        if let Ok(lookup) = resolver.lookup_ip(&host_clone).await {
                            let ips: Vec<_> = lookup.iter().collect();
                            let refresh_now = Instant::now();
                            let ttl_secs = lookup.valid_until()
                                .duration_since(refresh_now)
                                .as_secs()
                                .max(60);
                                
                            let new_resolved = ResolvedAddr {
                                ips,
                                expires_at: refresh_now + Duration::from_secs(ttl_secs),
                                stale_until: refresh_now + Duration::from_secs(ttl_secs + 300), // 5 min grace period
                                is_refreshing: Arc::new(AtomicBool::new(false)),
                            };
                            cache.insert(host_clone.clone(), new_resolved).await;
                            debug!("Background DNS refresh succeeded for: {}", host_clone);
                        } else {
                            warn!("Background DNS refresh failed for: {}", host_clone);
                            // If it failed, resetting is_refreshing allows another attempt on next stale hit
                            // Note: we'd need a reference to the cache entry, but since we didn't update it,
                            // the old entry with is_refreshing=true will prevent rapid retries. We can just
                            // leave it true to prevent thundering herd until stale_until expires.
                        }
                    });
                }
                
                // Return immediately with stale data
                return Ok(resolved.ips);
            }
            self.cache.invalidate(host).await;
        }

        debug!("DNS cache miss: resolving {}", host);
        let lookup = self.resolver.lookup_ip(host).await?;
        let ips: Vec<_> = lookup.iter().collect();
        
        // Respect TTL from DNS response
        let resolve_now = Instant::now();
        let ttl_secs = lookup.valid_until()
            .duration_since(resolve_now)
            .as_secs()
            .max(60); // Minimum 1 minute TTL to avoid thundering herd
        
        let resolved = ResolvedAddr {
            ips: ips.clone(),
            expires_at: resolve_now + Duration::from_secs(ttl_secs),
            stale_until: resolve_now + Duration::from_secs(ttl_secs + 300), // 5 min grace period
            is_refreshing: Arc::new(AtomicBool::new(false)),
        };

        self.cache.insert(host.to_string(), resolved).await;
        Ok(ips)
    }
}

impl Default for AsyncResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Global resolver instance
pub static GLOBAL_RESOLVER: once_cell::sync::Lazy<Arc<AsyncResolver>> = 
    once_cell::sync::Lazy::new(|| Arc::new(AsyncResolver::new()));
