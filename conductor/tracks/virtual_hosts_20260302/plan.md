# Implementation Plan: Virtual Hosts & Routing Engine (v0.17.0)

## Phase 1: Server Block Data Model & Parsing

- [x] Task: Define server block configuration model (`crates/proxy/src/vhost.rs`)
    - [x] Write tests for ServerBlock struct (server_name, listen, ssl_cert, ssl_key, locations)
    - [x] Implement ServerBlock with serde deserialization
    - [x] Write tests for server_name patterns: exact, wildcard (*.example.com, example.*), regex
    - [x] Implement ServerNameMatcher with wildcard and regex support
    - [x] Write tests for multi-server config parsing from TOML
    - [x] Implement config loading with validation

- [x] Task: Implement server block selection logic
    - [x] Write tests for exact server_name match
    - [x] Write tests for wildcard server_name match (leading *.example.com)
    - [x] Write tests for wildcard server_name match (trailing example.*)
    - [x] Write tests for regex server_name match
    - [x] Write tests for default_server fallback
    - [x] Implement select_server() following nginx priority: exact > leading wildcard > trailing wildcard > regex > default
    - [x] Write tests for SNI extraction from TLS ClientHello

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)


## Phase 2: Location Block Matching

- [x] Task: Define location block model (`crates/proxy/src/location.rs`)
    - [x] Write tests for LocationBlock struct (path, match_type, proxy_pass, root, try_files, return, rewrite)
    - [x] Implement LocationBlock with serde deserialization
    - [x] Write tests for LocationMatchType enum (Exact, Prefix, PreferredPrefix, Regex, RegexCaseInsensitive)
    - [x] Implement LocationMatchType parsing from path prefix (=, ^~, ~, ~*)

- [x] Task: Implement location matching engine
    - [x] Write tests for exact match (= /path)
    - [x] Write tests for prefix match (/api/)
    - [x] Write tests for preferred prefix (^~ /static/)
    - [x] Write tests for regex match (~ pattern)
    - [x] Write tests for case-insensitive regex (~* pattern)
    - [x] Write tests for priority ordering: exact > ^~ > regex (first match) > longest prefix
    - [x] Implement match_location() with correct priority algorithm
    - [x] Write tests for regex compilation and caching
    - [x] Implement compiled regex LRU cache

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)


## Phase 3: URL Rewriting & Redirects

- [x] Task: Implement variable resolver (`crates/proxy/src/variables.rs`)
    - [x] Write tests for $uri, $args, $host, $request_uri, $scheme resolution from request
    - [x] Implement VariableResolver that extracts variables from request context
    - [x] Write tests for $remote_addr, $server_name, $server_port
    - [x] Implement connection-level variable injection
    - [x] Write tests for variable interpolation in strings (e.g., "https://$host$request_uri")
    - [x] Implement interpolate() that replaces $variables in template strings

- [x] Task: Implement rewrite directive
    - [x] Write tests for regex pattern matching and capture group replacement
    - [x] Implement rewrite with regex::Regex and capture group substitution
    - [x] Write tests for `last` flag (re-evaluate location matching)
    - [x] Write tests for `break` flag (stop processing)
    - [x] Write tests for `redirect` flag (302 temporary redirect)
    - [x] Write tests for `permanent` flag (301 permanent redirect)
    - [x] Implement rewrite processing loop with flag handling

- [x] Task: Implement return directive
    - [x] Write tests for return with status code + body (e.g., 200 "OK")
    - [x] Write tests for return with redirect URL (301/302 + Location header)
    - [x] Write tests for variable interpolation in return values
    - [x] Implement return_response() builder

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)


## Phase 4: Header Manipulation

- [x] Task: Implement proxy_set_header
    - [x] Write tests for setting X-Real-IP, X-Forwarded-For, X-Forwarded-Proto
    - [x] Implement header injection before forwarding to upstream
    - [x] Write tests for variable interpolation in header values
    - [x] Write tests for Host header override

- [x] Task: Implement add_header (response headers)
    - [x] Write tests for adding custom response headers
    - [x] Implement response header injection
    - [x] Write tests for security headers preset (HSTS, X-Frame-Options, CSP, etc.)
    - [x] Implement security headers preset with enabling flag

- [x] Task: Implement proxy_hide_header
    - [x] Write tests for removing specific upstream response headers
    - [x] Implement header filtering on upstream responses

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Integration & SNI-Based TLS

- [x] Task: Implement SNI-based certificate selection
    - [x] Write tests for loading multiple TLS certificate/key pairs
    - [x] Implement multi-cert TLS acceptor using rustls ServerConfig with certificate resolver
    - [x] Write tests for SNI-to-server-block mapping
    - [x] Implement custom ResolvesServerCert trait for SNI dispatch

- [x] Task: Integrate vhost routing into main request handler
    - [x] Write tests for full request flow: TLS → SNI → server block → location → handler
    - [x] Implement request routing pipeline in http_proxy.rs
    - [x] Write tests for HTTP→HTTPS redirect (port 80 → 443)
    - [x] Implement force_https global option

- [x] Task: Implement configuration validation
    - [x] Write tests for overlapping server_name detection
    - [x] Write tests for invalid regex patterns in location blocks
    - [x] Write tests for missing TLS certificates for HTTPS listeners
    - [x] Implement comprehensive config validator with descriptive error messages

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
