use hyper::http::{HeaderMap, Method, Uri};

pub struct RequestContext<'a> {
    pub uri: &'a Uri,
    pub method: &'a Method,
    pub headers: &'a HeaderMap,
    pub remote_addr: &'a str,
    pub server_name: &'a str,
    pub server_port: u16,
    pub request_uri: &'a str,
    pub scheme: &'a str,
}

pub struct VariableResolver<'a> {
    ctx: RequestContext<'a>,
    config: Option<&'a crate::config::ProxyConfig>,
}

impl<'a> VariableResolver<'a> {
    pub fn new(ctx: RequestContext<'a>, config: Option<&'a crate::config::ProxyConfig>) -> Self {
        Self { ctx, config }
    }

    pub fn resolve(&self, var_name: &str) -> Option<String> {
        match var_name {
            "uri" => Some(self.ctx.uri.path().to_string()),
            "args" => Some(self.ctx.uri.query().unwrap_or("").to_string()),
            "host" => {
                // Try from host header first, then fallback to something else
                if let Some(host) = self.ctx.headers.get("host") {
                    if let Ok(host_str) = host.to_str() {
                        return Some(host_str.to_string());
                    }
                }
                Some(self.ctx.server_name.to_string())
            }
            "request_uri" => Some(self.ctx.request_uri.to_string()),
            "scheme" => Some(self.ctx.scheme.to_string()),
            "remote_addr" => Some(self.ctx.remote_addr.to_string()),
            "server_name" => Some(self.ctx.server_name.to_string()),
            "server_port" => Some(self.ctx.server_port.to_string()),
            "request_method" => Some(self.ctx.method.as_str().to_string()),
            _ => {
                // Check if it's an HTTP header e.g. $http_user_agent
                if let Some(header_name) = var_name.strip_prefix("http_") {
                    let header_name = header_name.replace("_", "-");
                    if let Some(val) = self.ctx.headers.get(&header_name) {
                        if let Ok(val_str) = val.to_str() {
                            return Some(val_str.to_string());
                        }
                    }
                }
                
                // Then check split_clients dynamically if a config was passed
                if let Some(cfg) = self.config {
                    // Create dummy request for evaluation since we don't hold the original request body
                    let req = hyper::Request::builder().uri(self.ctx.uri.clone()).body(()).unwrap();
                    for split in &cfg.split_clients {
                        // Nginx variables are defined WITH the '$' (e.g. `$variant`), but `var_name` here strips it.
                        // We check if the trimmed config match equals `var_name`.
                        if split.variable.trim_start_matches('$') == var_name {
                            return Some(crate::split_clients::evaluate_split_client(split, &req, self.ctx.remote_addr));
                        }
                    }
                }
                None
            }
        }
    }

    pub fn interpolate(&self, template: &str) -> String {
        // Fast path: if no $ sign, return as is
        if !template.contains('$') {
            return template.to_string();
        }

        let mut result = String::with_capacity(template.len() + 16);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                // Check for valid variable name characters [a-zA-Z0-9_]
                let mut var_name = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' {
                        var_name.push(next_c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if var_name.is_empty() {
                    result.push('$');
                } else if let Some(val) = self.resolve(&var_name) {
                    result.push_str(&val);
                }
                // If not found, Nginx inserts an empty string
            } else {
                result.push(c);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_resolution() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "api.example.com".parse().unwrap());
        headers.insert("user-agent", "curl/7.68.0".parse().unwrap());

        let uri = "https://api.example.com/search?q=rust".parse::<Uri>().unwrap();
        let method = Method::GET;

        let ctx = RequestContext {
            uri: &uri,
            method: &method,
            headers: &headers,
            remote_addr: "192.168.1.100",
            server_name: "example.com",
            server_port: 8080,
            request_uri: "/search?q=rust",
            scheme: "https",
        };

        let resolver = VariableResolver::new(ctx, None);

        assert_eq!(resolver.resolve("uri"), Some("/search".to_string()));
        assert_eq!(resolver.resolve("args"), Some("q=rust".to_string()));
        assert_eq!(resolver.resolve("host"), Some("api.example.com".to_string()));
        assert_eq!(resolver.resolve("scheme"), Some("https".to_string()));
        assert_eq!(resolver.resolve("remote_addr"), Some("192.168.1.100".to_string()));
        assert_eq!(resolver.resolve("server_port"), Some("8080".to_string()));
        assert_eq!(resolver.resolve("http_user_agent"), Some("curl/7.68.0".to_string()));
        assert_eq!(resolver.resolve("unknown_var"), None);

        // Interpolation
        assert_eq!(
            resolver.interpolate("http://$host$uri?$args"),
            "http://api.example.com/search?q=rust"
        );
        assert_eq!(
            resolver.interpolate("Client: $remote_addr, UA: $http_user_agent"),
            "Client: 192.168.1.100, UA: curl/7.68.0"
        );
        assert_eq!(
            resolver.interpolate("Missing: $unknown_var here"),
            "Missing:  here"
        );
    }
}
