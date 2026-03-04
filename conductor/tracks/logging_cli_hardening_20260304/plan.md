# Implementation Plan: Log Management & CLI Hardening (v0.46.0)

## Phase 1: `crates/cli` Bootstrap
- [ ] Task: Create CLI Binary package
    - [ ] Seed `crates/cli/Cargo.toml` with `clap`, `comfy-table`, `crossterm`, `ratatui`, `tokio`.
    - [ ] Map out subcommands in `crates/cli/src/main.rs`.
    - [ ] Establish fundamental UDS or TCP Client bindings to the daemon.

## Phase 2: Async Log Pipelines
- [ ] Task: Buffered Writer Implementation
    - [ ] In `access_log.rs`, create a worker `tokio::spawn` listening on an `mpsc::Receiver<String>`.
    - [ ] The worker buffers entries into a `tokio::io::BufWriter<tokio::fs::File>`.
    - [ ] Export a `Sender` clone to the `HttpProxy` struct.

## Phase 3: Proxy Request Logging
- [ ] Task: Integrate Access Log into Request Lifecycle
    - [ ] Intercept the start `Instant` of every request.
    - [ ] In the response path, construct an `AccessLogRecord`.
    - [ ] Format the record (JSON or Combined) and dispatch into the Sender channel.

## Phase 4: File Rotation System
- [ ] Task: Size-Based Rotation Daemon
    - [ ] In the background log writer task, track bytes written globally.
    - [ ] Once `max_size` is surpassed, trigger `File::rename`.
    - [ ] Execute an async Gzip compression task on the renamed file.
    - [ ] Clean up files violating the `max_files` limit.

## Phase 5: TUI and Extended Commands
- [ ] Task: Implement `aegis monit`
    - [ ] Write terminal-based renderers utilizing `ratatui` widgets.
    - [ ] Implement `aegis logs` using `notify` to tail the actively written files.
    - [ ] Ensure proper graceful exit logic using `crossterm` events.
    - [ ] Testing protocol in `workflow.md`.
