# Track Specification: Proxy Caching & Response Optimization (v0.20.0)

## 1. Overview

This track adds an nginx-style **proxy cache** to Aegis-Flow, enabling the caching of upstream responses on disk and in memory. This dramatically reduces backend load and improves response times for frequently accessed content. It includes cache key management, TTL control, cache invalidation (purge), stale-while-revalidate, and bypass rules.

## 2. Functional Requirements

### 2.1 Cache Storage Backend
- **Two-tier cache:** In-memory (hot tier) + disk (cold tier).
- In-memory cache: LRU eviction, configurable max size (default: 128MB).
- Disk cache: hierarchical directory structure based on cache key hash (e.g., `/cache/a/b3/abcd1234...`).
- Configurable `proxy_cache_path` for disk storage, `max_size` for disk quota.
- Cache metadata stored alongside content (headers, status, creation time, TTL).

### 2.2 Cache Key
- Default cache key: `$scheme$proxy_host$request_uri` (scheme + host + path + query).
- Custom cache key via configuration (e.g., include/exclude specific headers or cookies).
- Normalized query string sorting for better hit rate.

### 2.3 Cache Control Logic
- Respect upstream `Cache-Control` headers: `max-age`, `s-maxage`, `no-cache`, `no-store`, `private`, `public`.
- `proxy_cache_valid`: override TTL per status code (e.g., `200 = "10m"`, `404 = "1m"`).
- Respect `Expires` header as fallback when `Cache-Control` is absent.
- `proxy_cache_min_uses`: only cache after N requests for the same key (avoid caching one-off requests).

### 2.4 Cache Bypass & Skip
- `proxy_cache_bypass`: skip cache for requests matching conditions (e.g., if `$cookie_nocache` is set).
- `proxy_no_cache`: don't cache responses matching conditions (e.g., `Set-Cookie` present).
- Always bypass cache for POST, PUT, DELETE methods (only cache GET and HEAD by default).
- `pragma: no-cache` handling.

### 2.5 Stale Content Serving
- `proxy_cache_use_stale`: serve stale cache when upstream returns error, timeout, or is updating.
  - Options: `error`, `timeout`, `updating`, `http_500`, `http_502`, `http_503`, `http_504`.
- `proxy_cache_background_update`: asynchronously update stale cache while serving stale response.
- `stale-while-revalidate` and `stale-if-error` Cache-Control extensions support.

### 2.6 Cache Invalidation (Purge)
- HTTP PURGE method support: `PURGE /path` invalidates the cached entry for that path.
- Wildcard purge: `PURGE /api/*` clears all entries matching the pattern.
- Purge API: REST endpoint for programmatic cache management.
- Access control: only allow purge from configured IP addresses.
- Cache tag support: tag cached responses and purge by tag.

### 2.7 Cache Status Headers
- `X-Cache-Status` response header: `HIT`, `MISS`, `BYPASS`, `EXPIRED`, `STALE`, `UPDATING`, `REVALIDATED`.
- Keep track of cache hit rate and expose via Prometheus metrics.
- Metrics: `aegis_cache_hits_total`, `aegis_cache_misses_total`, `aegis_cache_size_bytes`, `aegis_cache_entries`.

### 2.8 Conditional Revalidation
- When cached content expires, send conditional request to upstream with `If-None-Match` (ETag) and `If-Modified-Since`.
- If upstream returns 304, refresh cache TTL without re-downloading body.
- Saves bandwidth for large responses that haven't changed.

### 2.9 Configuration Example
```toml
[cache]
enabled = true
path = "/var/cache/aegis"
max_size = "10G"
memory_size = "128M"
min_uses = 2
key = "$scheme$host$request_uri"

  [cache.valid]
  200 = "10m"
  301 = "1h"
  404 = "30s"
  any = "1m"

  [cache.stale]
  use_stale = ["error", "timeout", "updating", "http_500", "http_502", "http_503"]
  background_update = true

  [cache.purge]
  enabled = true
  allow = ["10.0.0.0/8", "127.0.0.1"]

# Per-location:
# [[server.location]]
# path = "/api/"
# proxy_cache = true
# proxy_cache_bypass = "$cookie_nocache"
# proxy_no_cache = "$http_set_cookie"
```

## 3. Non-Functional Requirements

- Cache lookup latency: < 1µs in-memory, < 1ms disk.
- Disk I/O: async with tokio::fs.
- Lock contention: per-key locking (not global) to avoid cache stampede.
- Thundering herd protection: cache lock — only one request populates cache, others wait.

## 4. Acceptance Criteria

- [ ] Upstream responses are cached in memory and on disk.
- [ ] Cache key includes scheme, host, and full URI.
- [ ] `Cache-Control` headers from upstream are respected.
- [ ] `proxy_cache_valid` overrides TTL per status code.
- [ ] Cache bypass works for conditioned requests and non-GET methods.
- [ ] Stale content is served during upstream errors.
- [ ] Background update refreshes cache while serving stale.
- [ ] PURGE method invalidates specific cache entries.
- [ ] Conditional revalidation with If-None-Match/If-Modified-Since works.
- [ ] X-Cache-Status header is present on all proxied responses.
- [ ] Prometheus metrics track cache hit rate and size.
- [ ] >90% test coverage.

## 5. Out of Scope

- CDN-style edge caching / multi-node cache synchronization.
- Cache warming (pre-population).
