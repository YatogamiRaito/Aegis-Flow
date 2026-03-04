# Track Plan: Process Manager CLI & Daemon Bootstrapping (v0.40.0)

## Phase 1: CLI Configuration
- [ ] Task: Integrate `clap` CLI subcommands (`start`, `stop`, `restart`, `list`, `daemon`) in the main binary entrypoint.
- [ ] Task: Map the CLI commands to their respective `procman::ipc::IpcClient` payload generations.
- [ ] Task: Conductor Verification 'CLI Mapping'

## Phase 2: Daemon Bootstrapping
- [ ] Task: Implement proxy daemon detachment (forking) using the `nix` crate or a double-fork pattern for `aegis daemon`.
- [ ] Task: Write the `run_daemon()` event loop that initializes `ProcessManager`, `EcosystemManager`, and listens on `IpcServer`.
- [ ] Task: Conductor Verification 'Daemon Bootstrapping'

## Phase 3: UX & Feedback Formatting
- [ ] Task: Format the `IpcResponse::ProcessList` return payload into a readable terminal table (PID, Name, Restarts, Uptime, Memory, CPU, Status).
- [ ] Task: Forward daemon logs nicely to the end user or a centralized log file (`~/.aegis/daemon.log`).
- [ ] Task: Conductor Verification 'UX Finishing'

## Phase 4: Integration testing
- [ ] Task: Write UI/CLI-level integration tests asserting output.
- [ ] Task: Release v0.40.0.
