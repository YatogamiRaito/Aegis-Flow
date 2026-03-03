use hyper::http::{HeaderMap, HeaderValue};
use std::io::Write;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashSet;
use once_cell::sync::Lazy;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgo {
    Gzip,
    Brotli,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub gzip_level: u32,
    pub brotli_level: u32,
    pub min_size: u64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gzip_level: 6,
            brotli_level: 4,
            min_size: 1024,
        }
    }
}

// Common uncompressible MIME types
static UNCOMPRESSIBLE_MIME_PREFIXES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "image/",
        "video/",
        "audio/",
    ]
});

static COMPRESSIBLE_EXCEPTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("image/svg+xml");
    s.insert("image/x-icon");
    s
});

pub fn is_compressible_mime(mime: &str) -> bool {
    let mime_lower = mime.to_lowercase();
    
    if COMPRESSIBLE_EXCEPTIONS.contains(mime_lower.as_str()) {
        return true;
    }

    // Binary formats like zip, pdf, etc.
    if mime_lower == "application/pdf" || 
       mime_lower == "application/zip" || 
       mime_lower == "application/x-rar-compressed" ||
       mime_lower == "application/octet-stream" {
        return false;
    }

    for prefix in UNCOMPRESSIBLE_MIME_PREFIXES.iter() {
        if mime_lower.starts_with(prefix) {
            return false;
        }
    }

    true
}

pub fn negotiate_encoding(req_headers: Option<&HeaderMap>, config: &CompressionConfig, mime: &str, size: u64) -> CompressionAlgo {
    if !config.enabled || size < config.min_size || !is_compressible_mime(mime) {
        return CompressionAlgo::None;
    }

    if let Some(headers) = req_headers {
        if let Some(accept_encoding) = headers.get(hyper::header::ACCEPT_ENCODING) {
            if let Ok(enc_str) = accept_encoding.to_str() {
                // Br takes priority over gzip if both present
                let enc_str_lower = enc_str.to_lowercase();
                if enc_str_lower.contains("br") {
                    return CompressionAlgo::Brotli;
                } else if enc_str_lower.contains("gzip") {
                    return CompressionAlgo::Gzip;
                }
            }
        }
    }

    CompressionAlgo::None
}

pub fn compress_body(body: &[u8], algo: CompressionAlgo, config: &CompressionConfig) -> Option<Vec<u8>> {
    match algo {
        CompressionAlgo::None => None,
        CompressionAlgo::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::new(config.gzip_level));
            if encoder.write_all(body).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    return Some(compressed);
                }
            }
            None
        }
        CompressionAlgo::Brotli => {
            let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, config.brotli_level, 20);
            if writer.write_all(body).is_ok() && writer.flush().is_ok() {
                let compressed = writer.into_inner();
                return Some(compressed);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_encoding() {
        let config = CompressionConfig::default();
        let mime = "text/html";
        let size = 2048;

        let mut headers = HeaderMap::new();
        headers.insert(hyper::header::ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        
        // Brotli has priority
        assert_eq!(negotiate_encoding(Some(&headers), &config, mime, size), CompressionAlgo::Brotli);
        
        let mut headers2 = HeaderMap::new();
        headers2.insert(hyper::header::ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
        assert_eq!(negotiate_encoding(Some(&headers2), &config, mime, size), CompressionAlgo::Gzip);
        
        let mut headers3 = HeaderMap::new();
        headers3.insert(hyper::header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
        assert_eq!(negotiate_encoding(Some(&headers3), &config, mime, size), CompressionAlgo::None);
    }

    #[test]
    fn test_is_compressible_mime() {
        assert!(is_compressible_mime("text/html"));
        assert!(is_compressible_mime("application/json"));
        assert!(is_compressible_mime("application/javascript"));
        assert!(is_compressible_mime("image/svg+xml")); // svg is ok
        
        assert!(!is_compressible_mime("image/png"));
        assert!(!is_compressible_mime("image/jpeg"));
        assert!(!is_compressible_mime("video/mp4"));
        assert!(!is_compressible_mime("application/zip"));
    }

    #[test]
    fn test_min_size_bypass() {
        let config = CompressionConfig::default();
        let mut headers = HeaderMap::new();
        headers.insert(hyper::header::ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        
        assert_eq!(negotiate_encoding(Some(&headers), &config, "text/html", 500), CompressionAlgo::None);
    }

    #[test]
    fn test_gzip_compression() {
        let config = CompressionConfig { gzip_level: 6, ..Default::default() };
        let data = b"hello world, this is a test string to be compressed multiple times for good measure... hello world!";
        
        let compressed = compress_body(data, CompressionAlgo::Gzip, &config).unwrap();
        assert!(compressed.len() < data.len());
        assert!(compressed[0] == 0x1f && compressed[1] == 0x8b); // gzip magic header
    }

    #[test]
    fn test_brotli_compression() {
        let config = CompressionConfig { brotli_level: 4, ..Default::default() };
        let data = b"hello world, this is a test string to be compressed multiple times for good measure... hello world!";
        
        let compressed = compress_body(data, CompressionAlgo::Brotli, &config).unwrap();
        // brotli usually compresses better
        assert!(!compressed.is_empty());
        assert!(compressed.len() != data.len()); // it changed
    }
}
