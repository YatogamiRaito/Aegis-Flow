# Implementation Plan: Proxy Caching & Response Optimization (v0.20.0)

## Phase 1: Cache Storage Engine

- [ ] Task: Create caching module (`crates/proxy/src/cache.rs`)
    - [ ] Write tests for CacheEntry struct (key, headers, body, status, created_at, ttl, etag, last_modified)
    - [ ] Implement CacheEntry with serialization
    - [ ] Write tests for CacheKey generation from request (scheme + host + URI)
    - [ ] Implement CacheKey builder with normalization (query string sorting)

- [ ] Task: Implement in-memory LRU cache
    - [ ] Write tests for LRU insertion, retrieval, and eviction
    - [ ] Implement MemoryCache using lru crate with configurable max size
    - [ ] Write tests for size-based eviction (evict by response body size, not just count)
    - [ ] Implement size tracking and eviction

- [ ] Task: Implement disk cache storage
    - [ ] Write tests for cache key → file path mapping (hierarchical: /a/b3/hash)
    - [ ] Implement path generation from hash
    - [ ] Write tests for writing cache entry to disk (metadata + body)
    - [ ] Implement async file write with atomic rename
    - [ ] Write tests for reading cache entry from disk
    - [ ] Implement async file read with deserialization
    - [ ] Write tests for max_size enforcement and disk space eviction (LRU by access time)
    - [ ] Implement disk quota enforcement with background cleanup task

- [ ] Task: Implement two-tier cache lookup (memory → disk → upstream)
    - [ ] Write tests for memory hit path
    - [ ] Write tests for disk hit path (promote to memory)
    - [ ] Write tests for miss path (fetch from upstream, store in both tiers)
    - [ ] Implement tiered lookup logic

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Cache Control & TTL Logic

- [ ] Task: Implement Cache-Control header parsing
    - [ ] Write tests for parsing max-age, s-maxage, no-cache, no-store, private, public
    - [ ] Implement CacheDirective parser
    - [ ] Write tests for TTL calculation: s-maxage > max-age > Expires > proxy_cache_valid
    - [ ] Implement TTL resolver with precedence chain

- [ ] Task: Implement proxy_cache_valid (per-status TTL override)
    - [ ] Write tests for status-specific TTLs (200=10m, 404=30s)
    - [ ] Write tests for "any" catch-all TTL
    - [ ] Implement config-based TTL override

- [ ] Task: Implement proxy_cache_min_uses
    - [ ] Write tests for request counting (don't cache until N requests for same key)
    - [ ] Implement request counter with expiration

- [ ] Task: Implement cache bypass and skip logic
    - [ ] Write tests for proxy_cache_bypass conditions (cookie, header, variable matching)
    - [ ] Write tests for proxy_no_cache conditions (Set-Cookie check)
    - [ ] Write tests for method filtering (only cache GET/HEAD)
    - [ ] Implement bypass/skip evaluation

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Stale Content & Background Update

- [ ] Task: Implement stale content serving
    - [ ] Write tests for serving stale on upstream error
    - [ ] Write tests for serving stale on upstream timeout
    - [ ] Write tests for serving stale during cache update (updating state)
    - [ ] Write tests for serving stale on specific HTTP status codes (500, 502, 503, 504)
    - [ ] Implement stale serving with configurable conditions

- [ ] Task: Implement background cache update
    - [ ] Write tests for async update triggered when stale content is served
    - [ ] Implement background task that fetches fresh content and updates cache
    - [ ] Write tests for cache lock (prevent thundering herd / stampede)
    - [ ] Implement per-key lock so only one request populates cache while others wait or get stale

- [ ] Task: Implement conditional revalidation
    - [ ] Write tests for If-None-Match sent to upstream with cached ETag
    - [ ] Write tests for If-Modified-Since sent to upstream with cached Last-Modified
    - [ ] Write tests for 304 Not Modified → refresh TTL without re-downloading
    - [ ] Implement revalidation logic

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Cache Invalidation & Purge

- [ ] Task: Implement HTTP PURGE method
    - [ ] Write tests for PURGE /path → delete specific cache entry
    - [ ] Implement single-key purge
    - [ ] Write tests for wildcard purge (PURGE /api/* → delete matching entries)
    - [ ] Implement prefix-based purge scan
    - [ ] Write tests for purge access control (only allow from configured CIDRs)
    - [ ] Implement ACL check on PURGE requests

- [ ] Task: Implement cache tag system
    - [ ] Write tests for tagging responses (X-Cache-Tags header from upstream)
    - [ ] Implement tag storage in cache metadata
    - [ ] Write tests for purge-by-tag (PURGE with X-Cache-Tag header)
    - [ ] Implement tag-based invalidation

- [ ] Task: Implement REST purge API
    - [ ] Write tests for POST /cache/purge { "key": "..." }
    - [ ] Write tests for POST /cache/purge { "tag": "..." }
    - [ ] Write tests for POST /cache/purge { "pattern": "/api/*" }
    - [ ] Write tests for GET /cache/stats (hit rate, size, entries)
    - [ ] Implement purge REST endpoints

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Cache Status & Metrics Integration

- [ ] Task: Implement X-Cache-Status header
    - [ ] Write tests for HIT status (served from cache)
    - [ ] Write tests for MISS status (fetched from upstream)
    - [ ] Write tests for BYPASS status (cache bypassed by condition)
    - [ ] Write tests for EXPIRED status (cache was expired, fetched fresh)
    - [ ] Write tests for STALE status (served stale due to upstream error)
    - [ ] Write tests for UPDATING status (served stale, background update in progress)
    - [ ] Write tests for REVALIDATED status (304 from upstream, TTL refreshed)
    - [ ] Implement X-Cache-Status header injection

- [ ] Task: Implement Prometheus cache metrics
    - [ ] Write tests for aegis_cache_hits_total counter
    - [ ] Write tests for aegis_cache_misses_total counter
    - [ ] Write tests for aegis_cache_size_bytes gauge
    - [ ] Write tests for aegis_cache_entries gauge
    - [ ] Write tests for aegis_cache_evictions_total counter
    - [ ] Register and implement all metrics

- [ ] Task: Integrate caching into proxy pipeline
    - [ ] Write tests for cache middleware position in request pipeline
    - [ ] Implement cache middleware as Tower layer
    - [ ] Write tests for end-to-end cache flow (miss → store → hit → stale → revalidate)

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
