use regex::Regex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    pub source_var: String,
    pub target_var: String,
    pub default_value: Option<String>,
    #[serde(default)]
    pub mapping: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum MapValue {
    String(String),
    Regex(Regex, String), // pattern, value
}

#[derive(Debug)]
pub struct MapBlock {
    pub source_var: String,
    pub target_var: String,
    pub default: Option<String>,
    pub entries: Vec<(String, String)>,         // exact match
    pub regex_entries: Vec<(Regex, String)>,    // regex match
}

impl MapBlock {
    pub fn new(source_var: &str, target_var: &str) -> Self {
        Self {
            source_var: source_var.to_string(),
            target_var: target_var.to_string(),
            default: None,
            entries: Vec::new(),
            regex_entries: Vec::new(),
        }
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn add_exact(&mut self, key: &str, value: &str) {
        self.entries.push((key.to_string(), value.to_string()));
    }

    pub fn add_regex(&mut self, pattern: &str, value: &str) {
        if let Ok(re) = Regex::new(pattern) {
            self.regex_entries.push((re, value.to_string()));
        }
    }

    pub fn resolve(&self, input: &str) -> Option<String> {
        // Exact match first
        for (key, val) in &self.entries {
            if key == input {
                return Some(val.clone());
            }
        }
        
        // Then regex match
        for (re, val) in &self.regex_entries {
            if re.is_match(input) {
                return Some(val.clone());
            }
        }
        
        // Default fallback
        self.default.clone()
    }
}

// Split clients: percentage-based A/B bucketing
pub struct SplitClients {
    pub source_var: String,
    pub buckets: Vec<(f64, String)>, // (cumulative_percent, value)
}

impl SplitClients {
    pub fn new(source_var: &str, buckets: Vec<(f64, String)>) -> Self {
        Self {
            source_var: source_var.to_string(),
            buckets,
        }
    }

    pub fn resolve(&self, key: &str) -> Option<&str> {
        // Simple hash of key → bucket
        let hash = simple_hash(key);
        let percent = (hash % 100) as f64;
        
        let mut cumulative = 0.0;
        for (bucket_percent, val) in &self.buckets {
            cumulative += bucket_percent;
            if percent < cumulative {
                return Some(val.as_str());
            }
        }
        None
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_exact_match() {
        let mut map = MapBlock::new("$request_method", "$backend");
        map.add_exact("GET", "read_pool");
        map.add_exact("POST", "write_pool");
        
        assert_eq!(map.resolve("GET"), Some("read_pool".to_string()));
        assert_eq!(map.resolve("POST"), Some("write_pool".to_string()));
    }

    #[test]
    fn test_map_regex_match() {
        let mut map = MapBlock::new("$uri", "$cache_zone");
        map.add_regex(r"^/api/", "api_cache");
        map.add_regex(r"^/static/", "static_cache");
        
        assert_eq!(map.resolve("/api/users"), Some("api_cache".to_string()));
        assert_eq!(map.resolve("/static/style.css"), Some("static_cache".to_string()));
    }

    #[test]
    fn test_map_default() {
        let mut map = MapBlock::new("$method", "$backend");
        map.add_exact("GET", "read_pool");
        map = map.with_default("default_pool");
        
        assert_eq!(map.resolve("DELETE"), Some("default_pool".to_string()));
    }

    #[test]
    fn test_map_no_match() {
        let mut map = MapBlock::new("$method", "$backend");
        map.add_exact("GET", "read_pool");
        
        assert_eq!(map.resolve("PATCH"), None);
    }

    #[test]
    fn test_split_clients_consistent() {
        let sc = SplitClients::new("$remote_addr", vec![
            (50.0, "version_a".to_string()),
            (50.0, "version_b".to_string()),
        ]);
        
        let v1 = sc.resolve("192.168.1.1");
        let v2 = sc.resolve("192.168.1.1");
        
        // Same IP should always get the same variant
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_split_clients_distribution() {
        let sc = SplitClients::new("$remote_addr", vec![
            (50.0, "a".to_string()),
            (50.0, "b".to_string()),
        ]);
        
        // Test a few known IPs
        let results: Vec<&str> = (0..10)
            .map(|i| sc.resolve(&format!("192.168.1.{}", i)).unwrap_or("unknown"))
            .collect();
            
        // Not all should be the same (very unlikely)
        let has_a = results.contains(&"a");
        let has_b = results.contains(&"b");
        // at least one of the variants should appear
        assert!(has_a || has_b);
    }
}
