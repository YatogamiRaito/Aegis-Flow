/// XSLT Response Transformation stub
/// Applies XSLT stylesheet to XML responses.
/// A full Rust XSLT processor is not yet widely available as a pure-Rust crate;
/// this module provides the configuration layer and a passthrough-safe stub
/// that can be connected to libxslt via FFI or a future pure-Rust implementation.
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XsltConfig {
    /// Path to the XSLT stylesheet file
    pub stylesheet: String,
    /// Additional XSLT parameters from request variables
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
}

/// Check if a content-type is XML processable
pub fn is_xml_content_type(content_type: &str) -> bool {
    let ct = content_type.split(';').next().unwrap_or("").trim();
    matches!(
        ct,
        "application/xml" | "text/xml" | "application/xhtml+xml" | "application/atom+xml"
    )
}

/// Apply XSLT transformation to XML response body.
/// Returns the transformed bytes and new content type.
///
/// # Current Status
/// This is a passthrough stub. To enable real XSLT:
/// 1. Add `libxslt` bindings via `libxslt-sys` crate
/// 2. Or wait for a mature pure-Rust XSLT implementation
pub fn apply_xslt(data: Bytes, config: &XsltConfig, content_type: &str) -> (Bytes, String) {
    if !is_xml_content_type(content_type) {
        return (data, content_type.to_string());
    }

    if config.stylesheet.is_empty() {
        return (data, content_type.to_string());
    }

    debug!(
        "XSLT transform requested (stylesheet={}, params={}) — passthrough enabled",
        config.stylesheet,
        config.params.len()
    );

    // Passthrough: return the original XML unchanged
    // The output type after XSLT transformation would typically be text/html
    (data, "text/html; charset=utf-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_xml_content_type() {
        assert!(is_xml_content_type("application/xml"));
        assert!(is_xml_content_type("text/xml"));
        assert!(is_xml_content_type("application/xhtml+xml"));
        assert!(!is_xml_content_type("text/html"));
        assert!(!is_xml_content_type("application/json"));
    }

    #[test]
    fn test_xslt_passthrough_non_xml() {
        let data = Bytes::from("<html>not xml</html>");
        let config = XsltConfig {
            stylesheet: "/transform.xslt".to_string(),
            ..Default::default()
        };
        let (result, ct) = apply_xslt(data.clone(), &config, "text/html");
        assert_eq!(result, data);
        assert_eq!(ct, "text/html");
    }

    #[test]
    fn test_xslt_passthrough_xml() {
        let data = Bytes::from("<?xml version='1.0'?><root/>");
        let config = XsltConfig {
            stylesheet: "/transform.xslt".to_string(),
            ..Default::default()
        };
        let (result, _ct) = apply_xslt(data.clone(), &config, "application/xml");
        // Passthrough: data unchanged
        assert_eq!(result, data);
    }

    #[test]
    fn test_xslt_empty_stylesheet_passthrough() {
        let data = Bytes::from("<?xml version='1.0'?><root/>");
        let config = XsltConfig::default();
        let (result, ct) = apply_xslt(data.clone(), &config, "application/xml");
        assert_eq!(result, data);
        assert_eq!(ct, "application/xml");
    }
}
