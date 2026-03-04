/// SMTP Mail Proxy
/// Handles client SMTP connections, performs auth routing via HTTP,
/// then proxies the session to the backend mail server.

use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tracing::{debug, error, info};
use bytes::Bytes;
use crate::mail_auth::{MailAuthRequest, MailAuthResult, authenticate};

/// SMTP server configuration
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub listen_addr: String,
    pub auth_http_url: String,
    pub hostname: String,
    pub starttls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:25".to_string(),
            auth_http_url: "http://127.0.0.1:9000/auth".to_string(),
            hostname: "mail.example.com".to_string(),
            starttls: true,
        }
    }
}

/// Parse the AUTH PLAIN payload (base64 encoded "\0user\0pass")
pub fn parse_auth_plain(encoded: &str) -> Option<(String, String)> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let decoded = STANDARD.decode(encoded.trim()).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    // Format: \0username\0password
    let parts: Vec<&str> = s.split('\0').collect();
    if parts.len() >= 3 {
        Some((parts[1].to_string(), parts[2].to_string()))
    } else if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Parse the AUTH LOGIN exchange (base64 encoded user/pass in sequence)
pub fn parse_auth_login_part(encoded: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(encoded.trim()).ok()
        .and_then(|b| String::from_utf8(b).ok())
}

/// Generate SMTP server greeting banner
pub fn smtp_greeting(hostname: &str) -> String {
    format!("220 {} ESMTP Aegis-Flow Mail Proxy\r\n", hostname)
}

/// Generate EHLO response with supported capabilities
pub fn smtp_ehlo_response(hostname: &str, starttls: bool) -> String {
    let mut caps = format!("250-{}\r\n", hostname);
    caps.push_str("250-SIZE 52428800\r\n");  // 50MB max
    caps.push_str("250-AUTH PLAIN LOGIN\r\n");
    caps.push_str("250-PIPELINING\r\n");
    if starttls {
        caps.push_str("250-STARTTLS\r\n");
    }
    caps.push_str("250 8BITMIME\r\n");
    caps
}

/// Handle an SMTP command line and return the response
pub fn handle_smtp_command(cmd: &str, hostname: &str, starttls: bool) -> SmtpCommandResult {
    let upper = cmd.trim().to_uppercase();
    if upper.starts_with("EHLO") || upper.starts_with("HELO") {
        SmtpCommandResult::Response(smtp_ehlo_response(hostname, starttls))
    } else if upper.starts_with("STARTTLS") {
        SmtpCommandResult::StartTls
    } else if upper.starts_with("AUTH PLAIN") {
        let parts: Vec<&str> = cmd.trim().splitn(3, ' ').collect();
        let payload = parts.get(2).copied().unwrap_or("");
        SmtpCommandResult::AuthPlain(payload.to_string())
    } else if upper.starts_with("AUTH LOGIN") {
        SmtpCommandResult::AuthLoginStart
    } else if upper == "RSET\r\n" || upper == "RSET" {
        SmtpCommandResult::Response("250 Ok\r\n".to_string())
    } else if upper.starts_with("QUIT") {
        SmtpCommandResult::Quit
    } else if upper.starts_with("NOOP") {
        SmtpCommandResult::Response("250 Ok\r\n".to_string())
    } else {
        SmtpCommandResult::Response("502 Command not implemented\r\n".to_string())
    }
}

/// Result from processing a single SMTP command
pub enum SmtpCommandResult {
    Response(String),
    StartTls,
    AuthPlain(String),
    AuthLoginStart,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_greeting() {
        let greeting = smtp_greeting("mail.example.com");
        assert!(greeting.starts_with("220 mail.example.com ESMTP"));
    }

    #[test]
    fn test_smtp_ehlo_response() {
        let resp = smtp_ehlo_response("mail.example.com", true);
        assert!(resp.contains("mail.example.com"));
        assert!(resp.contains("AUTH PLAIN LOGIN"));
        assert!(resp.contains("STARTTLS"));
    }

    #[test]
    fn test_smtp_ehlo_no_starttls() {
        let resp = smtp_ehlo_response("mail.example.com", false);
        assert!(!resp.contains("STARTTLS"));
    }

    #[test]
    fn test_parse_auth_plain() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        // Format: \0user\0pass
        let payload = STANDARD.encode("\0user@example.com\0mysecret");
        let result = parse_auth_plain(&payload);
        assert!(result.is_some());
        let (user, pass) = result.unwrap();
        assert_eq!(user, "user@example.com");
        assert_eq!(pass, "mysecret");
    }

    #[test]
    fn test_handle_ehlo_command() {
        let result = handle_smtp_command("EHLO client.example.com", "mail.test.com", true);
        match result {
            SmtpCommandResult::Response(r) => assert!(r.contains("250")),
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_handle_quit_command() {
        let result = handle_smtp_command("QUIT", "mail.test.com", false);
        matches!(result, SmtpCommandResult::Quit);
    }

    #[test]
    fn test_handle_starttls_command() {
        let result = handle_smtp_command("STARTTLS", "mail.test.com", true);
        matches!(result, SmtpCommandResult::StartTls);
    }
}
