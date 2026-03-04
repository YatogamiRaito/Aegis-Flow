# Track Specification: Multi-Worker Architecture Hardening (v0.53.0)

## 1. Overview
The current implementation runs entirely as a single process originating from `main.rs` directly into `bootstrap.rs`. The `master.rs` structs exist but are not used. Aegis-Flow fundamentally lacks the Nginx-style multi-process architecture required for zero-downtime reloads and multi-core scaling via `SO_REUSEPORT`.

This track aims to rewrite the daemon entrypoint, implement actual inter-process spawning, configure socket borrowing, and handle orchestration signals (`SIGHUP`, `SIGUSR2`).

## 2. Functional Requirements

### 2.1 Daemon / Master Entrypoint
- Rewrite `main.rs` to parse basic CLI arguments (e.g. `--worker=true`, `-c config.toml`).
- If not a worker, the process becomes the Master.
- The Master must instantiate `MasterProcess` and enter an event loop waiting for OS signals (using `tokio::signal::unix`).
- The Master uses `std::process::Command` to execute `current_exe()` passing the `--worker=true` flag.

### 2.2 Worker Socket Sharing (`SO_REUSEPORT`)
- In `bootstrap.rs`, when creating the `TcpListener`, instead of `tokio::net::TcpListener::bind`, it must use the `socket2` crate to create a raw socket.
- Set `SO_REUSEPORT` on the raw socket (this allows multiple independent processes to bind to the exact same port).
- Convert the raw socket into a `std::net::TcpListener`, set it to non-blocking, and construct the `tokio::net::TcpListener` from it.
- Note: This must be done for both HTTP and HTTPS listeners, and the Quic/HTTP3 UDP sockets.

### 2.3 Signal Orchestration (SIGHUP Graceful Reload)
- When the Master receives `SIGHUP`:
    1. Parse the config file from disk to ensure it's valid.
    2. Spawn N *new* worker processes.
    3. Wait a brief moment to ensure new workers are bound and accepting traffic.
    4. Send `SIGQUIT` (or `SIGTERM`) to the *old* worker processes.
- When a Worker receives `SIGQUIT`:
    1. Stop accepting *new* connections on the `TcpListener`.
    2. Wait for existing HTTP connections (tasks) to finish (up to a timeout, e.g. 30s).
    3. Exit gracefully.

### 2.4 Hot Binary Upgrade (SIGUSR2)
- When the Master receives `SIGUSR2`:
    - Fork a new Master process using the binary path (execve).
    - Pass the main config path and a special environment flag.
    - The old Master stops spawning new workers but leaves existing ones running.
    - If the new Master succeeds, the old Master shuts down its workers.

## 3. Non-Functional Requirements
- **Crash Recovery:** The Master must use `tokio::process::Command` or `waitpid` to detect if a child worker crashes prematurely and immediately respawn it (using `CrashTracker`).
- **Portability:** `SO_REUSEPORT` is Linux/BSD specific. On Windows/macOS built without it, the master must gracefully fallback or warn (though macOS does support `SO_REUSEPORT`).

## 4. Acceptance Criteria
- [ ] Running `aegis-proxy` spawns 1 master process and N worker processes (visible in `htop`).
- [ ] `curl localhost:8080` succeeds while handled by one of the workers.
- [ ] Sending `kill -HUP <master_pid>` cleanly starts new workers and kills old ones without terminating an active long-running curl request.
- [ ] If a worker process is `kill -9`'d, the master immediately spawns a replacement.
