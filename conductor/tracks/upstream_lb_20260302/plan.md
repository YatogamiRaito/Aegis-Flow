# Implementation Plan: Upstream Groups & Advanced Load Balancing (v0.18.0)

## Phase 1: Upstream Group Model & Configuration

- [ ] Task: Define upstream data model (`crates/proxy/src/upstream.rs`)
    - [ ] Write tests for UpstreamGroup struct (name, servers, strategy, health_check, sticky, circuit_breaker)
    - [ ] Implement UpstreamGroup with serde deserialization
    - [ ] Write tests for UpstreamServer struct (addr, weight, max_connections, backup, down)
    - [ ] Implement UpstreamServer with defaults
    - [ ] Write tests for config parsing from TOML (single and multiple upstream groups)
    - [ ] Implement config loading and validation

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Load Balancing Algorithms

- [ ] Task: Implement weighted round-robin
    - [ ] Write tests for equal-weight distribution
    - [ ] Write tests for weighted distribution (weight 5:3:2 produces correct ratio)
    - [ ] Write tests for skipping down/unhealthy servers
    - [ ] Implement weighted round-robin with atomic counter

- [ ] Task: Implement least connections
    - [ ] Write tests for selecting server with fewest active connections
    - [ ] Write tests for tie-breaking with weight
    - [ ] Implement least-connections with AtomicU64 connection counters

- [ ] Task: Implement IP hash (consistent hashing)
    - [ ] Write tests for same IP always routes to same backend
    - [ ] Write tests for redistribution when a backend is removed
    - [ ] Implement IP hash using consistent hashing ring (ketama)

- [ ] Task: Implement generic hash
    - [ ] Write tests for URI-based hash key
    - [ ] Write tests for header-based hash key
    - [ ] Write tests for cookie-based hash key
    - [ ] Implement configurable hash key extraction and hashing

- [ ] Task: Implement Power of Two Choices (P2C)
    - [ ] Write tests for random selection of two candidates
    - [ ] Write tests for choosing candidate with fewer connections
    - [ ] Implement P2C algorithm

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Health Checks

- [ ] Task: Implement active health checks
    - [ ] Write tests for periodic HTTP health check requests
    - [ ] Implement health check loop with configurable interval and timeout
    - [ ] Write tests for healthy_threshold (consecutive successes → healthy)
    - [ ] Write tests for unhealthy_threshold (consecutive failures → unhealthy)
    - [ ] Implement threshold tracking per backend
    - [ ] Write tests for custom health check path and expected status code
    - [ ] Write tests for health check with custom headers

- [ ] Task: Implement passive health checks
    - [ ] Write tests for tracking consecutive 5xx responses
    - [ ] Implement failure counter per backend
    - [ ] Write tests for max_fails → mark server down
    - [ ] Write tests for fail_timeout expiry → reintroduce server
    - [ ] Implement timed recovery with slow start

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Sticky Sessions & Circuit Breaker

- [ ] Task: Implement cookie-based sticky sessions
    - [ ] Write tests for session cookie injection in response
    - [ ] Implement cookie insertion with configurable name, domain, path, TTL, flags
    - [ ] Write tests for session cookie reading from request
    - [ ] Implement sticky session lookup and backend selection
    - [ ] Write tests for fallback when sticky backend is down

- [ ] Task: Implement circuit breaker
    - [ ] Write tests for closed → open transition (error rate exceeds threshold)
    - [ ] Write tests for open → half-open transition (after timeout)
    - [ ] Write tests for half-open → closed transition (recovery probes succeed)
    - [ ] Write tests for half-open → open transition (recovery probe fails)
    - [ ] Implement CircuitBreaker state machine with sliding window error rate
    - [ ] Write tests for circuit state exposed via metrics

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Connection Pooling & Timeouts

- [ ] Task: Implement HTTP connection pool
    - [ ] Write tests for connection reuse (keep-alive)
    - [ ] Implement connection pool with configurable max idle connections
    - [ ] Write tests for keepalive_timeout (evict idle connections)
    - [ ] Write tests for keepalive_requests (close after N requests)
    - [ ] Implement pool maintenance task

- [ ] Task: Implement proxy timeouts
    - [ ] Write tests for proxy_connect_timeout enforcement
    - [ ] Write tests for proxy_read_timeout enforcement
    - [ ] Write tests for proxy_send_timeout enforcement
    - [ ] Implement timeout wrappers around upstream operations
    - [ ] Write tests for proxy_buffering on/off behavior

- [ ] Task: Integrate upstream groups into routing pipeline
    - [ ] Write tests for proxy_pass resolving to upstream group name
    - [ ] Implement upstream resolution in location handler
    - [ ] Write tests for backup server activation when all primaries are down

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
