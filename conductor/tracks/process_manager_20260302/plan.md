# Implementation Plan: Process Manager Core (v0.15.0)

## Phase 1: Process Manager Crate Foundation

- [x] Task: Create `crates/procman` crate with Cargo.toml (dependencies: tokio, serde, nix, sysinfo)
    - [x] Define crate module structure: `lib.rs`, `daemon.rs`, `process.rs`, `table.rs`, `config.rs`, `ipc.rs`, `cluster.rs`
    - [x] Add crate to workspace `Cargo.toml`

- [x] Task: Implement process data model (`process.rs`)
    - [x] Write tests for ProcessInfo struct (status, pid, name, restarts, uptime, memory, cpu)
    - [x] Implement ProcessInfo with serialization support
    - [x] Write tests for ProcessStatus enum (Online, Stopping, Stopped, Errored, Launching)
    - [x] Implement ProcessStatus with Display and state transitions
    - [x] Write tests for ProcessConfig (script, args, env, cwd, instances, max_memory, max_restarts)
    - [x] Implement ProcessConfig with defaults and validation

- [x] Task: Implement process table persistence (`table.rs`)
    - [x] Write tests for ProcessTable CRUD operations (add, remove, get, list, update)
    - [x] Implement in-memory ProcessTable with RwLock
    - [x] Write tests for JSON persistence (save to disk, load from disk, handle corruption)
    - [x] Implement save/load with atomic file writes (write-to-temp + rename)
    - [x] Write tests for process re-adoption by PID on daemon restart

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Process Lifecycle Management

- [x] Task: Implement process spawning and management (`daemon.rs`)
    - [x] Write tests for process spawning (fork, exec, environment injection)
    - [x] Implement spawn_process() using tokio::process::Command
    - [x] Write tests for graceful stop (SIGTERM → wait → SIGKILL with timeout)
    - [x] Implement stop_process() with configurable grace period
    - [x] Write tests for restart_process() (stop + start with delay)
    - [x] Implement restart_process()
    - [x] Write tests for delete_process() (stop + remove from table)
    - [x] Implement delete_process()

- [x] Task: Implement automatic crash recovery
    - [x] Write tests for crash detection (process exit monitoring)
    - [x] Implement process exit monitoring using tokio::process::Child::wait()
    - [x] Write tests for exponential backoff (1s, 2s, 4s, 8s, 16s, max 30s)
    - [x] Implement ExponentialBackoff struct
    - [x] Write tests for max_restarts enforcement within restart_window
    - [x] Implement restart logic with window-based tracking
    - [x] Write tests for max_memory_restart (mock sysinfo readings)
    - [x] Implement memory monitoring with sysinfo crate and auto-restart trigger

- [x] Task: Implement zero-downtime reload
    - [x] Write tests for reload logic (new instance up → health check → old instance down)
    - [x] Implement reload_process() with readiness probe integration
    - [x] Write tests for rolling restart in cluster mode
    - [x] Implement rolling_restart() that restarts instances one-by-one

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Cluster Mode & Multi-Instance

- [x] Task: Implement cluster mode (`cluster.rs`)
    - [x] Write tests for CPU core detection (auto `max` instances)
    - [x] Implement cpu_count() using std::thread::available_parallelism
    - [x] Write tests for multi-instance spawning with unique INSTANCE_ID
    - [x] Implement spawn_cluster() that creates N instances with sequential IDs
    - [x] Write tests for cluster status aggregation
    - [x] Implement cluster-level status reporting (all instances' combined metrics)

- [x] Task: Implement per-instance environment variable injection
    - [x] Write tests for INSTANCE_ID, PM_ID, AEGIS_APP_NAME injection
    - [x] Implement automatic env var injection per instance
    - [x] Write tests for .env file loading and merging
    - [x] Implement dotenv loading with override precedence

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: IPC & Daemon Communication

- [x] Task: Implement IPC server (`ipc.rs`)
    - [x] Write tests for Unix domain socket server creation and cleanup
    - [x] Implement IPC server listening on `/tmp/aegis-flow.sock` (or XDG_RUNTIME_DIR)
    - [x] Write tests for IPC message protocol (JSON-RPC style: start, stop, restart, reload, delete, list, status)
    - [x] Implement IpcMessage enum with serialization
    - [x] Write tests for concurrent IPC client handling
    - [x] Implement multi-client handler with tokio tasks

- [x] Task: Implement IPC client (for CLI integration)
    - [x] Write tests for client connect, send command, receive response
    - [x] Implement IpcClient struct with connect/send/receive methods
    - [x] Write tests for timeout and error handling
    - [x] Implement connection timeout and retry logic

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Ecosystem Configuration

- [x] Task: Implement ecosystem config parser (`config.rs`)
    - [x] Write tests for TOML ecosystem file parsing (single/multiple apps)
    - [x] Implement EcosystemConfig struct with deserialization
    - [x] Write tests for YAML ecosystem file parsing
    - [x] Implement YAML format support
    - [x] Write tests for environment-specific overrides (env_production, env_staging)
    - [x] Implement environment profile merging logic
    - [x] Write tests for config validation (required fields, port conflicts, path existence)
    - [x] Implement config validation with descriptive error messages

- [x] Task: Implement ecosystem commands
    - [x] Write tests for `start ecosystem.toml` (launch all apps)
    - [x] Implement start_ecosystem() that iterates config and spawns each app
    - [x] Write tests for `stop all` and `restart all`
    - [x] Implement batch operations (stop_all, restart_all, reload_all)

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)

## Phase 6: Resource Monitoring & Metrics Integration

- [x] Task: Implement resource monitoring
    - [x] Write tests for CPU percentage calculation per process
    - [x] Implement CPU monitoring using sysinfo crate
    - [x] Write tests for RSS memory tracking per process
    - [x] Implement memory monitoring
    - [x] Write tests for periodic sampling (configurable interval, default 5s)
    - [x] Implement sampling loop with configurable interval

- [x] Task: Integrate with existing Prometheus metrics
    - [x] Write tests for new gauges (aegis_process_cpu_percent, aegis_process_memory_bytes)
    - [x] Register new metrics in metrics.rs
    - [x] Write tests for process-level label cardinality (app_name, instance_id)
    - [x] Implement metric recording in the monitoring loop
    - [x] Write tests for aegis_process_restarts_total counter
    - [x] Implement restart counter metric

- [x] Task: Conductor - User Manual Verification 'Phase 6' (Protocol in workflow.md)
