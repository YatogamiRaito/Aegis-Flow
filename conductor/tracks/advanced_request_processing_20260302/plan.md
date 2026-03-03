# Implementation Plan: Advanced Request Processing (v0.26.0)

## Phase 1: Map Directive & Variable Mapping

- [x] Task: Implement map engine (`crates/proxy/src/map_directive.rs`)
    - [x] Write tests for exact string matching (key → value)
    - [x] Implement exact match using HashMap
    - [x] Write tests for regex matching (~ pattern → value)
    - [x] Implement regex match with compiled regex cache
    - [x] Write tests for default value when no match
    - [x] Implement default fallback
    - [x] Write tests for lazy evaluation (only resolve when variable is accessed)
    - [x] Implement lazy evaluation using Cow or deferred resolver

- [x] Task: Integrate map variables into variable system
    - [x] Write tests for map variable usage in proxy_pass (e.g., `proxy_pass = "$backend"`)
    - [x] Implement variable resolution chain: built-in → map → env
    - [x] Write tests for multiple map blocks with different source/variable pairs
    - [x] Write tests for map config parsing from TOML

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Split Clients (A/B Testing)

- [x] Task: Implement split clients (`crates/proxy/src/split_clients.rs`)
    - [x] Write tests for percentage-based bucket assignment
    - [x] Implement hash-based bucketing (MurmurHash3 of key → bucket by percent ranges)
    - [x] Write tests for consistent hashing (same IP → same bucket always)
    - [x] Write tests for 100% total validation (buckets must sum to 100%)
    - [x] Implement config validation
    - [x] Write tests for $variant variable available in downstream directives
    - [x] Implement split_clients variable injection

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Auth Request (Subrequest Authentication)

- [x] Task: Implement auth subrequest (`crates/proxy/src/auth_request.rs`)
    - [x] Write tests for subrequest to external auth endpoint
    - [x] Implement HTTP client call to auth service with original request headers
    - [x] Write tests for 2xx → allow, 401/403 → deny, 5xx → configurable (allow/deny)
    - [x] Implement response code interpretation
    - [x] Write tests for header forwarding: Authorization, Cookie, X-Original-URI, X-Original-Method
    - [x] Write tests for response header capture (auth_request_set)
    - [x] Implement auth response header extraction and injection into upstream request

- [x] Task: Implement auth caching
    - [x] Write tests for caching auth responses for N seconds
    - [x] Implement TTL-based auth response cache keyed by request signature
    - [x] Write tests for satisfy=any (ACL OR auth passes)
    - [x] Implement satisfy logic combining ACL and auth_request results

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Traffic Mirroring

- [x] Task: Implement request mirroring (`crates/proxy/src/mirror.rs`)
    - [x] Write tests for full request duplication to mirror backend
    - [x] Implement async fire-and-forget request clone to mirror endpoint
    - [x] Write tests for mirror response discard (don't affect primary response)
    - [x] Write tests for mirror_percentage (only mirror N% of requests)
    - [x] Implement percentage-based sampling
    - [x] Write tests for mirror_request_body (include/exclude body in mirror)
    - [x] Implement body forwarding toggle
    - [x] Write tests for zero latency impact on primary request
    - [x] Implement tokio::spawn for async mirror (decoupled from primary)

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Limit Except & Stub Status

- [x] Task: Implement limit_except
    - [x] Write tests for method-based access control (allow GET, deny POST)
    - [x] Implement method filtering with allow/deny actions
    - [x] Write tests for integration with location blocks

- [x] Task: Implement stub status page
    - [x] Write tests for /aegis_status endpoint returning connection metrics
    - [x] Implement metrics collection: active connections, accepts, requests, reading, writing, waiting
    - [x] Write tests for HTML format output
    - [x] Write tests for JSON format output
    - [x] Implement dual-format status page

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
