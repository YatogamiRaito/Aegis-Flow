# Implementation Plan: Rate Limiting & Security Hardening (v0.44.0)

## Phase 1: Configuration Refactor
- [ ] Task: Integrate Security Directives into `ProxyConfig`
    - [ ] Update `crates/proxy/src/config.rs`: Add structs mapping to `rate_limit`, `acl`, `waf`, and authentication configurations.
    - [ ] Verify TOML parser successfully reads unified security constraints.
    - [ ] Add properties to `LocationBlock` (`auth_basic`, `rate_limit_zone_ref`, `acl_rules`).

## Phase 2: Middleware Initialization
- [ ] Task: Bootstrap `SecurityContext`
    - [ ] During `HttpProxy::new()`, instantiate `WafEngine` with loaded rules.
    - [ ] Instantiate `AclEngine` and load `include` files if provided.
    - [ ] Initialize global `BucketManager` for active rate-limit zones.
    - [ ] Store these engines in a shared `Arc<SecurityContext>` inside `HttpProxy`.

## Phase 3: Request Interception Pipeline
- [ ] Task: Intercept Requests in `handle_request`
    - [ ] **Connection Limit:** On request arrival, increment atomic connection counter. Decrement on drop.
    - [ ] **IP ACL:** Invoke `AclEngine::check_ip`. Yield 403 if `Deny`.
    - [ ] **Rate Limiter:** Query `BucketManager::check_limit`. Yield 429 with `Retry-After` header if throttled.
    - [ ] **Auth:** Extract `Authorization` header. Invoke `BasicAuthConfig::check_auth` or JWT validation. Yield 401 if invalid.
    - [ ] **WAF:** Invoke `WafEngine::handle_request`. Yield 403 if matched in `Block` mode.

## Phase 4: Size and Bandwidth Limits
- [ ] Task: Payload limits
    - [ ] Implement middleware to check `Content-Length` header upfront.
    - [ ] As body chunks arrive, enforce `client_max_body_size` byte limit (HTTP 413).
    - [ ] Wrap the outgoing Response stream with `limit_rate` logic if active on the Location block.

## Phase 5: Testing and Integration
- [ ] Task: End-to-End Pipeline Evaluation
    - [ ] Write integration test verifying WAF blocks payload despite passing ACL.
    - [ ] Verify Rate Limiting protects upstream from flooding.
    - [ ] Testing protocol in `workflow.md`.
