# Track Specification: Virtual Hosts & Routing Engine (v0.17.0)

## 1. Overview

This track implements nginx-style **virtual hosts (server blocks)** and a powerful **routing engine** with location matching, URL rewriting, and redirection. This allows a single Aegis-Flow instance to serve multiple domains/applications, each with its own routing rules, TLS certificates, and backend configurations.

## 2. Functional Requirements

### 2.1 Server Blocks (Virtual Hosts)
- Multiple `[[server]]` blocks in configuration, each representing a virtual host.
- Matching by `server_name` (domain): exact match, wildcard (`*.example.com`), and regex.
- Matching by `listen` address and port (e.g., `0.0.0.0:443`, `[::]:80`).
- Default server: the first server block (or explicitly marked `default_server`) handles unmatched requests.
- SNI-based TLS certificate selection: each server block can have its own `ssl_certificate` and `ssl_certificate_key`.

### 2.2 Location Blocks (URL Routing)
- Nested `[[server.location]]` blocks for path-based routing within a server.
- **Match types:**
  - **Exact match:** `= /api/health` — highest priority.
  - **Prefix match:** `/api/` — matches paths starting with prefix.
  - **Regex match:** `~ ^/api/v[0-9]+/` (case-sensitive) and `~* ^/api/v[0-9]+/` (case-insensitive).
  - **Longest prefix match:** `^~ /static/` — prevents regex evaluation for matched prefix.
- **Priority order:** Exact > Longest prefix with `^~` > Regex (first match) > Prefix (longest).
- Each location can have its own `proxy_pass`, `root`, `try_files`, `return`, or `rewrite` directives.

### 2.3 URL Rewriting
- **`rewrite` directive:** Regex-based URL transformation.
  - `rewrite = { pattern = "^/old/(.*)", replacement = "/new/$1", flag = "last" }`
  - Flags: `last` (re-evaluate), `break` (stop), `redirect` (302), `permanent` (301).
- **`return` directive:** Direct response with status code and optional body/URL.
  - `return = { code = 301, url = "https://example.com$request_uri" }` — redirect.
  - `return = { code = 200, body = "OK" }` — direct response.
- Variables in rewrite/return: `$uri`, `$args`, `$host`, `$request_uri`, `$scheme`, `$remote_addr`, `$server_name`.

### 2.4 Header Manipulation
- `proxy_set_header`: Set headers sent to upstream (e.g., `Host`, `X-Real-IP`, `X-Forwarded-For`, `X-Forwarded-Proto`).
- `add_header`: Add headers to the response sent to the client.
- `proxy_hide_header`: Remove specific headers from upstream response.
- Security headers preset:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
  - `Content-Security-Policy` (configurable)
  - `Referrer-Policy: strict-origin-when-cross-origin`

### 2.5 HTTP-to-HTTPS Redirect
- Per-server block redirect: `return = { code = 301, url = "https://$host$request_uri" }` on port 80 server.
- Global option: `force_https = true`.

### 2.6 Configuration Example
```toml
[[server]]
server_name = ["example.com", "www.example.com"]
listen = "0.0.0.0:443"
ssl_certificate = "/etc/certs/example.com.pem"
ssl_certificate_key = "/etc/certs/example.com-key.pem"

  [[server.location]]
  path = "= /health"
  return = { code = 200, body = '{"status":"ok"}' }

  [[server.location]]
  path = "/api/"
  proxy_pass = "http://api-backend:3000"
  proxy_set_header = { "X-Real-IP" = "$remote_addr", "X-Forwarded-For" = "$remote_addr" }

  [[server.location]]
  path = "^~ /static/"
  root = "/var/www/static"
  cache_control = "public, max-age=31536000, immutable"

  [[server.location]]
  path = "/"
  root = "/var/www/html"
  try_files = ["$uri", "$uri/", "/index.html"]

[[server]]
server_name = ["_"]  # default server
listen = "0.0.0.0:80"
return = { code = 301, url = "https://$host$request_uri" }
```

## 3. Non-Functional Requirements

### 3.1 Performance
- Server block lookup: O(1) for exact match, O(n) worst case for regex (with cached compiled regexes).
- Location matching: < 1µs for typical configurations with < 50 locations.
- Compiled regex caching to avoid re-compilation per request.

### 3.2 Reliability
- Invalid configuration MUST be rejected at startup with clear error messages indicating line/block.
- Configuration hot-reload MUST validate new config before applying (no downtime on bad config).

## 4. Acceptance Criteria

- [x] Multiple server blocks serve different domains from a single instance.
- [x] SNI-based TLS certificate selection works per domain.
- [x] Location matching follows correct priority order (exact > ^~ prefix > regex > prefix).
- [x] `rewrite` directive transforms URLs with regex capture groups.
- [x] `return` directive serves static responses and redirects.
- [x] Header manipulation (proxy_set_header, add_header, proxy_hide_header) works.
- [x] HTTP→HTTPS redirect works via return directive and force_https option.
- [x] Variables ($uri, $host, $remote_addr, etc.) are correctly resolved.
- [x] Config validation rejects invalid configurations with descriptive errors.
- [x] >90% test coverage.

## 5. Out of Scope

- Dynamic configuration via API (future track).
- Lua/scripting in location blocks (Wasm plugin system covers this).
