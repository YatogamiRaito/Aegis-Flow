# Implementation Plan: WebSocket, TCP/UDP Stream Proxy & Protocol Support (v0.22.0)

## Phase 1: WebSocket Proxy

- [ ] Task: Implement WebSocket upgrade detection (`crates/proxy/src/websocket.rs`)
    - [ ] Write tests for detecting Upgrade: websocket and Connection: upgrade headers
    - [ ] Implement is_websocket_upgrade() check
    - [ ] Write tests for Sec-WebSocket-Key/Accept handshake header generation
    - [ ] Implement WebSocket handshake response builder

- [ ] Task: Implement bidirectional WebSocket frame forwarding
    - [ ] Write tests for client→upstream frame forwarding (text, binary, close)
    - [ ] Write tests for upstream→client frame forwarding (text, binary, close)
    - [ ] Implement bidirectional forwarding using tokio-tungstenite
    - [ ] Write tests for connection close propagation (client close → upstream close and vice versa)
    - [ ] Write tests for error handling (upstream disconnects, client disconnects)

- [ ] Task: Implement WebSocket keepalive and timeouts
    - [ ] Write tests for ping/pong frame generation at configurable interval
    - [ ] Implement keepalive ping task
    - [ ] Write tests for idle timeout (close connection if no frames for N seconds)
    - [ ] Implement idle timeout enforcement
    - [ ] Write tests for proxy_read_timeout / proxy_send_timeout on WS frames

- [ ] Task: Implement WebSocket header injection
    - [ ] Write tests for X-Forwarded-For and X-Real-IP injection before upgrade
    - [ ] Implement header injection in upgrade request forwarding
    - [ ] Write tests for per-location websocket = true config

- [ ] Task: Implement WebSocket metrics
    - [ ] Write tests for aegis_websocket_connections_active gauge
    - [ ] Write tests for aegis_websocket_messages_total counter (by direction)
    - [ ] Implement metric recording

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Server-Sent Events (SSE) Proxy

- [ ] Task: Implement SSE detection and unbuffered proxying
    - [ ] Write tests for text/event-stream content-type detection
    - [ ] Implement SSE passthrough mode (disable response buffering)
    - [ ] Write tests for X-Accel-Buffering: no header support
    - [ ] Write tests for long-lived SSE connection without idle timeout
    - [ ] Implement SSE-specific timeout handling

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: TCP Stream Proxy

- [ ] Task: Create stream proxy module (`crates/proxy/src/stream_proxy.rs`)
    - [ ] Write tests for StreamConfig struct (listen, proxy_pass, protocol, servers, health_check)
    - [ ] Implement StreamConfig with serde deserialization
    - [ ] Write tests for TCP listener binding on configured port
    - [ ] Implement TCP stream listener using tokio::net::TcpListener

- [ ] Task: Implement raw TCP bidirectional forwarding
    - [ ] Write tests for client→upstream and upstream→client byte forwarding
    - [ ] Implement bidirectional copy using tokio::io::copy_bidirectional
    - [ ] Write tests for connection close propagation
    - [ ] Write tests for error handling (upstream unreachable, connection reset)

- [ ] Task: Implement TCP stream TLS handling
    - [ ] Write tests for TLS termination on TCP stream (decrypt at proxy, forward plain)
    - [ ] Implement TLS acceptor for stream connections using rustls
    - [ ] Write tests for TLS passthrough (SNI-based routing without decryption)
    - [ ] Implement SNI extraction from TLS ClientHello without decrypting
    - [ ] Write tests for SNI-to-upstream routing lookup

- [ ] Task: Implement TCP stream load balancing
    - [ ] Write tests for round-robin across TCP upstream servers
    - [ ] Write tests for least-connections for TCP streams
    - [ ] Implement load balancer reuse from upstream module
    - [ ] Write tests for TCP health check (connect probe)
    - [ ] Implement TCP connect health check with configurable interval

- [ ] Task: Implement TCP stream access control
    - [ ] Write tests for IP allow/deny on stream level
    - [ ] Implement ACL reuse from security module
    - [ ] Write tests for connection limit per stream
    - [ ] Implement connection counting with limit

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: UDP Stream Proxy

- [ ] Task: Implement UDP listener and session management (`crates/proxy/src/udp_proxy.rs`)
    - [ ] Write tests for UDP socket binding and datagram receiving
    - [ ] Implement UDP listener using tokio::net::UdpSocket
    - [ ] Write tests for session table (client_addr:port → upstream socket mapping)
    - [ ] Implement UdpSessionTable with HashMap and expiration

- [ ] Task: Implement UDP datagram forwarding
    - [ ] Write tests for client→upstream datagram forwarding
    - [ ] Write tests for upstream→client response datagram forwarding
    - [ ] Implement bidirectional UDP forwarding with per-session sockets
    - [ ] Write tests for proxy_responses parameter (expect N response datagrams)
    - [ ] Implement response counting per session

- [ ] Task: Implement UDP session management
    - [ ] Write tests for session timeout (expire after N seconds of inactivity)
    - [ ] Implement session timeout with background cleanup task
    - [ ] Write tests for max datagram size configuration
    - [ ] Implement datagram size validation and truncation
    - [ ] Write tests for session table size limits (prevent memory exhaustion)
    - [ ] Implement session table max entries with LRU eviction

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: PROXY Protocol

- [ ] Task: Implement PROXY Protocol v1 parser (`crates/proxy/src/proxy_protocol.rs`)
    - [ ] Write tests for parsing text-based v1 header: "PROXY TCP4 src_ip src_port dst_ip dst_port\r\n"
    - [ ] Implement v1 parser
    - [ ] Write tests for parsing TCP6 and UNKNOWN protocol variants
    - [ ] Write tests for malformed v1 header rejection
    - [ ] Write tests for max header length enforcement (107 bytes)

- [ ] Task: Implement PROXY Protocol v2 parser
    - [ ] Write tests for parsing binary v2 header (12-byte signature + header)
    - [ ] Implement v2 parser: signature validation, version/command extraction, address parsing
    - [ ] Write tests for IPv4 and IPv6 address extraction
    - [ ] Write tests for LOCAL command (no address information)
    - [ ] Write tests for TLV extensions parsing

- [ ] Task: Implement PROXY Protocol receiving
    - [ ] Write tests for listener with proxy_protocol = true
    - [ ] Implement proxy protocol detection on new connections (peek first bytes)
    - [ ] Write tests for trusted proxy list (only accept from configured CIDRs)
    - [ ] Implement trusted source validation
    - [ ] Write tests for $proxy_protocol_addr and $proxy_protocol_port variable availability
    - [ ] Implement variable injection into request context

- [ ] Task: Implement PROXY Protocol sending
    - [ ] Write tests for prepending v1 header when connecting to upstream
    - [ ] Write tests for prepending v2 header when connecting to upstream
    - [ ] Implement proxy protocol header generation and prepend to upstream connection

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)

## Phase 6: FastCGI, SCGI & gRPC Proxy

- [ ] Task: Implement FastCGI client (`crates/proxy/src/fastcgi.rs`)
    - [ ] Write tests for FastCGI record construction (BEGIN_REQUEST, PARAMS, STDIN)
    - [ ] Implement FastCGI record serializer
    - [ ] Write tests for FastCGI parameter encoding (length-value pairs)
    - [ ] Implement parameter encoder (handles 1-byte and 4-byte length prefixes)
    - [ ] Write tests for connecting to Unix domain socket
    - [ ] Write tests for connecting to TCP socket
    - [ ] Implement FastCGI connection manager (Unix or TCP)

- [ ] Task: Implement FastCGI request/response handling
    - [ ] Write tests for sending HTTP request as FastCGI params + stdin
    - [ ] Implement HTTP-to-FastCGI request translation (populate SCRIPT_FILENAME, REQUEST_URI, etc.)
    - [ ] Write tests for receiving FastCGI STDOUT response and parsing HTTP headers
    - [ ] Implement FastCGI response parser (split headers and body from STDOUT records)
    - [ ] Write tests for STDERR handling (log FastCGI errors)
    - [ ] Write tests for fastcgi_index directive (directory → index.php)
    - [ ] Write tests for PATH_INFO extraction

- [ ] Task: Implement SCGI client (`crates/proxy/src/scgi.rs`)
    - [ ] Write tests for SCGI netstring header encoding
    - [ ] Implement SCGI request builder
    - [ ] Write tests for SCGI response parsing
    - [ ] Implement SCGI response handler

- [ ] Task: Implement gRPC proxy (`crates/proxy/src/grpc_proxy.rs`)
    - [ ] Write tests for HTTP/2 gRPC frame forwarding
    - [ ] Implement gRPC passthrough proxy using hyper HTTP/2
    - [ ] Write tests for gRPC trailer header forwarding (grpc-status, grpc-message)
    - [ ] Implement trailer handling
    - [ ] Write tests for gRPC-Web (HTTP/1.1 encapsulation) detection and forwarding
    - [ ] Implement gRPC-Web bridge
    - [ ] Write tests for gRPC health check protocol
    - [ ] Implement gRPC health check integration

- [ ] Task: Conductor - User Manual Verification 'Phase 6' (Protocol in workflow.md)
