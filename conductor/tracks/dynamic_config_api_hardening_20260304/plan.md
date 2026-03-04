# Implementation Plan: Dynamic Config API Hardening (v0.50.0)

## Phase 1: Core State Integration
- [ ] Task: Integrate live config state into the proxy
    - [ ] Introduce `ArcSwap` (or `RwLock`) for `ProxyConfig` in `bootstrap.rs`.
    - [ ] Pass the observable/atomic config reference to `http_proxy::run_server`.
    - [ ] Modify `admin_api::create_router` to accept a writer handle that updates this state atomically.

## Phase 2: Complete the API Endpoints
- [ ] Task: Build out missing CRUD routes
    - [ ] Add axum handlers in `admin_api.rs` for Upstream CRUD (`/config/upstreams`).
    - [ ] Add handlers for adding/removing individual servers from an upstream group.
    - [ ] Implement config validation logic before applying state swaps.

## Phase 3: Authentication and Security
- [ ] Task: Secure the Admin API
    - [ ] Write Axum middleware for `X-API-Key` validation.
    - [ ] Write Axum middleware checking `ConnectInfo<SocketAddr>` to restrict to `127.0.0.1` or `::1` absent an API key.

## Phase 4: Status and Extensibility Endpoints
- [ ] Task: Monitoring & Utility
    - [ ] Hook into the internal metrics/cache managers to serve `/status/upstreams` and `/status/cache`.
    - [ ] Implement `/config/export` to serialize the current `ProxyConfig` to TOML.
    - [ ] Spawn the server in `bootstrap.rs` based on the configuration port.
