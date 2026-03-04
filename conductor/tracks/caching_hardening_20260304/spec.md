# Track Specification: Proxy Caching Hardening (v0.45.0)

## 1. Overview
The current proxy architecture contains a fully functioning in-memory LRU cache (`proxy_cache.rs`). Unlike previous tracks, this memory cache is actively used within `http_proxy.rs`. However, the advanced caching features (such as Disk Caching, Stale Content Serving, Background Updates, and the PURGE API) are entirely missing. The configuration is also disconnected from `config.rs`. This hardening track integrates configuration parsing and implements the missing advanced caching tiers.

## 2. Functional Requirements

### 2.1 Configuration Integration
- Augment `ProxyConfig` in `config.rs` with a `[cache]` block containing:
  - `enabled: bool`, `path: String`, `max_size: usize`, `memory_size: usize`, `min_uses: usize`.
  - `[cache.valid]`, `[cache.stale]`, `[cache.purge]`.
- Augment `LocationBlock` with boolean flag `proxy_cache` and strings for `proxy_cache_bypass` and `proxy_no_cache`.

### 2.2 Disk Caching (Tier 2)
- Implement `FileCache` leveraging `tokio::fs`.
- Hash the cache key to create hierarchical directory structures (e.g., `/var/cache/aegis/a/b3/hash_id`).
- Store the HTTP payload alongside metadata (Headers, TTL, ETag).
- On a Memory lookup miss, verify the `FileCache`. If found, promote to `MemoryCache` and stream from disk to the client.

### 2.3 Stale Content Serving & Background Update
- Refactor `proxy_cache.rs` to support `Updating` and `Stale` states.
- If a client requests a stale resource and `use_stale` is enabled for the current situation (e.g., downstream gets an error or the resource is just `Expired` but `background_update = true`), serve the stale content immediately.
- Trigger an asynchronous `tokio::spawn` task to fetch the latest resource and overwrite the cache transparently.

### 2.4 Caching PURGE API
- Intercept HTTP `PURGE` requests in `http_proxy.rs` globally or at specific locations.
- Only permit `PURGE` requests from IPs listed in `[cache.purge] allow`.
- Support purging by absolute path or wildcard path to invalidate entries from both Memory and Disk.

## 3. Non-Functional Requirements
- **Disk I/O:** Disk writes should utilize async I/O (`tokio::fs::File::write_all`) to prevent blocking the worker threads.
- **Race conditions:** Implement fine-grained, per-key synchronization to prevent the "Thundering Herd" problem when a cache expires under heavy load.

## 4. Acceptance Criteria
- [ ] `config.rs` parses `[cache]` blocks successfully and instantiates `TtlConfig` and `MemoryCache`/`FileCache` sizes accurately.
- [ ] Large payloads correctly fall back to disk caching and persist across Aegis-Flow restarts.
- [ ] Cache stampedes are avoided via background updating and stale serving.
- [ ] `PURGE /api` correctly deletes the associated cache entry, resulting in a MISS on the next request.
