# Implementation Plan: WebSocket, TCP/UDP Stream Proxy & Protocol Support (v0.22.0)

## Phase 1: WebSocket Proxy

- [x] Task: Implement WebSocket upgrade detection (`crates/proxy/src/websocket.rs`)
    - [x] Write tests for detecting Upgrade: websocket and Connection: upgrade headers
    - [x] Implement is_websocket_upgrade() check
    - [x] Write tests for Sec-WebSocket-Key/Accept handshake header generation
    - [x] Implement WebSocket handshake response builder

- [x] Task: Implement bidirectional WebSocket frame forwarding
    - [x] Write tests for client→upstream frame forwarding (text, binary, close)
    - [x] Write tests for upstream→client frame forwarding (text, binary, close)
    - [x] Implement bidirectional forwarding using tokio-tungstenite
    - [x] Write tests for connection close propagation (client close → upstream close and vice versa)
    - [x] Write tests for error handling (upstream disconnects, client disconnects)

- [x] Task: Implement WebSocket keepalive and timeouts
    - [x] Write tests for ping/pong frame generation at configurable interval
    - [x] Implement keepalive ping task
    - [x] Write tests for idle timeout (close connection if no frames for N seconds)
    - [x] Implement idle timeout enforcement
    - [x] Write tests for proxy_read_timeout / proxy_send_timeout on WS frames

- [x] Task: Implement WebSocket header injection
    - [x] Write tests for X-Forwarded-For and X-Real-IP injection before upgrade
    - [x] Implement header injection in upgrade request forwarding
    - [x] Write tests for per-location websocket = true config

- [x] Task: Implement WebSocket metrics
    - [x] Write tests for aegis_websocket_connections_active gauge
    - [x] Write tests for aegis_websocket_messages_total counter (by direction)
    - [x] Implement metric recording

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Server-Sent Events (SSE) Proxy

- [x] Task: Implement SSE detection and unbuffered proxying
    - [x] Write tests for text/event-stream content-type detection
    - [x] Implement SSE passthrough mode (disable response buffering)
    - [x] Write tests for X-Accel-Buffering: no header support
    - [x] Write tests for long-lived SSE connection without idle timeout
    - [x] Implement SSE-specific timeout handling

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: TCP Stream Proxy

- [x] Task: Create stream proxy module (`crates/proxy/src/stream_proxy.rs`)
    - [x] Write tests for StreamConfig struct (listen, proxy_pass, protocol, servers, health_check)
    - [x] Implement StreamConfig with serde deserialization
    - [x] Write tests for TCP listener binding on configured port
    - [x] Implement TCP stream listener using tokio::net::TcpListener

- [x] Task: Implement raw TCP bidirectional forwarding
    - [x] Write tests for client→upstream and upstream→client byte forwarding
    - [x] Implement bidirectional copy using tokio::io::copy_bidirectional
    - [x] Write tests for connection close propagation
    - [x] Write tests for error handling (upstream unreachable, connection reset)

- [x] Task: Implement TCP stream TLS handling
    - [x] Write tests for TLS termination on TCP stream (decrypt at proxy, forward plain)
    - [x] Implement TLS acceptor for stream connections using rustls
    - [x] Write tests for TLS passthrough (SNI-based routing without decryption)
    - [x] Implement SNI extraction from TLS ClientHello without decrypting
    - [x] Write tests for SNI-to-upstream routing lookup

- [x] Task: Implement TCP stream load balancing
    - [x] Write tests for round-robin across TCP upstream servers
    - [x] Write tests for least-connections for TCP streams
    - [x] Implement load balancer reuse from upstream module
    - [x] Write tests for TCP health check (connect probe)
    - [x] Implement TCP connect health check with configurable interval

- [x] Task: Implement TCP stream access control
    - [x] Write tests for IP allow/deny on stream level
    - [x] Implement ACL reuse from security module
    - [x] Write tests for connection limit per stream
    - [x] Implement connection counting with limit

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: UDP Stream Proxy

- [x] Task: Implement UDP listener and session management (`crates/proxy/src/udp_proxy.rs`)
    - [x] Write tests for UDP socket binding and datagram receiving
    - [x] Implement UDP listener using tokio::net::UdpSocket
    - [x] Write tests for session table (client_addr:port → upstream socket mapping)
    - [x] Implement UdpSessionTable with HashMap and expiration

- [x] Task: Implement UDP datagram forwarding
    - [x] Write tests for client→upstream datagram forwarding
    - [x] Write tests for upstream→client response datagram forwarding
    - [x] Implement bidirectional UDP forwarding with per-session sockets
    - [x] Write tests for proxy_responses parameter (expect N response datagrams)
    - [x] Implement response counting per session

- [x] Task: Implement UDP session management
    - [x] Write tests for session timeout (expire after N seconds of inactivity)
    - [x] Implement session timeout with background cleanup task
    - [x] Write tests for max datagram size configuration
    - [x] Implement datagram size validation and truncation
    - [x] Write tests for session table size limits (prevent memory exhaustion)
    - [x] Implement session table max entries with LRU eviction

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: PROXY Protocol

- [x] Task: Implement PROXY Protocol v1 parser (`crates/proxy/src/proxy_protocol.rs`)
    - [x] Write tests for parsing text-based v1 header: "PROXY TCP4 src_ip src_port dst_ip dst_port\r\n"
    - [x] Implement v1 parser
    - [x] Write tests for parsing TCP6 and UNKNOWN protocol variants
    - [x] Write tests for malformed v1 header rejection
    - [x] Write tests for max header length enforcement (107 bytes)

- [x] Task: Implement PROXY Protocol v2 parser
    - [x] Write tests for parsing binary v2 header (12-byte signature + header)
    - [x] Implement v2 parser: signature validation, version/command extraction, address parsing
    - [x] Write tests for IPv4 and IPv6 address extraction
    - [x] Write tests for LOCAL command (no address information)
    - [x] Write tests for TLV extensions parsing

- [x] Task: Implement PROXY Protocol receiving
    - [x] Write tests for listener with proxy_protocol = true
    - [x] Implement proxy protocol detection on new connections (peek first bytes)
    - [x] Write tests for trusted proxy list (only accept from configured CIDRs)
    - [x] Implement trusted source validation
    - [x] Write tests for $proxy_protocol_addr and $proxy_protocol_port variable availability
    - [x] Implement variable injection into request context

- [x] Task: Implement PROXY Protocol sending
    - [x] Write tests for prepending v1 header when connecting to upstream
    - [x] Write tests for prepending v2 header when connecting to upstream
    - [x] Implement proxy protocol header generation and prepend to upstream connection

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)

## Phase 6: FastCGI, SCGI & gRPC Proxy

- [x] Task: Implement FastCGI client (`crates/proxy/src/fastcgi.rs`)
    - [x] Write tests for FastCGI record construction (BEGIN_REQUEST, PARAMS, STDIN)
    - [x] Implement FastCGI record serializer
    - [x] Write tests for FastCGI parameter encoding (length-value pairs)
    - [x] Implement parameter encoder (handles 1-byte and 4-byte length prefixes)
    - [x] Write tests for connecting to Unix domain socket
    - [x] Write tests for connecting to TCP socket
    - [x] Implement FastCGI connection manager (Unix or TCP)

- [x] Task: Implement FastCGI request/response handling
    - [x] Write tests for sending HTTP request as FastCGI params + stdin
    - [x] Implement HTTP-to-FastCGI request translation (populate SCRIPT_FILENAME, REQUEST_URI, etc.)
    - [x] Write tests for receiving FastCGI STDOUT response and parsing HTTP headers
    - [x] Implement FastCGI response parser (split headers and body from STDOUT records)
    - [x] Write tests for STDERR handling (log FastCGI errors)
    - [x] Write tests for fastcgi_index directive (directory → index.php)
    - [x] Write tests for PATH_INFO extraction

- [x] Task: Implement SCGI client (`crates/proxy/src/scgi.rs`)
    - [x] Write tests for SCGI netstring header encoding
    - [x] Implement SCGI request builder
    - [x] Write tests for SCGI response parsing
    - [x] Implement SCGI response handler

- [x] Task: Implement gRPC proxy (`crates/proxy/src/grpc_proxy.rs`)
    - [x] Write tests for HTTP/2 gRPC frame forwarding
    - [x] Implement gRPC passthrough proxy using hyper HTTP/2
    - [x] Write tests for gRPC trailer header forwarding (grpc-status, grpc-message)
    - [x] Implement trailer handling
    - [x] Write tests for gRPC-Web (HTTP/1.1 encapsulation) detection and forwarding
    - [x] Implement gRPC-Web bridge
    - [x] Write tests for gRPC health check protocol
    - [x] Implement gRPC health check integration

- [x] Task: Conductor - User Manual Verification 'Phase 6' (Protocol in workflow.md)
