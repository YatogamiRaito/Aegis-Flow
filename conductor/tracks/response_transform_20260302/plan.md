# Implementation Plan: Response Transformation & Logging Extensions (v0.27.0)

## Phase 1: Response Body Rewriting (sub_filter)

- [x] Task: Implement sub_filter engine (`crates/proxy/src/sub_filter.rs`)
    - [x] Write tests for exact string replacement in response body
    - [x] Implement string search-and-replace on body bytes
    - [x] Write tests for regex replacement with capture groups
    - [x] Implement regex-based substitution
    - [x] Write tests for sub_filter_once (first occurrence only vs all)
    - [x] Implement once/all mode toggle
    - [x] Write tests for Content-Type filtering (only process text/html, etc.)
    - [x] Implement content-type check before processing

- [x] Task: Implement streaming substitution
    - [x] Write tests for chunked body processing (don't buffer entire response)
    - [x] Implement streaming search using Aho-Corasick or rolling buffer
    - [x] Write tests for Content-Length recalculation
    - [x] Implement content-length update after substitution
    - [x] Write tests for Transfer-Encoding: chunked passthrough

- [x] Task: Implement body injection (addition_before/after)
    - [x] Write tests for prepending content before response body
    - [x] Write tests for appending content after response body
    - [x] Implement file-based and inline injection
    - [x] Write tests for Content-Length adjustment

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Syslog Integration

- [x] Task: Implement syslog client (`crates/proxy/src/syslog.rs`)
    - [x] Write tests for RFC 5424 syslog message formatting
    - [x] Implement syslog message builder (priority, timestamp, hostname, app-name, PID, message)
    - [x] Write tests for UDP syslog delivery
    - [x] Implement UDP syslog sender
    - [x] Write tests for TCP syslog delivery
    - [x] Implement TCP syslog sender with reconnection
    - [x] Write tests for TCP+TLS syslog transport
    - [x] Implement TLS-encrypted syslog using rustls

- [x] Task: Integrate syslog with logging system
    - [x] Write tests for access log → syslog routing
    - [x] Write tests for error log → syslog routing
    - [x] Implement syslog as a log output backend alongside file output
    - [x] Write tests for structured data (SD-ELEMENT) in syslog messages
    - [x] Implement SD-ELEMENT serialization

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Server Side Includes (SSI)

- [x] Task: Implement SSI parser (`crates/proxy/src/ssi.rs`)
    - [x] Write tests for <!--#include virtual="..." --> parsing
    - [x] Write tests for <!--#include file="..." --> parsing
    - [x] Write tests for <!--#echo var="..." --> parsing
    - [x] Write tests for <!--#set var="..." value="..." --> parsing
    - [x] Write tests for <!--#if expr="..." --> conditional parsing
    - [x] Implement SSI directive parser using regex or custom scanner

- [x] Task: Implement SSI execution
    - [x] Write tests for virtual include (internal subrequest)
    - [x] Implement subrequest dispatch for virtual includes
    - [x] Write tests for file include (read local file)
    - [x] Implement file-based includes
    - [x] Write tests for variable echo and set
    - [x] Implement SSI variable scope
    - [x] Write tests for conditional evaluation
    - [x] Write tests for recursion depth limit (max 10 nested includes)
    - [x] Implement depth tracking and limit enforcement

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Image Filter & XSLT

- [x] Task: Implement basic image filter (`crates/proxy/src/image_filter.rs`)
    - [x] Write tests for image resize (width × height)
    - [x] Implement resize using image crate
    - [x] Write tests for image crop (center crop)
    - [x] Implement crop operation
    - [x] Write tests for JPEG quality adjustment
    - [x] Implement quality control
    - [x] Write tests for content-type detection (only process image/*)
    - [x] Write tests for rotation (90, 180, 270 degrees)

- [x] Task: Implement XSLT transformation
    - [x] Write tests for XML + XSLT → HTML transformation
    - [x] Implement XSLT processing (evaluate Rust XSLT crate availability, fallback to libxslt FFI)
    - [x] Write tests for XSLT parameter injection from request variables
    - [x] Write tests for content-type filtering (only XML responses)

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
