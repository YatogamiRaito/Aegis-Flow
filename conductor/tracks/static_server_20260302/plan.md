# Implementation Plan: Static File Server & Compression (v0.16.0)

## Phase 1: Core Static File Serving

- [x] Task: Create static file serving module in `crates/proxy/src/static_files.rs`
    - [x] Define StaticFileConfig struct with root, index, autoindex, follow_symlinks
    - [x] Implement file path resolution from URI (sanitize, join with root)
    - [x] Write tests for path traversal prevention (../, //, null bytes, encoded sequences)
    - [x] Implement path traversal guards with comprehensive checks
    - [x] Write tests for index file resolution (directory → index.html)
    - [x] Implement index file auto-resolution

- [x] Task: Implement MIME type detection module (`crates/proxy/src/mime_types.rs`)
    - [x] Write tests for common extensions (html, css, js, json, png, jpg, svg, woff2, mp4, etc.)
    - [x] Build comprehensive MIME type map (200+ entries) using phf compile-time map
    - [x] Write tests for custom MIME overrides via config
    - [x] Implement custom MIME override merging
    - [x] Write tests for charset=utf-8 auto-append for text types
    - [x] Implement charset handling

- [x] Task: Implement file response builder
    - [x] Write tests for reading file and building HTTP response with correct headers
    - [x] Implement serve_file() → Response with Content-Type, Content-Length, Last-Modified
    - [x] Write tests for dotfile blocking (hidden files)
    - [x] Implement dotfile check (configurable allow/deny)
    - [x] Write tests for symlink following (enabled/disabled)
    - [x] Implement symlink policy enforcement

- [x] Task: Implement `try_files` logic
    - [x] Write tests for $uri, $uri/, fallback path resolution
    - [x] Implement try_files() that iterates paths and returns first match
    - [x] Write tests for fallback to status code (=404, =502)
    - [x] Implement status code fallback

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Conditional Requests & Caching Headers

- [x] Task: Implement ETag generation
    - [x] Write tests for weak ETag (size + mtime based)
    - [x] Implement weak ETag: `W/"<hex(size)>-<hex(mtime_secs)}>""`
    - [x] Write tests for strong ETag (SHA-256 content hash)
    - [x] Implement strong ETag with content hashing (cached in LRU cache)
    - [x] Write tests for ETag configuration switching (weak/strong/disabled)

- [x] Task: Implement conditional response handling
    - [x] Write tests for If-None-Match → 304 Not Modified
    - [x] Implement If-None-Match comparison logic
    - [x] Write tests for If-Modified-Since → 304 Not Modified
    - [x] Implement If-Modified-Since comparison logic
    - [x] Write tests for combined If-None-Match + If-Modified-Since handling

- [x] Task: Implement Cache-Control and Expires headers
    - [x] Write tests for pattern-based Cache-Control assignment (e.g., *.js → max-age=31536000)
    - [x] Implement configurable cache header rules per file extension/pattern
    - [x] Write tests for Expires header generation
    - [x] Implement Expires header

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Range Requests (Byte Serving)

- [x] Task: Implement single-range request handling
    - [x] Write tests for Range: bytes=0-499 (first 500 bytes)
    - [x] Write tests for Range: bytes=500- (from byte 500 to end)
    - [x] Write tests for Range: bytes=-500 (last 500 bytes)
    - [x] Implement range parsing and 206 Partial Content response
    - [x] Write tests for invalid/unsatisfiable ranges (416 Range Not Satisfiable)

- [x] Task: Implement multi-part range requests
    - [x] Write tests for multiple ranges: bytes=0-100, 200-300
    - [x] Implement multipart/byteranges response with MIME boundary
    - [x] Write tests for Accept-Ranges: bytes header presence

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Compression (Gzip & Brotli)

- [x] Task: Implement Accept-Encoding negotiation
    - [x] Write tests for parsing Accept-Encoding header (gzip, br, identity, q-values)
    - [x] Implement encoding negotiation with priority: br > gzip > identity
    - [x] Write tests for q=0 exclusion

- [x] Task: Implement Gzip compression (`crates/proxy/src/compression.rs`)
    - [x] Write tests for on-the-fly Gzip with configurable level
    - [x] Implement Gzip streaming compression using flate2 crate
    - [x] Write tests for min_length threshold (skip compression for small responses)
    - [x] Write tests for content-type filtering (only compress text types)
    - [x] Implement content-type filter
    - [x] Write tests for Vary: Accept-Encoding header

- [x] Task: Implement Brotli compression
    - [x] Write tests for on-the-fly Brotli with configurable quality
    - [x] Implement Brotli streaming compression using brotli crate
    - [x] Write tests for Brotli priority over Gzip when both accepted

- [x] Task: Implement pre-compressed file serving
    - [x] Write tests for serving .gz files when they exist alongside original
    - [x] Write tests for serving .br files when they exist
    - [x] Implement check for pre-compressed variants before on-the-fly compression

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Directory Listing & Zero-Copy I/O

- [x] Task: Implement directory listing (autoindex)
    - [x] Write tests for HTML directory listing output
    - [x] Implement HTML template with file names, sizes, dates
    - [x] Write tests for JSON directory listing output
    - [x] Implement JSON format for API consumption
    - [x] Write tests for autoindex disabled (return 403 for directories)

- [x] Task: Implement zero-copy file I/O
    - [x] Write tests for sendfile-based file transfer (Linux)
    - [x] Implement platform-specific zero-copy (sendfile on Linux, fallback on macOS)
    - [x] Write tests for TCP_CORK / TCP_NODELAY optimization
    - [x] Implement socket option tuning for static file responses

- [x] Task: Integrate static file serving into main proxy handler
    - [x] Write tests for routing: static file requests vs proxy requests
    - [x] Implement routing logic in http_proxy.rs to dispatch to static handler
    - [x] Write tests for fallback from static → proxy when file not found

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
