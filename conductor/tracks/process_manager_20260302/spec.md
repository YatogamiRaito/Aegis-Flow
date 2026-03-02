# Track Specification: Process Manager Core (v0.15.0)

## 1. Overview

This track introduces a full-featured **process manager** into Aegis-Flow, inspired by PM2's functionality but built natively in Rust. The process manager enables users to start, stop, restart, monitor, and manage multiple application processes through a single daemon. It supports cluster mode for multi-core utilization, automatic crash recovery with exponential backoff, and a declarative ecosystem configuration file.

This is the foundational track that transforms Aegis-Flow from a pure proxy into a **combined proxy + process manager** solution.

## 2. Functional Requirements

### 2.1 Process Daemon (aegis-daemon)
- A long-running background daemon (`aegis-daemon`) that manages all child processes.
- The daemon MUST be forkable to run in the background or run in foreground mode.
- Inter-process communication (IPC) between CLI and daemon via Unix domain sockets.
- The daemon MUST maintain a process table with PID, status, CPU usage, memory usage, restart count, uptime, and logs for each managed process.
- The daemon MUST persist the process table to disk (JSON) so that it survives daemon restarts.

### 2.2 Process Lifecycle Management
- **Start:** Launch a new process from a binary path or script command.
- **Stop:** Gracefully stop a process (SIGTERM → wait → SIGKILL).
- **Restart:** Stop + Start with configurable delay.
- **Reload:** Zero-downtime reload — start new instances, wait for readiness, then stop old instances.
- **Delete:** Remove a process from the managed list entirely.
- **Status:** Query current status (online, stopping, stopped, errored, launching).

### 2.3 Cluster Mode
- Launch N instances of the same application (where N can be `max` for auto-detecting CPU cores).
- Each instance gets a unique `INSTANCE_ID` environment variable.
- Load balancing across instances is handled by the proxy module (integration point).
- Rolling restart support: restart instances one-by-one to maintain availability.

### 2.4 Automatic Crash Recovery
- When a managed process exits unexpectedly, the daemon MUST automatically restart it.
- Exponential backoff: 1s → 2s → 4s → 8s → 16s → max 30s between restarts.
- Configurable `max_restarts` (default: 15) within a `restart_window` (default: 15 minutes).
- If `max_restarts` is exceeded, mark the process as `errored` and stop restarting.
- Configurable `max_memory_restart`: automatically restart if RSS exceeds threshold.

### 2.5 Ecosystem Configuration File
- Support a declarative configuration file (`aegis.ecosystem.toml` or `.yaml`) that defines all managed processes.
- Example structure:
  ```toml
  [[apps]]
  name = "api-server"
  script = "./target/release/api-server"
  instances = 4
  exec_mode = "cluster"
  max_memory_restart = "512M"
  env.NODE_ENV = "production"
  env.PORT = "3000"
  watch = false
  log_file = "/var/log/aegis/api-server.log"

  [[apps]]
  name = "worker"
  script = "./target/release/worker"
  instances = 2
  cron_restart = "0 0 * * *"
  ```
- Support for environment-specific overrides (`env_production`, `env_staging`).

### 2.6 Environment Variable Management
- Per-process environment variable injection via ecosystem config.
- Support for `.env` file loading.
- Environment-specific profiles (production, staging, development).
- Sensitive variable masking in logs and status output.

### 2.7 Resource Monitoring
- Track per-process: CPU %, RSS memory, event loop latency (if applicable), restart count and uptime.
- Expose monitoring data via the existing Prometheus metrics endpoint.
- New metrics: `aegis_process_cpu_percent`, `aegis_process_memory_bytes`, `aegis_process_restarts_total`, `aegis_process_uptime_seconds`.

## 3. Non-Functional Requirements

### 3.1 Performance
- Daemon overhead: < 5 MB RSS for process management of up to 100 processes.
- IPC latency: < 1ms for status queries.
- Process startup time: < 50ms from command to fork.

### 3.2 Reliability
- Daemon crash should NOT affect managed processes (they continue running independently).
- Daemon restart should re-adopt existing managed processes by PID.
- Process table persistence ensures no data loss across daemon restarts.

### 3.3 Security
- Unix socket permissions: 0600 (owner only).
- Process isolation: each managed process runs with its own UID/GID if configured.
- No sensitive environment variables in log output.

## 4. Acceptance Criteria

- [ ] `aegis start <app>` launches a process and the daemon manages it.
- [ ] `aegis stop <app>` gracefully terminates the process.
- [ ] `aegis restart <app>` performs stop + start.
- [ ] `aegis reload <app>` performs zero-downtime reload.
- [ ] `aegis delete <app>` removes the process from management.
- [ ] `aegis start ecosystem.toml` launches all defined processes.
- [ ] Cluster mode with `instances = "max"` auto-detects CPU cores.
- [ ] Crashed processes are automatically restarted with exponential backoff.
- [ ] `max_memory_restart` triggers automatic restart when exceeded.
- [ ] Process table survives daemon restart.
- [ ] Prometheus metrics include process-level CPU and memory data.
- [ ] >90% test coverage for the process manager crate.

## 5. Out of Scope

- GUI/Web dashboard (will be a future track if needed).
- Container orchestration (Kubernetes handles this).
- Remote multi-host process management (future track).
