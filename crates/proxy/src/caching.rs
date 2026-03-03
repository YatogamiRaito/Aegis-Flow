use std::fs::File;
use std::path::Path;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use lru::LruCache;
use std::num::NonZeroUsize;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EtagMode {
    #[default]
    Weak,
    Strong,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub etag_mode: EtagMode,
    // e.g. ("js", "max-age=31536000")
    pub cache_control_rules: Vec<(String, String)>,
    // Add Expires header 30 days from now if true
    pub add_expires_header: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            etag_mode: EtagMode::Weak,
            cache_control_rules: vec![],
            add_expires_header: false,
        }
    }
}

#[derive(Clone)]
pub struct CachingManager {
    config: CacheConfig,
    strong_etag_cache: Arc<parking_lot::RwLock<LruCache<String, String>>>,
}

pub fn format_http_date(time: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

pub fn parse_http_date(date_str: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(date_str)
        .map(|d| d.with_timezone(&Utc))
        .ok()
}

impl CachingManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            strong_etag_cache: Arc::new(parking_lot::RwLock::new(LruCache::new(NonZeroUsize::new(1024).unwrap()))),
        }
    }

    pub fn generate_etag(&self, path: &Path, metadata: &std::fs::Metadata) -> Option<String> {
        match self.config.etag_mode {
            EtagMode::Disabled => None,
            EtagMode::Weak => {
                let size = metadata.len();
                let mtime = metadata.modified().ok()?
                    .duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                // Simple weak etag hash format
                Some(format!("W/\"{x}-{y}\"", x=hex::encode(size.to_be_bytes()), y=hex::encode(mtime.to_be_bytes())))
            }
            EtagMode::Strong => {
                let p = path.to_string_lossy().to_string();
                let mtime = metadata.modified().ok()?
                    .duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                let cache_key = format!("{}-{}", p, mtime);
                
                // check cache first
                {
                    let mut lock = self.strong_etag_cache.write();
                    if let Some(tag) = lock.get(&cache_key) {
                        return Some(tag.clone());
                    }
                }
                
                // compute sha256
                let mut f = File::open(path).ok()?;
                let mut hasher = Sha256::new();
                std::io::copy(&mut f, &mut hasher).ok()?;
                let result = hasher.finalize();
                let tag = format!("\"{}\"", hex::encode(result));
                
                let mut lock = self.strong_etag_cache.write();
                lock.put(cache_key, tag.clone());
                Some(tag)
            }
        }
    }

    pub fn check_conditional(&self, req_headers: &hyper::HeaderMap, etag: Option<&str>, mtime: Option<std::time::SystemTime>) -> bool {
        // Return true if we should return 304 Not Modified
        
        let mut not_modified = false;
        let mut has_etag_conditional = false;

        // ETag check
        if let Some(if_none_match) = req_headers.get(hyper::header::IF_NONE_MATCH) {
             has_etag_conditional = true;
             if let Ok(if_none_match_str) = if_none_match.to_str() {
                 if let Some(tag) = etag {
                     // Check if tag is in the if-none-match list. For weak etags, W/"hash" will match W/"hash"
                     if if_none_match_str.contains(tag) || if_none_match_str == "*" {
                         not_modified = true;
                     }
                 }
             }
        }

        // If If-None-Match is present, it takes precedence (HTTP/1.1 spec says evaluate IMS only if INM not present or doesn't match, 
        // but if INM doesn't match, we must send 200, bypassing IMS)
        if has_etag_conditional {
            return not_modified;
        }

        // Mtime check
        if let Some(if_modified_since) = req_headers.get(hyper::header::IF_MODIFIED_SINCE) {
            if let Ok(ims_str) = if_modified_since.to_str() {
                // HTTP specs allow a few formats, we parse RFC2822 as it matches what most browsers send
                if let Some(ims_date) = parse_http_date(ims_str) {
                    if let Some(mt) = mtime {
                        let mtime_date: DateTime<Utc> = mt.into();
                        // Truncate sub-second precision to match RFC2822
                        if mtime_date.timestamp() <= ims_date.timestamp() {
                            not_modified = true;
                        }
                    }
                }
            }
        }

        not_modified
    }

    pub fn get_cache_control(&self, path: &Path) -> Option<String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        for (pattern, header) in &self.config.cache_control_rules {
            if pattern.starts_with("*.") {
                let p_ext = &pattern[2..];
                if ext == p_ext { return Some(header.clone()); }
            } else if pattern == ext { // Check exact ext without *.
                return Some(header.clone());
            }
        }
        None
    }

    pub fn get_expires_header(&self) -> Option<String> {
        if self.config.add_expires_header {
            let expires_dt = Utc::now() + chrono::Duration::days(30);
            Some(expires_dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;
    use hyper::header::{HeaderMap, HeaderValue, IF_NONE_MATCH, IF_MODIFIED_SINCE};

    #[test]
    fn test_weak_etag() {
        let manager = CachingManager::new(CacheConfig {
            etag_mode: EtagMode::Weak,
            ..Default::default()
        });

        let dir = tempdir().unwrap();
        let path = dir.path().join("weak.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"weak etag test").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();

        let etag = manager.generate_etag(&path, &metadata).unwrap();
        assert!(etag.starts_with("W/\""));
    }

    #[test]
    fn test_strong_etag() {
        let manager = CachingManager::new(CacheConfig {
            etag_mode: EtagMode::Strong,
            ..Default::default()
        });

        let dir = tempdir().unwrap();
        let path = dir.path().join("strong.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"strong etag test").unwrap();
        let metadata = std::fs::metadata(&path).unwrap();

        let etag = manager.generate_etag(&path, &metadata).unwrap();
        // hash of "strong etag test"
        // echo -n "strong etag test" | shasum -a 256
        assert!(etag.starts_with("\"") && !etag.starts_with("W/"));
        
        let etag_cache = manager.generate_etag(&path, &metadata).unwrap();
        assert_eq!(etag, etag_cache); // Hit cache
    }

    #[test]
    fn test_etag_disabled() {
        let manager = CachingManager::new(CacheConfig {
            etag_mode: EtagMode::Disabled,
            ..Default::default()
        });
        
        let dir = tempdir().unwrap();
        let path = dir.path().join("dis.txt");
        File::create(&path).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(manager.generate_etag(&path, &metadata), None);
    }

    #[test]
    fn test_if_none_match() {
        let manager = CachingManager::new(CacheConfig::default());
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("W/\"abc\""));

        assert!(manager.check_conditional(&headers, Some("W/\"abc\""), None));
        assert!(!manager.check_conditional(&headers, Some("W/\"xyz\""), None));
    }

    #[test]
    fn test_if_modified_since() {
        let manager = CachingManager::new(CacheConfig::default());
        let mut headers = HeaderMap::new();
        
        let mtime = std::time::SystemTime::now();
        
        // Use a future date for If-Modified-Since to trigger Not Modified
        let ims = mtime.checked_add(std::time::Duration::from_secs(3600)).unwrap();
        let ims_str = format_http_date(ims);
        
        headers.insert(IF_MODIFIED_SINCE, HeaderValue::from_str(&ims_str).unwrap());
        
        assert!(manager.check_conditional(&headers, None, Some(mtime)));
        
        // Use past date for If-Modified-Since to trigger 200 OK (not modified = false)
        let mut old_headers = HeaderMap::new();
        let past = mtime.checked_sub(std::time::Duration::from_secs(3600)).unwrap();
        let past_str = format_http_date(past);
        old_headers.insert(IF_MODIFIED_SINCE, HeaderValue::from_str(&past_str).unwrap());
        
        assert!(!manager.check_conditional(&old_headers, None, Some(mtime)));
    }

    #[test]
    fn test_cache_control_assignment() {
        let mut rules = Vec::new();
        rules.push(("*.js".to_string(), "max-age=31536000".to_string()));
        rules.push(("css".to_string(), "max-age=86400".to_string()));

        let manager = CachingManager::new(CacheConfig {
            cache_control_rules: rules,
            ..Default::default()
        });

        assert_eq!(manager.get_cache_control(Path::new("app.js")).unwrap(), "max-age=31536000");
        assert_eq!(manager.get_cache_control(Path::new("style.css")).unwrap(), "max-age=86400");
        assert_eq!(manager.get_cache_control(Path::new("index.html")), None);
    }

    #[test]
    fn test_expires_header() {
        let manager = CachingManager::new(CacheConfig {
            add_expires_header: true,
            ..Default::default()
        });

        let expires = manager.get_expires_header().unwrap();
        // Should be RFC2822 valid format containing GMT
        assert!(expires.ends_with("GMT"));
    }
}
