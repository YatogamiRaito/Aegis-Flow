# Implementation Plan: Log Management & CLI Interface (v0.21.0)

## Phase 1: Access & Error Logging

- [x] Task: Create logging module (`crates/proxy/src/access_log.rs`)
    - [x] Write tests for combined format template rendering
    - [x] Implement combined format: $remote_addr, $time_local, $request, $status, etc.
    - [x] Write tests for JSON format log entry serialization
    - [x] Implement JSON log format with all fields
    - [x] Write tests for custom format template parsing and rendering
    - [x] Implement custom format with variable interpolation

- [x] Task: Implement async buffered log writer
    - [x] Write tests for buffered writes (batch N entries before flushing)
    - [x] Implement AsyncLogWriter with configurable buffer size using tokio::io::BufWriter
    - [x] Write tests for non-blocking write behavior (log writes don't block request handling)
    - [x] Implement channel-based async log pipeline (mpsc sender in handler → receiver in writer task)

- [x] Task: Implement error log
    - [x] Write tests for severity level filtering (only log >= configured level)
    - [x] Implement error log with level filtering
    - [x] Write tests for structured error entries (timestamp, level, module, message, context)
    - [x] Implement structured error formatter
    - [x] Write tests for per-module log files (separate proxy.log, waf.log, etc.)
    - [x] Implement module-based log routing

- [x] Task: Implement conditional logging
    - [x] Write tests for status code conditions (log only 4xx/5xx)
    - [x] Implement condition evaluator for log entries
    - [x] Write tests for request path conditions

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Log Rotation

- [x] Task: Implement size-based rotation
    - [x] Write tests for rotation trigger at max_size threshold
    - [x] Implement file size monitoring and rotation
    - [x] Write tests for file renaming chain (access.log → access.log.1 → access.log.2)
    - [x] Implement rotation numbering logic

- [x] Task: Implement time-based rotation
    - [x] Write tests for daily rotation at midnight
    - [x] Write tests for hourly rotation
    - [x] Implement timer-based rotation using tokio::time::interval

- [x] Task: Implement compression and retention
    - [x] Write tests for gzip compression of rotated files
    - [x] Implement async gzip compression of old log files
    - [x] Write tests for max_files retention (delete oldest beyond limit)
    - [x] Implement retention enforcement

- [x] Task: Implement signal-based rotation
    - [x] Write tests for SIGUSR1 handler that triggers rotation
    - [x] Implement signal handler integration with rotation task
    - [x] Write tests for log file reopen after rotation (important for scripted log management)

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Process Log Capture

- [x] Task: Implement stdout/stderr capture for managed processes
    - [x] Write tests for redirecting child process stdout to a file
    - [x] Write tests for redirecting child process stderr to a file
    - [x] Implement pipe-based stdout/stderr capture in process spawner
    - [x] Write tests for per-process log file paths (configurable)
    - [x] Implement log path resolution from ecosystem config

- [x] Task: Implement live log streaming
    - [x] Write tests for `aegis logs <app>` streaming output
    - [x] Implement file tail + watch for new lines using notify crate (file watcher)
    - [x] Write tests for `--lines N` flag (show last N lines)
    - [x] Implement tail -n functionality
    - [x] Write tests for merged log output with colored app name prefix
    - [x] Implement interleaved multi-app log merging

- [x] Task: Implement log flush
    - [x] Write tests for `aegis flush` truncating all log files
    - [x] Implement flush command via IPC

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: CLI Interface

- [x] Task: Create CLI binary (`crates/cli/src/main.rs`)
    - [x] Set up CLI argument parsing using clap crate (derive mode)
    - [x] Define all commands: start, stop, restart, reload, delete, list, monit, logs, status, info, save, resurrect, startup, flush
    - [x] Implement IPC client connection to daemon

- [x] Task: Implement `aegis list` command
    - [x] Write tests for table formatting with columns: id, name, mode, pid, status, restarts, cpu, mem
    - [x] Implement table renderer using comfy-table or tabled crate
    - [x] Write tests for colored status indicators (green=online, red=errored, yellow=stopping)
    - [x] Implement ANSI color output

- [x] Task: Implement `aegis status <app>` command
    - [x] Write tests for detailed process info output
    - [x] Implement detailed view: config, env vars (masked), log paths, metrics, uptime, restarts

- [x] Task: Implement `aegis info` command
    - [x] Write tests for system info output (version, daemon PID, uptime, OS)
    - [x] Implement system info collection

- [x] Task: Implement `aegis save` and `aegis resurrect`
    - [x] Write tests for saving process list to dump file
    - [x] Implement save (serialize process table to ~/.aegis/dump.json)
    - [x] Write tests for resurrect (load dump and restart all processes)
    - [x] Implement resurrect command

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: TUI Monitor & Startup Scripts

- [x] Task: Implement `aegis monit` TUI dashboard
    - [x] Set up ratatui + crossterm for terminal UI
    - [x] Write tests for process list widget rendering
    - [x] Implement process table widget with live CPU/memory bar charts
    - [x] Implement log tail widget showing latest logs
    - [x] Implement system resource overview widget (total CPU, memory, uptime)
    - [x] Implement keyboard handler (q=quit, r=restart, s=stop, d=delete, ↑↓=navigate)
    - [x] Implement 2 FPS refresh loop with IPC polling

- [x] Task: Implement `aegis startup` command
    - [x] Write tests for systemd service file generation
    - [x] Implement systemd unit file template with ExecStart, Restart=always, WantedBy=multi-user.target
    - [x] Write tests for launchd plist generation
    - [x] Implement launchd plist template with KeepAlive, RunAtLoad
    - [x] Implement platform detection (Linux vs macOS)
    - [x] Write tests for `aegis unstartup` (service file removal)
    - [x] Implement unstartup command

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
