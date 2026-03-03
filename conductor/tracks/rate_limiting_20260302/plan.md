# Implementation Plan: Rate Limiting & Security (v0.19.0)

## Phase 1: Token Bucket Rate Limiter

- [x] Task: Create rate limiting module (`crates/proxy/src/rate_limit.rs`)
    - [x] Write tests for TokenBucket struct (capacity, refill_rate, available_tokens)
    - [x] Implement TokenBucket with atomic operations and timestamp-based refill
    - [x] Write tests for request allow/deny based on available tokens
    - [x] Write tests for burst handling (queue up to burst size)
    - [x] Write tests for nodelay mode (immediately serve burst requests)
    - [x] Implement nodelay flag

- [x] Task: Implement rate limit zone registry
    - [x] Write tests for RateLimitZone (name, key_type, rate, burst, nodelay)
    - [x] Implement zone definition and config parsing
    - [x] Write tests for per-key bucket management (HashMap<String, TokenBucket>)
    - [x] Implement key extraction ($remote_addr, $server_name, custom header)
    - [x] Write tests for bucket eviction (LRU eviction for stale keys after TTL)
    - [x] Implement background eviction task

- [x] Task: Implement 429 response with Retry-After header
    - [x] Write tests for 429 response generation
    - [x] Implement Retry-After calculation based on token refill time

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Connection Limiting & Bandwidth Throttling

- [x] Task: Implement connection limiter (`crates/proxy/src/conn_limit.rs`)
    - [x] Write tests for per-key concurrent connection counting
    - [x] Implement connection counter with AtomicU64 per key
    - [x] Write tests for max_connections enforcement (503 when exceeded)
    - [x] Implement connection guard that decrements on drop

- [x] Task: Implement bandwidth limiter
    - [x] Write tests for limit_rate (bytes/sec throttling)
    - [x] Implement ThrottledWriter that sleeping to maintain target rate
    - [x] Write tests for limit_rate_after (free burst before throttling)
    - [x] Implement burst-then-throttle logic

- [x] Task: Implement request size and timeout limits
    - [x] Write tests for client_max_body_size (413 Payload Too Large)
    - [x] Implement body size checking middleware
    - [x] Write tests for client_header_timeout enforcement
    - [x] Write tests for client_body_timeout enforcement
    - [x] Implement timeout wrappers on request reading

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: IP Access Control Lists

- [x] Task: Implement IP ACL engine (`crates/proxy/src/acl.rs`)
    - [x] Write tests for CIDR parsing (IPv4: 10.0.0.0/8, IPv6: 2001:db8::/32)
    - [x] Implement CIDR parser using ipnetwork crate
    - [x] Write tests for allow/deny rule evaluation (first match wins)
    - [x] Implement AclRule evaluation chain
    - [x] Write tests for "all" keyword (match any IP)
    - [x] Write tests for single IP match
    - [x] Write tests for implicit deny when rules exist but none match

- [x] Task: Implement IP list file loading
    - [x] Write tests for loading IP list from external file
    - [x] Implement file-based ACL loading with hot-reload support
    - [x] Write tests for GeoIP country lookup (mock MaxMind DB)
    - [x] Implement optional GeoIP integration with maxminddb crate

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: HTTP Authentication

- [x] Task: Implement HTTP Basic Authentication (`crates/proxy/src/auth.rs`)
    - [x] Write tests for Basic auth header parsing (base64 decode)
    - [x] Implement auth header extraction
    - [x] Write tests for htpasswd file parsing (user:hash format)
    - [x] Implement htpasswd file loader
    - [x] Write tests for bcrypt password verification
    - [x] Implement bcrypt verification using bcrypt crate
    - [x] Write tests for 401 response with WWW-Authenticate header
    - [x] Write tests for satisfy=any (ACL OR auth passes) vs satisfy=all (both required)

- [x] Task: Implement JWT Token Authentication
    - [x] Write tests for JWT extraction from Authorization: Bearer header
    - [x] Implement JWT parsing using jsonwebtoken crate
    - [x] Write tests for signature verification (RS256, ES256, HS256)
    - [x] Write tests for claims validation (exp, iss, aud)
    - [x] Implement JWKS endpoint fetching with TTL cache
    - [x] Write tests for expired token rejection
    - [x] Write tests for invalid signature rejection

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Web Application Firewall (WAF)

- [x] Task: Implement WAF rule engine (`crates/proxy/src/waf.rs`)
    - [x] Write tests for SQL injection detection (UNION SELECT, OR 1=1, DROP TABLE, etc.)
    - [x] Implement SQLi regex patterns
    - [x] Write tests for XSS detection (<script>, javascript:, onerror=, onload=, etc.)
    - [x] Implement XSS regex patterns
    - [x] Write tests for path traversal detection (../, ..%2f, ....// etc.)
    - [x] Implement traversal patterns
    - [x] Write tests for command injection detection (;, |, $(), backticks)
    - [x] Implement command injection patterns

- [x] Task: Implement WAF action modes
    - [x] Write tests for "block" mode (reject with 403)
    - [x] Write tests for "log_only" mode (allow but log match)
    - [x] Implement mode switching
    - [x] Write tests for custom rule definitions (user-provided regex + severity)
    - [x] Implement custom rule loading from config

- [x] Task: Integrate all security middleware into request pipeline
    - [x] Write tests for middleware chain order: ACL → Rate Limit → Auth → WAF → Handler
    - [x] Implement security middleware stack as Tower layers
    - [x] Write tests for bypass configuration (specific paths excluded from WAF)

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
