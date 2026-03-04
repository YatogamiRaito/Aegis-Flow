# Track Specification: Rate Limiting & Security Hardening (v0.44.0)

## 1. Overview
Aegis-Flow contains excellent standalone implementations for Rate Limiting (`rate_limit.rs`, `limit_rate.rs`), Connection Limiting (`conn_limit.rs`), Access Control (`acl.rs`), Web Application Firewall (`waf.rs`), and Authentication (`auth.rs`, `jwt.rs`). However, they are currently disconnected from the main request lifecycle in `http_proxy.rs` and the configuration system in `config.rs`. This track integrates these security components into the proxy pipeline.

## 2. Functional Requirements

### 2.1 Configuration Integration
- Augment `ProxyConfig` and `LocationBlock` data models in `config.rs` to support security directives:
  - `[rate_limit]` zones and `rate_limit_zone` references per location.
  - `[security.acl]` and per-location `allow`/`deny` rules.
  - `[security.waf]` configuration.
  - `auth_basic` and `auth_basic_user_file`.
  - `client_max_body_size`, `client_header_timeout`, `client_body_timeout`.

### 2.2 Security Middleware Pipeline
- Introduce a structured security evaluation phase in `HttpProxy::handle_request` before any upstream connection occurs (`forward_to_upstream`).
- **Evaluation Order:**
  1. **Connection Limiting:** Reject with 503 if max concurrent connections are exceeded.
  2. **IP Access Control (ACL):** Reject with 403 Forbidden if the client IP is denied by explicit rules.
  3. **Rate Limiting:** Reject with 429 Too Many Requests (with `Retry-After`) if the token bucket is empty.
  4. **Authentication (Basic/JWT):** Reject with 401 Unauthorized (with `WWW-Authenticate`) if credentials are required but missing/invalid.
  5. **Web Application Firewall (WAF):** Reject with 403 Forbidden if the request URI or Headers match malicious patterns configured in `WafEngine`.

### 2.3 Body limits and Bandwidth Throttling
- Enforce `client_max_body_size` while streaming the hyper request body. Return 413 Payload Too Large if exceeded.
- Implement the `limit_rate` response wrapping logic using the `ThrottledWriter` pattern to cap download bandwidth on outbound response streams.

## 3. Non-Functional Requirements
- **Performance:** Security checks (ACL, Rate Limit, WAF) must add `< 500µs` of latency per request. Use atomic operations for limits and cached regex engines for the WAF.
- **Modularity:** Ensure the security abstractions are loosely coupled to `HttpProxy` so they can theoretically be reused in `UdpProxyServer` or `stream_proxy.rs`.

## 4. Acceptance Criteria
- [ ] `config.rs` successfully parses `[security]` and `[rate_limit]` tables from `aegis.toml`.
- [ ] HTTP requests mapping to a rate-limited zone correctly return 429 after exceeding the configured burst limit.
- [ ] WAF drops SQLi (`%20OR%201=1`) and XSS queries immediately with a 403 response.
- [ ] Valid requests passing all security checks reach the upstream servers transparently.
- [ ] Integration test suite covers the entire pipeline execution order.
