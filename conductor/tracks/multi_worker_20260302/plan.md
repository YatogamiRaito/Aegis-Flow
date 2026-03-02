# Implementation Plan: Multi-Worker Architecture (v0.28.0)

## Phase 1: Master Process & Worker Spawning

- [ ] Task: Create master process module (`crates/proxy/src/master.rs`)
    - [ ] Write tests for master process initialization (parse config, setup signals)
    - [ ] Implement master setup without HTTP handling
    - [ ] Write tests for worker spawning (fork N processes)
    - [ ] Implement worker spawn using std::process::Command (self-exec with --worker flag)
    - [ ] Write tests for CPU core detection (auto worker count)
    - [ ] Implement auto-detection using std::thread::available_parallelism
    - [ ] Write tests for PID file creation and cleanup
    - [ ] Implement PID file management

- [ ] Task: Implement daemonization
    - [ ] Write tests for fork-to-background (double fork technique)
    - [ ] Implement daemonize using nix::unistd::fork or daemonize crate
    - [ ] Write tests for stdout/stderr redirect to /dev/null after daemonization
    - [ ] Write tests for setsid (new session leader)

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Worker Process & Socket Sharing

- [ ] Task: Implement worker mode (`crates/proxy/src/worker.rs`)
    - [ ] Write tests for worker startup with --worker flag
    - [ ] Implement worker CLI flag detection and mode switch
    - [ ] Write tests for SO_REUSEPORT socket binding (multiple workers on same port)
    - [ ] Implement SO_REUSEPORT on TcpListener
    - [ ] Write tests for worker_connections limit (max concurrent connections)
    - [ ] Implement connection counter with limit (reject after limit with 503)

- [ ] Task: Implement CPU affinity
    - [ ] Write tests for CPU pinning on Linux (sched_setaffinity)
    - [ ] Implement CPU affinity using nix or libc
    - [ ] Write tests for auto affinity (worker 0 → core 0, worker 1 → core 1, etc.)
    - [ ] Write tests for macOS fallback (no CPU pinning, document limitation)

- [ ] Task: Implement file descriptor limits
    - [ ] Write tests for rlimit_nofile setting per worker
    - [ ] Implement setrlimit using nix::sys::resource

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Worker Monitoring & Crash Recovery

- [ ] Task: Implement worker monitoring in master
    - [ ] Write tests for worker PID tracking
    - [ ] Implement worker process table in master
    - [ ] Write tests for worker exit detection (waitpid)
    - [ ] Implement worker monitoring loop with tokio::signal or nix::sys::wait
    - [ ] Write tests for automatic respawn on crash (< 500ms)
    - [ ] Implement respawn with crash counter and backoff

- [ ] Task: Implement worker health tracking
    - [ ] Write tests for excessive crash detection (N crashes in M seconds → stop respawning)
    - [ ] Implement crash rate limiting
    - [ ] Write tests for worker status reporting to master (via pipe or shared memory)

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Graceful Reload (SIGHUP)

- [ ] Task: Implement signal handling in master
    - [ ] Write tests for SIGHUP → reload sequence
    - [ ] Implement signal handler registration for SIGHUP, SIGUSR1, SIGUSR2, SIGTERM, SIGQUIT
    - [ ] Write tests for config re-parse and validation on SIGHUP
    - [ ] Implement config reload with validation (reject bad config, keep old)

- [ ] Task: Implement graceful reload sequence
    - [ ] Write tests for: spawn new workers → new workers ready → signal old workers to quit
    - [ ] Implement new worker spawning with new config
    - [ ] Write tests for old worker graceful shutdown (drain connections, timeout)
    - [ ] Implement SIGQUIT to old workers with shutdown_timeout
    - [ ] Write tests for zero dropped connections during reload
    - [ ] Implement connection draining in worker shutdown handler

- [ ] Task: Implement SIGUSR1 (log reopen)
    - [ ] Write tests for SIGUSR1 relay to all workers
    - [ ] Implement log file reopen in workers on signal

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Hot Binary Upgrade

- [ ] Task: Implement SIGUSR2 hot upgrade
    - [ ] Write tests for SIGUSR2 → fork new master with new binary
    - [ ] Implement exec of new binary preserving listening socket fd
    - [ ] Write tests for fd inheritance (pass socket fd via environment variable)
    - [ ] Implement socket fd passing using AEGIS_LISTEN_FD env var
    - [ ] Write tests for old master → SIGQUIT → graceful shutdown
    - [ ] Implement old master shutdown coordination
    - [ ] Write tests for rollback (new master fails → old master resumes)
    - [ ] Implement rollback detection (new master exits quickly → old master re-accepts)

- [ ] Task: Integration testing
    - [ ] Write integration test for full master-worker lifecycle
    - [ ] Write integration test for reload under load (no dropped requests)
    - [ ] Write integration test for hot upgrade under load

- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
