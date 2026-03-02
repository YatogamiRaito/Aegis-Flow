# Track Specification: Response Transformation & Logging Extensions (v0.27.0)

## 1. Overview

This track adds **response body transformation** (`sub_filter` — rewrite content in proxied responses), **syslog integration** for centralized logging, and **Server Side Includes (SSI)** support. These are the final content manipulation features needed for full nginx compatibility.

## 2. Functional Requirements

### 2.1 Response Body Rewriting (`sub_filter`)
- Replace strings in the response body on-the-fly.
- Support multiple replacement rules per location.
- Example:
  ```toml
  [[server.location]]
  path = "/"
  proxy_pass = "http://backend:3000"
  sub_filter = [
    { match = "http://internal.example.com", replace = "https://public.example.com" },
    { match = "old-api", replace = "new-api" },
  ]
  sub_filter_once = false  # replace all occurrences, not just first
  sub_filter_types = ["text/html", "text/css", "application/javascript"]
  ```
- Only apply to responses with matching Content-Type.
- `sub_filter_once`: if true, only replace first occurrence (default: true, like nginx).
- Regex support: `sub_filter = [{ match = "~v[0-9]+", replace = "v2" }]`.
- Content-Length recalculation after substitution.
- Streaming substitution: process chunks without buffering entire response.

### 2.2 Response Body Injection
- `addition_before`: inject content before the response body.
- `addition_after`: inject content after the response body.
- Use case: inject analytics scripts, tracking pixels, debugging headers.
- Example:
  ```toml
  [[server.location]]
  path = "/"
  addition_before = "/inject/header.html"   # file path or inline
  addition_after = "/inject/analytics.html"
  ```

### 2.3 Syslog Integration
- Send access/error logs to remote syslog server (RFC 5424).
- Support UDP syslog (port 514) and TCP syslog (with TLS option).
- Configurable facility and severity mapping.
- Example:
  ```toml
  [logging.syslog]
  enabled = true
  server = "syslog.example.com:514"
  transport = "udp"  # udp, tcp, tcp+tls
  facility = "local7"
  tag = "aegis-flow"
  ```
- Per-location syslog target override.
- Structured syslog (RFC 5424) with SD-ELEMENT for key-value metadata.

### 2.4 Server Side Includes (SSI)
- Process `<!--#include virtual="/header" -->` directives in HTML responses.
- Support for SSI directives:
  - `<!--#include virtual="..." -->`: include content from URI (subrequest).
  - `<!--#include file="..." -->`: include content from local file.
  - `<!--#echo var="..." -->`: output variable value.
  - `<!--#set var="..." value="..." -->`: set variable.
  - `<!--#if expr="..." -->...<!--#endif -->`: conditional blocks.
- Configurable depth limit for nested includes (default: 10).
- SSI processing enabled per-location: `ssi = true`.

### 2.5 XSLT Response Transformation
- Apply XSLT stylesheet to XML responses (for API format transformation).
- `xslt_stylesheet = "/path/to/transform.xslt"`.
- Support for XSLT parameters from request variables.
- Use case: XML → HTML transformation, API response reshaping.

### 2.6 Image Filter (Basic)
- On-the-fly image operations for proxied image responses.
- Resize: `image_filter = { resize = { width = 200, height = 200 } }`.
- Crop: center crop to specified dimensions.
- Rotate: 0, 90, 180, 270 degrees.
- Quality: JPEG quality adjustment.
- Only process image/* content types.

## 3. Non-Functional Requirements

- sub_filter streaming: < 1% latency increase for typical HTML pages.
- Syslog delivery: best-effort (UDP), at-least-once (TCP).
- Image filter: < 50ms for typical resize operations on 1MP images.

## 4. Acceptance Criteria

- [ ] sub_filter replaces strings in response body.
- [ ] Regex-based sub_filter replacement works.
- [ ] sub_filter_once controls single vs all replacements.
- [ ] Content-Length is recalculated after substitution.
- [ ] Body injection (before/after) works.
- [ ] Syslog UDP/TCP log delivery works.
- [ ] Syslog TLS transport works.
- [ ] SSI include virtual (subrequest) works.
- [ ] SSI conditional blocks work.
- [ ] Image resize/crop works for proxied images.
- [ ] >90% test coverage.

## 5. Out of Scope

- Full image CDN (imgproxy-level features).
- PDF transformation.
