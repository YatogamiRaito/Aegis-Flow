# Implementation Plan: Static File Server & Compression (v0.16.0)

## Phase 1: Core Static File Serving

- [ ] Task: Create static file serving module in `crates/proxy/src/static_files.rs`
    - [ ] Define StaticFileConfig struct with root, index, autoindex, follow_symlinks
    - [ ] Implement file path resolution from URI (sanitize, join with root)
    - [ ] Write tests for path traversal prevention (../, //, null bytes, encoded sequences)
    - [ ] Implement path traversal guards with comprehensive checks
    - [ ] Write tests for index file resolution (directory → index.html)
    - [ ] Implement index file auto-resolution

- [ ] Task: Implement MIME type detection module (`crates/proxy/src/mime_types.rs`)
    - [ ] Write tests for common extensions (html, css, js, json, png, jpg, svg, woff2, mp4, etc.)
    - [ ] Build comprehensive MIME type map (200+ entries) using phf compile-time map
    - [ ] Write tests for custom MIME overrides via config
    - [ ] Implement custom MIME override merging
    - [ ] Write tests for charset=utf-8 auto-append for text types
    - [ ] Implement charset handling

- [ ] Task: Implement file response builder
    - [ ] Write tests for reading file and building HTTP response with correct headers
    - [ ] Implement serve_file() → Response with Content-Type, Content-Length, Last-Modified
    - [ ] Write tests for dotfile blocking (hidden files)
    - [ ] Implement dotfile check (configurable allow/deny)
    - [ ] Write tests for symlink following (enabled/disabled)
    - [ ] Implement symlink policy enforcement

- [ ] Task: Implement `try_files` logic
    - [ ] Write tests for $uri, $uri/, fallback path resolution
    - [ ] Implement try_files() that iterates paths and returns first match
    - [ ] Write tests for fallback to status code (=404, =502)
    - [ ] Implement status code fallback

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Conditional Requests & Caching Headers

- [ ] Task: Implement ETag generation
    - [ ] Write tests for weak ETag (size + mtime based)
    - [ ] Implement weak ETag: `W/"<hex(size)>-<hex(mtime_secs)}>""`
    - [ ] Write tests for strong ETag (SHA-256 content hash)
    - [ ] Implement strong ETag with content hashing (cached in LRU cache)
    - [ ] Write tests for ETag configuration switching (weak/strong/disabled)

- [ ] Task: Implement conditional response handling
    - [ ] Write tests for If-None-Match → 304 Not Modified
    - [ ] Implement If-None-Match comparison logic
    - [ ] Write tests for If-Modified-Since → 304 Not Modified
    - [ ] Implement If-Modified-Since comparison logic
    - [ ] Write tests for combined If-None-Match + If-Modified-Since handling

- [ ] Task: Implement Cache-Control and Expires headers
    - [ ] Write tests for pattern-based Cache-Control assignment (e.g., *.js → max-age=31536000)
    - [ ] Implement configurable cache header rules per file extension/pattern
    - [ ] Write tests for Expires header generation
    - [ ] Implement Expires header

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Range Requests (Byte Serving)

- [ ] Task: Implement single-range request handling
    - [ ] Write tests for Range: bytes=0-499 (first 500 bytes)
    - [ ] Write tests for Range: bytes=500- (from byte 500 to end)
    - [ ] Write tests for Range: bytes=-500 (last 500 bytes)
    - [ ] Implement range parsing and 206 Partial Content response
    - [ ] Write tests for invalid/unsatisfiable ranges (416 Range Not Satisfiable)

- [ ] Task: Implement multi-part range requests
    - [ ] Write tests for multiple ranges: bytes=0-100, 200-300
    - [ ] Implement multipart/byteranges response with MIME boundary
    - [ ] Write tests for Accept-Ranges: bytes header presence

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Compression (Gzip & Brotli)

- [ ] Task: Implement Accept-Encoding negotiation
    - [ ] Write tests for parsing Accept-Encoding header (gzip, br, identity, q-values)
    - [ ] Implement encoding negotiation with priority: br > gzip > identity
    - [ ] Write tests for q=0 exclusion

- [ ] Task: Implement Gzip compression (`crates/proxy/src/compression.rs`)
    - [ ] Write tests for on-the-fly Gzip with configurable level
    - [ ] Implement Gzip streaming compression using flate2 crate
    - [ ] Write tests for min_length threshold (skip compression for small responses)
    - [ ] Write tests for content-type filtering (only compress text types)
    - [ ] Implement content-type filter
    - [ ] Write tests for Vary: Accept-Encoding header

- [ ] Task: Implement Brotli compression
    - [ ] Write tests for on-the-fly Brotli with configurable quality
    - [ ] Implement Brotli streaming compression using brotli crate
    - [ ] Write tests for Brotli priority over Gzip when both accepted

- [ ] Task: Implement pre-compressed file serving
    - [ ] Write tests for serving .gz files when they exist alongside original
    - [ ] Write tests for serving .br files when they exist
    - [ ] Implement check for pre-compressed variants before on-the-fly compression

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Directory Listing & Zero-Copy I/O

- [ ] Task: Implement directory listing (autoindex)
    - [ ] Write tests for HTML directory listing output
    - [ ] Implement HTML template with file names, sizes, dates
    - [ ] Write tests for JSON directory listing output
    - [ ] Implement JSON format for API consumption
    - [ ] Write tests for autoindex disabled (return 403 for directories)

- [ ] Task: Implement zero-copy file I/O
    - [ ] Write tests for sendfile-based file transfer (Linux)
    - [ ] Implement platform-specific zero-copy (sendfile on Linux, fallback on macOS)
    - [ ] Write tests for TCP_CORK / TCP_NODELAY optimization
    - [ ] Implement socket option tuning for static file responses

- [ ] Task: Integrate static file serving into main proxy handler
    - [ ] Write tests for routing: static file requests vs proxy requests
    - [ ] Implement routing logic in http_proxy.rs to dispatch to static handler
    - [ ] Write tests for fallback from static → proxy when file not found

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
