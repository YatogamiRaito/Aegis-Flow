use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StaticFileError {
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StaticFileConfig {
    pub root: PathBuf,
    pub index: Vec<String>,
    pub autoindex: bool,
    pub follow_symlinks: bool,
    pub hide_dot_files: bool,
    #[serde(default)]
    pub compression: crate::compression::CompressionConfig,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            index: vec!["index.html".to_string(), "index.htm".to_string()],
            autoindex: false,
            follow_symlinks: true,
            hide_dot_files: true,
            compression: Default::default(),
        }
    }
}

pub struct StaticFileServer {
    config: StaticFileConfig,
}

impl StaticFileServer {
    pub fn new(config: StaticFileConfig) -> Self {
        Self { config }
    }

    /// Resolves returning absolute normalized path avoiding directory traversal.
    pub fn resolve_path(&self, uri_path: &str) -> Result<PathBuf, StaticFileError> {
        let uri_path = uri_path.trim_start_matches('/');
        
        // Prevent obvious directory traversal attempts
        if uri_path.contains("../") || uri_path.contains("..\\") {
            return Err(StaticFileError::PathTraversal(uri_path.to_string()));
        }

        // Basic normalization
        let joined = self.config.root.join(uri_path);
        
        // Attempt canonicalize to completely resolve symlinks and '..'s
        // Because canonicalize fails if path doesn't exist, we must handle carefully.
        let resolved = match joined.canonicalize() {
            Ok(c) => c,
            Err(e) => {
                // Return NotFound if piece doesn't exist
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Err(StaticFileError::NotFound(uri_path.to_string()));
                }
                return Err(StaticFileError::Io(e));
            }
        };

        let root_canonical = self.config.root.canonicalize().map_err(|_| {
            StaticFileError::NotFound("Root directory does not exist".to_string())
        })?;

        // After all resolutions, path must still start with root
        if !resolved.starts_with(&root_canonical) {
            return Err(StaticFileError::PathTraversal("Path escapes root".to_string()));
        }
        
        // Check for dotfiles exclusion
        if self.config.hide_dot_files && Self::contains_dotfile(Path::new(uri_path)) {
             return Err(StaticFileError::Forbidden("Hidden file access denied".to_string()));
         }

        Ok(resolved)
    }

    /// Checks if the path contains any hidden files/directories (.git, .env, etc.)
    fn contains_dotfile(path: &Path) -> bool {
        for component in path.components() {
            if let std::path::Component::Normal(p) = component {
                if p.to_string_lossy().starts_with('.') {
                    return true;
                }
            }
        }
        false
    }
    
    /// Resolve an index file if the path is a directory
    pub fn resolve_index(&self, dir_path: &Path) -> Option<PathBuf> {
        if !dir_path.is_dir() {
            return None;
        }

        for idx in &self.config.index {
            let candidate = dir_path.join(idx);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        None
    }

    /// Try multiple paths in order, fallback to uri_path if None matched
    pub fn try_files(&self, uri_path: &str, try_paths: &[String]) -> Result<PathBuf, StaticFileError> {
        for path_expr in try_paths {
            // Replace $uri with actual uri
            let candidate_uri = path_expr.replace("$uri", uri_path);
            
            // If it ends with /, it could be a directory matching index
            let is_dir_check = candidate_uri.ends_with('/');
            
            match self.resolve_path(&candidate_uri) {
                Ok(resolved) => {
                    if resolved.is_file() && !is_dir_check {
                        return Ok(resolved);
                    } else if resolved.is_dir() {
                        if let Some(index_path) = self.resolve_index(&resolved) {
                            return Ok(index_path);
                        }
                    }
                }
                Err(StaticFileError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        // If all fail, return NotFound or fallback logic
        Err(StaticFileError::NotFound(uri_path.to_string()))
    }

    /// Builds an HTTP response with proper headers for a file
    /// Builds an HTTP response with proper headers for a file, handling Range requests if headers provided
    pub fn serve_file(
        &self, 
        path: &Path, 
        req_headers: Option<&hyper::HeaderMap>,
        override_mime: Option<&std::collections::HashMap<String, String>>
    ) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, StaticFileError> {
        use std::os::unix::fs::MetadataExt;
        
        let metadata = std::fs::metadata(path).map_err(StaticFileError::Io)?;
        
        if metadata.is_dir() {
            return Err(StaticFileError::Forbidden("Cannot serve directory".to_string()));
        }

        let mime = crate::mime_types::get_mime_type(path, override_mime);
        let mut file = std::fs::File::open(path).map_err(StaticFileError::Io)?;
        let size = metadata.len();
        let mtime_str = metadata.mtime().to_string(); // Placeholder format

        let mut status = hyper::StatusCode::OK;
        let mut content_length = size;
        let mut content_type = mime.clone();
        let mut content_range = None;
        let mut body_bytes = Vec::new();

        // Check for range request
        let mut is_range = false;
        if let Some(headers) = req_headers {
            if let Some(range_val) = headers.get(hyper::header::RANGE) {
                if let Ok(range_str) = range_val.to_str() {
                    if let Some(parsed_ranges) = crate::ranges::HttpRange::parse(range_str) {
                        is_range = true;
                        
                        let mut resolved_ranges = Vec::new();
                        for r in parsed_ranges {
                            if let Some(resolved) = r.resolve(size) {
                                resolved_ranges.push(resolved);
                            }
                        }

                        if resolved_ranges.is_empty() {
                            // Unsatisfiable
                            let resp = hyper::Response::builder()
                                .status(hyper::StatusCode::RANGE_NOT_SATISFIABLE)
                                .header("Content-Range", format!("bytes */{}", size))
                                .body(http_body_util::Full::new(bytes::Bytes::new()))
                                .unwrap();
                            return Ok(resp);
                        }

                        status = hyper::StatusCode::PARTIAL_CONTENT;

                        if resolved_ranges.len() == 1 {
                            // Single range
                            use std::io::{Read, Seek, SeekFrom};
                            let (start, end) = resolved_ranges[0];
                            let length = end - start + 1;
                            
                            file.seek(SeekFrom::Start(start)).map_err(StaticFileError::Io)?;
                            let mut buf = vec![0; length as usize];
                            file.read_exact(&mut buf).map_err(StaticFileError::Io)?;
                            
                            body_bytes = buf;
                            content_length = length;
                            content_range = Some(format!("bytes {}-{}/{}", start, end, size));
                        } else {
                            // Multi-part ranges
                            use std::io::{Read, Seek, SeekFrom};
                            let boundary = crate::ranges::generate_boundary();
                            content_type = format!("multipart/byteranges; boundary={}", boundary);
                            
                            for (start, end) in resolved_ranges {
                                let mut part = format!("\r\n--{}\r\nContent-Type: {}\r\nContent-Range: bytes {}-{}/{}\r\n\r\n", 
                                    boundary, mime, start, end, size).into_bytes();
                                body_bytes.append(&mut part);
                                
                                let length = end - start + 1;
                                file.seek(SeekFrom::Start(start)).map_err(StaticFileError::Io)?;
                                let mut buf = vec![0; length as usize];
                                file.read_exact(&mut buf).map_err(StaticFileError::Io)?;
                                body_bytes.append(&mut buf);
                            }
                            
                            let mut end_boundary = format!("\r\n--{}--\r\n", boundary).into_bytes();
                            body_bytes.append(&mut end_boundary);
                            content_length = body_bytes.len() as u64;
                        }
                    }
                }
            }
        }

        if !is_range {
            use std::io::Read;
            // Read whole file
            body_bytes = vec![0; size as usize];
            file.read_exact(&mut body_bytes).map_err(StaticFileError::Io)?;
        }

        let mut content_encoding = None;

        // Apply compression only if not a range request and status is OK
        if !is_range && status == hyper::StatusCode::OK {
            let algo = crate::compression::negotiate_encoding(req_headers, &self.config.compression, &content_type, size);
            if algo != crate::compression::CompressionAlgo::None {
                if let Some(compressed) = crate::compression::compress_body(&body_bytes, algo, &self.config.compression) {
                    body_bytes = compressed;
                    content_length = body_bytes.len() as u64;
                    content_encoding = Some(match algo {
                        crate::compression::CompressionAlgo::Gzip => "gzip",
                        crate::compression::CompressionAlgo::Brotli => "br",
                        _ => unreachable!(),
                    });
                }
            }
        }

        let mut builder = hyper::Response::builder()
            .status(status)
            .header("Content-Type", content_type)
            .header("Content-Length", content_length.to_string())
            .header("Last-Modified", mtime_str)
            .header("Accept-Ranges", "bytes");

        if let Some(cr) = content_range {
            builder = builder.header("Content-Range", cr);
        }

        if let Some(ce) = content_encoding {
            builder = builder.header("Content-Encoding", ce);
        }

        let response = builder
            .body(http_body_util::Full::new(bytes::Bytes::from(body_bytes)))
            .unwrap();

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;

    #[test]
    fn test_path_traversal_prevention() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        let config = StaticFileConfig {
            root: root.clone(),
            ..Default::default()
        };
        let server = StaticFileServer::new(config);

        // Valid path within root
        let safe_file = root.join("safe.txt");
        File::create(&safe_file).unwrap();
        let resolved = server.resolve_path("safe.txt").unwrap();
        assert_eq!(resolved, safe_file.canonicalize().unwrap());

        // Literal traversal attempt string
        let err = server.resolve_path("../etc/passwd");
        assert!(matches!(err, Err(StaticFileError::PathTraversal(_))));

        // Sneak traversal attempt resolving behind back
        let outside_file = root.parent().unwrap().join("outside.txt");
        File::create(&outside_file).ok(); // Might fail due to perms, but test logic holds
        if outside_file.exists() {
             let sneak_err = server.resolve_path("../outside.txt");
             assert!(matches!(sneak_err, Err(StaticFileError::PathTraversal(_))));
        }
    }

    #[test]
    fn test_index_resolution() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        File::create(root.join("index.html")).unwrap();
        
        let config = StaticFileConfig {
            root: root.clone(),
            ..Default::default()
        };
        let server = StaticFileServer::new(config);

        let resolved_index = server.resolve_index(&root).unwrap();
        assert_eq!(resolved_index, root.join("index.html"));
        
        // No index
        let empty_dir = tempdir().unwrap();
        assert!(server.resolve_index(empty_dir.path()).is_none());
    }

    #[test]
    fn test_dotfile_blocking() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = StaticFileConfig {
            root: root.clone(),
            hide_dot_files: true,
            ..Default::default()
        };
        let server = StaticFileServer::new(config);

        let dotfile = root.join(".env");
        File::create(&dotfile).unwrap();

        let err = server.resolve_path(".env");
        assert!(matches!(err, Err(StaticFileError::Forbidden(_))));

        let secret_dir = root.join(".git");
        std::fs::create_dir(&secret_dir).unwrap();
        let nested_file = secret_dir.join("config");
        File::create(&nested_file).unwrap();

        let err2 = server.resolve_path(".git/config");
        assert!(matches!(err2, Err(StaticFileError::Forbidden(_))));
    }

    #[test]
    fn test_serve_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = StaticFileConfig {
            root: root.clone(),
            ..Default::default()
        };
        let server = StaticFileServer::new(config);

        let file_path = root.join("hello.txt");
        std::fs::write(&file_path, b"Hello World").unwrap();

        // No range headers
        let resp = server.serve_file(&file_path, None, None).unwrap();
        assert_eq!(resp.status(), hyper::StatusCode::OK);
        assert_eq!(resp.headers().get("Content-Type").unwrap(), "text/plain; charset=utf-8");
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "11");
        assert_eq!(resp.headers().get("Accept-Ranges").unwrap(), "bytes");
    }

    #[test]
    fn test_serve_file_range() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let server = StaticFileServer::new(StaticFileConfig { root: root.clone(), ..Default::default() });
        let file_path = root.join("test.txt");
        std::fs::write(&file_path, b"0123456789").unwrap();

        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::RANGE, "bytes=0-4".parse().unwrap()); // first 5
        let resp = server.serve_file(&file_path, Some(&headers), None).unwrap();
        assert_eq!(resp.status(), hyper::StatusCode::PARTIAL_CONTENT);
        assert_eq!(resp.headers().get("Content-Range").unwrap(), "bytes 0-4/10");
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "5");
        
        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::RANGE, "bytes=5-".parse().unwrap()); // 5 to end
        let resp = server.serve_file(&file_path, Some(&headers), None).unwrap();
        assert_eq!(resp.status(), hyper::StatusCode::PARTIAL_CONTENT);
        assert_eq!(resp.headers().get("Content-Range").unwrap(), "bytes 5-9/10");
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "5");

        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::RANGE, "bytes=-3".parse().unwrap()); // last 3
        let resp = server.serve_file(&file_path, Some(&headers), None).unwrap();
        assert_eq!(resp.status(), hyper::StatusCode::PARTIAL_CONTENT);
        assert_eq!(resp.headers().get("Content-Range").unwrap(), "bytes 7-9/10");
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "3");
    }

    #[tokio::test]
    async fn test_serve_file_multipart_range() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let server = StaticFileServer::new(StaticFileConfig { root: root.clone(), ..Default::default() });
        let file_path = root.join("test.txt");
        std::fs::write(&file_path, b"0123456789").unwrap();

        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::RANGE, "bytes=0-2, 7-9".parse().unwrap());
        let resp = server.serve_file(&file_path, Some(&headers), None).unwrap();
        
        assert_eq!(resp.status(), hyper::StatusCode::PARTIAL_CONTENT);
        let ct = resp.headers().get("Content-Type").unwrap().to_str().unwrap();
        assert!(ct.starts_with("multipart/byteranges; boundary="));
        
        use http_body_util::BodyExt;
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Content-Range: bytes 0-2/10"));
        assert!(body_str.contains("Content-Range: bytes 7-9/10"));
    }

    #[test]
    fn test_serve_file_invalid_range() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let server = StaticFileServer::new(StaticFileConfig { root: root.clone(), ..Default::default() });
        let file_path = root.join("test.txt");
        std::fs::write(&file_path, b"01234").unwrap();

        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::RANGE, "bytes=10-20".parse().unwrap()); // unsatisfiable
        let resp = server.serve_file(&file_path, Some(&headers), None).unwrap();
        
        assert_eq!(resp.status(), hyper::StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(resp.headers().get("Content-Range").unwrap(), "bytes */5");
    }

    #[test]
    fn test_try_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let config = StaticFileConfig {
            root: root.clone(),
            ..Default::default()
        };
        let server = StaticFileServer::new(config);

        let fb_path = root.join("fallback.html");
        std::fs::write(&fb_path, b"Fallback").unwrap();

        let try_paths = vec![
            "$uri".to_string(),
            "$uri/".to_string(),
            "/fallback.html".to_string()
        ];

        // 1. Missing file falls back to fallback
        let resolved = server.try_files("/missing.txt", &try_paths).unwrap();
        assert_eq!(resolved, fb_path.canonicalize().unwrap());
        
        // 2. Existing file is returned early
        let existing = root.join("existing.txt");
        std::fs::write(&existing, b"Exist").unwrap();
        let resolved2 = server.try_files("/existing.txt", &try_paths).unwrap();
        assert_eq!(resolved2, existing.canonicalize().unwrap());

        // 3. Fallback not found returns NotFound
        let try_paths_bad = vec!["$uri".to_string(), "/missing2.html".to_string()];
        let err = server.try_files("/missing.txt", &try_paths_bad);
        assert!(matches!(err, Err(StaticFileError::NotFound(_))));
    }

    #[test]
    fn test_serve_file_compressed() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let mut config = StaticFileConfig {
            root: root.clone(),
            ..Default::default()
        };
        // Enable gzip
        config.compression.enabled = true;
        config.compression.gzip_level = 6;
        config.compression.min_size = 5; // allow small bodies for test

        let server = StaticFileServer::new(config);
        let file_path = root.join("test.html");
        // Need a compressible mime and string
        let content = b"compress me compress me compress me compress me";
        std::fs::write(&file_path, content).unwrap();

        let mut headers = hyper::HeaderMap::new();
        headers.insert(hyper::header::ACCEPT_ENCODING, "gzip".parse().unwrap());
        
        let mut override_mime = std::collections::HashMap::new();
        override_mime.insert("html".to_string(), "text/html".to_string());

        let resp = server.serve_file(&file_path, Some(&headers), Some(&override_mime)).unwrap();
        
        assert_eq!(resp.status(), hyper::StatusCode::OK);
        assert_eq!(resp.headers().get("Content-Encoding").unwrap(), "gzip");
        // Body length should be different
        let content_len: usize = resp.headers().get("Content-Length").unwrap().to_str().unwrap().parse().unwrap();
        assert!(content_len > 0 && content_len != content.len());
    }
}
