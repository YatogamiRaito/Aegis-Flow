# Implementation Plan: Process Manager Core (v0.15.0)

## Phase 1: Process Manager Crate Foundation

- [ ] Task: Create `crates/procman` crate with Cargo.toml (dependencies: tokio, serde, nix, sysinfo)
    - [ ] Define crate module structure: `lib.rs`, `daemon.rs`, `process.rs`, `table.rs`, `config.rs`, `ipc.rs`, `cluster.rs`
    - [ ] Add crate to workspace `Cargo.toml`

- [ ] Task: Implement process data model (`process.rs`)
    - [ ] Write tests for ProcessInfo struct (status, pid, name, restarts, uptime, memory, cpu)
    - [ ] Implement ProcessInfo with serialization support
    - [ ] Write tests for ProcessStatus enum (Online, Stopping, Stopped, Errored, Launching)
    - [ ] Implement ProcessStatus with Display and state transitions
    - [ ] Write tests for ProcessConfig (script, args, env, cwd, instances, max_memory, max_restarts)
    - [ ] Implement ProcessConfig with defaults and validation

- [ ] Task: Implement process table persistence (`table.rs`)
    - [ ] Write tests for ProcessTable CRUD operations (add, remove, get, list, update)
    - [ ] Implement in-memory ProcessTable with RwLock
    - [ ] Write tests for JSON persistence (save to disk, load from disk, handle corruption)
    - [ ] Implement save/load with atomic file writes (write-to-temp + rename)
    - [ ] Write tests for process re-adoption by PID on daemon restart

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Process Lifecycle Management

- [ ] Task: Implement process spawning and management (`daemon.rs`)
    - [ ] Write tests for process spawning (fork, exec, environment injection)
    - [ ] Implement spawn_process() using tokio::process::Command
    - [ ] Write tests for graceful stop (SIGTERM → wait → SIGKILL with timeout)
    - [ ] Implement stop_process() with configurable grace period
    - [ ] Write tests for restart_process() (stop + start with delay)
    - [ ] Implement restart_process()
    - [ ] Write tests for delete_process() (stop + remove from table)
    - [ ] Implement delete_process()

- [ ] Task: Implement automatic crash recovery
    - [ ] Write tests for crash detection (process exit monitoring)
    - [ ] Implement process exit monitoring using tokio::process::Child::wait()
    - [ ] Write tests for exponential backoff (1s, 2s, 4s, 8s, 16s, max 30s)
    - [ ] Implement ExponentialBackoff struct
    - [ ] Write tests for max_restarts enforcement within restart_window
    - [ ] Implement restart logic with window-based tracking
    - [ ] Write tests for max_memory_restart (mock sysinfo readings)
    - [ ] Implement memory monitoring with sysinfo crate and auto-restart trigger

- [ ] Task: Implement zero-downtime reload
    - [ ] Write tests for reload logic (new instance up → health check → old instance down)
    - [ ] Implement reload_process() with readiness probe integration
    - [ ] Write tests for rolling restart in cluster mode
    - [ ] Implement rolling_restart() that restarts instances one-by-one

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Cluster Mode & Multi-Instance

- [ ] Task: Implement cluster mode (`cluster.rs`)
    - [ ] Write tests for CPU core detection (auto `max` instances)
    - [ ] Implement cpu_count() using std::thread::available_parallelism
    - [ ] Write tests for multi-instance spawning with unique INSTANCE_ID
    - [ ] Implement spawn_cluster() that creates N instances with sequential IDs
    - [ ] Write tests for cluster status aggregation
    - [ ] Implement cluster-level status reporting (all instances' combined metrics)

- [ ] Task: Implement per-instance environment variable injection
    - [ ] Write tests for INSTANCE_ID, PM_ID, AEGIS_APP_NAME injection
    - [ ] Implement automatic env var injection per instance
    - [ ] Write tests for .env file loading and merging
    - [ ] Implement dotenv loading with override precedence

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: IPC & Daemon Communication

- [ ] Task: Implement IPC server (`ipc.rs`)
    - [ ] Write tests for Unix domain socket server creation and cleanup
    - [ ] Implement IPC server listening on `/tmp/aegis-flow.sock` (or XDG_RUNTIME_DIR)
    - [ ] Write tests for IPC message protocol (JSON-RPC style: start, stop, restart, reload, delete, list, status)
    - [ ] Implement IpcMessage enum with serialization
    - [ ] Write tests for concurrent IPC client handling
    - [ ] Implement multi-client handler with tokio tasks

- [ ] Task: Implement IPC client (for CLI integration)
    - [ ] Write tests for client connect, send command, receive response
    - [ ] Implement IpcClient struct with connect/send/receive methods
    - [ ] Write tests for timeout and error handling
    - [ ] Implement connection timeout and retry logic

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Ecosystem Configuration

- [ ] Task: Implement ecosystem config parser (`config.rs`)
    - [ ] Write tests for TOML ecosystem file parsing (single app)
    - [ ] Write tests for TOML ecosystem file parsing (multiple apps)
    - [ ] Implement EcosystemConfig struct with deserialization
    - [ ] Write tests for YAML ecosystem file parsing
    - [ ] Implement YAML format support
    - [ ] Write tests for environment-specific overrides (env_production, env_staging)
    - [ ] Implement environment profile merging logic
    - [ ] Write tests for config validation (required fields, port conflicts, path existence)
    - [ ] Implement config validation with descriptive error messages

- [ ] Task: Implement ecosystem commands
    - [ ] Write tests for `start ecosystem.toml` (launch all apps)
    - [ ] Implement start_ecosystem() that iterates config and spawns each app
    - [ ] Write tests for `stop all` and `restart all`
    - [ ] Implement batch operations (stop_all, restart_all, reload_all)

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)

## Phase 6: Resource Monitoring & Metrics Integration

- [ ] Task: Implement resource monitoring
    - [ ] Write tests for CPU percentage calculation per process
    - [ ] Implement CPU monitoring using sysinfo crate
    - [ ] Write tests for RSS memory tracking per process
    - [ ] Implement memory monitoring
    - [ ] Write tests for periodic sampling (configurable interval, default 5s)
    - [ ] Implement sampling loop with configurable interval

- [ ] Task: Integrate with existing Prometheus metrics
    - [ ] Write tests for new gauges (aegis_process_cpu_percent, aegis_process_memory_bytes)
    - [ ] Register new metrics in metrics.rs
    - [ ] Write tests for process-level label cardinality (app_name, instance_id)
    - [ ] Implement metric recording in the monitoring loop
    - [ ] Write tests for aegis_process_restarts_total counter
    - [ ] Implement restart counter metric

- [ ] Task: Conductor - User Manual Verification 'Phase 6' (Protocol in workflow.md)
