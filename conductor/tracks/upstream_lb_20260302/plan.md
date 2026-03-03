# Implementation Plan: Upstream Groups & Advanced Load Balancing (v0.18.0)

## Phase 1: Upstream Group Model & Configuration

- [x] Task: Define upstream data model (`crates/proxy/src/upstream.rs`)
    - [x] Write tests for UpstreamGroup struct (name, servers, strategy, health_check, sticky, circuit_breaker)
    - [x] Implement UpstreamGroup with serde deserialization
    - [x] Write tests for UpstreamServer struct (addr, weight, max_connections, backup, down)
    - [x] Implement UpstreamServer with defaults
    - [x] Write tests for config parsing from TOML (single and multiple upstream groups)
    - [x] Implement config loading and validation

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Load Balancing Algorithms

- [x] Task: Implement weighted round-robin
    - [x] Write tests for equal-weight distribution
    - [x] Write tests for weighted distribution (weight 5:3:2 produces correct ratio)
    - [x] Write tests for skipping down/unhealthy servers
    - [x] Implement weighted round-robin with atomic counter

- [x] Task: Implement least connections
    - [x] Write tests for selecting server with fewest active connections
    - [x] Write tests for tie-breaking with weight
    - [x] Implement least-connections with AtomicU64 connection counters

- [x] Task: Implement IP hash (consistent hashing)
    - [x] Write tests for same IP always routes to same backend
    - [x] Write tests for redistribution when a backend is removed
    - [x] Implement IP hash using consistent hashing ring (ketama)

- [x] Task: Implement generic hash
    - [x] Write tests for URI-based hash key
    - [x] Write tests for header-based hash key
    - [x] Write tests for cookie-based hash key
    - [x] Implement configurable hash key extraction and hashing

- [x] Task: Implement Power of Two Choices (P2C)
    - [x] Write tests for random selection of two candidates
    - [x] Write tests for choosing candidate with fewer connections
    - [x] Implement P2C algorithm

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Health Checks

- [x] Task: Implement active health checks
    - [x] Write tests for periodic HTTP health check requests
    - [x] Implement health check loop with configurable interval and timeout
    - [x] Write tests for healthy_threshold (consecutive successes → healthy)
    - [x] Write tests for unhealthy_threshold (consecutive failures → unhealthy)
    - [x] Implement threshold tracking per backend
    - [x] Write tests for custom health check path and expected status code
    - [x] Write tests for health check with custom headers

- [x] Task: Implement passive health checks
    - [x] Write tests for tracking consecutive 5xx responses
    - [x] Implement failure counter per backend
    - [x] Write tests for max_fails → mark server down
    - [x] Write tests for fail_timeout expiry → reintroduce server
    - [x] Implement timed recovery with slow start

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Sticky Sessions & Circuit Breaker

- [x] Task: Implement cookie-based sticky sessions
    - [x] Write tests for session cookie injection in response
    - [x] Implement cookie insertion with configurable name, domain, path, TTL, flags
    - [x] Write tests for session cookie reading from request
    - [x] Implement sticky session lookup and backend selection
    - [x] Write tests for fallback when sticky backend is down

- [x] Task: Implement circuit breaker
    - [x] Write tests for closed → open transition (error rate exceeds threshold)
    - [x] Write tests for open → half-open transition (after timeout)
    - [x] Write tests for half-open → closed transition (recovery probes succeed)
    - [x] Write tests for half-open → open transition (recovery probe fails)
    - [x] Implement CircuitBreaker state machine with sliding window error rate
    - [x] Write tests for circuit state exposed via metrics

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Connection Pooling & Timeouts

- [x] Task: Implement HTTP connection pool
    - [x] Write tests for connection reuse (keep-alive)
    - [x] Implement connection pool with configurable max idle connections
    - [x] Write tests for keepalive_timeout (evict idle connections)
    - [x] Write tests for keepalive_requests (close after N requests)
    - [x] Implement pool maintenance task

- [x] Task: Implement proxy timeouts
    - [x] Write tests for proxy_connect_timeout enforcement
    - [x] Write tests for proxy_read_timeout enforcement
    - [x] Write tests for proxy_send_timeout enforcement
    - [x] Implement timeout wrappers around upstream operations
    - [x] Write tests for proxy_buffering on/off behavior

- [x] Task: Integrate upstream groups into routing pipeline
    - [x] Write tests for proxy_pass resolving to upstream group name
    - [x] Implement upstream resolution in location handler
    - [x] Write tests for backup server activation when all primaries are down

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
