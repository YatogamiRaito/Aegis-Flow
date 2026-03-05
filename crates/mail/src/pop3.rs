/// POP3 Mail Proxy
/// Handles client POP3 connections, performs auth routing via HTTP,
/// then proxies the session to the backend mail server.
/// Supports STARTTLS, POP3S (port 995).

/// POP3 server configuration
#[derive(Debug, Clone)]
pub struct Pop3Config {
    pub listen_addr: String,
    pub pop3s_addr: Option<String>, // TLS-direct POP3S on port 995
    pub auth_http_url: String,
    pub starttls: bool,
}

impl Default for Pop3Config {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:110".to_string(),
            pop3s_addr: Some("0.0.0.0:995".to_string()),
            auth_http_url: "http://127.0.0.1:9000/auth".to_string(),
            starttls: true,
        }
    }
}

/// Generate POP3 server greeting
pub fn pop3_greeting(hostname: &str) -> String {
    format!("+OK {} Aegis-Flow POP3 Proxy ready\r\n", hostname)
}

/// POP3 command types relevant to the proxy
#[derive(Debug, PartialEq)]
pub enum Pop3Command {
    User(String),
    Pass(String),
    Capa,
    StartTls,
    Quit,
    Other(String),
}

/// Parse a raw POP3 command line
pub fn parse_pop3_command(line: &str) -> Pop3Command {
    let upper = line.trim().to_uppercase();
    if upper.starts_with("USER ") {
        Pop3Command::User(line.trim()[5..].trim().to_string())
    } else if upper.starts_with("PASS ") {
        Pop3Command::Pass(line.trim()[5..].trim().to_string())
    } else if upper == "CAPA" {
        Pop3Command::Capa
    } else if upper == "STLS" || upper == "STARTTLS" {
        Pop3Command::StartTls
    } else if upper == "QUIT" {
        Pop3Command::Quit
    } else {
        Pop3Command::Other(line.trim().to_string())
    }
}

/// Generate POP3 capability list
pub fn pop3_capa(starttls: bool) -> String {
    let mut capa = "+OK Capability list follows\r\n".to_string();
    capa.push_str("USER\r\n");
    capa.push_str("PIPELINING\r\n");
    if starttls {
        capa.push_str("STLS\r\n");
    }
    capa.push_str(".\r\n");
    capa
}

/// Generate POP3 AUTH error response
pub fn pop3_auth_error(msg: &str, error_code: Option<&str>, wait_secs: u32) -> String {
    if let Some(code) = error_code {
        format!("-ERR [AUTH] {} {}\r\n", code, msg)
    } else {
        format!("-ERR {}\r\n", msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop3_greeting() {
        let g = pop3_greeting("pop.example.com");
        assert!(g.starts_with("+OK pop.example.com"));
        assert!(g.contains("Aegis-Flow POP3 Proxy ready"));
    }

    #[test]
    fn test_pop3_capa_with_starttls() {
        let capa = pop3_capa(true);
        assert!(capa.contains("STLS"));
        assert!(capa.contains("USER"));
        assert!(capa.contains("."));
    }

    #[test]
    fn test_pop3_capa_without_starttls() {
        let capa = pop3_capa(false);
        assert!(!capa.contains("STLS"));
    }

    #[test]
    fn test_parse_pop3_user() {
        let cmd = parse_pop3_command("USER user@example.com");
        assert_eq!(cmd, Pop3Command::User("user@example.com".to_string()));
    }

    #[test]
    fn test_parse_pop3_pass() {
        let cmd = parse_pop3_command("PASS mysecret");
        assert_eq!(cmd, Pop3Command::Pass("mysecret".to_string()));
    }

    #[test]
    fn test_parse_pop3_quit() {
        let cmd = parse_pop3_command("QUIT");
        assert_eq!(cmd, Pop3Command::Quit);
    }

    #[test]
    fn test_parse_pop3_starttls() {
        let cmd = parse_pop3_command("STLS");
        assert_eq!(cmd, Pop3Command::StartTls);
    }

    #[test]
    fn test_parse_pop3_capa() {
        let cmd = parse_pop3_command("CAPA");
        assert_eq!(cmd, Pop3Command::Capa);
    }

    #[test]
    fn test_pop3_auth_error() {
        let err = pop3_auth_error("Invalid credentials", Some("535 5.7.8"), 3);
        assert!(err.starts_with("-ERR"));
        assert!(err.contains("535"));
    }

    #[test]
    fn test_pop3_auth_error_no_code() {
        let err = pop3_auth_error("Invalid credentials", None, 0);
        assert!(err.starts_with("-ERR Invalid credentials"));
    }
}
