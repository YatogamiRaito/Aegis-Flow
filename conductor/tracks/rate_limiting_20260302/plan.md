# Implementation Plan: Rate Limiting & Security (v0.19.0)

## Phase 1: Token Bucket Rate Limiter

- [ ] Task: Create rate limiting module (`crates/proxy/src/rate_limit.rs`)
    - [ ] Write tests for TokenBucket struct (capacity, refill_rate, available_tokens)
    - [ ] Implement TokenBucket with atomic operations and timestamp-based refill
    - [ ] Write tests for request allow/deny based on available tokens
    - [ ] Write tests for burst handling (queue up to burst size)
    - [ ] Write tests for nodelay mode (immediately serve burst requests)
    - [ ] Implement nodelay flag

- [ ] Task: Implement rate limit zone registry
    - [ ] Write tests for RateLimitZone (name, key_type, rate, burst, nodelay)
    - [ ] Implement zone definition and config parsing
    - [ ] Write tests for per-key bucket management (HashMap<String, TokenBucket>)
    - [ ] Implement key extraction ($remote_addr, $server_name, custom header)
    - [ ] Write tests for bucket eviction (LRU eviction for stale keys after TTL)
    - [ ] Implement background eviction task

- [ ] Task: Implement 429 response with Retry-After header
    - [ ] Write tests for 429 response generation
    - [ ] Implement Retry-After calculation based on token refill time

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Connection Limiting & Bandwidth Throttling

- [ ] Task: Implement connection limiter (`crates/proxy/src/conn_limit.rs`)
    - [ ] Write tests for per-key concurrent connection counting
    - [ ] Implement connection counter with AtomicU64 per key
    - [ ] Write tests for max_connections enforcement (503 when exceeded)
    - [ ] Implement connection guard that decrements on drop

- [ ] Task: Implement bandwidth limiter
    - [ ] Write tests for limit_rate (bytes/sec throttling)
    - [ ] Implement ThrottledWriter that sleeps to maintain target rate
    - [ ] Write tests for limit_rate_after (free burst before throttling)
    - [ ] Implement burst-then-throttle logic

- [ ] Task: Implement request size and timeout limits
    - [ ] Write tests for client_max_body_size (413 Payload Too Large)
    - [ ] Implement body size checking middleware
    - [ ] Write tests for client_header_timeout enforcement
    - [ ] Write tests for client_body_timeout enforcement
    - [ ] Implement timeout wrappers on request reading

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: IP Access Control Lists

- [ ] Task: Implement IP ACL engine (`crates/proxy/src/acl.rs`)
    - [ ] Write tests for CIDR parsing (IPv4: 10.0.0.0/8, IPv6: 2001:db8::/32)
    - [ ] Implement CIDR parser using ipnetwork crate
    - [ ] Write tests for allow/deny rule evaluation (first match wins)
    - [ ] Implement AclRule evaluation chain
    - [ ] Write tests for "all" keyword (match any IP)
    - [ ] Write tests for single IP match
    - [ ] Write tests for implicit deny when rules exist but none match

- [ ] Task: Implement IP list file loading
    - [ ] Write tests for loading IP list from external file
    - [ ] Implement file-based ACL loading with hot-reload support
    - [ ] Write tests for GeoIP country lookup (mock MaxMind DB)
    - [ ] Implement optional GeoIP integration with maxminddb crate

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: HTTP Authentication

- [ ] Task: Implement HTTP Basic Authentication (`crates/proxy/src/auth.rs`)
    - [ ] Write tests for Basic auth header parsing (base64 decode)
    - [ ] Implement auth header extraction
    - [ ] Write tests for htpasswd file parsing (user:hash format)
    - [ ] Implement htpasswd file loader
    - [ ] Write tests for bcrypt password verification
    - [ ] Implement bcrypt verification using bcrypt crate
    - [ ] Write tests for 401 response with WWW-Authenticate header
    - [ ] Write tests for satisfy=any (ACL OR auth passes) vs satisfy=all (both required)

- [ ] Task: Implement JWT Token Authentication
    - [ ] Write tests for JWT extraction from Authorization: Bearer header
    - [ ] Implement JWT parsing using jsonwebtoken crate
    - [ ] Write tests for signature verification (RS256, ES256, HS256)
    - [ ] Write tests for claims validation (exp, iss, aud)
    - [ ] Implement JWKS endpoint fetching with TTL cache
    - [ ] Write tests for expired token rejection
    - [ ] Write tests for invalid signature rejection

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Web Application Firewall (WAF)

- [ ] Task: Implement WAF rule engine (`crates/proxy/src/waf.rs`)
    - [ ] Write tests for SQL injection detection (UNION SELECT, OR 1=1, DROP TABLE, etc.)
    - [ ] Implement SQLi regex patterns
    - [ ] Write tests for XSS detection (<script>, javascript:, onerror=, onload=, etc.)
    - [ ] Implement XSS regex patterns
    - [ ] Write tests for path traversal detection (../, ..%2f, ....// etc.)
    - [ ] Implement traversal patterns
    - [ ] Write tests for command injection detection (;, |, $(), backticks)
    - [ ] Implement command injection patterns

- [ ] Task: Implement WAF action modes
    - [ ] Write tests for "block" mode (reject with 403)
    - [ ] Write tests for "log_only" mode (allow but log match)
    - [ ] Implement mode switching
    - [ ] Write tests for custom rule definitions (user-provided regex + severity)
    - [ ] Implement custom rule loading from config

- [ ] Task: Integrate all security middleware into request pipeline
    - [ ] Write tests for middleware chain order: ACL → Rate Limit → Auth → WAF → Handler
    - [ ] Implement security middleware stack as Tower layers
    - [ ] Write tests for bypass configuration (specific paths excluded from WAF)

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
