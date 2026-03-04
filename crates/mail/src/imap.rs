/// IMAP Mail Proxy
/// Handles client IMAP connections, performs auth routing via HTTP,
/// then proxies the IMAP session to the backend mail server.
/// Supports STARTTLS, IMAPS, IDLE passthrough for push notifications.

use tracing::debug;

/// IMAP server configuration
#[derive(Debug, Clone)]
pub struct ImapConfig {
    pub listen_addr: String,
    pub imaps_addr: Option<String>,    // TLS-direct IMAP on port 993
    pub auth_http_url: String,
    pub starttls: bool,
}

impl Default for ImapConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:143".to_string(),
            imaps_addr: Some("0.0.0.0:993".to_string()),
            auth_http_url: "http://127.0.0.1:9000/auth".to_string(),
            starttls: true,
        }
    }
}

/// Generate IMAP server greeting
pub fn imap_greeting(hostname: &str) -> String {
    format!("* OK {} Aegis-Flow IMAP Proxy ready\r\n", hostname)
}

/// Generate IMAP capability response
pub fn imap_capability(starttls: bool) -> String {
    let mut caps = "* CAPABILITY IMAP4rev1 AUTH=PLAIN AUTH=LOGIN SASL-IR".to_string();
    if starttls {
        caps.push_str(" STARTTLS");
    }
    caps.push_str(" IDLE LITERAL+\r\n");
    caps
}

/// IMAP command types relevant to the proxy
#[derive(Debug, PartialEq)]
pub enum ImapCommand {
    Capability,
    Login { tag: String, user: String, pass: String },
    Authenticate { tag: String, mechanism: String },
    StartTls { tag: String },
    Idle { tag: String },
    Logout { tag: String },
    Other { tag: String, cmd: String },
}

/// Parse a raw IMAP command line into an ImapCommand
pub fn parse_imap_command(line: &str) -> Option<ImapCommand> {
    let parts: Vec<&str> = line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let tag = parts[0].to_string();
    let cmd = parts[1].to_uppercase();
    let args = parts.get(2).copied().unwrap_or("");

    Some(match cmd.as_str() {
        "CAPABILITY" => ImapCommand::Capability,
        "STARTTLS" => ImapCommand::StartTls { tag },
        "IDLE" => ImapCommand::Idle { tag },
        "LOGOUT" => ImapCommand::Logout { tag },
        "LOGIN" => {
            // LOGIN user pass (may be quoted)
            let login_parts: Vec<&str> = args.splitn(2, ' ').collect();
            let user = login_parts.get(0).copied().unwrap_or("").trim_matches('"').to_string();
            let pass = login_parts.get(1).copied().unwrap_or("").trim_matches('"').to_string();
            ImapCommand::Login { tag, user, pass }
        }
        "AUTHENTICATE" => {
            let mechanism = args.to_uppercase();
            ImapCommand::Authenticate { tag, mechanism }
        }
        _ => ImapCommand::Other { tag, cmd: cmd.to_string() },
    })
}

/// Generate IMAP IDLE acknowledgment
pub fn imap_idle_ack(tag: &str) -> String {
    format!("+ idling\r\n")
}

/// Generate IMAP IDLE done response
pub fn imap_idle_done(tag: &str) -> String {
    format!("{} OK IDLE terminated\r\n", tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imap_greeting() {
        let g = imap_greeting("imap.example.com");
        assert!(g.starts_with("* OK imap.example.com"));
        assert!(g.contains("Aegis-Flow IMAP Proxy ready"));
    }

    #[test]
    fn test_imap_capability_with_starttls() {
        let caps = imap_capability(true);
        assert!(caps.contains("STARTTLS"));
        assert!(caps.contains("IMAP4rev1"));
        assert!(caps.contains("IDLE"));
    }

    #[test]
    fn test_imap_capability_without_starttls() {
        let caps = imap_capability(false);
        assert!(!caps.contains("STARTTLS"));
    }

    #[test]
    fn test_parse_imap_login() {
        let cmd = parse_imap_command("a1 LOGIN user@example.com mysecret").unwrap();
        match cmd {
            ImapCommand::Login { tag, user, pass } => {
                assert_eq!(tag, "a1");
                assert_eq!(user, "user@example.com");
                assert_eq!(pass, "mysecret");
            }
            _ => panic!("Expected Login"),
        }
    }

    #[test]
    fn test_parse_imap_starttls() {
        let cmd = parse_imap_command("a2 STARTTLS").unwrap();
        assert!(matches!(cmd, ImapCommand::StartTls { .. }));
    }

    #[test]
    fn test_parse_imap_idle() {
        let cmd = parse_imap_command("a3 IDLE").unwrap();
        assert!(matches!(cmd, ImapCommand::Idle { .. }));
    }

    #[test]
    fn test_parse_imap_logout() {
        let cmd = parse_imap_command("a4 LOGOUT").unwrap();
        assert!(matches!(cmd, ImapCommand::Logout { .. }));
    }

    #[test]
    fn test_parse_imap_capability() {
        let cmd = parse_imap_command("a5 CAPABILITY").unwrap();
        assert!(matches!(cmd, ImapCommand::Capability));
    }

    #[test]
    fn test_imap_idle_ack() {
        let ack = imap_idle_ack("a3");
        assert!(ack.starts_with('+'));
    }
}
