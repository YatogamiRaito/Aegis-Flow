# Track Specification: Static File Server & Compression (v0.16.0)

## 1. Overview

This track adds a high-performance **static file server** to Aegis-Flow, comparable to nginx's static content serving capabilities. It includes MIME type detection, directory listing, `try_files` logic, ETag/Last-Modified conditional responses, range requests for media streaming, and on-the-fly Gzip/Brotli compression.

## 2. Functional Requirements

### 2.1 Static File Serving
- Serve files from a configurable `root` directory.
- Support for `index` directive: auto-serve `index.html`, `index.htm` when a directory is requested.
- **`try_files` logic:** Attempt to serve multiple paths in order, falling back to a final URI or status code.
  - Example: `try_files = ["$uri", "$uri/index.html", "/fallback.html"]`
- Path traversal prevention: reject requests containing `..`, `//`, or null bytes.
- Symbolic link following: configurable (enabled/disabled).

### 2.2 MIME Type Detection
- Automatic `Content-Type` header based on file extension.
- Built-in comprehensive MIME type map covering 200+ extensions.
- Custom MIME type overrides via configuration.
- Default MIME type for unknown extensions: `application/octet-stream`.
- `charset=utf-8` automatically appended for text types.

### 2.3 Directory Listing (autoindex)
- When enabled, display an HTML directory listing for directories without an index file.
- Configurable: `autoindex = true/false` (default: false).
- Show file name, size (human-readable), and modification date.
- Support JSON format output (`autoindex_format = "json"`) for API consumers.

### 2.4 Conditional Requests (Caching Headers)
- Generate `ETag` header based on file size + modification time (weak ETag).
- Strong ETag option using file content hash (SHA-256, configurable).
- `Last-Modified` header from file metadata.
- Handle `If-None-Match` → return 304 Not Modified.
- Handle `If-Modified-Since` → return 304 Not Modified.
- Configurable `Cache-Control` headers per file pattern (e.g., `max-age=31536000` for assets).
- `Expires` header support.

### 2.5 Range Requests (Byte Serving)
- Support `Range` header for partial content delivery (HTTP 206).
- Single range and multi-part range support.
- Required for video/audio streaming and download resumption.
- `Accept-Ranges: bytes` header on all static responses.
- Proper `Content-Range` header in 206 responses.

### 2.6 Gzip Compression
- On-the-fly Gzip compression for eligible responses.
- Configurable compression level (1-9, default: 6).
- Configurable minimum body size threshold (default: 256 bytes).
- Content-type filter: only compress text-based types (text/html, application/json, text/css, application/javascript, etc.).
- `Vary: Accept-Encoding` header.
- Check for pre-compressed `.gz` files first (static compression).

### 2.7 Brotli Compression
- Brotli support with higher compression ratios.
- Quality level configuration (0-11, default: 4).
- Same content-type filtering as Gzip.
- Priority: Brotli > Gzip when client supports both (based on `Accept-Encoding` q-values).
- Check for pre-compressed `.br` files first.

### 2.8 Sendfile / Zero-Copy I/O
- Use `sendfile(2)` (Linux) or equivalent for zero-copy file transfer.
- Fallback to `tokio::fs::File` read + write for non-Linux systems.
- `tcp_nopush` / `TCP_CORK` optimization for header + body coalescing.

### 2.9 Configuration
```toml
[static_files]
enabled = true
root = "/var/www/html"
index = ["index.html", "index.htm"]
try_files = ["$uri", "$uri/", "/index.html"]
autoindex = false
autoindex_format = "html"
follow_symlinks = false

[static_files.cache]
etag = true
etag_mode = "weak"
cache_control = "public, max-age=3600"

[static_files.compression]
gzip = true
gzip_level = 6
brotli = true
brotli_quality = 4
min_length = 256
types = ["text/html", "text/css", "text/plain", "application/json", "application/javascript", "image/svg+xml"]
```

## 3. Non-Functional Requirements

### 3.1 Performance
- Static file throughput: >50k requests/second for cached small files (1KB).
- Large file streaming: >1 Gbps throughput using zero-copy I/O.
- Compression overhead: <5% latency increase for typical HTML pages.

### 3.2 Security
- Path traversal attacks: 100% prevention (no directory escape).
- No serving of hidden files (dotfiles) unless explicitly configured.
- File permission checks before serving.

## 4. Acceptance Criteria

- [ ] Static file serving from configurable root directory.
- [ ] Automatic MIME type detection for 200+ file extensions.
- [ ] `index` directive serves index.html automatically.
- [ ] `try_files` logic falls through paths correctly.
- [ ] Directory listing works when autoindex is enabled.
- [ ] ETag and Last-Modified headers are generated correctly.
- [ ] 304 Not Modified returned for conditional requests.
- [ ] Range requests return 206 Partial Content.
- [ ] Gzip compression works with configurable level and content types.
- [ ] Brotli compression works with proper priority over Gzip.
- [ ] Pre-compressed files (.gz, .br) are served when available.
- [ ] Path traversal attempts return 400/403.
- [ ] >90% test coverage.

## 5. Out of Scope

- Server-side rendering or template engines.
- CGI/FastCGI execution (separate track).
- Image optimization or transformation.
