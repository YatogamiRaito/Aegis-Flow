# Track Specification: Static File Server Hardening (v0.41.0)

## 1. Overview
The current static file server in Aegis-Flow provides excellent functionality but lacks integration with the conditional caching mechanisms (`caching.rs`) and zero-copy optimization (`zero_copy.rs`). This hardening track addresses these gaps by refactoring `serve_file` to employ asynchronous streams and correctly interpreting `If-Modified-Since` and `If-None-Match` HTTP headers.

## 2. Functional Requirements

### 2.1 Caching Header Integration
- Invoke `CachingManager::generate_etag` and append the `ETag` header to responses.
- Evaluate `If-None-Match` and `If-Modified-Since` using `CachingManager::check_conditional`.
- If a file is unmodified, return `HTTP 304 Not Modified` without reading the file body.
- Apply pattern-based `Cache-Control` headers from configuration mapping (e.g. `*.js -> max-age=31536000`).
- Apply the `Expires` header if configured.

### 2.2 Streaming File Body
- Replace `std::fs::File::read_exact` with asynchronous streaming for `serve_file`.
- Use `tokio_util::io::ReaderStream` and Hyper's incoming body stream to stream file chunks asynchronously.
- Ensure that memory usage stays constant regardless of file size.

### 2.3 Zero-Copy / Socket Optimization
- Apply `TCP_CORK` (via `crate::zero_copy::apply_tcp_cork`) to the upstream connection when dispatching a static file response to ensure headers and body frames are optimally packed.

## 3. Non-Functional Requirements
- **Efficiency:** The proxy must not load entire multi-gigabyte static files into memory buffers.
- **Latency:** Conditional requests (`HTTP 304`) must bypass all disk read operations.

## 4. Acceptance Criteria
- [ ] `serve_file` uses `tokio_util::io::ReaderStream` (or equivalent) for transferring file body bounds asynchronously.
- [ ] Conditional endpoints respond with `304 Not Modified` when ETags match.
- [ ] `Cache-Control` and `Expires` headers are validated in integration tests.
- [ ] OOM regression tests pass when serving 10GB static files.
