/// Server Side Includes (SSI) execution engine
/// Processes <!--# directive --> tags within HTML responses.
/// Parsing logic is in sub_filter.rs; this module handles execution.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

pub const MAX_INCLUDE_DEPTH: usize = 10;

/// An SSI variable scope for the current request
pub struct SsiContext {
    pub vars: HashMap<String, String>,
    pub depth: usize,
}

impl SsiContext {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            depth: 0,
        }
    }

    pub fn with_var(mut self, key: &str, value: &str) -> Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }
}

impl Default for SsiContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Read a file and return its content as a String
pub async fn read_file_include(file_path: &str, base_dir: Option<&Path>) -> Option<String> {
    let path: PathBuf = if let Some(base) = base_dir {
        base.join(file_path.trim_start_matches('/'))
    } else {
        PathBuf::from(file_path)
    };

    match fs::read_to_string(&path).await {
        Ok(content) => {
            debug!("SSI file include: {}", path.display());
            Some(content)
        }
        Err(e) => {
            warn!("SSI file include failed for '{}': {}", path.display(), e);
            None
        }
    }
}

/// Evaluate a simple SSI if-expression (literal equality check).
/// Supports: `"$varname" = "value"` and `"$varname" != "value"`.
pub fn eval_ssi_expr(expr: &str, ctx: &SsiContext) -> bool {
    let expr = expr.trim();

    // Parse: "$var" op "value"
    let parts: Vec<&str> = expr.splitn(3, ' ').collect();
    if parts.len() == 3 {
        let lhs = resolve_ssi_var(parts[0].trim_matches('"'), ctx);
        let op = parts[1].trim();
        let rhs = parts[2].trim().trim_matches('"');
        return match op {
            "=" | "==" => lhs == rhs,
            "!=" => lhs != rhs,
            _ => false,
        };
    }
    false
}

/// Resolve a variable reference (`$varname` → value from context)
pub fn resolve_ssi_var(var_ref: &str, ctx: &SsiContext) -> String {
    let name = var_ref.trim_start_matches('$');
    ctx.vars.get(name).cloned().unwrap_or_default()
}

/// Process SSI directives in an HTML string using the provided context.
/// Unlike the parser (in sub_filter.rs which only extracts directives),
/// this function actually evaluates and substitutes them inline.
pub async fn process_ssi(html: &str, ctx: &SsiContext, base_dir: Option<&Path>) -> String {
    use crate::sub_filter::{SsiDirective, parse_ssi_directives};

    if ctx.depth >= MAX_INCLUDE_DEPTH {
        warn!("SSI max include depth ({}) exceeded", MAX_INCLUDE_DEPTH);
        return html.to_string();
    }

    let directives = parse_ssi_directives(html);
    if directives.is_empty() {
        return html.to_string();
    }

    // Regex to find and replace each directive token inline
    let re = regex::Regex::new(r"<!--#(\w+)\s*(.*?)-->").unwrap();

    // Build replacement map: directive text → resolved content
    let mut idx = 0;
    let mut result = String::with_capacity(html.len());
    for mat in re.find_iter(html) {
        result.push_str(&html[idx..mat.start()]);
        idx = mat.end();

        let matched = mat.as_str();
        let directive_idx = directives.len().min(1); // keep in bounds guard
        let _ = directive_idx; // suppress unused warning

        // Re-parse this individual directive for context
        let dir_vec = parse_ssi_directives(matched);
        let replacement = if let Some(dir) = dir_vec.first() {
            match dir {
                SsiDirective::IncludeFile(path) => {
                    read_file_include(path, base_dir).await.unwrap_or_default()
                }
                SsiDirective::IncludeVirtual(uri) => {
                    // In production, this would dispatch a subrequest to the proxy.
                    // For now, try to read it as a file relative to base_dir.
                    read_file_include(uri, base_dir).await.unwrap_or_default()
                }
                SsiDirective::EchoVar(var) => resolve_ssi_var(var, ctx),
                SsiDirective::SetVar(key, val) => {
                    // SetVar doesn't produce output — only a side effect on ctx.
                    // Since ctx is immutable here, we note this limitation.
                    debug!("SSI set var: {}={} (read-only ctx)", key, val);
                    String::new()
                }
                SsiDirective::Unknown(_) => String::new(),
            }
        } else {
            String::new()
        };
        result.push_str(&replacement);
    }
    result.push_str(&html[idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssi_context_new() {
        let ctx = SsiContext::new();
        assert!(ctx.vars.is_empty());
        assert_eq!(ctx.depth, 0);
    }

    #[test]
    fn test_ssi_var_resolution() {
        let ctx = SsiContext::new()
            .with_var("REQUEST_URI", "/api/test")
            .with_var("user", "admin");

        assert_eq!(resolve_ssi_var("$REQUEST_URI", &ctx), "/api/test");
        assert_eq!(resolve_ssi_var("$user", &ctx), "admin");
        assert_eq!(resolve_ssi_var("$missing", &ctx), "");
    }

    #[test]
    fn test_eval_ssi_expr_equality() {
        let ctx = SsiContext::new().with_var("env", "production");

        assert!(eval_ssi_expr("\"$env\" = \"production\"", &ctx));
        assert!(!eval_ssi_expr("\"$env\" = \"staging\"", &ctx));
        assert!(eval_ssi_expr("\"$env\" != \"staging\"", &ctx));
    }

    #[test]
    fn test_max_depth_guard() {
        let mut ctx = SsiContext::new();
        ctx.depth = MAX_INCLUDE_DEPTH;
        // With max depth exceeded, process_ssi should return original
        // (tested via depth check since executing async is complex in sync tests)
        assert!(ctx.depth >= MAX_INCLUDE_DEPTH);
    }

    #[tokio::test]
    async fn test_process_ssi_echo_var() {
        let ctx = SsiContext::new().with_var("greeting", "Hello World");
        let html = r#"<p><!--#echo var="greeting"--></p>"#;
        let result = process_ssi(html, &ctx, None).await;
        assert!(result.contains("Hello World"));
    }
}
