# Track Specification: Protocol & Stream Proxy Hardening (v0.47.0)

## 1. Overview
The codebase currently contains isolated modules for WebSocket proxying, TCP/UDP stream proxying, FastCGI, SCGI, gRPC, and PROXY Protocol v1/v2. However, these powerful features are completely disconnected from the actual Aegis-Flow proxy lifecycle (`bootstrap.rs` and `http_proxy.rs`). This track resolves the disconnect by injecting stream listeners and HTTP upgrade mechanisms into the active runtime.

## 2. Functional Requirements

### 2.1 Configuration Integrations
- Extend `LocationBlock` in `crates/proxy/src/config.rs`:
  - `websocket: bool`
  - `fastcgi_pass: Option<String>`
  - `grpc_pass: Option<String>`
  - `scgi_pass: Option<String>`
- Extend `ServerBlock` and `StreamConfig` with `proxy_protocol: bool`.

### 2.2 TCP and UDP Stream Listeners
- Modify `crates/proxy/src/bootstrap.rs` to loop through `config.streams`.
- If the stream is `Tcp`, bind `tokio::net::TcpListener` and spawn a task running `crates::proxy::stream_proxy::StreamProxy`.
- If the stream is `Udp`, bind `tokio::net::UdpSocket` and spawn a task running `crates::proxy::udp_proxy::UdpProxy`.

### 2.3 WebSocket Upgrades
- Intercept HTTP requests in `crates/proxy/src/http_proxy.rs` inside `handle_request()`.
- If the request has `Upgrade: websocket` and the location has `websocket = true`, immediately branch to `crates::proxy::websocket::handle_upgrade`.
- Detach the websocket frame forwarder from the HTTP request-response cycle.

### 2.4 FastCGI, SCGI, and gRPC Routing
- If a route matches a location with `fastcgi_pass`, parse the HTTP request headers and body, encode them into FastCGI records (`crates::proxy::fastcgi.rs`), and forward them to the backend socket.
- Similarly, route traffic via `scgi_pass` or `grpc_pass` using the respective existing modules.

### 2.5 PROXY Protocol Processing
- If `proxy_protocol = true` on the listener, peek the first 107 bytes.
- Pass the buffer to `proxy_protocol::parse_v1_or_v2`.
- Extract the real client IP and make it available to the downstream HTTP logic (`$proxy_protocol_addr`).

## 3. Non-Functional Requirements
- **Performance:** Bypassing HTTP parsing for raw streams (TCP/UDP) must use zero-copy operations (`tokio::io::copy_bidirectional`) to sustain maximal throughput.
- **Graceful degradation:** If a websocket upgrade fails, return a `400 Bad Request` gracefully instead of panic.

## 4. Acceptance Criteria
- [ ] Specifying `[[stream]]` in `aegis.toml` opens a working TCP or UDP port natively routing traffic.
- [ ] WebSocket echo clients can connect through the `http_proxy` block.
- [ ] Standard PHP-FPM servers can receive valid HTTP requests translated to FastCGI format over unix sockets.
- [ ] Nginx-like `proxy_protocol` tests successfully parse and inject real IP addresses.
