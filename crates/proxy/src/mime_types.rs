use phf::phf_map;
use std::path::Path;

/// A comprehensive map of file extensions to their corresponding MIME types.
static MIME_TYPES: phf::Map<&'static str, &'static str> = phf_map! {
    "html" => "text/html",
    "htm" => "text/html",
    "css" => "text/css",
    "js" => "application/javascript",
    "json" => "application/json",
    "png" => "image/png",
    "jpg" => "image/jpeg",
    "jpeg" => "image/jpeg",
    "gif" => "image/gif",
    "svg" => "image/svg+xml",
    "ico" => "image/x-icon",
    "webp" => "image/webp",
    "woff" => "font/woff",
    "woff2" => "font/woff2",
    "ttf" => "font/ttf",
    "eot" => "application/vnd.ms-fontobject",
    "otf" => "font/otf",
    "mp4" => "video/mp4",
    "webm" => "video/webm",
    "ogg" => "audio/ogg",
    "mp3" => "audio/mpeg",
    "wav" => "audio/wav",
    "txt" => "text/plain",
    "csv" => "text/csv",
    "xml" => "text/xml",
    "pdf" => "application/pdf",
    "zip" => "application/zip",
    "tar" => "application/x-tar",
    "gz" => "application/gzip",
    "md" => "text/markdown",
    "wasm" => "application/wasm",
    "m3u8" => "application/vnd.apple.mpegurl",
    "ts" => "video/mp2t",
};

/// Get the MIME type for a given file path based on its extension.
/// Supports a custom override map and auto-appending charset=UTF-8 for text types.
pub fn get_mime_type(
    path: &Path,
    custom_overrides: Option<&std::collections::HashMap<String, String>>,
) -> String {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // 1. Check custom overrides first
    if let Some(overrides) = custom_overrides {
        if let Some(mime) = overrides.get(&ext) {
            return append_charset(mime);
        }
    }

    // 2. Check compiled map
    if let Some(mime) = MIME_TYPES.get(ext.as_str()) {
        return append_charset(mime);
    }

    // 3. Fallback
    "application/octet-stream".to_string()
}

fn append_charset(mime: &str) -> String {
    if mime.starts_with("text/") || mime == "application/javascript" || mime == "application/json" {
        format!("{}; charset=utf-8", mime)
    } else {
        mime.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_common_extensions() {
        assert_eq!(get_mime_type(Path::new("index.html"), None), "text/html; charset=utf-8");
        assert_eq!(get_mime_type(Path::new("style.css"), None), "text/css; charset=utf-8");
        assert_eq!(get_mime_type(Path::new("app.js"), None), "application/javascript; charset=utf-8");
        assert_eq!(get_mime_type(Path::new("data.json"), None), "application/json; charset=utf-8");
        assert_eq!(get_mime_type(Path::new("image.png"), None), "image/png");
        assert_eq!(get_mime_type(Path::new("video.mp4"), None), "video/mp4");
        // Unknown extension
        assert_eq!(get_mime_type(Path::new("unknown.foo"), None), "application/octet-stream");
        // No extension
        assert_eq!(get_mime_type(Path::new("filewithout"), None), "application/octet-stream");
    }

    #[test]
    fn test_custom_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("foo".to_string(), "text/x-foo".to_string());
        overrides.insert("png".to_string(), "image/custom-png".to_string()); // Override default

        assert_eq!(
            get_mime_type(Path::new("file.foo"), Some(&overrides)),
            "text/x-foo; charset=utf-8"
        );
        assert_eq!(
            get_mime_type(Path::new("image.png"), Some(&overrides)),
            "image/custom-png" // Override doesn't trigger text/ so no charset appended normally unless specified
        );
    }
}
