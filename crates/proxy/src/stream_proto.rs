use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// WebSocket upgrade detection
// ---------------------------------------------------------------------------

/// Returns true if the request headers indicate an HTTP → WebSocket upgrade.
pub fn is_websocket_upgrade(headers: &[(String, String)]) -> bool {
    let upgrade = headers.iter()
        .find(|(k, _)| k.to_lowercase() == "upgrade")
        .map(|(_, v)| v.to_lowercase());
    let connection = headers.iter()
        .find(|(k, _)| k.to_lowercase() == "connection")
        .map(|(_, v)| v.to_lowercase());

    upgrade.as_deref() == Some("websocket")
        && connection.as_deref().map(|c| c.contains("upgrade")).unwrap_or(false)
}

/// Compute the Sec-WebSocket-Accept value from a Sec-WebSocket-Key.
pub fn websocket_accept_key(sec_ws_key: &str) -> String {
    use sha1::{Digest, Sha1};
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut hasher = Sha1::new();
    hasher.update(format!("{}{}", sec_ws_key, GUID).as_bytes());
    let digest = hasher.finalize();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &digest)
}

// ---------------------------------------------------------------------------
// PROXY Protocol v1 parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ProxyProtocolHeader {
    pub protocol: String,   // TCP4 | TCP6 | UNKNOWN
    pub src_addr: String,
    pub dst_addr: String,
    pub src_port: u16,
    pub dst_port: u16,
}

#[derive(Debug, PartialEq)]
pub enum ProxyProtocolError {
    Invalid,
    TooLong,
    Unknown,
}

/// Parse a PROXY Protocol v1 text header (terminated with \r\n).
/// Max length per spec: 107 bytes.
pub fn parse_proxy_protocol_v1(line: &str) -> Result<ProxyProtocolHeader, ProxyProtocolError> {
    if line.len() > 107 {
        return Err(ProxyProtocolError::TooLong);
    }
    let line = line.trim_end_matches("\r\n").trim_end_matches('\n');
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() < 2 || parts[0] != "PROXY" {
        return Err(ProxyProtocolError::Invalid);
    }
    let protocol = parts[1].to_string();
    if protocol == "UNKNOWN" {
        return Ok(ProxyProtocolHeader {
            protocol,
            src_addr: "0.0.0.0".to_string(),
            dst_addr: "0.0.0.0".to_string(),
            src_port: 0,
            dst_port: 0,
        });
    }
    if parts.len() < 6 {
        return Err(ProxyProtocolError::Invalid);
    }
    if !matches!(protocol.as_str(), "TCP4" | "TCP6") {
        return Err(ProxyProtocolError::Invalid);
    }
    let src_port: u16 = parts[4].parse().map_err(|_| ProxyProtocolError::Invalid)?;
    let dst_port: u16 = parts[5].parse().map_err(|_| ProxyProtocolError::Invalid)?;
    Ok(ProxyProtocolHeader {
        protocol,
        src_addr: parts[2].to_string(),
        dst_addr: parts[3].to_string(),
        src_port,
        dst_port,
    })
}

/// Compose a PROXY Protocol v1 header string.
pub fn build_proxy_protocol_v1(hdr: &ProxyProtocolHeader) -> String {
    format!(
        "PROXY {} {} {} {} {}\r\n",
        hdr.protocol, hdr.src_addr, hdr.dst_addr, hdr.src_port, hdr.dst_port
    )
}

// ---------------------------------------------------------------------------
// FastCGI record types (RFC 3875 / FastCGI spec)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FastCgiRecordType {
    BeginRequest,
    AbortRequest,
    EndRequest,
    Params,
    Stdin,
    Stdout,
    Stderr,
    Data,
    GetValues,
    GetValuesResult,
    Unknown(u8),
}

impl From<u8> for FastCgiRecordType {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::BeginRequest,
            2 => Self::AbortRequest,
            3 => Self::EndRequest,
            4 => Self::Params,
            5 => Self::Stdin,
            6 => Self::Stdout,
            7 => Self::Stderr,
            8 => Self::Data,
            9 => Self::GetValues,
            10 => Self::GetValuesResult,
            other => Self::Unknown(other),
        }
    }
}

impl FastCgiRecordType {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::BeginRequest   => 1,
            Self::AbortRequest   => 2,
            Self::EndRequest     => 3,
            Self::Params         => 4,
            Self::Stdin          => 5,
            Self::Stdout         => 6,
            Self::Stderr         => 7,
            Self::Data           => 8,
            Self::GetValues      => 9,
            Self::GetValuesResult => 10,
            Self::Unknown(v)     => *v,
        }
    }
}

/// Encode a FastCGI name-value pair (length-value encoding per spec).
pub fn encode_fastcgi_param(name: &str, value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    encode_fastcgi_length(&mut out, name.len() as u32);
    encode_fastcgi_length(&mut out, value.len() as u32);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(value.as_bytes());
    out
}

fn encode_fastcgi_length(buf: &mut Vec<u8>, len: u32) {
    if len <= 127 {
        buf.push(len as u8);
    } else {
        // 4-byte encoding with high bit set
        buf.push(((len >> 24) as u8) | 0x80);
        buf.push((len >> 16) as u8);
        buf.push((len >> 8) as u8);
        buf.push(len as u8);
    }
}

/// Build a FastCGI PARAMS record with common HTTP-to-CGI mappings.
pub fn build_fastcgi_params(
    script_filename: &str,
    request_method: &str,
    request_uri: &str,
    server_name: &str,
    server_port: u16,
    content_type: Option<&str>,
    content_length: Option<u64>,
) -> Vec<u8> {
    let mut params = Vec::new();
    let pairs = [
        ("SCRIPT_FILENAME", script_filename),
        ("REQUEST_METHOD", request_method),
        ("REQUEST_URI", request_uri),
        ("SERVER_NAME", server_name),
        ("SERVER_PORT", &server_port.to_string()),
        ("GATEWAY_INTERFACE", "CGI/1.1"),
        ("SERVER_PROTOCOL", "HTTP/1.1"),
        ("CONTENT_TYPE", content_type.unwrap_or("")),
    ];
    for (k, v) in &pairs {
        params.extend(encode_fastcgi_param(k, v));
    }
    if let Some(cl) = content_length {
        params.extend(encode_fastcgi_param("CONTENT_LENGTH", &cl.to_string()));
    }
    params
}

// ---------------------------------------------------------------------------
// UDP session table
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct UdpSession {
    pub client_addr: SocketAddr,
    pub upstream_addr: SocketAddr,
    pub last_activity: Instant,
    pub datagram_count: u64,
}

pub struct UdpSessionTable {
    sessions: HashMap<SocketAddr, UdpSession>,
    timeout: Duration,
    max_sessions: usize,
}

impl UdpSessionTable {
    pub fn new(timeout_secs: u64, max_sessions: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            timeout: Duration::from_secs(timeout_secs),
            max_sessions,
        }
    }

    pub fn get_or_create(
        &mut self,
        client: SocketAddr,
        upstream: SocketAddr,
    ) -> Option<&mut UdpSession> {
        // Evict if at limit (LRU-like: evict oldest)
        if !self.sessions.contains_key(&client) && self.sessions.len() >= self.max_sessions {
            let oldest = self.sessions.iter()
                .min_by_key(|(_, s)| s.last_activity)
                .map(|(k, _)| *k);
            if let Some(k) = oldest {
                self.sessions.remove(&k);
            }
        }

        let session = self.sessions.entry(client).or_insert_with(|| UdpSession {
            client_addr: client,
            upstream_addr: upstream,
            last_activity: Instant::now(),
            datagram_count: 0,
        });
        session.last_activity = Instant::now();
        session.datagram_count += 1;
        Some(session)
    }

    pub fn cleanup_expired(&mut self) {
        let timeout = self.timeout;
        self.sessions.retain(|_, s| s.last_activity.elapsed() < timeout);
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// SSE detection
// ---------------------------------------------------------------------------

/// Returns true if the response is a Server-Sent Events stream.
pub fn is_sse_response(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(k, v)| {
        k.to_lowercase() == "content-type"
            && v.to_lowercase().starts_with("text/event-stream")
    })
}

// ---------------------------------------------------------------------------
// gRPC detection
// ---------------------------------------------------------------------------

pub fn is_grpc_request(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(k, v)| {
        k.to_lowercase() == "content-type"
            && (v.starts_with("application/grpc") || v.starts_with("application/grpc-web"))
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- WebSocket ---
    #[test]
    fn test_websocket_upgrade_detected() {
        let headers = vec![
            ("Upgrade".to_string(), "websocket".to_string()),
            ("Connection".to_string(), "Upgrade".to_string()),
            ("Sec-WebSocket-Key".to_string(), "dGhlIHNhbXBsZSBub25jZQ==".to_string()),
        ];
        assert!(is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_not_websocket_upgrade() {
        let headers = vec![
            ("Content-Type".to_string(), "text/html".to_string()),
        ];
        assert!(!is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_websocket_accept_key() {
        // Known test vector from RFC 6455 section 1.3
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = websocket_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    // --- PROXY Protocol ---
    #[test]
    fn test_proxy_protocol_v1_tcp4() {
        let hdr = parse_proxy_protocol_v1("PROXY TCP4 192.168.1.1 10.0.0.1 54321 80\r\n").unwrap();
        assert_eq!(hdr.protocol, "TCP4");
        assert_eq!(hdr.src_addr, "192.168.1.1");
        assert_eq!(hdr.src_port, 54321);
        assert_eq!(hdr.dst_port, 80);
    }

    #[test]
    fn test_proxy_protocol_v1_tcp6() {
        let hdr = parse_proxy_protocol_v1("PROXY TCP6 ::1 ::1 8080 443\r\n").unwrap();
        assert_eq!(hdr.protocol, "TCP6");
    }

    #[test]
    fn test_proxy_protocol_v1_unknown() {
        let hdr = parse_proxy_protocol_v1("PROXY UNKNOWN\r\n").unwrap();
        assert_eq!(hdr.protocol, "UNKNOWN");
    }

    #[test]
    fn test_proxy_protocol_v1_invalid() {
        assert_eq!(
            parse_proxy_protocol_v1("NOT A PROXY HEADER"),
            Err(ProxyProtocolError::Invalid)
        );
    }

    #[test]
    fn test_proxy_protocol_v1_too_long() {
        let long = "PROXY TCP4 ".to_string() + &"1".repeat(200);
        assert_eq!(
            parse_proxy_protocol_v1(&long),
            Err(ProxyProtocolError::TooLong)
        );
    }

    #[test]
    fn test_proxy_protocol_v1_build() {
        let hdr = ProxyProtocolHeader {
            protocol: "TCP4".to_string(),
            src_addr: "1.2.3.4".to_string(),
            dst_addr: "5.6.7.8".to_string(),
            src_port: 1234,
            dst_port: 80,
        };
        let built = build_proxy_protocol_v1(&hdr);
        assert_eq!(built, "PROXY TCP4 1.2.3.4 5.6.7.8 1234 80\r\n");
    }

    // --- FastCGI ---
    #[test]
    fn test_fastcgi_record_types() {
        assert_eq!(FastCgiRecordType::from(1), FastCgiRecordType::BeginRequest);
        assert_eq!(FastCgiRecordType::from(4), FastCgiRecordType::Params);
        assert_eq!(FastCgiRecordType::from(6), FastCgiRecordType::Stdout);
        assert_eq!(FastCgiRecordType::BeginRequest.as_u8(), 1);
        assert_eq!(FastCgiRecordType::Stdout.as_u8(), 6);
    }

    #[test]
    fn test_fastcgi_param_encoding_short() {
        let encoded = encode_fastcgi_param("REQUEST_METHOD", "GET");
        // 1 byte name len + 1 byte val len + name + val
        assert_eq!(encoded.len(), 1 + 1 + "REQUEST_METHOD".len() + "GET".len());
        assert_eq!(encoded[0], "REQUEST_METHOD".len() as u8);
        assert_eq!(encoded[1], "GET".len() as u8);
    }

    #[test]
    fn test_fastcgi_params_build() {
        let params = build_fastcgi_params(
            "/var/www/index.php",
            "GET",
            "/index.php",
            "example.com",
            80,
            None,
            None,
        );
        assert!(!params.is_empty());
        // Check that SCRIPT_FILENAME value is encoded
        let as_str = String::from_utf8_lossy(&params);
        assert!(as_str.contains("SCRIPT_FILENAME"));
        assert!(as_str.contains("/var/www/index.php"));
    }

    // --- UDP Session Table ---
    #[test]
    fn test_udp_session_create() {
        let mut table = UdpSessionTable::new(30, 100);
        let client: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let upstream: SocketAddr = "10.0.0.1:8080".parse().unwrap();

        let session = table.get_or_create(client, upstream).unwrap();
        assert_eq!(session.datagram_count, 1);

        // Get again → same session, incremented counter
        let session = table.get_or_create(client, upstream).unwrap();
        assert_eq!(session.datagram_count, 2);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_udp_session_max_limit() {
        let mut table = UdpSessionTable::new(30, 2);
        let up: SocketAddr = "10.0.0.1:8080".parse().unwrap();
        let c1: SocketAddr = "1.1.1.1:100".parse().unwrap();
        let c2: SocketAddr = "2.2.2.2:200".parse().unwrap();
        let c3: SocketAddr = "3.3.3.3:300".parse().unwrap();

        table.get_or_create(c1, up).unwrap();
        table.get_or_create(c2, up).unwrap();
        table.get_or_create(c3, up).unwrap(); // evicts oldest

        assert_eq!(table.len(), 2);
    }

    // --- SSE & gRPC detection ---
    #[test]
    fn test_sse_detection() {
        let headers = vec![("content-type".to_string(), "text/event-stream".to_string())];
        assert!(is_sse_response(&headers));
        let other = vec![("content-type".to_string(), "application/json".to_string())];
        assert!(!is_sse_response(&other));
    }

    #[test]
    fn test_grpc_detection() {
        let grpc = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(is_grpc_request(&grpc));
        let grpc_web = vec![("content-type".to_string(), "application/grpc-web".to_string())];
        assert!(is_grpc_request(&grpc_web));
        let other = vec![("content-type".to_string(), "text/html".to_string())];
        assert!(!is_grpc_request(&other));
    }
}
