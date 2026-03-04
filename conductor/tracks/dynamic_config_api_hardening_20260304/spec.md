# Track Specification: Dynamic Config API Hardening (v0.50.0)

## 1. Overview
The current implementation of the Admin API in `crates/proxy/src/admin_api.rs` provides a basic Axum web server and tests for updating a dummy `RuntimeConfig`. However, it lacks over 80% of the specified endpoints, handles no validations, lacks authentication, and most critically, is never actually run or connected to the live proxy state.

This track aims to build out the full CRUD surface for the configuration API, secure it with API keys and origin restrictions, and inject its state into the proxy's core request-handling loop.

## 2. Functional Requirements

### 2.1 State injection
- Replace the static `Arc<ProxyConfig>` in `bootstrap.rs` and `http_proxy.rs` with a dynamic construct, such as `ArcSwap<ProxyConfig>` or `RwLock<Arc<ProxyConfig>>`.
- Ensure `http_proxy::handle_request` reads the latest configuration efficiently without locking overhead per request.

### 2.2 Complete CRUD Endpoints
- Implement `/config/upstreams` (GET, POST, PUT, DELETE).
- Implement `/config/upstreams/{name}/servers` (POST, DELETE) to dynamically alter load balancer backends.
- Add comprehensive validation to `POST` and `PUT` operations to return HTTP 400 Bad Request before mutating state.
- Implement `/config/reload` to trigger a re-read of `aegis.yaml` or `Aegisfile`.
- Implement `/config/export` to dump the live config to TOML/YAML.

### 2.3 Status & Monitoring Endpoints
- Implement `/status/connections` (active/idle count).
- Implement `/status/upstreams` (health check status of backends).
- Implement `/status/certs` (loaded SNI certificates and ACME expiry dates).
- Implement `/status/cache` (hit/miss ratio, memory usage).

### 2.4 Security & Authentication
- Implement a middleware for `Axum` that checks `X-API-Key` against `[admin].api_key` in the config.
- Implement origin enforcement (reject non-localhost IP addresses if `enforce_origin = true` and no API key is provided).

### 2.5 Daemon Integration
- In `bootstrap.rs`, start the Axum router via `tokio::spawn` if `[admin].enabled == true`.

## 3. Non-Functional Requirements
- **Performance:** Reading the configuration state in the HTTP request loop must be lock-free on the read path (use `ArcSwap` or similar). Config mutations can take a write-lock.
- **Safety:** Malformed JSON or conflicting upstream names must not crash the proxy.

## 4. Acceptance Criteria
- [ ] Changing an upstream's list of servers via `POST` takes immediate effect for the next HTTP request.
- [ ] Deleting a server block immediately stops routing new traffic to those paths.
- [ ] The API responds on `localhost:2019` when Aegis-Flow is started.
- [ ] Invalid config updates return a 400 response with helpful error messages.
- [ ] Security middlewares block unauthorized remote access.
