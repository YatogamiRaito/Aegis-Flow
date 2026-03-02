# Implementation Plan: Virtual Hosts & Routing Engine (v0.17.0)

## Phase 1: Server Block Data Model & Parsing

- [ ] Task: Define server block configuration model (`crates/proxy/src/vhost.rs`)
    - [ ] Write tests for ServerBlock struct (server_name, listen, ssl_cert, ssl_key, locations)
    - [ ] Implement ServerBlock with serde deserialization
    - [ ] Write tests for server_name patterns: exact, wildcard (*.example.com, example.*), regex
    - [ ] Implement ServerNameMatcher with wildcard and regex support
    - [ ] Write tests for multi-server config parsing from TOML
    - [ ] Implement config loading with validation

- [ ] Task: Implement server block selection logic
    - [ ] Write tests for exact server_name match
    - [ ] Write tests for wildcard server_name match (leading *.example.com)
    - [ ] Write tests for wildcard server_name match (trailing example.*)
    - [ ] Write tests for regex server_name match
    - [ ] Write tests for default_server fallback
    - [ ] Implement select_server() following nginx priority: exact > leading wildcard > trailing wildcard > regex > default
    - [ ] Write tests for SNI extraction from TLS ClientHello

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Location Block Matching

- [ ] Task: Define location block model (`crates/proxy/src/location.rs`)
    - [ ] Write tests for LocationBlock struct (path, match_type, proxy_pass, root, try_files, return, rewrite)
    - [ ] Implement LocationBlock with serde deserialization
    - [ ] Write tests for LocationMatchType enum (Exact, Prefix, PreferredPrefix, Regex, RegexCaseInsensitive)
    - [ ] Implement LocationMatchType parsing from path prefix (=, ^~, ~, ~*)

- [ ] Task: Implement location matching engine
    - [ ] Write tests for exact match (= /path)
    - [ ] Write tests for prefix match (/api/)
    - [ ] Write tests for preferred prefix (^~ /static/)
    - [ ] Write tests for regex match (~ pattern)
    - [ ] Write tests for case-insensitive regex (~* pattern)
    - [ ] Write tests for priority ordering: exact > ^~ > regex (first match) > longest prefix
    - [ ] Implement match_location() with correct priority algorithm
    - [ ] Write tests for regex compilation and caching
    - [ ] Implement compiled regex LRU cache

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: URL Rewriting & Redirects

- [ ] Task: Implement variable resolver (`crates/proxy/src/variables.rs`)
    - [ ] Write tests for $uri, $args, $host, $request_uri, $scheme resolution from request
    - [ ] Implement VariableResolver that extracts variables from request context
    - [ ] Write tests for $remote_addr, $server_name, $server_port
    - [ ] Implement connection-level variable injection
    - [ ] Write tests for variable interpolation in strings (e.g., "https://$host$request_uri")
    - [ ] Implement interpolate() that replaces $variables in template strings

- [ ] Task: Implement rewrite directive
    - [ ] Write tests for regex pattern matching and capture group replacement
    - [ ] Implement rewrite with regex::Regex and capture group substitution
    - [ ] Write tests for `last` flag (re-evaluate location matching)
    - [ ] Write tests for `break` flag (stop processing)
    - [ ] Write tests for `redirect` flag (302 temporary redirect)
    - [ ] Write tests for `permanent` flag (301 permanent redirect)
    - [ ] Implement rewrite processing loop with flag handling

- [ ] Task: Implement return directive
    - [ ] Write tests for return with status code + body (e.g., 200 "OK")
    - [ ] Write tests for return with redirect URL (301/302 + Location header)
    - [ ] Write tests for variable interpolation in return values
    - [ ] Implement return_response() builder

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Header Manipulation

- [ ] Task: Implement proxy_set_header
    - [ ] Write tests for setting X-Real-IP, X-Forwarded-For, X-Forwarded-Proto
    - [ ] Implement header injection before forwarding to upstream
    - [ ] Write tests for variable interpolation in header values
    - [ ] Write tests for Host header override

- [ ] Task: Implement add_header (response headers)
    - [ ] Write tests for adding custom response headers
    - [ ] Implement response header injection
    - [ ] Write tests for security headers preset (HSTS, X-Frame-Options, CSP, etc.)
    - [ ] Implement security headers preset with enabling flag

- [ ] Task: Implement proxy_hide_header
    - [ ] Write tests for removing specific upstream response headers
    - [ ] Implement header filtering on upstream responses

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Integration & SNI-Based TLS

- [ ] Task: Implement SNI-based certificate selection
    - [ ] Write tests for loading multiple TLS certificate/key pairs
    - [ ] Implement multi-cert TLS acceptor using rustls ServerConfig with certificate resolver
    - [ ] Write tests for SNI-to-server-block mapping
    - [ ] Implement custom ResolvesServerCert trait for SNI dispatch

- [ ] Task: Integrate vhost routing into main request handler
    - [ ] Write tests for full request flow: TLS → SNI → server block → location → handler
    - [ ] Implement request routing pipeline in http_proxy.rs
    - [ ] Write tests for HTTP→HTTPS redirect (port 80 → 443)
    - [ ] Implement force_https global option

- [ ] Task: Implement configuration validation
    - [ ] Write tests for overlapping server_name detection
    - [ ] Write tests for invalid regex patterns in location blocks
    - [ ] Write tests for missing TLS certificates for HTTPS listeners
    - [ ] Implement comprehensive config validator with descriptive error messages

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
