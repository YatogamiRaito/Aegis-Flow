# Implementation Plan: Response Transformation & Logging Extensions (v0.27.0)

## Phase 1: Response Body Rewriting (sub_filter)

- [ ] Task: Implement sub_filter engine (`crates/proxy/src/sub_filter.rs`)
    - [ ] Write tests for exact string replacement in response body
    - [ ] Implement string search-and-replace on body bytes
    - [ ] Write tests for regex replacement with capture groups
    - [ ] Implement regex-based substitution
    - [ ] Write tests for sub_filter_once (first occurrence only vs all)
    - [ ] Implement once/all mode toggle
    - [ ] Write tests for Content-Type filtering (only process text/html, etc.)
    - [ ] Implement content-type check before processing

- [ ] Task: Implement streaming substitution
    - [ ] Write tests for chunked body processing (don't buffer entire response)
    - [ ] Implement streaming search using Aho-Corasick or rolling buffer
    - [ ] Write tests for Content-Length recalculation
    - [ ] Implement content-length update after substitution
    - [ ] Write tests for Transfer-Encoding: chunked passthrough

- [ ] Task: Implement body injection (addition_before/after)
    - [ ] Write tests for prepending content before response body
    - [ ] Write tests for appending content after response body
    - [ ] Implement file-based and inline injection
    - [ ] Write tests for Content-Length adjustment

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Syslog Integration

- [ ] Task: Implement syslog client (`crates/proxy/src/syslog.rs`)
    - [ ] Write tests for RFC 5424 syslog message formatting
    - [ ] Implement syslog message builder (priority, timestamp, hostname, app-name, PID, message)
    - [ ] Write tests for UDP syslog delivery
    - [ ] Implement UDP syslog sender
    - [ ] Write tests for TCP syslog delivery
    - [ ] Implement TCP syslog sender with reconnection
    - [ ] Write tests for TCP+TLS syslog transport
    - [ ] Implement TLS-encrypted syslog using rustls

- [ ] Task: Integrate syslog with logging system
    - [ ] Write tests for access log → syslog routing
    - [ ] Write tests for error log → syslog routing
    - [ ] Implement syslog as a log output backend alongside file output
    - [ ] Write tests for structured data (SD-ELEMENT) in syslog messages
    - [ ] Implement SD-ELEMENT serialization

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Server Side Includes (SSI)

- [ ] Task: Implement SSI parser (`crates/proxy/src/ssi.rs`)
    - [ ] Write tests for <!--#include virtual="..." --> parsing
    - [ ] Write tests for <!--#include file="..." --> parsing
    - [ ] Write tests for <!--#echo var="..." --> parsing
    - [ ] Write tests for <!--#set var="..." value="..." --> parsing
    - [ ] Write tests for <!--#if expr="..." --> conditional parsing
    - [ ] Implement SSI directive parser using regex or custom scanner

- [ ] Task: Implement SSI execution
    - [ ] Write tests for virtual include (internal subrequest)
    - [ ] Implement subrequest dispatch for virtual includes
    - [ ] Write tests for file include (read local file)
    - [ ] Implement file-based includes
    - [ ] Write tests for variable echo and set
    - [ ] Implement SSI variable scope
    - [ ] Write tests for conditional evaluation
    - [ ] Write tests for recursion depth limit (max 10 nested includes)
    - [ ] Implement depth tracking and limit enforcement

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Image Filter & XSLT

- [ ] Task: Implement basic image filter (`crates/proxy/src/image_filter.rs`)
    - [ ] Write tests for image resize (width × height)
    - [ ] Implement resize using image crate
    - [ ] Write tests for image crop (center crop)
    - [ ] Implement crop operation
    - [ ] Write tests for JPEG quality adjustment
    - [ ] Implement quality control
    - [ ] Write tests for content-type detection (only process image/*)
    - [ ] Write tests for rotation (90, 180, 270 degrees)

- [ ] Task: Implement XSLT transformation
    - [ ] Write tests for XML + XSLT → HTML transformation
    - [ ] Implement XSLT processing (evaluate Rust XSLT crate availability, fallback to libxslt FFI)
    - [ ] Write tests for XSLT parameter injection from request variables
    - [ ] Write tests for content-type filtering (only XML responses)

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
