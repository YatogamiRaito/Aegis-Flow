# Track Specification: Multi-Worker Architecture (v0.28.0)

## 1. Overview

This track implements nginx's **master-worker process model** in Aegis-Flow. A master process manages N worker processes (one per CPU core), each handling requests independently. This maximizes CPU utilization, enables zero-downtime binary upgrades, and provides complete process isolation for fault tolerance.

## 2. Functional Requirements

### 2.1 Master Process
- The master process does NOT handle HTTP traffic directly.
- Responsibilities:
  - **Spawn workers:** Fork N worker processes (configurable, default: auto = CPU cores).
  - **Monitor workers:** Detect worker crashes and respawn immediately.
  - **Signal relay:** Forward SIGHUP (reload), SIGUSR1 (reopen logs), SIGTERM (shutdown) to workers.
  - **Binary upgrade:** Support hot binary upgrade without dropping connections.
  - **Config parse:** Parse and validate config, pass to workers via shared memory or file.

### 2.2 Worker Processes
- Each worker is an independent process with its own tokio runtime.
- Workers share the same listening socket(s) via `SO_REUSEPORT` (kernel load balancing).
- Each worker maintains its own:
  - Connection pool to upstreams.
  - In-memory cache (not shared across workers — per-worker LRU).
  - Rate limiter state (per-worker counters, slightly less precise but no cross-process locking).
- Worker count configurable: `worker_processes = "auto"` or explicit number.

### 2.3 Socket Sharing
- `SO_REUSEPORT`: each worker binds to the same port, kernel distributes connections.
- Alternative: master listens, passes fd to workers via Unix socket (like nginx).
- Worker affinity: optionally pin workers to specific CPU cores (`worker_cpu_affinity`).

### 2.4 Graceful Reload (SIGHUP)
1. Master receives SIGHUP.
2. Master re-parses config file (validate first).
3. Master spawns N new workers with new config.
4. New workers start accepting connections.
5. Master signals old workers to shut down gracefully.
6. Old workers stop accepting new connections but finish existing ones.
7. Old workers exit after all connections are drained (or timeout).

### 2.5 Hot Binary Upgrade (Zero-Downtime Upgrade)
1. New binary is placed on disk.
2. Send SIGUSR2 to master → master forks new master with new binary.
3. New master starts new workers.
4. Old master receives SIGQUIT → gracefully stops old workers.
5. Result: zero-downtime binary upgrade without dropping a single connection.

### 2.6 Worker Connection Limits
- `worker_connections`: max connections per worker (default: 1024).
- `worker_rlimit_nofile`: set file descriptor limit per worker.
- Total max connections = `worker_processes × worker_connections`.

### 2.7 Shared Memory Zones (Optional)
- For rate limiting and session persistence that needs cross-worker consistency:
  - Shared memory segments (shmem) for counters.
  - Or Unix socket IPC for coordination.
- Trade-off: exact rate limiting vs performance.

### 2.8 Configuration
```toml
[worker]
processes = "auto"           # auto = CPU cores, or explicit number
connections = 1024           # max connections per worker
cpu_affinity = "auto"        # auto, or explicit mask (e.g., "0-3")
rlimit_nofile = 65535
shutdown_timeout = "30s"     # max time to drain connections during reload

[master]
pid_file = "/run/aegis-flow.pid"
daemon = true               # fork to background
```

## 3. Non-Functional Requirements

- Worker startup time: < 100ms.
- Master overhead: < 5 MB RSS.
- Worker crash → respawn: < 500ms.
- Zero connections dropped during graceful reload.
- Hot binary upgrade: zero connections dropped.

## 4. Acceptance Criteria

- [ ] Master spawns configured number of worker processes.
- [ ] Workers independently accept and handle HTTP requests via SO_REUSEPORT.
- [ ] Worker crash is detected and worker is respawned within 500ms.
- [ ] SIGHUP triggers graceful reload (new workers up → old workers drain → exit).
- [ ] Zero connections dropped during reload.
- [ ] SIGUSR2 triggers hot binary upgrade.
- [ ] worker_connections limit enforced per worker.
- [ ] Worker CPU affinity pinning works (Linux).
- [ ] SIGUSR1 triggers log file reopen across all workers.
- [ ] Daemonization works (fork to background, write PID file).
- [ ] >90% test coverage.

## 5. Out of Scope

- Thread-per-connection model (we use async).
- Shared memory cache between workers.
