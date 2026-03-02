# Track Specification: Advanced Request Processing (v0.26.0)

## 1. Overview

This track adds nginx's advanced request processing capabilities: **`map` directive** (variable mapping), **`split_clients`** (A/B testing), **`auth_request`** (subrequest-based authentication), and **traffic mirroring** (request duplication to a secondary backend). These are the remaining "power-user" features that differentiate a professional proxy from a basic one.

## 2. Functional Requirements

### 2.1 Map Directive (Variable Mapping)
- Create new variables based on the value of other variables.
- Support exact match, regex match, and default value.
- Use case: set backend based on User-Agent, set cache policy based on URI pattern.
- Example:
  ```toml
  [[map]]
  source = "$uri"
  variable = "$backend"
  default = "http://default-backend:3000"
  entries = [
    { match = "~^/api/v1/", value = "http://api-v1:3000" },
    { match = "~^/api/v2/", value = "http://api-v2:3000" },
    { match = "/health", value = "http://health-service:8080" },
  ]
  ```
- Evaluated lazily (only when the variable is referenced).
- Support for hostnames map (input `$host` → output `$backend`).

### 2.2 Split Clients (A/B Testing)
- Distribute requests into groups based on a hash of a key variable.
- Percentage-based splitting for canary deployments and A/B testing.
- Example:
  ```toml
  [[split_clients]]
  key = "$remote_addr"
  variable = "$variant"
  buckets = [
    { percent = 10, value = "canary" },
    { percent = 90, value = "stable" },
  ]
  ```
- Consistent hashing: same client IP always gets the same variant.
- The resulting variable (`$variant`) can be used in `proxy_pass`, headers, etc.

### 2.3 Auth Request (Subrequest Authentication)
- Before processing a request, send a subrequest to an external authentication service.
- If the auth service returns 2xx → proceed. If 401/403 → reject the original request.
- Forward original request headers (Authorization, Cookie, X-Original-URI) to auth service.
- Capture response headers from auth service and pass to upstream (e.g., `X-User-ID`).
- Example:
  ```toml
  [[server.location]]
  path = "/api/"
  auth_request = "http://auth-service:4000/verify"
  auth_request_set = { "X-User-ID" = "$upstream_http_x_user_id" }
  proxy_pass = "http://api-backend:3000"
  ```
- Caching: optionally cache auth responses for N seconds to reduce auth service load.
- `satisfy = "any"`: allow if EITHER auth_request OR ACL passes.

### 2.4 Traffic Mirroring (Request Duplication)
- Duplicate incoming requests to a secondary backend (for testing, shadowing, or logging).
- Mirror requests are fire-and-forget: response is discarded.
- Configurable mirror percentage (e.g., mirror 10% of traffic).
- Mirror does NOT affect the original request's response or latency.
- Example:
  ```toml
  [[server.location]]
  path = "/api/"
  proxy_pass = "http://production:3000"
  mirror = "http://staging:3000"
  mirror_percentage = 100
  mirror_request_body = true
  ```

### 2.5 Limit Except (Method-Based Access Control)
- Allow/deny access based on HTTP method within a location.
- Example: allow GET/HEAD for everyone, require auth for POST/PUT/DELETE.
  ```toml
  [[server.location]]
  path = "/api/data"
  limit_except = { methods = ["GET", "HEAD"], deny = "all" }
  ```

### 2.6 Stub Status / Server Info
- Built-in status page showing server metrics: active connections, accepts, handled, requests, reading, writing, waiting.
- Accessible at configurable path (e.g., `/aegis_status`).
- Both HTML and JSON output formats.

## 3. Non-Functional Requirements

- Map evaluation: < 100ns for hash lookup, < 1µs for regex match.
- Auth subrequest: adds auth service latency to response time (unavoidable but must not add proxy-side overhead).
- Mirror: zero impact on primary request latency (async fire-and-forget).
- Split clients: deterministic — same input always produces same bucket.

## 4. Acceptance Criteria

- [ ] Map directive creates variables from source variables with exact and regex matching.
- [ ] Split clients distributes requests by percentage with consistent hashing.
- [ ] Auth request sends subrequest and blocks/allows based on response.
- [ ] Auth response headers are captured and forwarded to upstream.
- [ ] Traffic mirroring duplicates requests without affecting primary response.
- [ ] Mirror percentage controls what fraction of traffic is mirrored.
- [ ] Limit except restricts methods within a location.
- [ ] Stub status page shows real-time connection metrics.
- [ ] >90% test coverage.

## 5. Out of Scope

- Complex scripting / Lua-like logic (Wasm plugin system handles this).
- Request transformation beyond header/variable manipulation.
