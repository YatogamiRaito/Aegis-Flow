use crate::variables::VariableResolver;
use hyper::http::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeadersConfig {
    #[serde(default)]
    pub proxy_set_header: HashMap<String, String>,
    #[serde(default)]
    pub add_header: HashMap<String, String>,
    #[serde(default)]
    pub proxy_hide_header: Vec<String>,
    #[serde(default)]
    pub security_headers: bool,
}

pub fn apply_proxy_set_header(
    headers: &mut HeaderMap,
    config: &HeadersConfig,
    resolver: &VariableResolver,
) {
    for (k, v) in &config.proxy_set_header {
        let interpolated = resolver.interpolate(v);
        if let Ok(val) = HeaderValue::from_str(&interpolated) {
            if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                headers.insert(name, val);
            }
        }
    }
}

pub fn apply_add_header(
    headers: &mut HeaderMap,
    config: &HeadersConfig,
    resolver: &VariableResolver,
) {
    for (k, v) in &config.add_header {
        let interpolated = resolver.interpolate(v);
        if let Ok(val) = HeaderValue::from_str(&interpolated) {
            if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                headers.insert(name, val);
            }
        }
    }

    if config.security_headers {
        let presets = [
            ("strict-transport-security", "max-age=63072000; includeSubDomains; preload"),
            ("x-frame-options", "DENY"),
            ("x-content-type-options", "nosniff"),
            ("x-xss-protection", "1; mode=block"),
            ("referrer-policy", "strict-origin-when-cross-origin"),
        ];
        for (k, v) in presets {
            if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                let val = HeaderValue::from_static(v);
                // Only insert if it doesn't already exist to allow overrides
                if !headers.contains_key(&name) {
                    headers.insert(name, val);
                }
            }
        }
    }
}

pub fn apply_proxy_hide_header(headers: &mut HeaderMap, config: &HeadersConfig) {
    for k in &config.proxy_hide_header {
        if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
            headers.remove(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variables::RequestContext;
    use hyper::http::{Method, Uri};

    #[test]
    fn test_apply_proxy_set_header() {
        let mut config = HeadersConfig::default();
        config.proxy_set_header.insert("x-real-ip".to_string(), "$remote_addr".to_string());
        config.proxy_set_header.insert("x-forwarded-for".to_string(), "$remote_addr".to_string());
        config.proxy_set_header.insert("host".to_string(), "$server_name".to_string());

        let req_headers = HeaderMap::new();
        let uri = "/".parse::<Uri>().unwrap();
        let method = Method::GET;
        let ctx = RequestContext {
            uri: &uri,
            method: &method,
            headers: &req_headers,
            remote_addr: "10.0.0.1",
            server_name: "backend.internal",
            server_port: 80,
            request_uri: "/",
            scheme: "http",
        };
        let resolver = VariableResolver::new(ctx, None);

        let mut out_headers = HeaderMap::new();
        apply_proxy_set_header(&mut out_headers, &config, &resolver);

        assert_eq!(out_headers.get("x-real-ip").unwrap(), "10.0.0.1");
        assert_eq!(out_headers.get("x-forwarded-for").unwrap(), "10.0.0.1");
        assert_eq!(out_headers.get("host").unwrap(), "backend.internal");
    }

    #[test]
    fn test_apply_add_header_and_security() {
        let mut config = HeadersConfig::default();
        config.add_header.insert("x-custom-id".to_string(), "req-$server_port".to_string());
        config.security_headers = true;

        let req_headers = HeaderMap::new();
        let uri = "/".parse::<Uri>().unwrap();
        let method = Method::GET;
        let ctx = RequestContext {
            uri: &uri,
            method: &method,
            headers: &req_headers,
            remote_addr: "10.0.0.1",
            server_name: "backend.internal",
            server_port: 443,
            request_uri: "/",
            scheme: "https",
        };
        let resolver = VariableResolver::new(ctx, None);

        let mut out_headers = HeaderMap::new();
        apply_add_header(&mut out_headers, &config, &resolver);

        assert_eq!(out_headers.get("x-custom-id").unwrap(), "req-443");
        assert_eq!(out_headers.get("x-frame-options").unwrap(), "DENY");
        assert_eq!(out_headers.get("x-content-type-options").unwrap(), "nosniff");
    }

    #[test]
    fn test_apply_proxy_hide_header() {
        let mut config = HeadersConfig::default();
        config.proxy_hide_header.push("x-powered-by".to_string());
        config.proxy_hide_header.push("server".to_string());

        let mut out_headers = HeaderMap::new();
        out_headers.insert("x-powered-by", HeaderValue::from_static("Express"));
        out_headers.insert("server", HeaderValue::from_static("nginx"));
        out_headers.insert("content-type", HeaderValue::from_static("text/html"));

        apply_proxy_hide_header(&mut out_headers, &config);

        assert!(out_headers.get("x-powered-by").is_none());
        assert!(out_headers.get("server").is_none());
        assert_eq!(out_headers.get("content-type").unwrap(), "text/html");
    }
}
