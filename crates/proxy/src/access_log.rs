use chrono::{DateTime, Utc};
use std::fmt::Write as FmtWrite;
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Log format tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum LogToken {
    /// Literal text
    Literal(String),
    /// $remote_addr
    RemoteAddr,
    /// $time_local
    TimeLocal,
    /// $request (METHOD URI PROTO)
    Request,
    /// $status
    Status,
    /// $body_bytes_sent
    BodyBytesSent,
    /// $http_referer
    Referer,
    /// $http_user_agent
    UserAgent,
    /// $request_time (float seconds)
    RequestTime,
    /// $upstream_response_time
    UpstreamResponseTime,
    /// $host
    Host,
    /// Unknown variable passthrough
    Unknown(String),
}

/// Parse a log format string into tokens.
pub fn parse_log_format(format: &str) -> Vec<LogToken> {
    let mut tokens = Vec::new();
    let mut chars = format.chars().peekable();
    let mut literal = String::new();

    while let Some(c) = chars.next() {
        if c == '$' {
            if !literal.is_empty() {
                tokens.push(LogToken::Literal(std::mem::take(&mut literal)));
            }
            let mut var = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_alphanumeric() || nc == '_' {
                    var.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(match var.as_str() {
                "remote_addr" => LogToken::RemoteAddr,
                "time_local" => LogToken::TimeLocal,
                "request" => LogToken::Request,
                "status" => LogToken::Status,
                "body_bytes_sent" => LogToken::BodyBytesSent,
                "http_referer" => LogToken::Referer,
                "http_user_agent" => LogToken::UserAgent,
                "request_time" => LogToken::RequestTime,
                "upstream_response_time" => LogToken::UpstreamResponseTime,
                "host" => LogToken::Host,
                other => LogToken::Unknown(other.to_string()),
            });
        } else {
            literal.push(c);
        }
    }
    if !literal.is_empty() {
        tokens.push(LogToken::Literal(literal));
    }
    tokens
}

// ---------------------------------------------------------------------------
// Log record
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AccessLogRecord {
    pub remote_addr: String,
    pub time: SystemTime,
    pub method: String,
    pub uri: String,
    pub proto: String,
    pub status: u16,
    pub body_bytes_sent: u64,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
    pub request_time_ms: u64,
    pub upstream_response_time_ms: Option<u64>,
    pub host: String,
}

impl AccessLogRecord {
    pub fn render_combined(&self) -> String {
        let dt: DateTime<Utc> = self.time.into();
        let time_local = dt.format("[%d/%b/%Y:%H:%M:%S +0000]").to_string();
        let referer = self.referer.as_deref().unwrap_or("-");
        let ua = self.user_agent.as_deref().unwrap_or("-");
        format!(
            "{} - - {} \"{} {} {}\" {} {} \"{}\" \"{}\"",
            self.remote_addr,
            time_local,
            self.method,
            self.uri,
            self.proto,
            self.status,
            self.body_bytes_sent,
            referer,
            ua,
        )
    }

    pub fn render_json(&self) -> String {
        let dt: DateTime<Utc> = self.time.into();
        serde_json::json!({
            "remote_addr": self.remote_addr,
            "time": dt.to_rfc3339(),
            "method": self.method,
            "uri": self.uri,
            "proto": self.proto,
            "status": self.status,
            "body_bytes_sent": self.body_bytes_sent,
            "referer": self.referer,
            "user_agent": self.user_agent,
            "request_time_ms": self.request_time_ms,
            "upstream_response_time_ms": self.upstream_response_time_ms,
            "host": self.host,
        })
        .to_string()
    }

    pub fn render_custom(&self, tokens: &[LogToken]) -> String {
        let dt: DateTime<Utc> = self.time.into();
        let mut out = String::new();
        for token in tokens {
            let part = match token {
                LogToken::Literal(s) => s.clone(),
                LogToken::RemoteAddr => self.remote_addr.clone(),
                LogToken::TimeLocal => dt.format("[%d/%b/%Y:%H:%M:%S +0000]").to_string(),
                LogToken::Request => format!("{} {} {}", self.method, self.uri, self.proto),
                LogToken::Status => self.status.to_string(),
                LogToken::BodyBytesSent => self.body_bytes_sent.to_string(),
                LogToken::Referer => self.referer.clone().unwrap_or_else(|| "-".to_string()),
                LogToken::UserAgent => self.user_agent.clone().unwrap_or_else(|| "-".to_string()),
                LogToken::RequestTime => format!("{:.3}", self.request_time_ms as f64 / 1000.0),
                LogToken::UpstreamResponseTime => self
                    .upstream_response_time_ms
                    .map(|ms| format!("{:.3}", ms as f64 / 1000.0))
                    .unwrap_or_else(|| "-".to_string()),
                LogToken::Host => self.host.clone(),
                LogToken::Unknown(v) => format!("${v}"),
            };
            out.push_str(&part);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Log rotation config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RotationTrigger {
    OnSizeBytes(u64),
    Daily,
    Hourly,
}

#[derive(Debug, Clone)]
pub struct RotationConfig {
    pub trigger: RotationTrigger,
    pub max_files: usize,
    pub compress: bool,
}

impl RotationConfig {
    pub fn daily(max_files: usize, compress: bool) -> Self {
        Self {
            trigger: RotationTrigger::Daily,
            max_files,
            compress,
        }
    }

    pub fn on_size(bytes: u64, max_files: usize, compress: bool) -> Self {
        Self {
            trigger: RotationTrigger::OnSizeBytes(bytes),
            max_files,
            compress,
        }
    }

    /// Compute the rotated filename for sequence number n.
    pub fn rotated_name(base: &str, n: usize) -> String {
        format!("{}.{}", base, n)
    }

    /// Whether size threshold is exceeded
    pub fn size_exceeded(&self, current_bytes: u64) -> bool {
        if let RotationTrigger::OnSizeBytes(max) = self.trigger {
            current_bytes >= max
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Startup script generator
// ---------------------------------------------------------------------------

pub fn generate_systemd_unit(app_name: &str, exec_start: &str, user: &str) -> String {
    format!(
        "[Unit]\n\
         Description={app_name} - Aegis-Flow managed process\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         User={user}\n\
         ExecStart={exec_start}\n\
         Restart=always\n\
         RestartSec=1\n\
         StandardOutput=journal\n\
         StandardError=journal\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        app_name = app_name,
        user = user,
        exec_start = exec_start,
    )
}

pub fn generate_launchd_plist(app_name: &str, program_args: &[&str]) -> String {
    let args = program_args
        .iter()
        .map(|a| format!("        <string>{}</string>", a))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
             <key>Label</key>\n\
             <string>com.aegis-flow.{app_name}</string>\n\
             <key>ProgramArguments</key>\n\
             <array>\n\
         {args}\n\
             </array>\n\
             <key>KeepAlive</key>\n\
             <true/>\n\
             <key>RunAtLoad</key>\n\
             <true/>\n\
             <key>StandardOutPath</key>\n\
             <string>/tmp/{app_name}-stdout.log</string>\n\
             <key>StandardErrorPath</key>\n\
             <string>/tmp/{app_name}-stderr.log</string>\n\
         </dict>\n\
         </plist>\n",
        app_name = app_name,
        args = args,
    )
}

pub fn detect_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown"
    }
}

// ---------------------------------------------------------------------------
// Process list table renderer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProcessRow {
    pub id: usize,
    pub name: String,
    pub pid: Option<u32>,
    pub status: String,
    pub restarts: u32,
    pub cpu_pct: f32,
    pub mem_mb: f32,
}

pub fn render_process_table(rows: &[ProcessRow]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<4} {:<20} {:<8} {:<10} {:<8} {:<6} {:<8}",
        "ID", "Name", "PID", "Status", "Restart", "CPU%", "Mem(MB)"
    );
    let _ = writeln!(out, "{}", "-".repeat(70));
    for row in rows {
        let pid = row
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        let _ = writeln!(
            out,
            "{:<4} {:<20} {:<8} {:<10} {:<8} {:<6.1} {:<8.1}",
            row.id, row.name, pid, row.status, row.restarts, row.cpu_pct, row.mem_mb
        );
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn sample_record() -> AccessLogRecord {
        AccessLogRecord {
            remote_addr: "127.0.0.1".to_string(),
            time: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
            method: "GET".to_string(),
            uri: "/index.html".to_string(),
            proto: "HTTP/1.1".to_string(),
            status: 200,
            body_bytes_sent: 512,
            referer: Some("https://google.com".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            request_time_ms: 42,
            upstream_response_time_ms: Some(38),
            host: "example.com".to_string(),
        }
    }

    // --- Combined format ---
    #[test]
    fn test_combined_format() {
        let record = sample_record();
        let line = record.render_combined();
        assert!(line.starts_with("127.0.0.1"));
        assert!(line.contains("GET /index.html HTTP/1.1"));
        assert!(line.contains("200"));
        assert!(line.contains("512"));
        assert!(line.contains("google.com"));
    }

    // --- JSON format ---
    #[test]
    fn test_json_format() {
        let record = sample_record();
        let json_str = record.render_json();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["remote_addr"], "127.0.0.1");
        assert_eq!(v["status"], 200);
        assert_eq!(v["method"], "GET");
    }

    // --- Custom format ---
    #[test]
    fn test_custom_format_parse() {
        let tokens =
            parse_log_format("$remote_addr [$time_local] \"$request\" $status $body_bytes_sent");
        assert!(tokens.contains(&LogToken::RemoteAddr));
        assert!(tokens.contains(&LogToken::Status));
        assert!(tokens.contains(&LogToken::Request));
    }

    #[test]
    fn test_custom_format_render() {
        let tokens = parse_log_format("$status $request_time");
        let record = sample_record();
        let out = record.render_custom(&tokens);
        assert_eq!(out, "200 0.042");
    }

    // --- Log rotation ---
    #[test]
    fn test_rotation_size_exceeded() {
        let cfg = RotationConfig::on_size(1024 * 1024, 5, true);
        assert!(cfg.size_exceeded(2 * 1024 * 1024));
        assert!(!cfg.size_exceeded(512 * 1024));
    }

    #[test]
    fn test_rotation_daily_no_size_trigger() {
        let cfg = RotationConfig::daily(7, true);
        assert!(!cfg.size_exceeded(999_999_999));
    }

    #[test]
    fn test_rotated_filename() {
        assert_eq!(
            RotationConfig::rotated_name("access.log", 1),
            "access.log.1"
        );
        assert_eq!(
            RotationConfig::rotated_name("access.log", 3),
            "access.log.3"
        );
    }

    // --- Systemd unit ---
    #[test]
    fn test_systemd_unit_generation() {
        let unit =
            generate_systemd_unit("myapp", "/usr/bin/myapp --config /etc/myapp.toml", "root");
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("ExecStart=/usr/bin/myapp"));
        assert!(unit.contains("Restart=always"));
        assert!(unit.contains("WantedBy=multi-user.target"));
    }

    // --- Launchd plist ---
    #[test]
    fn test_launchd_plist_generation() {
        let plist =
            generate_launchd_plist("myapp", &["/usr/bin/myapp", "--config", "/etc/myapp.toml"]);
        assert!(plist.contains("com.aegis-flow.myapp"));
        assert!(plist.contains("KeepAlive"));
        assert!(plist.contains("RunAtLoad"));
        assert!(plist.contains("/usr/bin/myapp"));
    }

    // --- Platform detection ---
    #[test]
    fn test_platform_detection() {
        let platform = detect_platform();
        assert!(matches!(platform, "macos" | "linux" | "unknown"));
    }

    // --- Process table ---
    #[test]
    fn test_process_table_render() {
        let rows = vec![
            ProcessRow {
                id: 0,
                name: "api".to_string(),
                pid: Some(1234),
                status: "online".to_string(),
                restarts: 0,
                cpu_pct: 0.5,
                mem_mb: 64.0,
            },
            ProcessRow {
                id: 1,
                name: "worker".to_string(),
                pid: None,
                status: "stopped".to_string(),
                restarts: 3,
                cpu_pct: 0.0,
                mem_mb: 0.0,
            },
        ];
        let table = render_process_table(&rows);
        assert!(table.contains("api"));
        assert!(table.contains("online"));
        assert!(table.contains("1234"));
        assert!(table.contains("stopped"));
    }
}
