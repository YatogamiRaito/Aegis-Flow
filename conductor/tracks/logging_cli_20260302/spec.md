# Track Specification: Log Management & CLI Interface (v0.21.0)

## 1. Overview

This track adds a comprehensive **log management system** (nginx-style access/error logs with rotation) and a **rich CLI interface** (PM2-style `list`, `monit`, `logs` commands) to Aegis-Flow. It also includes startup script generation for system services (systemd/launchd) so that Aegis-Flow can run as a system daemon.

## 2. Functional Requirements

### 2.1 Access Logging
- nginx-style access log with configurable format.
- **Built-in log formats:**
  - `combined`: `$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent"`
  - `json`: Structured JSON output with all fields.
  - `custom`: User-defined template using variables.
- Per-location and per-server access log configuration.
- Conditional logging: only log requests matching conditions (e.g., `log_condition = "$status >= 400"`).
- Buffer size: configurable write buffer (default: 64KB) for batched writes.
- Async I/O: log writes are non-blocking.

### 2.2 Error Logging
- Severity levels: `debug`, `info`, `notice`, `warn`, `error`, `crit`, `alert`, `emerg`.
- Configurable minimum log level (default: `error`).
- Per-module error log files (e.g., separate proxy.log, waf.log, cache.log).
- Structured error log entries with timestamp, level, module, message, and context data.

### 2.3 Log Rotation
- **Size-based rotation:** Rotate when log file exceeds configurable size (default: 100MB).
- **Time-based rotation:** Rotate daily, hourly, or at custom intervals.
- **Retention:** Keep configurable number of rotated files (default: 10).
- **Compression:** Optionally compress rotated files with gzip.
- File naming: `access.log`, `access.log.1`, `access.log.2.gz`, etc.
- Signal-based rotation: `SIGUSR1` triggers immediate rotation and file reopen.

### 2.4 Process Log Management (PM2-style)
- Capture stdout/stderr from managed processes and redirect to per-process log files.
- `aegis logs <app>` streams live log output (like `pm2 logs`).
- `aegis logs <app> --lines 100` shows last 100 lines.
- `aegis flush` truncates all log files.
- Per-process log paths configurable in ecosystem config.
- Log merge: `aegis logs` (no app) interleaves logs from all apps with colored prefixes.

### 2.5 CLI Interface (aegis command)
- **`aegis list` / `aegis ls`:** Table output showing all managed processes.
  ```
  ┌────┬──────────────┬──────┬─────┬────────┬──────┬─────────┬───────────┐
  │ id │ name         │ mode │ pid │ status │ ↺   │ cpu     │ mem       │
  ├────┼──────────────┼──────┼─────┼────────┼──────┼─────────┼───────────┤
  │ 0  │ api-server   │ cluster│1234│ online │ 0   │ 2.1%   │ 45.2 MB   │
  │ 1  │ api-server   │ cluster│1235│ online │ 0   │ 1.8%   │ 42.1 MB   │
  │ 2  │ worker       │ fork  │1240│ online │ 3   │ 0.5%   │ 28.0 MB   │
  └────┴──────────────┴──────┴─────┴────────┴──────┴─────────┴───────────┘
  ```
- **`aegis monit`:** Real-time TUI dashboard using ratatui.
  - Process list with live CPU/memory bars.
  - Log tail view.
  - System resource overview (total CPU, memory, network).
  - Keyboard navigation and process control (r=restart, s=stop, d=delete).
- **`aegis status <app>`:** Detailed info for a specific process (config, env, logs path, metrics).
- **`aegis info`:** System information (Aegis-Flow version, daemon status, uptime, Rust version).
- **`aegis save`:** Save current process list to dump file for restart recovery.
- **`aegis resurrect`:** Restore previously saved process list.

### 2.6 Startup Script Generation
- **`aegis startup`:** Generate and install system service files.
  - **Linux (systemd):** Generate `/etc/systemd/system/aegis-flow.service`.
  - **macOS (launchd):** Generate `~/Library/LaunchAgents/com.aegis-flow.plist`.
  - Auto-detect platform and generate appropriate file.
- **`aegis unstartup`:** Remove the installed service file.
- Service should auto-start `aegis resurrect` on boot.

### 2.7 Configuration Example
```toml
[logging]
  [logging.access]
  enabled = true
  path = "/var/log/aegis/access.log"
  format = "combined"
  buffer_size = "64K"
  condition = ""

  [logging.error]
  enabled = true
  path = "/var/log/aegis/error.log"
  level = "warn"

  [logging.rotation]
  max_size = "100M"
  max_files = 10
  compress = true
  interval = "daily"

[cli]
  color = true
  timestamp_format = "ISO8601"
```

## 3. Non-Functional Requirements

- Log write latency: < 1µs (async buffered I/O).
- CLI responsiveness: < 100ms for any command.
- TUI refresh rate: 2 FPS minimum, < 5% CPU.
- Startup script generation: idempotent.

## 4. Acceptance Criteria

- [ ] Access log written in configurable format (combined, json, custom).
- [ ] Error log with configurable severity levels.
- [ ] Log rotation works by size and time.
- [ ] Compressed rotated files.
- [ ] SIGUSR1 triggers log file rotation.
- [ ] `aegis list` shows formatted process table.
- [ ] `aegis monit` shows TUI dashboard with live data.
- [ ] `aegis logs <app>` streams live logs.
- [ ] `aegis startup` generates systemd/launchd service files.
- [ ] `aegis save` and `aegis resurrect` persist and restore processes.
- [ ] Per-process stdout/stderr capture works.
- [ ] >90% test coverage.

## 5. Out of Scope

- Centralized log aggregation (ELK/Loki integration).
- Log analytics or search.
