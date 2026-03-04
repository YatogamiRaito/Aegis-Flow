# Track Specification: Response Transformation & Logging Hardening (v0.52.0)

## 1. Overview
The fundamental logic for `sub_filter`, `ssi`, `image_filter`, and `syslog` exist as disconnected Rust modules. They are not invoked anywhere in the HTTP request/response lifecycle.

This track aims to integrate these modules into the main proxy flow. It will establish a Response Middleware mechanism to intercept and manipulate response bodies before they are streamed to the client, and wire the syslog output into the background logger.

## 2. Functional Requirements

### 2.1 Response Middleware Pipeline
- Refactor `http_proxy::handle_request` so that the `Response<BoxBody<Bytes, BoxError>>` returned by the static file server or upstream proxy can be intercepted.
- Create a middleware function that checks the matched location for `sub_filter`, `image_filter`, and `ssi` configs.
- If transformations are required, the middleware must collect the chunked response or buffer it up to a safe limit.
- Apply the transformations in order (e.g., SSI -> Sub-Filter or Image Filter).
- If the body was transformed, the middleware MUST update or strip the `Content-Length` header to prevent truncation or protocol errors in the client.

### 2.2 Server Side Includes (SSI) Subrequests
- Update `ssi::process_ssi` or the virtual include logic to dispatch a real recursive `handle_request` call to the proxy engine instead of just attempting to read a local file.
- Prevent infinite recursion by enforcing the `MAX_INCLUDE_DEPTH`.

### 2.3 Image Filter Integration
- The `image_filter` stub needs to be integrated into the response middleware pipeline, triggered when `image_filter` config is present and the response `Content-Type` is an image.
- Stub integration is sufficient for now (as defined in `image_filter.rs`), but it must be invoked so that real image processing can be dropped in seamlessly later.

### 2.4 Access Log via Syslog
- Update `access_log::start_logger_task` and the main log processing loop to submit log strings to `syslog::send_log` when syslog is enabled in the configuration.
- Implement proper formatting for standard access log JSON or Apache-style strings within the RFC 5424 payload.

## 3. Non-Functional Requirements
- **Streaming Efficiency:** The response interception should avoid fully buffering large files if `sub_filter` is not active for that content type (e.g., skip interception for `video/mp4`).
- **Resilience:** Syslog sending (especially UDP) must not block the main logger task or crash the application on network failure.

## 4. Acceptance Criteria
- [ ] A proxied HTML page with a `sub_filter` rule has its content dynamically replaced before reaching the client.
- [ ] `Content-Length` is accurately modified after a `sub_filter` replacement.
- [ ] An SSI `<!--#include virtual="/api/foo" -->` successfully injects the upstream proxy response into the parent HTML body.
- [ ] Access logs are successfully transmitted to a local UDP syslog receiver when configured.
