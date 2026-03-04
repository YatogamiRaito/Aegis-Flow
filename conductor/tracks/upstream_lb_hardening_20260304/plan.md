# Implementation Plan: Upstream Groups Hardening (v0.43.0)

## Phase 1: Configuration Refactor
- [ ] Task: Integrate `UpstreamGroup` into `ProxyConfig`
    - [ ] Update `crates/proxy/src/config.rs`: Add `pub upstreams: Vec<UpstreamGroup>` into `ProxyConfig`.
    - [ ] Verify that TOML deserializers build correctly.
    - [ ] Update `bootstrap.rs` and `http_proxy.rs` to initialize `Vec<LoadBalancer>` instances.

## Phase 2: Background Health Checks
- [ ] Task: Spawn Active Health Probes
    - [ ] Modify `bootstrap.rs` or `HttpProxy::new()`.
    - [ ] For every configured backend with health constraints, run tokio `spawn` of `start_active_health_checks`.
    - [ ] Create a thread-safe update channel or an `Arc<AtomicBool>` array mapping the backend's status to the `LoadBalancer`'s available list.

## Phase 3: Runtime Proxy Pass Pipeline
- [ ] Task: Integrate LoadBalancer in Request Lifecycle
    - [ ] Refactor `forward_to_upstream` in `http_proxy.rs` to take an abstract representation of the target.
    - [ ] Locate the named `UpstreamGroup` during resolution.
    - [ ] Apply `check_sticky_session` parsing and routing.
    - [ ] If not sticky, use `LoadBalancer::select_server()` with the appropriate variable key.
    - [ ] Apply `issue_sticky_session` to the proxy response if applicable.

## Phase 4: Circuit Breaker and Passive Checks
- [ ] Task: State Tracking during Dispatch
    - [ ] Look up the `CircuitBreaker` instance for the `UpstreamGroup`.
    - [ ] Before executing `Client::send`, call `cb.acquire()`. If false, instantly yield HTTP 503.
    - [ ] Execute request. On `Err()` or HTTP 5XX, emit `record_failure()`. On success, emit `record_success()`.
    - [ ] Maintain metrics counters.

## Phase 5: Testing and Polish
- [ ] Task: End-to-End Load Testing
    - [ ] Write tests ensuring a backend that dies is successfully evicted and requests continue routing.
    - [ ] Verify HTTP 503 fast-fail functionality on active Circuit Breakers.
    - [ ] Testing protocol in `workflow.md`.
