# Implementation Plan: Protocol & Stream Proxy Hardening (v0.47.0)

## Phase 1: Configuration Binding
- [ ] Task: Expand Configuration structures
    - [ ] Update `crates/proxy/src/config.rs` `LocationBlock` with protocol pass variants (fastcgi, scgi, grpc) and WebSocket boolean.
    - [ ] Add `proxy_protocol` boolean to `ServerBlock`.
    - [ ] Reconfigure `proxy_pass` to be optional (so `fastcgi_pass` can be mutual exclusive).

## Phase 2: TCP and UDP Listeners
- [ ] Task: Bootstrap Stream Listeners
    - [ ] Update `crates/proxy/src/bootstrap.rs` to process `config.streams`.
    - [ ] Spawn identical async network bound loops for TCP and UDP that forward directly to the respective proxy logic modules.

## Phase 3: WebSocket Upgrade Integration
- [ ] Task: Intercept UPGRADE connections
    - [ ] Inside `HttpProxy::handle_request` (`crates/proxy/src/http_proxy.rs`), evaluate HTTP headers.
    - [ ] If `Upgrade: websocket`, invoke `websocket::is_websocket_upgrade`.
    - [ ] Generate the `101 Switching Protocols` response, call `hyper::upgrade::on()`, and hand off the duplex stream to `websocket::forward_frames`.

## Phase 4: FastCGI and RPC Interpreters
- [ ] Task: Branch Request Handlers in Proxy
    - [ ] Modify `HttpProxy` location matching block.
    - [ ] If `fastcgi_pass` is defined, translate the HTTP Req/Res objects over the existing `fastcgi` Client module.
    - [ ] Do the same for `grpc_pass` (extract trailers) and `scgi_pass`.

## Phase 5: PROXY Protocol Reception
- [ ] Task: Inspect Bytes at Listener Entry
    - [ ] In `bootstrap.rs`, wrap the HTTP/Stream TcpStreams in a `Peekable` or custom buffer layer.
    - [ ] Extract the PROXY v1/v2 header, update the connection origin metadata, and pass the clean stream downstream to Hyper/StreamHandlers.
    - [ ] Testing protocol in `workflow.md`.
