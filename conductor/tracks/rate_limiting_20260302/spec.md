# Track Specification: Rate Limiting & Security (v0.19.0)

## 1. Overview

This track adds comprehensive **rate limiting**, **IP access control**, **HTTP authentication**, and **security hardening** to Aegis-Flow. These features are essential for protecting backends from abuse, controlling access, and ensuring compliance with security best practices.

## 2. Functional Requirements

### 2.1 Request Rate Limiting (`limit_req`)
- Token bucket algorithm for per-key rate limiting.
- **Key options:** `$remote_addr` (per-IP), `$server_name` (per-vhost), custom header value, API key, or combination.
- Configurable `rate` (e.g., `10r/s`, `100r/m`).
- Configurable `burst` size (requests allowed to exceed rate, queued or rejected).
- `nodelay` option: serve burst requests immediately without queuing.
- Named rate limit zones for reuse across locations.
- Response: 429 Too Many Requests with `Retry-After` header.

### 2.2 Connection Rate Limiting (`limit_conn`)
- Limit concurrent connections per key (IP, vhost, etc.).
- Configurable `max_connections` per zone.
- Response: 503 Service Unavailable when limit is exceeded.

### 2.3 Bandwidth Limiting
- `limit_rate` (bytes per second): throttle response speed per connection.
- `limit_rate_after` (bytes): apply rate limit only after N bytes (allow fast bursts for page load).
- Useful for preventing bandwidth abuse on file downloads.

### 2.4 IP Access Control Lists (ACL)
- `allow` and `deny` rules per server or location block.
- Support CIDR notation: `10.0.0.0/8`, `2001:db8::/32`, single IPs, and `all`.
- Evaluation order: first matching rule wins, implicit deny at end (if any rules exist).
- Support for external IP list files (`include = "/etc/aegis/blocklist.txt"`).
- GeoIP-based rules (optional): block/allow by country code using MaxMind GeoLite2.

### 2.5 HTTP Basic Authentication
- Per-location `auth_basic` directive with realm message.
- Password file support (htpasswd-compatible format: `user:bcrypt_hash`).
- bcrypt verification for secure password storage.
- `satisfy = "any"` or `satisfy = "all"` for combining with IP ACL.

### 2.6 JWT Token Authentication
- Validate JWT tokens from `Authorization: Bearer <token>` header.
- Configurable JWKS endpoint for key retrieval (with caching).
- Claims validation: `iss`, `aud`, `exp`, custom claims.
- Failed auth returns 401 Unauthorized with `WWW-Authenticate` header.

### 2.7 Web Application Firewall (WAF) Basics
- Block common attack patterns:
  - SQL Injection: detect `UNION SELECT`, `OR 1=1`, `DROP TABLE` patterns in URI/body.
  - XSS: detect `<script>`, `javascript:`, event handlers in parameters.
  - Path Traversal: `../`, `..%2f`, encoded variants.
  - Command Injection: backticks, `$()`, pipe characters in parameters.
- Configurable rule engine with severity levels (block, log, pass).
- Custom rule definitions (regex-based pattern matching on URI, headers, body).
- ModSecurity OWASP CRS-inspired rule set (subset).

### 2.8 DDoS Mitigation Basics
- SYN cookie support (OS-level, documented configuration).
- Slowloris protection: configurable `client_header_timeout` and `client_body_timeout`.
- Limit request body size: `client_max_body_size` (default: 1MB).
- Reject requests with excessively large headers.

### 2.9 Configuration Example
```toml
[rate_limit]
  [[rate_limit.zone]]
  name = "api_zone"
  key = "$remote_addr"
  rate = "10r/s"
  burst = 20
  nodelay = true

[security]
client_max_body_size = "10M"
client_header_timeout = "10s"
client_body_timeout = "30s"

  [[security.acl]]
  action = "allow"
  cidr = "10.0.0.0/8"

  [[security.acl]]
  action = "deny"
  cidr = "all"

  [security.waf]
  enabled = true
  mode = "block"  # or "log_only"
  rules = ["sqli", "xss", "path_traversal", "command_injection"]

# Per-location usage:
# [[server.location]]
# path = "/api/"
# rate_limit_zone = "api_zone"
# auth_basic = "API Access"
# auth_basic_user_file = "/etc/aegis/htpasswd"
```

## 3. Non-Functional Requirements

- Rate limiter overhead: < 100ns per request lookup.
- Token bucket state: < 100 bytes per tracked key.
- WAF rule evaluation: < 10µs per request for default rule set.
- Must handle 100k+ tracked IPs without significant memory growth.

## 4. Acceptance Criteria

- [ ] Per-IP rate limiting with token bucket works correctly.
- [ ] Burst and nodelay options function as specified.
- [ ] 429 Too Many Requests returned with Retry-After header.
- [ ] Connection limiting enforces max concurrent connections.
- [ ] Bandwidth limiting throttles response delivery.
- [ ] IP ACL allow/deny rules with CIDR support work correctly.
- [ ] HTTP Basic Auth validates against htpasswd files.
- [ ] JWT token validation with JWKS endpoint works.
- [ ] WAF blocks SQL injection, XSS, path traversal, command injection.
- [ ] client_max_body_size rejects oversized requests (413 Payload Too Large).
- [ ] Request timeouts protect against slow clients.
- [ ] >90% test coverage.

## 5. Out of Scope

- Full ModSecurity compatibility.
- Captcha/challenge pages.
- Bot detection / fingerprinting.
