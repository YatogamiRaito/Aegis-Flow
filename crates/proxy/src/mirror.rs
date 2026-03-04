use serde_json;

/// Traffic mirroring: sends a copy of request to a mirror backend asynchronously
pub struct MirrorConfig {
    pub mirror_url: String,
    pub mirror_percentage: f64, // 0.0 to 100.0
    pub include_body: bool,
}

impl MirrorConfig {
    pub fn new(mirror_url: &str) -> Self {
        Self {
            mirror_url: mirror_url.to_string(),
            mirror_percentage: 100.0,
            include_body: false,
        }
    }

    pub fn with_percentage(mut self, pct: f64) -> Self {
        self.mirror_percentage = pct;
        self
    }

    /// Determine if this request should be mirrored based on percentage
    pub fn should_mirror(&self, request_id: u64) -> bool {
        let pct = (request_id % 100) as f64;
        pct < self.mirror_percentage
    }
}

/// Method-based access control (nginx limit_except)
#[derive(Debug, Clone)]
pub struct LimitExcept {
    pub allowed_methods: Vec<String>,
    pub deny_status: u16,
}

impl LimitExcept {
    pub fn new(allowed_methods: Vec<&str>, deny_status: u16) -> Self {
        Self {
            allowed_methods: allowed_methods.iter().map(|s| s.to_uppercase()).collect(),
            deny_status,
        }
    }

    pub fn is_allowed(&self, method: &str) -> bool {
        self.allowed_methods.contains(&method.to_uppercase())
    }
}

/// Stub status metrics
#[derive(Default, Debug, Clone)]
pub struct StubStatus {
    pub active_connections: u64,
    pub total_accepts: u64,
    pub total_requests: u64,
    pub reading: u64,
    pub writing: u64,
    pub waiting: u64,
}

impl StubStatus {
    pub fn to_html(&self) -> String {
        format!(
            "<html><body><pre>Active connections: {}\nserver accepts handled requests\n {} {} {}\nReading: {} Writing: {} Waiting: {}\n</pre></body></html>",
            self.active_connections,
            self.total_accepts,
            self.total_accepts,
            self.total_requests,
            self.reading,
            self.writing,
            self.waiting
        )
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "active_connections": self.active_connections,
            "total_accepts": self.total_accepts,
            "total_requests": self.total_requests,
            "reading": self.reading,
            "writing": self.writing,
            "waiting": self.waiting,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_percentage() {
        let mirror = MirrorConfig::new("http://mirror:8080").with_percentage(50.0);

        // At 50%, roughly half should be mirrored
        let mirrored: Vec<bool> = (0..100).map(|i| mirror.should_mirror(i)).collect();
        let count = mirrored.iter().filter(|&&b| b).count();
        assert_eq!(count, 50);
    }

    #[test]
    fn test_mirror_100_pct() {
        let mirror = MirrorConfig::new("http://mirror:8080").with_percentage(100.0);
        assert!(mirror.should_mirror(0));
        assert!(mirror.should_mirror(99));
    }

    #[test]
    fn test_limit_except_allow() {
        let le = LimitExcept::new(vec!["GET", "HEAD"], 405);
        assert!(le.is_allowed("GET"));
        assert!(le.is_allowed("get")); // case insensitive
        assert!(le.is_allowed("HEAD"));
    }

    #[test]
    fn test_limit_except_deny() {
        let le = LimitExcept::new(vec!["GET"], 405);
        assert!(!le.is_allowed("POST"));
        assert!(!le.is_allowed("PUT"));
        assert!(!le.is_allowed("DELETE"));
    }

    #[test]
    fn test_stub_status_html() {
        let status = StubStatus {
            active_connections: 3,
            total_accepts: 1000,
            total_requests: 999,
            reading: 0,
            writing: 1,
            waiting: 2,
        };
        let html = status.to_html();
        assert!(html.contains("Active connections: 3"));
        assert!(html.contains("1000"));
    }

    #[test]
    fn test_stub_status_json() {
        let status = StubStatus {
            active_connections: 5,
            ..Default::default()
        };
        let json = status.to_json();
        assert_eq!(json["active_connections"], 5);
    }
}
