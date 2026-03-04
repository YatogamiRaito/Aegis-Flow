# Track Specification: Upstream Groups Hardening (v0.43.0)

## 1. Overview
The current proxy architecture contains fully functioning libraries for Upstream Groups (`upstream.rs`), Load Balancing Algorithms (`lb.rs`), Health Checks (`health_check.rs`), Circuit Breakers (`circuit_breaker.rs`), and Sticky Sessions (`sticky.rs`). However, `http_proxy.rs` and `config.rs` do not integrate these libraries. This hardening track aims to integrate these components into the proxy's core request pipeline, officially bringing advanced load balancing to Aegis-Flow.

## 2. Functional Requirements

### 2.1 Configuration Integration
- Add `pub upstreams: Vec<UpstreamGroup>` into `ProxyConfig`.
- Validate that the TOML parser correctly constructs these groups and their underlying configurations on boot.

### 2.2 Health Check Background Tasks
- In `bootstrap.rs`, iterate through `ProxyConfig::upstreams`.
- For each backend in an upstream group that has `health_check` enabled, spawn a background Tokio task running `start_active_health_checks`.
- Ensure changes in health status (Healthy/Unhealthy) correctly toggle the `down` state of the corresponding `RuntimeServer` in the load balancer instances.

### 2.3 Router Integration
- In `HttpProxy::handle_request`, when computing the upstream destination, look for a matching `UpstreamGroup` instead of assuming `upstream_addr` is a direct network socket.
- If an `UpstreamGroup` is found, invoke `LoadBalancer::select_server()` to determine the backend.
- Pass the appropriate hash key (`$remote_addr`, `$uri`, or a custom variable) if `IpHash` or `GenericHash` is configured.
- Inject and Extract Sticky Session cookies if `StickyConfig` is active. Route back to the previously designated backend if the backend is healthy.

### 2.4 Circuit Breaker & Passive Failures
- Wrap the remote `reqwest` upstream call inside a circuit breaker context. 
- If the `reqwest` dispatch fails (or returns `502`/`503`/`504`), invoke `circuit_breaker.record_failure()` and `server_health.record_passive_failure()`.
- If the circuit breaker trips (`Open` state), immediately return HTTP 503 Service Unavailable without attempting the connection.

## 3. Non-Functional Requirements
- **Performance:** `LoadBalancer::select_server` execution path must remain lock-free or use highly optimized atomic operations (`AtomicU64`, `AtomicUsize`) to prevent bottlenecking the HTTP/2 event loop.
- **Resilience:** The health check tasks should gracefully shutdown when the `HttpProxy` server terminates.

## 4. Acceptance Criteria
- [ ] `config.rs` parses `[[upstream]]` blocks successfully.
- [ ] Requests routed via `proxy_pass` to a named upstream correctly distribute traffic using the selected load balancing algorithms (RoundRobin, LeastConnections, etc.).
- [ ] Background health check loops actively ping backend endpoints and gracefully remove failed backends from the active rotation.
- [ ] Simulated 5xx errors trigger the circuit breaker into an open state, shedding load instantly.
