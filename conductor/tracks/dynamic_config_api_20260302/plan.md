# Implementation Plan: Dynamic Configuration API (v0.25.0)

## Phase 1: Admin API Server

- [ ] Task: Create admin API server (`crates/proxy/src/admin_api.rs`)
    - [ ] Write tests for admin HTTP server binding on localhost:2019
    - [ ] Implement admin server using axum or hyper
    - [ ] Write tests for localhost-only restriction (reject non-local unless authed)
    - [ ] Implement origin enforcement
    - [ ] Write tests for API key authentication (X-API-Key header)
    - [ ] Implement API key middleware

- [ ] Task: Implement shared config state
    - [ ] Write tests for Arc<RwLock<RuntimeConfig>> concurrent access
    - [ ] Implement RuntimeConfig wrapper with version tracking
    - [ ] Write tests for config validation before applying changes
    - [ ] Implement transactional config update (validate → swap)

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Configuration CRUD Endpoints

- [ ] Task: Implement server block endpoints
    - [ ] Write tests for GET /config/servers (list all)
    - [ ] Write tests for GET /config/servers/{id} (single)
    - [ ] Write tests for POST /config/servers (add new)
    - [ ] Write tests for PUT /config/servers/{id} (update)
    - [ ] Write tests for DELETE /config/servers/{id} (remove)
    - [ ] Implement all server block CRUD handlers
    - [ ] Write tests for invalid server config rejection (400 error)

- [ ] Task: Implement upstream endpoints
    - [ ] Write tests for GET/POST/PUT/DELETE /config/upstreams
    - [ ] Write tests for POST /config/upstreams/{name}/servers (add backend)
    - [ ] Write tests for DELETE /config/upstreams/{name}/servers/{addr} (remove backend)
    - [ ] Implement upstream CRUD handlers
    - [ ] Write tests for live upstream changes reflected in load balancer

- [ ] Task: Implement config reload and export
    - [ ] Write tests for POST /config/reload (re-read from file)
    - [ ] Write tests for GET /config/export?format=toml
    - [ ] Write tests for GET /config/export?format=yaml
    - [ ] Implement reload and export handlers

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Status Endpoints & Versioning

- [ ] Task: Implement status endpoints
    - [ ] Write tests for GET /status (overview: uptime, version, connections)
    - [ ] Write tests for GET /status/connections (active connection details)
    - [ ] Write tests for GET /status/upstreams (per-backend health, latency, connections)
    - [ ] Write tests for GET /status/certs (cert expiry, issuer, domains)
    - [ ] Write tests for GET /status/cache (hit rate, entries, size)
    - [ ] Write tests for GET /status/processes (managed process table)
    - [ ] Implement all status handlers

- [ ] Task: Implement config versioning
    - [ ] Write tests for version increment on each config change
    - [ ] Implement version counter in RuntimeConfig
    - [ ] Write tests for GET /config/history (last N versions)
    - [ ] Implement config history ring buffer (keep last 50 versions)
    - [ ] Write tests for POST /config/rollback/{version}
    - [ ] Implement rollback by restoring historical config snapshot

- [ ] Task: Implement batch operations
    - [ ] Write tests for POST /config/batch (array of operations, all-or-nothing)
    - [ ] Implement batch handler with transactional semantics

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)
