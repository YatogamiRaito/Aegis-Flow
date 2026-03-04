# Implementation Plan: Multi-Worker Architecture Hardening (v0.53.0)

## Phase 1: SO_REUSEPORT Integration
- [ ] Task: Modify `bootstrap.rs` Socket Binding
    - [ ] Import `socket2::{Socket, Domain, Type, Protocol}`.
    - [ ] Create a utility function `bind_reuseport(addr: SocketAddr)`.
    - [ ] Apply `set_reuse_port(true)` and `set_reuse_address(true)` before binding.
    - [ ] Update HTTP/HTTPS `TcpListener`s and Quic `UdpSocket`s to use this utility.

## Phase 2: CLI and Master Process Orchestration
- [ ] Task: Rewrite Entrypoint
    - [ ] Parse CLI flags in `main.rs` to detect `--worker`.
    - [ ] If `--worker`, invoke `bootstrap::bootstrap()`.
    - [ ] If Master, initialize `MasterProcess`, determine core count, and spawn children using `tokio::process::Command::new(std::env::current_exe())`.

## Phase 3: Signal Handling (Graceful Reload)
- [ ] Task: Implement Master Signal Loop
    - [ ] Use `tokio::signal::unix::signal` for `SIGHUP` and `SIGCHLD`.
    - [ ] On `SIGCHLD`, check which worker exited. If unexpected, spawn replacement.
    - [ ] On `SIGHUP`, validate config, spawn new fleet, record old PIDs, send `SIGQUIT` to old PIDs via `libc::kill`.

## Phase 4: Worker Graceful Shutdown
- [ ] Task: Implement Worker Signal Handling
    - [ ] Introduce a `CancellationToken` in `bootstrap.rs` passed to the server loops.
    - [ ] In the worker, listen for `SIGQUIT`. When received, fire the cancellation token.
    - [ ] Update `http_proxy::run_server` / hyper connections to stop listening but utilize `hyper_util::server::graceful::GracefulShutdown` to finish in-flight requests.
