# Track Specification: Aegisfile — Simple Configuration Format (v0.24.0)

## 1. Overview

This track creates **Aegisfile**, a human-friendly, minimal configuration format for Aegis-Flow — inspired by Caddyfile's simplicity. While TOML/YAML remain available for power users, Aegisfile provides a zero-learning-curve experience for beginners. A single line should be enough to get a production reverse proxy running with automatic HTTPS.

## 2. Functional Requirements

### 2.1 Aegisfile Syntax
- Block-based, indentation-free syntax using `{ }` braces.
- Domain names as top-level block identifiers (implicit HTTPS).
- Directives are single-line keywords with arguments.
- Comments with `#`.

### 2.2 Basic Examples

**Simplest reverse proxy:**
```
example.com {
    reverse_proxy localhost:3000
}
```

**Static file server:**
```
static.example.com {
    root /var/www/html
    file_server
}
```

**Multi-site with features:**
```
# API with rate limiting and auth
api.example.com {
    reverse_proxy /api/* http://api-backend:3000
    reverse_proxy /ws    http://ws-backend:8080 {
        websocket
    }
    rate_limit 100r/s
    jwt_auth {
        jwks_url https://auth.example.com/.well-known/jwks.json
    }
    log /var/log/aegis/api-access.log
}

# Static site with compression
www.example.com {
    root /var/www/html
    file_server {
        gzip
        brotli
    }
    header {
        X-Frame-Options DENY
        Strict-Transport-Security "max-age=31536000"
    }
}

# Catch-all redirect
:80 {
    redirect https://{host}{uri} permanent
}
```

**Process management (PM2 equivalent):**
```
apps {
    process api-server {
        command ./target/release/api
        instances max
        env NODE_ENV=production
        max_memory 512M
    }
    process worker {
        command ./target/release/worker
        instances 2
    }
}
```

### 2.3 Supported Directives

| Directive | Arguments | Description |
|---|---|---|
| `reverse_proxy` | `[matcher] <upstream>` | Proxy to upstream backend |
| `root` | `<path>` | Set document root for static files |
| `file_server` | `[browse]` | Enable static file serving |
| `redirect` | `<url> [code]` | HTTP redirect |
| `rewrite` | `<from> <to>` | URL rewriting |
| `header` | `<name> <value>` | Add/set response header |
| `rate_limit` | `<rate>` | Apply rate limiting |
| `basicauth` | `<realm> { users }` | HTTP basic authentication |
| `jwt_auth` | `{ config }` | JWT token validation |
| `log` | `<path> [format]` | Access logging |
| `tls` | `<cert> <key>` or `internal` | Manual TLS config |
| `encode` | `gzip brotli` | Compression |
| `handle_path` | `<path> { ... }` | Strip path prefix and handle |
| `respond` | `<body> [code]` | Static response |
| `import` | `<file>` | Include another Aegisfile |
| `process` | `<name> { config }` | Managed process definition |

### 2.4 Matchers
- Path matcher: `/api/*`, `*.php`
- Method matcher: `@method GET`
- Header matcher: `@header Content-Type application/json`
- Remote IP matcher: `@remote_ip 10.0.0.0/8`
- Named matchers: `@api { path /api/* }`

### 2.5 Config Conversion
- **`aegis adapt --from aegisfile`:** Convert Aegisfile to TOML/YAML format.
- **`aegis import --from nginx`:** Import and convert nginx.conf to Aegisfile.
- **`aegis import --from caddyfile`:** Import Caddyfile format.
- **`aegis fmt`:** Auto-format Aegisfile (consistent indentation and ordering).
- **`aegis validate`:** Validate Aegisfile syntax without starting server.

### 2.6 Configuration Priority
1. CLI flags (highest priority).
2. Environment variables.
3. Aegisfile (if present).
4. TOML/YAML config file.
5. Default values.

## 3. Non-Functional Requirements

- Parse time: < 1ms for typical Aegisfile (< 500 lines).
- Error messages: human-readable with line number, column, and suggestion.
- Syntax highlighting: provide TextMate/VS Code grammar file.

## 4. Acceptance Criteria

- [x] Single-line `reverse_proxy` config works.
- [x] Domain names auto-trigger HTTPS.
- [x] `file_server` serves static files from `root`.
- [x] Nested directives with `{ }` blocks work.
- [x] Named matchers (`@api`) route correctly.
- [x] `aegis adapt` converts Aegisfile to TOML.
- [x] `aegis import --from nginx` converts basic nginx.conf.
- [x] `aegis fmt` formats Aegisfile consistently.
- [x] `aegis validate` catches syntax errors with helpful messages.
- [x] Process management blocks integrate with process manager.
- [x] >90% test coverage on parser.

## 5. Out of Scope

- Visual config editor.
- Config generation from GUI.
