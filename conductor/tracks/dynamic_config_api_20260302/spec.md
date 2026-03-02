# Track Specification: Dynamic Configuration API (v0.25.0)

## 1. Overview

This track adds a **REST-based admin API** to Aegis-Flow for runtime configuration changes — similar to Caddy's admin API and nginx Plus's dynamic upstream management. Users can add/remove upstreams, modify routes, update rate limits, and manage certificates without restarting or reloading the proxy.

## 2. Functional Requirements

### 2.1 Admin API Server
- Separate HTTP listener on configurable port (default: `localhost:2019`, same as Caddy).
- Only listens on localhost by default (security).
- Optional mTLS authentication for remote access.
- API key authentication option (`X-API-Key` header).
- JSON request/response format.

### 2.2 Configuration CRUD Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/config/` | Get entire running configuration |
| `GET` | `/config/servers` | List all server blocks |
| `GET` | `/config/servers/{id}` | Get specific server block |
| `POST` | `/config/servers` | Add new server block |
| `PUT` | `/config/servers/{id}` | Update server block |
| `DELETE` | `/config/servers/{id}` | Remove server block |
| `GET` | `/config/upstreams` | List all upstream groups |
| `POST` | `/config/upstreams` | Add upstream group |
| `PUT` | `/config/upstreams/{name}` | Update upstream group |
| `DELETE` | `/config/upstreams/{name}` | Remove upstream group |
| `POST` | `/config/upstreams/{name}/servers` | Add server to upstream |
| `DELETE` | `/config/upstreams/{name}/servers/{addr}` | Remove server from upstream |
| `POST` | `/config/reload` | Trigger graceful config reload from file |
| `GET` | `/config/export` | Export current config as TOML/YAML/Aegisfile |

### 2.3 Runtime Status Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/status` | Server status overview |
| `GET` | `/status/connections` | Active connection details |
| `GET` | `/status/upstreams` | Upstream health status per backend |
| `GET` | `/status/certs` | Certificate status (expiry, issuer) |
| `GET` | `/status/cache` | Cache statistics |
| `GET` | `/status/processes` | Managed process status |

### 2.4 Atomic Configuration Updates
- All config changes are **transactional**: validate before applying.
- If validation fails, return 400 with error details; running config unchanged.
- Support batch updates: `POST /config/batch` with array of operations.

### 2.5 Config Versioning
- Each config change increments a version number.
- `GET /config/` returns current version.
- `GET /config/history` returns last N config versions.
- `POST /config/rollback/{version}` rolls back to a previous config.

### 2.6 Configuration
```toml
[admin]
enabled = true
listen = "localhost:2019"
api_key = "${AEGIS_ADMIN_KEY}"   # optional
enforce_origin = true            # reject non-localhost requests without auth
```

## 3. Non-Functional Requirements

- API latency: < 10ms for config reads, < 100ms for config writes.
- Concurrent access: safe for multiple concurrent API clients.
- Config changes applied within 50ms (no connection drops).

## 4. Acceptance Criteria

- [ ] Admin API listens on configured port (localhost only by default).
- [ ] GET /config/ returns full running configuration.
- [ ] POST/PUT/DELETE on server blocks dynamically adds/modifies/removes routes.
- [ ] POST/DELETE on upstream servers dynamically changes backends.
- [ ] Invalid config changes return 400 with descriptive error.
- [ ] Config changes are applied without dropping active connections.
- [ ] API key authentication works when configured.
- [ ] Config versioning tracks changes and supports rollback.
- [ ] Status endpoints return real-time connection and health data.
- [ ] >90% test coverage.

## 5. Out of Scope

- WebSocket-based config streaming.
- Multi-node config synchronization.
