# Implementation Plan: Response Transformation & Logging Hardening (v0.52.0)

## Phase 1: Response Middleware Pipeline
- [ ] Task: Intercept responses in `http_proxy.rs`
    - [ ] Create a `transform_response(response, location_config)` function.
    - [ ] Check if `sub_filter`, `ssi`, or `image_filter` are active and apply to the `Content-Type`.
    - [ ] Implement `http_body_util::BodyExt::collect` to buffer the response if transformation is needed.
    - [ ] Strip `Content-Length` and re-inject modified body bytes.

## Phase 2: Wiring Sub-Filter and SSI
- [ ] Task: Connect specific transformers
    - [ ] Pass the buffered response body to `sub_filter::apply()`.
    - [ ] Pass the buffered response body to `ssi::process_ssi()`.
    - [ ] Refactor `ssi::process_ssi` to accept a callback or handle for making true internal subrequests, rather than just local file reads.

## Phase 3: Syslog Integration
- [ ] Task: Wire syslog to `access_log.rs`
    - [ ] Read `logging.syslog` config during logger initialization.
    - [ ] Inside the MPSC receive loop in `access_log.rs`, asynchronously call `syslog::send_log` with the generated log line.

## Phase 4: Verification and Cleanup
- [ ] Task: Ensure tests pass and performance is acceptable
    - [ ] Write an integration-style test in `http_proxy.rs` proving that a `proxy_pass` to a mock backend is successfully rewritten by `sub_filter`.
    - [ ] Ensure non-text responses (like massive binaries) bypass the buffering phase if no matching filter is configured.
