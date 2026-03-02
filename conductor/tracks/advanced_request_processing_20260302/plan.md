# Implementation Plan: Advanced Request Processing (v0.26.0)

## Phase 1: Map Directive & Variable Mapping

- [ ] Task: Implement map engine (`crates/proxy/src/map_directive.rs`)
    - [ ] Write tests for exact string matching (key → value)
    - [ ] Implement exact match using HashMap
    - [ ] Write tests for regex matching (~ pattern → value)
    - [ ] Implement regex match with compiled regex cache
    - [ ] Write tests for default value when no match
    - [ ] Implement default fallback
    - [ ] Write tests for lazy evaluation (only resolve when variable is accessed)
    - [ ] Implement lazy evaluation using Cow or deferred resolver

- [ ] Task: Integrate map variables into variable system
    - [ ] Write tests for map variable usage in proxy_pass (e.g., `proxy_pass = "$backend"`)
    - [ ] Implement variable resolution chain: built-in → map → env
    - [ ] Write tests for multiple map blocks with different source/variable pairs
    - [ ] Write tests for map config parsing from TOML

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Split Clients (A/B Testing)

- [ ] Task: Implement split clients (`crates/proxy/src/split_clients.rs`)
    - [ ] Write tests for percentage-based bucket assignment
    - [ ] Implement hash-based bucketing (MurmurHash3 of key → bucket by percent ranges)
    - [ ] Write tests for consistent hashing (same IP → same bucket always)
    - [ ] Write tests for 100% total validation (buckets must sum to 100%)
    - [ ] Implement config validation
    - [ ] Write tests for $variant variable available in downstream directives
    - [ ] Implement split_clients variable injection

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Auth Request (Subrequest Authentication)

- [ ] Task: Implement auth subrequest (`crates/proxy/src/auth_request.rs`)
    - [ ] Write tests for subrequest to external auth endpoint
    - [ ] Implement HTTP client call to auth service with original request headers
    - [ ] Write tests for 2xx → allow, 401/403 → deny, 5xx → configurable (allow/deny)
    - [ ] Implement response code interpretation
    - [ ] Write tests for header forwarding: Authorization, Cookie, X-Original-URI, X-Original-Method
    - [ ] Write tests for response header capture (auth_request_set)
    - [ ] Implement auth response header extraction and injection into upstream request

- [ ] Task: Implement auth caching
    - [ ] Write tests for caching auth responses for N seconds
    - [ ] Implement TTL-based auth response cache keyed by request signature
    - [ ] Write tests for satisfy=any (ACL OR auth passes)
    - [ ] Implement satisfy logic combining ACL and auth_request results

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Traffic Mirroring

- [ ] Task: Implement request mirroring (`crates/proxy/src/mirror.rs`)
    - [ ] Write tests for full request duplication to mirror backend
    - [ ] Implement async fire-and-forget request clone to mirror endpoint
    - [ ] Write tests for mirror response discard (don't affect primary response)
    - [ ] Write tests for mirror_percentage (only mirror N% of requests)
    - [ ] Implement percentage-based sampling
    - [ ] Write tests for mirror_request_body (include/exclude body in mirror)
    - [ ] Implement body forwarding toggle
    - [ ] Write tests for zero latency impact on primary request
    - [ ] Implement tokio::spawn for async mirror (decoupled from primary)

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Limit Except & Stub Status

- [ ] Task: Implement limit_except
    - [ ] Write tests for method-based access control (allow GET, deny POST)
    - [ ] Implement method filtering with allow/deny actions
    - [ ] Write tests for integration with location blocks

- [ ] Task: Implement stub status page
    - [ ] Write tests for /aegis_status endpoint returning connection metrics
    - [ ] Implement metrics collection: active connections, accepts, requests, reading, writing, waiting
    - [ ] Write tests for HTML format output
    - [ ] Write tests for JSON format output
    - [ ] Implement dual-format status page

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
