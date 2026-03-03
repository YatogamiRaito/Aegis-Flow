# Implementation Plan: Dynamic Configuration API (v0.25.0)

## Phase 1: Admin API Server

- [x] Task: Create admin API server (`crates/proxy/src/admin_api.rs`)
    - [x] Write tests for admin HTTP server binding on localhost:2019
    - [x] Implement admin server using axum or hyper
    - [x] Write tests for localhost-only restriction (reject non-local unless authed)
    - [x] Implement origin enforcement
    - [x] Write tests for API key authentication (X-API-Key header)
    - [x] Implement API key middleware

- [x] Task: Implement shared config state
    - [x] Write tests for Arc<RwLock<RuntimeConfig>> concurrent access
    - [x] Implement RuntimeConfig wrapper with version tracking
    - [x] Write tests for config validation before applying changes
    - [x] Implement transactional config update (validate → swap)

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Configuration CRUD Endpoints

- [x] Task: Implement server block endpoints
    - [x] Write tests for GET /config/servers (list all)
    - [x] Write tests for GET /config/servers/{id} (single)
    - [x] Write tests for POST /config/servers (add new)
    - [x] Write tests for PUT /config/servers/{id} (update)
    - [x] Write tests for DELETE /config/servers/{id} (remove)
    - [x] Implement all server block CRUD handlers
    - [x] Write tests for invalid server config rejection (400 error)

- [x] Task: Implement upstream endpoints
    - [x] Write tests for GET/POST/PUT/DELETE /config/upstreams
    - [x] Write tests for POST /config/upstreams/{name}/servers (add backend)
    - [x] Write tests for DELETE /config/upstreams/{name}/servers/{addr} (remove backend)
    - [x] Implement upstream CRUD handlers
    - [x] Write tests for live upstream changes reflected in load balancer

- [x] Task: Implement config reload and export
    - [x] Write tests for POST /config/reload (re-read from file)
    - [x] Write tests for GET /config/export?format=toml
    - [x] Write tests for GET /config/export?format=yaml
    - [x] Implement reload and export handlers

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Status Endpoints & Versioning

- [x] Task: Implement status endpoints
    - [x] Write tests for GET /status (overview: uptime, version, connections)
    - [x] Write tests for GET /status/connections (active connection details)
    - [x] Write tests for GET /status/upstreams (per-backend health, latency, connections)
    - [x] Write tests for GET /status/certs (cert expiry, issuer, domains)
    - [x] Write tests for GET /status/cache (hit rate, entries, size)
    - [x] Write tests for GET /status/processes (managed process table)
    - [x] Implement all status handlers

- [x] Task: Implement config versioning
    - [x] Write tests for version increment on each config change
    - [x] Implement version counter in RuntimeConfig
    - [x] Write tests for GET /config/history (last N versions)
    - [x] Implement config history ring buffer (keep last 50 versions)
    - [x] Write tests for POST /config/rollback/{version}
    - [x] Implement rollback by restoring historical config snapshot

- [x] Task: Implement batch operations
    - [x] Write tests for POST /config/batch (array of operations, all-or-nothing)
    - [x] Implement batch handler with transactional semantics

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)
