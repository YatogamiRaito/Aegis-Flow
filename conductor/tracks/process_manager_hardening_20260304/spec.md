# Specification: Process Manager CLI & Daemon Bootstrapping (v0.40.0)

## Overview
Fills the integration gap identified in the "Process Manager Core" audit. While the internal `aegis-procman` library works perfectly, it is not connected to the `aegis` Proxy CLI or the application's lifecycle. This track implements the `clap`-based CLI commands and the UNIX domain socket backend daemon loop to make Process Manager usable for end-users.

## Functional Requirements

### 1. CLI Subcommands (`aegis ...`)
Add to the main CLI (`crates/proxy/src/main.rs` or `bootstrap.rs`) utilizing the existing `procman::ipc::IpcClient`:
- `aegis start <app_path | ecosystem.toml>`
- `aegis stop <app_name | all>`
- `aegis restart <app_name | all>`
- `aegis reload <app_name | all>`
- `aegis delete <app_name | all>`
- `aegis status` / `aegis list`

### 2. Daemon Lifecycle
- Command `aegis daemon` to manually start the background daemon process which runs the `IpcServer`.
- Auto-start daemon: if a user runs `aegis start` and the daemon socket is unreachable, automatically fork the daemon in the background before sending the IPC payload.

### 3. State Persistence Wiring
- When the daemon starts, it must read from the local JSON process table and automatically attempt to re-adopt and monitor already running processes, or restart ones that were previously marked as `Online` but died.

## Acceptance Criteria
- [ ] User can run `aegis --help` and see the process manager subcommands.
- [ ] User can run `aegis start app.sh` which forks the daemon (if not running) and launches the application.
- [ ] Output of `aegis status` is formatted beautifully in the terminal (e.g. using `cli-table` or `prettytable-rs`).
- [ ] Integration test successfully sends start and stop commands via CLI and observes process changes.
