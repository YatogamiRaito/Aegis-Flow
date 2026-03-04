# Track Specification: Log Management & CLI Hardening (v0.46.0)

## 1. Overview
The current proxy architecture contains a very basic implementation of access log formatting and system script generation within `access_log.rs`. However, the critical runtime features of Track 21 (Async buffered log writing, Log Rotation, IPC Log Streaming, and the `crates/cli` package) were not implemented. This track serves to build these missing observability and control systems to complete the logging and CLI ecosystem for Aegis-Flow.

## 2. Functional Requirements

### 2.1 The CLI Application (`aegis`)
- Create the `crates/cli` package using `clap` for command parsing and `comfy-table` for terminal output.
- Support `list`, `monit` (dashboard using `ratatui`), and `logs` streaming.
- Support proxy lifecycle flags: `start`, `stop`, `restart`, `reload`, `save`, and `resurrect`.
- Send remote commands to the Daemon/Process Manager via an IPC socket.

### 2.2 Access Logging & Async Buffers
- Update `config.rs` to ingest `[logging.access]`, `[logging.error]`, and `[logging.rotation]`.
- Instantiate an `AsyncLogWriter` backed by `tokio::io::BufWriter` and a multi-producer single-consumer (`mpsc`) channel.
- Inside `http_proxy.rs`, construct `AccessLogRecord` instances upon request completion (`Drop` handler or end of `handle_request()`).
- Send records into the `mpsc` channel so as not to block the event loop while writing to the disk.

### 2.3 Log Rotation
- Embed a `tokio::spawn` background task that actively monitors log file sizes.
- Upon exceeding `max_size` or receiving a `SIGUSR1`, safely rotate `access.log` to `access.log.1`, `access.log.2`, etc.
- Compress old logs as `.gz` and purge files that exceed the `max_files` retention limit.

## 3. Non-Functional Requirements
- **Performance:** Logging must add < 10µs to the critical path. String allocations should be minimized. The queue should be bounded to applying back-pressure.
- **Resilience:** The CLI and Daemon must communicate over a secure UDS (Unix Domain Socket). The socket must have the correct permissions.

## 4. Acceptance Criteria
- [ ] `aegis list` successfully queries and displays running Aegis-Flow instances and their metrics.
- [ ] Valid HTTP requests emit formatted lines to `access.log` instantaneously.
- [ ] Log files exceeding their size limits are correctly rotated into `.gz` archives.
- [ ] `aegis monit` opens a working TUI dashboard displaying real-time metrics.
