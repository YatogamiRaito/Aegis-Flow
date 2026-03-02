# Track Specification: Upstream Groups & Advanced Load Balancing (v0.18.0)

## 1. Overview

This track implements nginx-style **upstream groups** with advanced load balancing, active/passive health checks, sticky sessions, connection pooling, circuit breaker patterns, and failover mechanisms. This transforms Aegis-Flow's existing basic service registry into a production-grade backend management system.

## 2. Functional Requirements

### 2.1 Upstream Group Definition
- Named upstream groups referenced by `proxy_pass` in location blocks.
- Multiple backend servers per group with individual weight, max_connections, and fail settings.
- Support for `backup` servers that receive traffic only when all primary servers are down.
- Support for `down` flag to temporarily disable a server without removing it.

### 2.2 Load Balancing Algorithms
- **Round Robin** (default): Distribute equally among healthy backends, weighted by `weight` parameter.
- **Least Connections:** Route to the backend with fewest active connections.
- **IP Hash:** Consistent hashing based on client IP for session affinity.
- **Generic Hash:** Consistent hashing on configurable key (URI, header, cookie value).
- **Random with Two Choices (P2C):** Pick two random backends, choose the one with fewer connections.

### 2.3 Sticky Sessions
- **Cookie-based:** Insert a session cookie (`AEGIS_UPSTREAM`) mapping the client to a specific backend.
  - Configurable cookie name, domain, path, httponly, secure, SameSite, and TTL.
- **Route-based:** Extract route hint from URL or header to determine backend.
- Session persistence survives backend restarts if the same backend IP is re-added.

### 2.4 Active Health Checks
- Periodic HTTP(S) health check requests to each backend.
- Configurable: `interval` (default: 5s), `timeout` (default: 3s), `path` (default: `/health`), `expected_status` (default: 200).
- Configurable `healthy_threshold` (consecutive successes to mark healthy, default: 2).
- Configurable `unhealthy_threshold` (consecutive failures to mark unhealthy, default: 3).
- Support for custom health check headers and body matching.

### 2.5 Passive Health Checks (Fail Detection)
- Track response status codes from upstream — consecutive 5xx codes mark the backend as unhealthy.
- `max_fails` (default: 3): number of failures before marking down.
- `fail_timeout` (default: 30s): duration to consider server as unavailable after max_fails.
- After `fail_timeout` expires, gradually reintroduce traffic (slow start).

### 2.6 Circuit Breaker
- Three states: **Closed** (normal), **Open** (all requests fail-fast), **Half-Open** (probe with limited traffic).
- Configurable error rate threshold to trip the circuit (default: 50% over 10s window).
- Configurable recovery probe count in half-open state (default: 3 successful requests).
- Expose circuit state via metrics and health endpoints.

### 2.7 Connection Pooling
- Reuse TCP connections to upstream backends (HTTP/1.1 keep-alive, HTTP/2 multiplexing).
- Configurable `keepalive` connections per upstream (default: 64).
- Configurable `keepalive_timeout` (default: 60s).
- Configurable `keepalive_requests` (max requests per connection, default: 1000).

### 2.8 Timeouts & Buffering
- `proxy_connect_timeout` (default: 5s): time to establish connection to upstream.
- `proxy_read_timeout` (default: 60s): time to wait for upstream response.
- `proxy_send_timeout` (default: 60s): time to send request to upstream.
- `proxy_buffer_size` (default: 4k): initial response buffer.
- `proxy_buffering` (default: on): buffer full response before sending to client.

### 2.9 Configuration Example
```toml
[[upstream]]
name = "api-backend"
strategy = "least_conn"
keepalive = 64

  [[upstream.server]]
  addr = "10.0.0.1:3000"
  weight = 5
  max_connections = 100

  [[upstream.server]]
  addr = "10.0.0.2:3000"
  weight = 3

  [[upstream.server]]
  addr = "10.0.0.3:3000"
  backup = true

  [upstream.health_check]
  enabled = true
  path = "/health"
  interval = "5s"
  timeout = "3s"
  healthy_threshold = 2
  unhealthy_threshold = 3

  [upstream.sticky]
  type = "cookie"
  cookie_name = "AEGIS_UPSTREAM"
  cookie_ttl = "1h"
  cookie_httponly = true

  [upstream.circuit_breaker]
  enabled = true
  error_threshold = 0.5
  window = "10s"
  recovery_probes = 3
```

## 3. Non-Functional Requirements

- Health check overhead: < 0.1% CPU for 100 backends checked every 5s.
- Connection pool memory: < 100KB per idle keep-alive connection.
- Load balancing decision latency: < 1µs.

## 4. Acceptance Criteria

- [ ] Upstream groups are configurable with multiple servers.
- [ ] All 5 load balancing algorithms (round-robin, least-conn, ip-hash, generic-hash, P2C) work correctly.
- [ ] Sticky sessions persist client-to-backend mapping via cookies.
- [ ] Active health checks mark backends healthy/unhealthy based on thresholds.
- [ ] Passive health checks detect consecutive failures and mark backends down.
- [ ] Circuit breaker transitions through closed → open → half-open → closed.
- [ ] Connection pooling reuses connections with configurable limits.
- [ ] Backup servers receive traffic when all primaries are down.
- [ ] Timeout and buffering settings are enforced.
- [ ] >90% test coverage.

## 5. Out of Scope

- DNS-based upstream resolution (already in discovery.rs).
- External service mesh integration (Envoy xDS).
