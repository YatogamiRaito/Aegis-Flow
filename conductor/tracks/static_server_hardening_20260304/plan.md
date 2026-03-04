# Implementation Plan: Static File Server Hardening (v0.41.0)

## Phase 1: Asynchronous Streaming Body
- [ ] Task: Replace synchronous file reads in `serve_file`
    - [ ] Update `crates/proxy/src/static_files.rs` to open files asynchronously via `tokio::fs::File`.
    - [ ] Implement `tokio_util::io::ReaderStream` mapping to `http_body_util::StreamBody`.
    - [ ] Ensure `content_range` and multipart ranges still function via `AsyncSeek`.

## Phase 2: Caching Headers Integration
- [ ] Task: Integrate `caching.rs` into `StaticFileServer`
    - [ ] Inject `CachingManager` instance into `StaticFileServer`.
    - [ ] Generate ETags via `generate_etag()` for static assets.
    - [ ] Add `ETag`, `Cache-Control`, and `Expires` headers to the builder.
    - [ ] Short-circuit the file read process if `check_conditional()` returns true, yielding `304 Not Modified`.

## Phase 3: Validation and Load Testing
- [ ] Task: Integration Tests
    - [ ] Write tests ensuring a request with a valid `If-None-Match` ETag responds with `304 Not Modified`.
    - [ ] Write integration test validating memory usage does not spike while downloading large pseudo-files.
    - [ ] Conductor - User Manual Verification (Protocol in workflow.md).
