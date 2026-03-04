# Implementation Plan: Proxy Caching Hardening (v0.45.0)

## Phase 1: Configuration Refactor & Storage Tiers
- [ ] Task: Integrate Cache Directives into `ProxyConfig`
    - [ ] Update `crates/proxy/src/config.rs`: Add structs mapping to `[cache]`.
    - [ ] Add `proxy_cache` properties to `LocationBlock`.
- [ ] Task: Implement Disk Caching `FileCache`
    - [ ] Add `tokio::fs` asynchronous write/read logic inside `proxy_cache.rs`.
    - [ ] Enforce the total cache `max_size` on the disk layer using background cleanup functions.

## Phase 2: Pipeline Integration and Two-Tier Lookup
- [ ] Task: Integrate `FileCache` in `HttpProxy`
    - [ ] In `http_proxy.rs`'s handle_request, if `MemoryCache` misses, query `FileCache`.
    - [ ] If found in `FileCache`, promote it to `MemoryCache` and return `CacheStatus::Hit`.
    - [ ] If missed in both, proceed to upstream. When the response streams back, simultaneously store the payload to both `MemoryCache` and `FileCache`.

## Phase 3: Stale Serving and Background Updating
- [ ] Task: Track "Updating" State
    - [ ] Introduce a lock or `Set<String>` of currently updating keys in `proxy_cache.rs`.
    - [ ] If an expired entry is hit, and it's already "Updating", aggressively return `CacheStatus::Stale`.
- [ ] Task: Run Background Fetch
    - [ ] Implement a standalone detached task that invokes `reqwest` for the upstream URL, replaces the file cache and memory cache upon completion, and releases the "Updating" lock.

## Phase 4: Cache Invalidations (PURGE)
- [ ] Task: Implementing the `PURGE` Method
    - [ ] Check if `req.method() == Method::PURGE` at the start of `handle_request`.
    - [ ] Validate client IP against `purge_allow` ACL.
    - [ ] Delete from `MemoryCache` and `FileCache`. Return 200 OK or 404 NOT FOUND based on presence.

## Phase 5: Testing and Polish
- [ ] Task: End-to-End Testing
    - [ ] Validate standard disk cache creation on disk using standard `reqwest` calls.
    - [ ] Validate HTTP PURGE clears disk files.
    - [ ] Validate Stale Serving using a mock slow server.
    - [ ] Testing protocol in `workflow.md`.
