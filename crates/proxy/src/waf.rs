use hyper::{Request, Response, StatusCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WafMode {
    Off,
    LogOnly,
    Block,
}

#[derive(Debug, Clone)]
pub struct WafRule {
    pub id: u32,
    pub pattern: regex::Regex,
    pub description: String,
}

pub struct WafEngine {
    pub mode: WafMode,
    pub rules: Vec<WafRule>,
}

impl WafEngine {
    pub fn new(mode: WafMode) -> Self {
        let mut engine = Self {
            mode,
            rules: Vec::new(),
        };
        engine.load_builtin_rules();
        engine
    }

    fn load_builtin_rules(&mut self) {
        // SQL Injection rules
        self.rules.push(WafRule {
            id: 1001,
            pattern: regex::Regex::new(r"(?i)(union(\s+|%20)+select|or(\s+|%20)+1=1|drop(\s+|%20)+table)").unwrap(),
            description: "SQL Injection".to_string(),
        });

        // XSS rules
        self.rules.push(WafRule {
            id: 2001,
            // Match <script> or %3Cscript%3E
            pattern: regex::Regex::new(r"(?i)(<script>|%3Cscript%3E|javascript:|onerror=|onload=)").unwrap(),
            description: "Cross-Site Scripting (XSS)".to_string(),
        });

        // Path Traversal rules
        self.rules.push(WafRule {
            id: 3001,
            pattern: regex::Regex::new(r"(?i)(\.\./|\.\.%2f|\.\.\\)").unwrap(),
            description: "Path Traversal".to_string(),
        });

        // Command Injection rules
        self.rules.push(WafRule {
            id: 4001,
            pattern: regex::Regex::new(r"(?i)(;|\||\$\(|`|;.*rm\s+-rf)").unwrap(),
            description: "Command Injection".to_string(),
        });
    }

    pub fn inspect_uri(&self, uri: &str) -> Option<&WafRule> {
        if self.mode == WafMode::Off {
            return None;
        }

        for rule in &self.rules {
            if rule.pattern.is_match(uri) {
                return Some(rule);
            }
        }
        None
    }

    pub fn handle_request<B>(&self, req: &Request<B>) -> Result<(), Response<String>> {
        if self.mode == WafMode::Off {
            return Ok(());
        }

        if let Some(uri) = req.uri().path_and_query() {
            let uri_str = uri.as_str();
            if let Some(rule) = self.inspect_uri(uri_str) {
                if self.mode == WafMode::Block {
                    let mut res = Response::new("Forbidden by WAF".to_string());
                    *res.status_mut() = StatusCode::FORBIDDEN;
                    return Err(res);
                } else if self.mode == WafMode::LogOnly {
                    // Log the event, but allow request
                    // tracing::warn!("WAF log_only: matched rule {} on URI {}", rule.id, uri_str);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waf_sql_injection() {
        let waf = WafEngine::new(WafMode::Block);
        let req = Request::builder()
            .uri("/login?user=admin'%20OR%201=1%20--")
            .body(())
            .unwrap();
            
        assert!(waf.handle_request(&req).is_err());
    }

    #[test]
    fn test_waf_xss() {
        let waf = WafEngine::new(WafMode::Block);
        let req = Request::builder()
            .uri("/search?q=%3Cscript%3Ealert(1)%3C/script%3E")
            .body(())
            .unwrap();
            
        assert!(waf.handle_request(&req).is_err());
    }

    #[test]
    fn test_waf_path_traversal() {
        let waf = WafEngine::new(WafMode::Block);
        let req = Request::builder()
            .uri("/download?file=../../etc/passwd")
            .body(())
            .unwrap();
            
        assert!(waf.handle_request(&req).is_err());
    }

    #[test]
    fn test_waf_command_injection() {
        let waf = WafEngine::new(WafMode::Block);
        let req = Request::builder()
            .uri("/ping?host=8.8.8.8;cat%20/etc/passwd")
            .body(())
            .unwrap();
            
        assert!(waf.handle_request(&req).is_err());
    }

    #[test]
    fn test_waf_log_only_mode() {
        let waf = WafEngine::new(WafMode::LogOnly);
        let req = Request::builder()
            .uri("/login?user=admin'%20OR%201=1%20--")
            .body(())
            .unwrap();
            
        // Should not block in LogOnly mode
        assert!(waf.handle_request(&req).is_ok());
    }

    #[test]
    fn test_waf_off_mode() {
        let waf = WafEngine::new(WafMode::Off);
        let req = Request::builder()
            .uri("/search?q=%3Cscript%3Ealert(1)%3C/script%3E")
            .body(())
            .unwrap();
            
        // Should not block in Off mode
        assert!(waf.handle_request(&req).is_ok());
    }
}
