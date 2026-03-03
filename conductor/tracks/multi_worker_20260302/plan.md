# Implementation Plan: Multi-Worker Architecture (v0.28.0)

## Phase 1: Master Process & Worker Spawning

- [x] Task: Create master process module (`crates/proxy/src/master.rs`)
    - [x] Write tests for master process initialization (parse config, setup signals)
    - [x] Implement master setup without HTTP handling
    - [x] Write tests for worker spawning (fork N processes)
    - [x] Implement worker spawn using std::process::Command (self-exec with --worker flag)
    - [x] Write tests for CPU core detection (auto worker count)
    - [x] Implement auto-detection using std::thread::available_parallelism
    - [x] Write tests for PID file creation and cleanup
    - [x] Implement PID file management

- [x] Task: Implement daemonization
    - [x] Write tests for fork-to-background (double fork technique)
    - [x] Implement daemonize using nix::unistd::fork or daemonize crate
    - [x] Write tests for stdout/stderr redirect to /dev/null after daemonization
    - [x] Write tests for setsid (new session leader)

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Worker Process & Socket Sharing

- [x] Task: Implement worker mode (`crates/proxy/src/worker.rs`)
    - [x] Write tests for worker startup with --worker flag
    - [x] Implement worker CLI flag detection and mode switch
    - [x] Write tests for SO_REUSEPORT socket binding (multiple workers on same port)
    - [x] Implement SO_REUSEPORT on TcpListener
    - [x] Write tests for worker_connections limit (max concurrent connections)
    - [x] Implement connection counter with limit (reject after limit with 503)

- [x] Task: Implement CPU affinity
    - [x] Write tests for CPU pinning on Linux (sched_setaffinity)
    - [x] Implement CPU affinity using nix or libc
    - [x] Write tests for auto affinity (worker 0 → core 0, worker 1 → core 1, etc.)
    - [x] Write tests for macOS fallback (no CPU pinning, document limitation)

- [x] Task: Implement file descriptor limits
    - [x] Write tests for rlimit_nofile setting per worker
    - [x] Implement setrlimit using nix::sys::resource

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Worker Monitoring & Crash Recovery

- [x] Task: Implement worker monitoring in master
    - [x] Write tests for worker PID tracking
    - [x] Implement worker process table in master
    - [x] Write tests for worker exit detection (waitpid)
    - [x] Implement worker monitoring loop with tokio::signal or nix::sys::wait
    - [x] Write tests for automatic respawn on crash (< 500ms)
    - [x] Implement respawn with crash counter and backoff

- [x] Task: Implement worker health tracking
    - [x] Write tests for excessive crash detection (N crashes in M seconds → stop respawning)
    - [x] Implement crash rate limiting
    - [x] Write tests for worker status reporting to master (via pipe or shared memory)

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Graceful Reload (SIGHUP)

- [x] Task: Implement signal handling in master
    - [x] Write tests for SIGHUP → reload sequence
    - [x] Implement signal handler registration for SIGHUP, SIGUSR1, SIGUSR2, SIGTERM, SIGQUIT
    - [x] Write tests for config re-parse and validation on SIGHUP
    - [x] Implement config reload with validation (reject bad config, keep old)

- [x] Task: Implement graceful reload sequence
    - [x] Write tests for: spawn new workers → new workers ready → signal old workers to quit
    - [x] Implement new worker spawning with new config
    - [x] Write tests for old worker graceful shutdown (drain connections, timeout)
    - [x] Implement SIGQUIT to old workers with shutdown_timeout
    - [x] Write tests for zero dropped connections during reload
    - [x] Implement connection draining in worker shutdown handler

- [x] Task: Implement SIGUSR1 (log reopen)
    - [x] Write tests for SIGUSR1 relay to all workers
    - [x] Implement log file reopen in workers on signal

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Hot Binary Upgrade

- [x] Task: Implement SIGUSR2 hot upgrade
    - [x] Write tests for SIGUSR2 → fork new master with new binary
    - [x] Implement exec of new binary preserving listening socket fd
    - [x] Write tests for fd inheritance (pass socket fd via environment variable)
    - [x] Implement socket fd passing using AEGIS_LISTEN_FD env var
    - [x] Write tests for old master → SIGQUIT → graceful shutdown
    - [x] Implement old master shutdown coordination
    - [x] Write tests for rollback (new master fails → old master resumes)
    - [x] Implement rollback detection (new master exits quickly → old master re-accepts)

- [x] Task: Integration testing
    - [x] Write integration test for full master-worker lifecycle
    - [x] Write integration test for reload under load (no dropped requests)
    - [x] Write integration test for hot upgrade under load

- [x] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
