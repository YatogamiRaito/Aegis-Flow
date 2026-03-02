# Implementation Plan: Log Management & CLI Interface (v0.21.0)

## Phase 1: Access & Error Logging

- [ ] Task: Create logging module (`crates/proxy/src/access_log.rs`)
    - [ ] Write tests for combined format template rendering
    - [ ] Implement combined format: $remote_addr, $time_local, $request, $status, etc.
    - [ ] Write tests for JSON format log entry serialization
    - [ ] Implement JSON log format with all fields
    - [ ] Write tests for custom format template parsing and rendering
    - [ ] Implement custom format with variable interpolation

- [ ] Task: Implement async buffered log writer
    - [ ] Write tests for buffered writes (batch N entries before flushing)
    - [ ] Implement AsyncLogWriter with configurable buffer size using tokio::io::BufWriter
    - [ ] Write tests for non-blocking write behavior (log writes don't block request handling)
    - [ ] Implement channel-based async log pipeline (mpsc sender in handler → receiver in writer task)

- [ ] Task: Implement error log
    - [ ] Write tests for severity level filtering (only log >= configured level)
    - [ ] Implement error log with level filtering
    - [ ] Write tests for structured error entries (timestamp, level, module, message, context)
    - [ ] Implement structured error formatter
    - [ ] Write tests for per-module log files (separate proxy.log, waf.log, etc.)
    - [ ] Implement module-based log routing

- [ ] Task: Implement conditional logging
    - [ ] Write tests for status code conditions (log only 4xx/5xx)
    - [ ] Implement condition evaluator for log entries
    - [ ] Write tests for request path conditions

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Log Rotation

- [ ] Task: Implement size-based rotation
    - [ ] Write tests for rotation trigger at max_size threshold
    - [ ] Implement file size monitoring and rotation
    - [ ] Write tests for file renaming chain (access.log → access.log.1 → access.log.2)
    - [ ] Implement rotation numbering logic

- [ ] Task: Implement time-based rotation
    - [ ] Write tests for daily rotation at midnight
    - [ ] Write tests for hourly rotation
    - [ ] Implement timer-based rotation using tokio::time::interval

- [ ] Task: Implement compression and retention
    - [ ] Write tests for gzip compression of rotated files
    - [ ] Implement async gzip compression of old log files
    - [ ] Write tests for max_files retention (delete oldest beyond limit)
    - [ ] Implement retention enforcement

- [ ] Task: Implement signal-based rotation
    - [ ] Write tests for SIGUSR1 handler that triggers rotation
    - [ ] Implement signal handler integration with rotation task
    - [ ] Write tests for log file reopen after rotation (important for scripted log management)

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Process Log Capture

- [ ] Task: Implement stdout/stderr capture for managed processes
    - [ ] Write tests for redirecting child process stdout to a file
    - [ ] Write tests for redirecting child process stderr to a file
    - [ ] Implement pipe-based stdout/stderr capture in process spawner
    - [ ] Write tests for per-process log file paths (configurable)
    - [ ] Implement log path resolution from ecosystem config

- [ ] Task: Implement live log streaming
    - [ ] Write tests for `aegis logs <app>` streaming output
    - [ ] Implement file tail + watch for new lines using notify crate (file watcher)
    - [ ] Write tests for `--lines N` flag (show last N lines)
    - [ ] Implement tail -n functionality
    - [ ] Write tests for merged log output with colored app name prefix
    - [ ] Implement interleaved multi-app log merging

- [ ] Task: Implement log flush
    - [ ] Write tests for `aegis flush` truncating all log files
    - [ ] Implement flush command via IPC

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: CLI Interface

- [ ] Task: Create CLI binary (`crates/cli/src/main.rs`)
    - [ ] Set up CLI argument parsing using clap crate (derive mode)
    - [ ] Define all commands: start, stop, restart, reload, delete, list, monit, logs, status, info, save, resurrect, startup, flush
    - [ ] Implement IPC client connection to daemon

- [ ] Task: Implement `aegis list` command
    - [ ] Write tests for table formatting with columns: id, name, mode, pid, status, restarts, cpu, mem
    - [ ] Implement table renderer using comfy-table or tabled crate
    - [ ] Write tests for colored status indicators (green=online, red=errored, yellow=stopping)
    - [ ] Implement ANSI color output

- [ ] Task: Implement `aegis status <app>` command
    - [ ] Write tests for detailed process info output
    - [ ] Implement detailed view: config, env vars (masked), log paths, metrics, uptime, restarts

- [ ] Task: Implement `aegis info` command
    - [ ] Write tests for system info output (version, daemon PID, uptime, OS)
    - [ ] Implement system info collection

- [ ] Task: Implement `aegis save` and `aegis resurrect`
    - [ ] Write tests for saving process list to dump file
    - [ ] Implement save (serialize process table to ~/.aegis/dump.json)
    - [ ] Write tests for resurrect (load dump and restart all processes)
    - [ ] Implement resurrect command

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: TUI Monitor & Startup Scripts

- [ ] Task: Implement `aegis monit` TUI dashboard
    - [ ] Set up ratatui + crossterm for terminal UI
    - [ ] Write tests for process list widget rendering
    - [ ] Implement process table widget with live CPU/memory bar charts
    - [ ] Implement log tail widget showing latest logs
    - [ ] Implement system resource overview widget (total CPU, memory, uptime)
    - [ ] Implement keyboard handler (q=quit, r=restart, s=stop, d=delete, ↑↓=navigate)
    - [ ] Implement 2 FPS refresh loop with IPC polling

- [ ] Task: Implement `aegis startup` command
    - [ ] Write tests for systemd service file generation
    - [ ] Implement systemd unit file template with ExecStart, Restart=always, WantedBy=multi-user.target
    - [ ] Write tests for launchd plist generation
    - [ ] Implement launchd plist template with KeepAlive, RunAtLoad
    - [ ] Implement platform detection (Linux vs macOS)
    - [ ] Write tests for `aegis unstartup` (service file removal)
    - [ ] Implement unstartup command

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
