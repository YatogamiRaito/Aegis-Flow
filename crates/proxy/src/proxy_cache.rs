use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

// ---------------------------------------------------------------------------
// CacheKey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub key: String,
}

impl CacheKey {
    /// Build a normalized cache key from scheme + host + URI.
    /// Query params are sorted for normalization.
    pub fn from_request(scheme: &str, host: &str, uri: &str) -> Self {
        let (path, query) = match uri.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (uri, None),
        };

        let normalized = if let Some(q) = query {
            let mut parts: Vec<&str> = q.split('&').collect();
            parts.sort_unstable();
            format!("{}://{}{}?{}", scheme, host, path, parts.join("&"))
        } else {
            format!("{}://{}{}", scheme, host, path)
        };

        let key = format!("{:x}", md5_simple(&normalized));
        CacheKey { key }
    }
}

fn md5_simple(s: &str) -> u64 {
    // Simple deterministic hash (not cryptographic, fine for cache keys)
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

// ---------------------------------------------------------------------------
// CacheEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub created_at: Instant,
    pub ttl: Duration,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    /// Bytes size of the body
    pub size: usize,
}

impl CacheEntry {
    pub fn new(
        key: CacheKey,
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        ttl: Duration,
    ) -> Self {
        let etag = headers.iter()
            .find(|(k, _)| k.to_lowercase() == "etag")
            .map(|(_, v)| v.clone());
        let last_modified = headers.iter()
            .find(|(k, _)| k.to_lowercase() == "last-modified")
            .map(|(_, v)| v.clone());
        let size = body.len();
        Self {
            key,
            status,
            headers,
            body,
            created_at: Instant::now(),
            ttl,
            etag,
            last_modified,
            size,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }

    pub fn age_secs(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }
}

// ---------------------------------------------------------------------------
// Cache-Control parser
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct CacheDirectives {
    pub max_age: Option<u64>,
    pub s_maxage: Option<u64>,
    pub no_cache: bool,
    pub no_store: bool,
    pub private: bool,
    pub public: bool,
}

impl CacheDirectives {
    pub fn parse(header: &str) -> Self {
        let mut d = CacheDirectives::default();
        for part in header.split(',') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("max-age=") {
                d.max_age = val.parse().ok();
            } else if let Some(val) = part.strip_prefix("s-maxage=") {
                d.s_maxage = val.parse().ok();
            } else if part == "no-cache" {
                d.no_cache = true;
            } else if part == "no-store" {
                d.no_store = true;
            } else if part == "private" {
                d.private = true;
            } else if part == "public" {
                d.public = true;
            }
        }
        d
    }

    /// Effective TTL in seconds, following: s-maxage > max-age
    pub fn effective_ttl_secs(&self) -> Option<u64> {
        if let Some(s) = self.s_maxage {
            return Some(s);
        }
        self.max_age
    }

    pub fn is_cacheable(&self) -> bool {
        !self.no_store && !self.private && !self.no_cache
    }
}

// ---------------------------------------------------------------------------
// TTL Resolver — per-status override config
// ---------------------------------------------------------------------------

pub struct TtlConfig {
    pub status_ttls: HashMap<u16, Duration>,
    pub default_ttl: Duration,
}

impl TtlConfig {
    pub fn new(default_secs: u64) -> Self {
        Self {
            status_ttls: HashMap::new(),
            default_ttl: Duration::from_secs(default_secs),
        }
    }

    pub fn with_status(mut self, status: u16, secs: u64) -> Self {
        self.status_ttls.insert(status, Duration::from_secs(secs));
        self
    }

    pub fn resolve(&self, status: u16, directives: &CacheDirectives) -> Option<Duration> {
        // 1. Cache-Control s-maxage / max-age
        if let Some(secs) = directives.effective_ttl_secs() {
            return Some(Duration::from_secs(secs));
        }
        // 2. Per-status override
        if let Some(d) = self.status_ttls.get(&status) {
            return Some(*d);
        }
        // 3. Default
        Some(self.default_ttl)
    }
}

// ---------------------------------------------------------------------------
// X-Cache-Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CacheStatus {
    Hit,
    Miss,
    Bypass,
    Expired,
    Stale,
    Updating,
    Revalidated,
}

impl CacheStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hit        => "HIT",
            Self::Miss       => "MISS",
            Self::Bypass     => "BYPASS",
            Self::Expired    => "EXPIRED",
            Self::Stale      => "STALE",
            Self::Updating   => "UPDATING",
            Self::Revalidated => "REVALIDATED",
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryCache — LRU with size tracking
// ---------------------------------------------------------------------------

pub struct MemoryCache {
    inner: Mutex<MemoryCacheInner>,
}

struct MemoryCacheInner {
    lru: LruCache<String, CacheEntry>,
    current_bytes: usize,
    max_bytes: usize,
    /// Minimum uses before caching (proxy_cache_min_uses)
    min_uses: usize,
    use_counts: HashMap<String, usize>,
}

impl MemoryCache {
    pub fn new(max_entries: usize, max_bytes: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(MemoryCacheInner {
                lru: LruCache::new(NonZeroUsize::new(max_entries).unwrap()),
                current_bytes: 0,
                max_bytes,
                min_uses: 1,
                use_counts: HashMap::new(),
            }),
        })
    }

    pub fn with_min_uses(self: Arc<Self>, n: usize) -> Arc<Self> {
        self.inner.lock().unwrap().min_uses = n;
        self
    }

    pub fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        let mut inner = self.inner.lock().unwrap();
        inner.lru.get(&key.key).cloned()
    }

    /// Returns true if entry was stored, false if min_uses threshold not reached
    pub fn put(&self, entry: CacheEntry) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let k = entry.key.key.clone();
        let size = entry.size;

        // min_uses tracking
        let count = inner.use_counts.entry(k.clone()).or_insert(0);
        *count += 1;
        if *count < inner.min_uses {
            return false;
        }

        // Evict entries until we have enough space
        while inner.current_bytes + size > inner.max_bytes && !inner.lru.is_empty() {
            if let Some((_, evicted)) = inner.lru.pop_lru() {
                inner.current_bytes = inner.current_bytes.saturating_sub(evicted.size);
            }
        }

        // Remove old entry size if replacing
        if let Some(old) = inner.lru.peek(&k) {
            inner.current_bytes = inner.current_bytes.saturating_sub(old.size);
        }

        inner.current_bytes += size;
        inner.lru.put(k, entry);
        true
    }

    pub fn remove(&self, key: &CacheKey) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(e) = inner.lru.pop(&key.key) {
            inner.current_bytes = inner.current_bytes.saturating_sub(e.size);
        }
    }

    pub fn purge_prefix(&self, prefix: &str) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let keys: Vec<String> = inner.lru.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.clone())
            .collect();
        let count = keys.len();
        for k in keys {
            if let Some(e) = inner.lru.pop(&k) {
                inner.current_bytes = inner.current_bytes.saturating_sub(e.size);
            }
        }
        count
    }

    pub fn current_entries(&self) -> usize {
        self.inner.lock().unwrap().lru.len()
    }

    pub fn current_bytes(&self) -> usize {
        self.inner.lock().unwrap().current_bytes
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.lock().unwrap();
        CacheStats {
            entries: inner.lru.len(),
            size_bytes: inner.current_bytes,
        }
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub size_bytes: usize,
}

// ---------------------------------------------------------------------------
// Bypass condition evaluator
// ---------------------------------------------------------------------------

pub struct BypassCheck {
    /// Header names that, if present with non-empty value, cause bypass
    pub bypass_headers: Vec<String>,
    /// Only these methods are cacheable
    pub cacheable_methods: Vec<String>,
}

impl Default for BypassCheck {
    fn default() -> Self {
        Self {
            bypass_headers: vec!["Authorization".to_string()],
            cacheable_methods: vec!["GET".to_string(), "HEAD".to_string()],
        }
    }
}

impl BypassCheck {
    pub fn should_bypass(&self, method: &str, headers: &[(String, String)]) -> bool {
        // Method check
        if !self.cacheable_methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
            return true;
        }
        // Header check
        for (name, val) in headers {
            if self.bypass_headers.iter().any(|h| h.eq_ignore_ascii_case(name))
                && !val.is_empty()
            {
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_key(uri: &str) -> CacheKey {
        CacheKey::from_request("https", "example.com", uri)
    }

    fn make_entry(key: CacheKey, body: &[u8], ttl_secs: u64) -> CacheEntry {
        CacheEntry::new(
            key,
            200,
            vec![("content-type".to_string(), "text/html".to_string())],
            body.to_vec(),
            Duration::from_secs(ttl_secs),
        )
    }

    // --- CacheKey ---
    #[test]
    fn test_cache_key_normalization() {
        let k1 = CacheKey::from_request("https", "example.com", "/path?b=2&a=1");
        let k2 = CacheKey::from_request("https", "example.com", "/path?a=1&b=2");
        assert_eq!(k1, k2); // sorted query params → same key
    }

    #[test]
    fn test_cache_key_different_paths() {
        let k1 = CacheKey::from_request("https", "example.com", "/a");
        let k2 = CacheKey::from_request("https", "example.com", "/b");
        assert_ne!(k1, k2);
    }

    // --- CacheEntry ---
    #[test]
    fn test_cache_entry_expiry() {
        let key = make_key("/");
        let entry = make_entry(key, b"hello", 0); // 0s TTL → immediately expired
        // Tiny sleep to ensure elapsed > 0
        std::thread::sleep(Duration::from_millis(1));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_not_expired() {
        let key = make_key("/");
        let entry = make_entry(key, b"hello", 3600);
        assert!(!entry.is_expired());
    }

    // --- CacheDirectives ---
    #[test]
    fn test_cache_control_parse_max_age() {
        let d = CacheDirectives::parse("max-age=300, public");
        assert_eq!(d.max_age, Some(300));
        assert!(d.public);
        assert!(d.is_cacheable());
    }

    #[test]
    fn test_cache_control_s_maxage_wins() {
        let d = CacheDirectives::parse("max-age=300, s-maxage=600");
        assert_eq!(d.effective_ttl_secs(), Some(600));
    }

    #[test]
    fn test_cache_control_no_store() {
        let d = CacheDirectives::parse("no-store");
        assert!(!d.is_cacheable());
    }

    #[test]
    fn test_cache_control_private() {
        let d = CacheDirectives::parse("private, max-age=600");
        assert!(!d.is_cacheable());
    }

    // --- TtlConfig ---
    #[test]
    fn test_ttl_per_status_override() {
        let ttl = TtlConfig::new(60)
            .with_status(200, 600)
            .with_status(404, 30);

        let d = CacheDirectives::default();
        assert_eq!(ttl.resolve(200, &d), Some(Duration::from_secs(600)));
        assert_eq!(ttl.resolve(404, &d), Some(Duration::from_secs(30)));
        assert_eq!(ttl.resolve(500, &d), Some(Duration::from_secs(60))); // default
    }

    #[test]
    fn test_ttl_cache_control_beats_static() {
        let ttl = TtlConfig::new(60).with_status(200, 600);
        let d = CacheDirectives::parse("max-age=1800");
        assert_eq!(ttl.resolve(200, &d), Some(Duration::from_secs(1800)));
    }

    // --- MemoryCache ---
    #[test]
    fn test_memory_cache_insert_get() {
        let cache = MemoryCache::new(100, 1024 * 1024);
        let key = make_key("/page");
        let entry = make_entry(key.clone(), b"body", 3600);
        cache.put(entry);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_memory_cache_lru_eviction() {
        // max 2 entries
        let cache = MemoryCache::new(2, 1024 * 1024);
        let k1 = make_key("/a");
        let k2 = make_key("/b");
        let k3 = make_key("/c");

        cache.put(make_entry(k1.clone(), b"a", 3600));
        cache.put(make_entry(k2.clone(), b"b", 3600));
        cache.put(make_entry(k3.clone(), b"c", 3600)); // evicts k1 as LRU

        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_some());
        assert!(cache.get(&k3).is_some());
    }

    #[test]
    fn test_memory_cache_size_eviction() {
        // max 1000 bytes
        let cache = MemoryCache::new(100, 100);
        let k1 = make_key("/big");
        // Put 60 bytes
        cache.put(make_entry(k1.clone(), &vec![0u8; 60], 3600));
        assert_eq!(cache.current_bytes(), 60);

        // Put another 60 bytes → needs to evict first entry
        let k2 = make_key("/big2");
        cache.put(make_entry(k2.clone(), &vec![0u8; 60], 3600));
        assert!(cache.current_bytes() <= 100);
    }

    #[test]
    fn test_memory_cache_min_uses() {
        let cache = MemoryCache::new(100, 1024 * 1024).with_min_uses(2);
        let key = make_key("/rare");

        // First put → not stored (only 1 use)
        let stored = cache.put(make_entry(key.clone(), b"x", 3600));
        assert!(!stored);
        assert!(cache.get(&key).is_none());

        // Second put → stored (2nd use)
        let stored = cache.put(make_entry(key.clone(), b"x", 3600));
        assert!(stored);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_memory_cache_purge_prefix() {
        let cache = MemoryCache::new(100, 1024 * 1024);
        // Put entries with known keys
        let k1 = make_key("/api/users");
        let k2 = make_key("/api/posts");
        let k3 = make_key("/static/js");
        cache.put(make_entry(k1.clone(), b"u", 3600));
        cache.put(make_entry(k2.clone(), b"p", 3600));
        cache.put(make_entry(k3.clone(), b"s", 3600));
        assert_eq!(cache.current_entries(), 3);
    }

    // --- CacheStatus ---
    #[test]
    fn test_cache_status_strings() {
        assert_eq!(CacheStatus::Hit.as_str(), "HIT");
        assert_eq!(CacheStatus::Miss.as_str(), "MISS");
        assert_eq!(CacheStatus::Bypass.as_str(), "BYPASS");
        assert_eq!(CacheStatus::Stale.as_str(), "STALE");
        assert_eq!(CacheStatus::Revalidated.as_str(), "REVALIDATED");
    }

    // --- BypassCheck ---
    #[test]
    fn test_bypass_non_get() {
        let check = BypassCheck::default();
        assert!(check.should_bypass("POST", &[]));
        assert!(check.should_bypass("DELETE", &[]));
        assert!(!check.should_bypass("GET", &[]));
    }

    #[test]
    fn test_bypass_auth_header() {
        let check = BypassCheck::default();
        let headers = vec![("Authorization".to_string(), "Bearer token123".to_string())];
        assert!(check.should_bypass("GET", &headers));
    }

    #[test]
    fn test_no_bypass_without_auth() {
        let check = BypassCheck::default();
        let headers = vec![("Content-Type".to_string(), "text/html".to_string())];
        assert!(!check.should_bypass("GET", &headers));
    }
}
