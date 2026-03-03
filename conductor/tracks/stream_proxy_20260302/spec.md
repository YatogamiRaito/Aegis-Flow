# Track Specification: WebSocket, TCP/UDP Stream Proxy & Protocol Support (v0.22.0)

## 1. Overview

This track adds **Layer 4 (TCP/UDP) stream proxying**, **WebSocket proxy support**, **PROXY Protocol v1/v2**, and **FastCGI/SCGI backend protocol** capabilities to Aegis-Flow. This completes the nginx feature parity by enabling Aegis-Flow to handle non-HTTP traffic (databases, game servers, DNS, mail relays) and real-time bidirectional communication (WebSocket, Server-Sent Events).

## 2. Functional Requirements

### 2.1 WebSocket Proxy
- Transparent WebSocket proxying via HTTP/1.1 Upgrade mechanism.
- Automatic detection of `Upgrade: websocket` and `Connection: upgrade` headers.
- Bidirectional frame forwarding between client and upstream.
- Support for `wss://` (WebSocket over TLS) with TLS termination at proxy.
- Configurable timeouts:
  - `proxy_read_timeout` applies to WebSocket idle timeout (how long to keep an idle WS connection open, default: 60s).
  - `proxy_send_timeout` applies to frame send timeout.
- Per-location WebSocket enable: `websocket = true` in location block.
- Automatic injection of `X-Forwarded-For`, `X-Real-IP` headers before upgrade.
- Support ping/pong keepalive frames (configurable interval, default: 30s).
- Connection count metrics: `aegis_websocket_connections_active`, `aegis_websocket_messages_total`.

### 2.2 Server-Sent Events (SSE) Proxy
- Support for `text/event-stream` content type passthrough.
- Disable response buffering for SSE connections (`proxy_buffering = off`).
- Long-lived connection handling without idle timeouts.
- `X-Accel-Buffering: no` header support.

### 2.3 TCP Stream Proxy (L4)
- Raw TCP proxying between client and upstream (no HTTP parsing).
- Use case: database proxying (PostgreSQL, MySQL, Redis), custom TCP protocols.
- Configurable `[[stream]]` blocks (separate from `[[server]]` HTTP blocks).
- Support for:
  - TLS termination on TCP streams (STARTTLS, direct TLS).
  - TLS passthrough (SNI-based routing without decryption).
  - Access control (IP allow/deny on stream level).
  - Connection limits per stream.
- Load balancing across multiple upstream TCP servers (round-robin, least-conn).
- Health checks for TCP upstreams (TCP connect check, or optional send/expect probe).

### 2.4 UDP Stream Proxy (L4)
- UDP datagram proxying.
- Use case: DNS proxying, game server proxying, VoIP/SIP.
- Session affinity: map client IP:port to upstream for bidirectional UDP.
- Configurable session timeout (default: 30s — how long to keep UDP session mapping after last packet).
- Configurable `responses` parameter: number of expected response datagrams (default: 1).
- Maximum datagram size configuration (default: 4096 bytes).

### 2.5 PROXY Protocol (v1 & v2)
- **Receiving:** Accept incoming connections with PROXY Protocol header.
  - Parse PROXY Protocol v1 (text-based) and v2 (binary) headers.
  - Extract real client IP/port and make available as `$proxy_protocol_addr` and `$proxy_protocol_port`.
  - Configurable per-listener: `proxy_protocol = true`.
  - Trusted proxy list: only accept PROXY Protocol from configured CIDRs.
- **Sending:** Prepend PROXY Protocol header when connecting to upstream.
  - `proxy_protocol = "send"` in upstream configuration.
  - Support v1 and v2 sending.

### 2.6 FastCGI Proxy
- Forward requests to FastCGI backends (e.g., PHP-FPM).
- `fastcgi_pass = "unix:/run/php-fpm/php-fpm.sock"` or `fastcgi_pass = "127.0.0.1:9000"`.
- FastCGI parameter injection:
  - `SCRIPT_FILENAME`, `SCRIPT_NAME`, `REQUEST_URI`, `DOCUMENT_ROOT`, `QUERY_STRING`, `REQUEST_METHOD`, `CONTENT_TYPE`, `CONTENT_LENGTH`, `SERVER_NAME`, `SERVER_PORT`, `SERVER_PROTOCOL`, `REMOTE_ADDR`, `REMOTE_PORT`.
- `fastcgi_index`: default script name for directory requests (e.g., `index.php`).
- `fastcgi_params`: include file for common parameter definitions.
- Support for `PATH_INFO` extraction from URI.

### 2.7 SCGI Proxy
- Support SCGI protocol for backend communication.
- `scgi_pass` directive.
- Simpler than FastCGI (netstring-based header encoding).

### 2.8 gRPC Proxy
- Transparent gRPC proxying over HTTP/2.
- `grpc_pass = "grpc://backend:50051"` directive.
- Support for gRPC-Web (HTTP/1.1 encapsulation of gRPC).
- Trailer header forwarding (required by gRPC).
- gRPC health check support (grpc.health.v1.Health/Check).

### 2.9 Configuration Example
```toml
# WebSocket location
[[server.location]]
path = "/ws"
proxy_pass = "http://websocket-backend:8080"
websocket = true
proxy_read_timeout = "3600s"

# SSE location
[[server.location]]
path = "/events"
proxy_pass = "http://sse-backend:8080"
proxy_buffering = false

# TCP Stream block
[[stream]]
listen = "0.0.0.0:5432"
proxy_pass = "postgresql-pool:5432"
proxy_protocol = true

  [[stream.server]]
  addr = "10.0.0.1:5432"
  weight = 5

  [[stream.server]]
  addr = "10.0.0.2:5432"
  weight = 3

  [stream.health_check]
  type = "tcp_connect"
  interval = "5s"

# UDP Stream block
[[stream]]
listen = "0.0.0.0:53"
protocol = "udp"
proxy_pass = "dns-upstream:53"
proxy_responses = 1
proxy_timeout = "10s"

# FastCGI location
[[server.location]]
path = "~ \\.php$"
fastcgi_pass = "unix:/run/php-fpm/php-fpm.sock"
fastcgi_index = "index.php"
```

## 3. Non-Functional Requirements

### 3.1 Performance
- WebSocket: < 100µs per frame forwarding latency.
- TCP stream: near kernel-level throughput using splice(2) or zero-copy where possible.
- UDP: handle 100k+ packets/sec per stream.
- PROXY Protocol parsing: < 1µs overhead per connection.

### 3.2 Reliability
- WebSocket connections survive proxy config reloads (don't disconnect active WS).
- TCP stream connections are gracefully drained on shutdown.
- UDP session table limits prevent memory exhaustion.

### 3.3 Security
- TLS passthrough (SNI routing) never exposes plaintext to the proxy.
- PROXY Protocol only accepted from trusted sources.
- FastCGI: sanitize parameters to prevent header injection.

## 4. Acceptance Criteria

- [x] WebSocket upgrade detected and bidirectional frame forwarding works.
- [x] `wss://` connections are TLS-terminated and forwarded correctly.
- [x] WebSocket ping/pong keepalive frames are handled.
- [x] SSE connections are proxied without buffering.
- [x] TCP stream proxy forwards raw TCP between client and upstream.
- [x] TCP stream supports TLS termination and TLS passthrough (SNI routing).
- [x] TCP stream load balancing across multiple backends works.
- [x] UDP stream proxy handles request-response datagram flows.
- [x] UDP session affinity maps client to consistent upstream.
- [x] PROXY Protocol v1 and v2 headers are correctly parsed.
- [x] PROXY Protocol sending prepends correct header to upstream connections.
- [x] FastCGI proxy communicates with PHP-FPM backends.
- [x] gRPC proxy forwards HTTP/2 gRPC traffic with trailers.
- [x] >90% test coverage.

## 5. Out of Scope

- SMTP/IMAP protocol-aware proxying (mail proxy).
- HTTP/2 server push.
- QUIC-based stream multiplexing (already covered by HTTP/3 track).
