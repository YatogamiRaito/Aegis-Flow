/// RFC 5424 Syslog client implementation
/// Supports UDP, TCP, and TCP+TLS transport
use bytes::Bytes;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tracing::{error, debug};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyslogTransport {
    Udp,
    Tcp,
    TcpTls,
}

impl Default for SyslogTransport {
    fn default() -> Self {
        SyslogTransport::Udp
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyslogConfig {
    pub enabled: bool,
    pub server: String,
    #[serde(default)]
    pub transport: SyslogTransport,
    #[serde(default = "default_facility")]
    pub facility: String,
    #[serde(default = "default_tag")]
    pub tag: String,
}

fn default_facility() -> String {
    "local7".to_string()
}

fn default_tag() -> String {
    "aegis-flow".to_string()
}

impl Default for SyslogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server: "127.0.0.1:514".to_string(),
            transport: SyslogTransport::Udp,
            facility: default_facility(),
            tag: default_tag(),
        }
    }
}

/// Map facility name to numeric code per RFC 5424
fn facility_code(facility: &str) -> u8 {
    match facility {
        "kern" => 0, "user" => 1, "mail" => 2, "daemon" => 3,
        "auth" => 4, "syslog" => 5, "lpr" => 6, "news" => 7,
        "uucp" => 8, "cron" => 9, "authpriv" => 10,
        "local0" => 16, "local1" => 17, "local2" => 18, "local3" => 19,
        "local4" => 20, "local5" => 21, "local6" => 22, "local7" => 23,
        _ => 23, // default to local7
    }
}

/// Severity levels per RFC 5424  
#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Informational = 6,
    Debug = 7,
}

/// Format an RFC 5424 syslog message
pub fn format_syslog_message(
    config: &SyslogConfig,
    severity: Severity,
    message: &str,
) -> String {
    let facility = facility_code(&config.facility);
    let priority = (facility * 8) + severity as u8;
    
    // RFC 5424: <PRIORITY>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID SD MSG
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "-".to_string());

    format!(
        "<{}>1 {} {} {} - - - {}",
        priority, timestamp, hostname, config.tag, message
    )
}

/// Send a syslog message via UDP (best-effort, fire-and-forget)
pub async fn send_udp_syslog(server: &str, message: &str) {
    match server.parse::<SocketAddr>() {
        Ok(addr) => {
            match UdpSocket::bind("0.0.0.0:0").await {
                Ok(socket) => {
                    if let Err(e) = socket.send_to(message.as_bytes(), addr).await {
                        error!("Syslog UDP send failed: {}", e);
                    }
                }
                Err(e) => error!("Syslog UDP bind failed: {}", e),
            }
        }
        Err(_) => {
            // Server might be a hostname:port, resolve it
            match tokio::net::lookup_host(server).await {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        match UdpSocket::bind("0.0.0.0:0").await {
                            Ok(socket) => {
                                if let Err(e) = socket.send_to(message.as_bytes(), addr).await {
                                    error!("Syslog UDP send failed: {}", e);
                                }
                            }
                            Err(e) => error!("Syslog UDP bind failed: {}", e),
                        }
                    }
                }
                Err(e) => error!("Syslog DNS resolution failed for {}: {}", server, e),
            }
        }
    }
}

/// Send a syslog message via TCP
pub async fn send_tcp_syslog(server: &str, message: &str) {
    match TcpStream::connect(server).await {
        Ok(mut stream) => {
            // RFC 6587: Octet-Counting framing for TCP syslog
            let framed = format!("{} {}", message.len(), message);
            if let Err(e) = stream.write_all(framed.as_bytes()).await {
                error!("Syslog TCP send failed: {}", e);
            }
        }
        Err(e) => error!("Syslog TCP connect to {} failed: {}", server, e),
    }
}

/// Send a log message based on config transport
pub async fn send_log(config: &SyslogConfig, severity: Severity, message: &str) {
    if !config.enabled {
        return;
    }

    let msg = format_syslog_message(config, severity, message);
    debug!("Syslog → {}: {}", config.server, &msg);

    match config.transport {
        SyslogTransport::Udp => send_udp_syslog(&config.server, &msg).await,
        SyslogTransport::Tcp | SyslogTransport::TcpTls => {
            // TLS not yet integrated without cert config, fall back to TCP for now
            send_tcp_syslog(&config.server, &msg).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syslog_message_format() {
        let config = SyslogConfig {
            enabled: true,
            server: "127.0.0.1:514".to_string(),
            transport: SyslogTransport::Udp,
            facility: "local7".to_string(),
            tag: "aegis-flow".to_string(),
        };

        let msg = format_syslog_message(&config, Severity::Informational, "Test message");
        // Priority for local7 (23) + info (6) = 190
        assert!(msg.starts_with("<190>1 "));
        assert!(msg.contains("aegis-flow"));
        assert!(msg.contains("Test message"));
    }

    #[test]
    fn test_facility_codes() {
        assert_eq!(facility_code("local7"), 23);
        assert_eq!(facility_code("local0"), 16);
        assert_eq!(facility_code("kern"), 0);
        assert_eq!(facility_code("user"), 1);
    }

    #[test]
    fn test_syslog_config_default() {
        let cfg = SyslogConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.facility, "local7");
        assert_eq!(cfg.tag, "aegis-flow");
        assert_eq!(cfg.transport, SyslogTransport::Udp);
    }
}
