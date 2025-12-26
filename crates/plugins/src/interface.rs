//! Plugin Interface
//!
//! Defines the data structures for plugin communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request data passed to plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    /// Request ID
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (if any)
    pub body: Option<Vec<u8>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PluginRequest {
    /// Create a new plugin request
    pub fn new(id: &str, method: &str, path: &str) -> Self {
        Self {
            id: id.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    /// Set the body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Response data returned from plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Whether the request should continue processing
    pub continue_processing: bool,
    /// Modified headers (if any)
    pub modified_headers: Option<HashMap<String, String>>,
    /// Modified body (if any)
    pub modified_body: Option<Vec<u8>>,
    /// Response to send immediately (if set, skips upstream)
    pub immediate_response: Option<ImmediateResponse>,
    /// Metadata to pass to next plugin
    pub metadata: HashMap<String, String>,
}

impl Default for PluginResponse {
    fn default() -> Self {
        Self {
            continue_processing: true,
            modified_headers: None,
            modified_body: None,
            immediate_response: None,
            metadata: HashMap::new(),
        }
    }
}

impl PluginResponse {
    /// Create a continue response
    pub fn continue_request() -> Self {
        Self::default()
    }

    /// Create a response that stops processing
    pub fn stop() -> Self {
        Self {
            continue_processing: false,
            ..Default::default()
        }
    }

    /// Create an immediate response
    pub fn immediate(status: u16, body: &str) -> Self {
        Self {
            continue_processing: false,
            immediate_response: Some(ImmediateResponse {
                status,
                body: body.to_string(),
                headers: HashMap::new(),
            }),
            ..Default::default()
        }
    }

    /// Add modified header
    pub fn with_modified_header(mut self, name: &str, value: &str) -> Self {
        self.modified_headers
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value.to_string());
        self
    }
}

/// Immediate response to send without upstream processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmediateResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body
    pub body: String,
    /// Response headers
    pub headers: HashMap<String, String>,
}

/// Result of plugin execution
#[derive(Debug, Clone)]
pub struct PluginResult {
    /// Plugin name
    pub plugin_name: String,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Response from plugin
    pub response: PluginResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_request_creation() {
        let req = PluginRequest::new("req-1", "GET", "/api/data")
            .with_header("content-type", "application/json")
            .with_metadata("user_id", "123");

        assert_eq!(req.id, "req-1");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/data");
        assert_eq!(req.headers.get("content-type").unwrap(), "application/json");
        assert_eq!(req.metadata.get("user_id").unwrap(), "123");
    }

    #[test]
    fn test_plugin_response_continue() {
        let resp = PluginResponse::continue_request();
        assert!(resp.continue_processing);
        assert!(resp.immediate_response.is_none());
    }

    #[test]
    fn test_plugin_response_immediate() {
        let resp = PluginResponse::immediate(403, "Forbidden");
        assert!(!resp.continue_processing);
        assert!(resp.immediate_response.is_some());

        let imm = resp.immediate_response.unwrap();
        assert_eq!(imm.status, 403);
        assert_eq!(imm.body, "Forbidden");
    }

    #[test]
    fn test_plugin_response_with_modified_headers() {
        let resp = PluginResponse::continue_request().with_modified_header("x-plugin", "processed");

        assert!(resp.modified_headers.is_some());
        let headers = resp.modified_headers.unwrap();
        assert_eq!(headers.get("x-plugin").unwrap(), "processed");
    }
}
