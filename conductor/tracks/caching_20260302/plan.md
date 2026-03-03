# Implementation Plan: Proxy Caching & Response Optimization (v0.20.0)

## Phase 1: Cache Storage Engine

- [x] Task: Create caching module (`crates/proxy/src/cache.rs`)
    - [x] Write tests for CacheEntry struct (key, headers, body, status, created_at, ttl, etag, last_modified)
    - [x] Implement CacheEntry with serialization
    - [x] Write tests for CacheKey generation from request (scheme + host + URI)
    - [x] Implement CacheKey builder with normalization (query string sorting)

- [x] Task: Implement in-memory LRU cache
    - [x] Write tests for LRU insertion, retrieval, and eviction
    - [x] Implement MemoryCache using lru crate with configurable max size
    - [x] Write tests for size-based eviction (evict by response body size, not just count)
    - [x] Implement size tracking and eviction

- [x] Task: Implement disk cache storage
    - [x] Write tests for cache key → file path mapping (hierarchical: /a/b3/hash)
    - [x] Implement path generation from hash
    - [x] Write tests for writing cache entry to disk (metadata + body)
    - [x] Implement async file write with atomic rename
    - [x] Write tests for reading cache entry from disk
    - [x] Implement async file read with deserialization
    - [x] Write tests for max_size enforcement and disk space eviction (LRU by access time)
    - [x] Implement disk quota enforcement with background cleanup task

- [x] Task: Implement two-tier cache lookup (memory → disk → upstream)
    - [x] Write tests for memory hit path
    - [x] Write tests for disk hit path (promote to memory)
    - [x] Write tests for miss path (fetch from upstream, store in both tiers)
    - [x] Implement tiered lookup logic

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Cache Control & TTL Logic

- [x] Task: Implement Cache-Control header parsing
    - [x] Write tests for parsing max-age, s-maxage, no-cache, no-store, private, public
    - [x] Implement CacheDirective parser
    - [x] Write tests for TTL calculation: s-maxage > max-age > Expires > proxy_cache_valid
    - [x] Implement TTL resolver with precedence chain

- [x] Task: Implement proxy_cache_valid (per-status TTL override)
    - [x] Write tests for status-specific TTLs (200=10m, 404=30s)
    - [x] Write tests for "any" catch-all TTL
    - [x] Implement config-based TTL override

- [x] Task: Implement proxy_cache_min_uses
    - [x] Write tests for request counting (don't cache until N requests for same key)
    - [x] Implement request counter with expiration

- [x] Task: Implement cache bypass and skip logic
    - [x] Write tests for proxy_cache_bypass conditions (cookie, header, variable matching)
    - [x] Write tests for proxy_no_cache conditions (Set-Cookie check)
    - [x] Write tests for method filtering (only cache GET/HEAD)
    - [x] Implement bypass/skip evaluation

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Stale Content & Background Update

- [x] Task: Implement stale content serving
    - [x] Write tests for serving stale on upstream error
    - [x] Write tests for serving stale on upstream timeout
    - [x] Write tests for serving stale during cache update (updating state)
    - [x] Write tests for serving stale on specific HTTP status codes (500, 502, 503, 504)
    - [x] Implement stale serving with configurable conditions

- [x] Task: Implement background cache update
    - [x] Write tests for async update triggered when stale content is served
    - [x] Implement background task that fetches fresh content and updates cache
    - [x] Write tests for cache lock (prevent thundering herd / stampede)
    - [x] Implement per-key lock so only one request populates cache while others wait or get stale

- [x] Task: Implement conditional revalidation
    - [x] Write tests for If-None-Match sent to upstream with cached ETag
    - [x] Write tests for If-Modified-Since sent to upstream with cached Last-Modified
    - [x] Write tests for 304 Not Modified → refresh TTL without re-downloading
    - [x] Implement revalidation logic

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Cache Invalidation & Purge

- [x] Task: Implement HTTP PURGE method
    - [x] Write tests for PURGE /path → delete specific cache entry
    - [x] Implement single-key purge
    - [x] Write tests for wildcard purge (PURGE /api/* → delete matching entries)
    - [x] Implement prefix-based purge scan
    - [x] Write tests for purge access control (only allow from configured CIDRs)
    - [x] Implement ACL check on PURGE requests

- [x] Task: Implement cache tag system
    - [x] Write tests for tagging responses (X-Cache-Tags header from upstream)
    - [x] Implement tag storage in cache metadata
    - [x] Write tests for purge-by-tag (PURGE with X-Cache-Tag header)
    - [x] Implement tag-based invalidation

- [x] Task: Implement REST purge API
    - [x] Write tests for POST /cache/purge { "key": "..." }
    - [x] Write tests for POST /cache/purge { "tag": "..." }
    - [x] Write tests for POST /cache/purge { "pattern": "/api/*" }
    - [x] Write tests for GET /cache/stats (hit rate, size, entries)
    - [x] Implement purge REST endpoints

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Cache Status & Metrics Integration

- [x] Task: Implement X-Cache-Status header
    - [x] Write tests for HIT status (served from cache)
    - [x] Write tests for MISS status (fetched from upstream)
    - [x] Write tests for BYPASS status (cache bypassed by condition)
    - [x] Write tests for EXPIRED status (cache was expired, fetched fresh)
    - [x] Write tests for STALE status (served stale due to upstream error)
    - [x] Write tests for UPDATING status (served stale, background update in progress)
    - [x] Write tests for REVALIDATED status (304 from upstream, TTL refreshed)
    - [x] Implement X-Cache-Status header injection

- [x] Task: Implement Prometheus cache metrics
    - [x] Write tests for aegis_cache_hits_total counter
    - [x] Write tests for aegis_cache_misses_total counter
    - [x] Write tests for aegis_cache_size_bytes gauge
    - [x] Write tests for aegis_cache_entries gauge
    - [x] Write tests for aegis_cache_evictions_total counter
    - [x] Register and implement all metrics

- [x] Task: Integrate caching into proxy pipeline
    - [x] Write tests for cache middleware position in request pipeline
    - [x] Implement cache middleware as Tower layer
    - [x] Write tests for end-to-end cache flow (miss → store → hit → stale → revalidate)

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
