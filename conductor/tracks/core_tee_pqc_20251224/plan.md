# Track Plan: Core TEE-Native PQC Data Plane

## Phase 1: Project Scaffolding & CI/CD
- [x] Task: Initialize Rust Project Structure
    - Create a workspace with `proxy`, `common`, and `crypto` crates.
    - Configure `Cargo.toml` with project metadata and dependencies.
- [x] Task: Set up Quality Tools
    - Configure `clippy`, `rustfmt`, and `deny.toml`.
    - Add GitHub Actions workflow for CI (build, test, lint, audit).
- [ ] Task: Implement SLSA L3 Stub
    - Add `syft` and `cosign` steps to the CI pipeline (mock or dry-run for now).
- [ ] Task: Conductor - User Manual Verification 'Project Scaffolding & CI/CD' (Protocol in workflow.md)

## Phase 2: Post-Quantum Crypto Integration
- [ ] Task: Implement Hybrid Key Exchange Wrapper (TDD)
    - **RFC Required:** Draft RFC for "Hybrid Kyber+X25519 Integration Strategy".
    - Write tests for a generic `KeyExchange` trait.
    - Implement `Kyber1024` + `X25519` wrapper using `pqcrypto` and `x25519-dalek`.
    - *Formal Verification:* Use Kani to verify the state machine of the handshake wrapper.
- [ ] Task: Integrate PQC with Transport Layer (TDD)
    - Create a custom `rustls::CryptoProvider` or `s2n-quic` crypto configuration that uses the hybrid wrapper.
    - Implement an echo server using this secure transport.
- [ ] Task: Conductor - User Manual Verification 'Post-Quantum Crypto Integration' (Protocol in workflow.md)

## Phase 3: Basic Proxy Implementation
- [ ] Task: Implement Async Proxy Core (TDD)
    - Create a `tokio`-based TCP/UDP listener.
    - Implement basic traffic forwarding (Echo or transparent proxy).
    - Ensure `tokio-uring` is utilized (if supported by kernel) or fallback to epoll.
- [ ] Task: Add Observability Hooks
    - Integrate `tracing` and `metrics` crates.
    - Expose basic latency and throughput metrics.
- [ ] Task: Conductor - User Manual Verification 'Basic Proxy Implementation' (Protocol in workflow.md)

## Phase 4: TEE Simulation with Gramine
- [ ] Task: Containerize Proxy
    - Create a standard Dockerfile for the proxy binary.
- [ ] Task: Configure Gramine Manifest
    - Create `proxy.manifest.template` for Gramine.
    - Define trusted files and entry points.
- [ ] Task: Run in Simulation Mode
    - Update Makefile/Scripts to build and run the SGX enclave in simulation mode.
    - Verify the proxy starts and accepts traffic inside the enclave.
- [ ] Task: Conductor - User Manual Verification 'TEE Simulation with Gramine' (Protocol in workflow.md)

## Phase 5: Final Validation & Release
- [ ] Task: Performance Benchmark
    - Run `criterion` benchmarks on the PQC handshake.
    - Measure end-to-end latency and compare against the <2ms overhead target.
- [ ] Task: Security Audit
    - Run `cargo audit` and `cargo deny`.
    - Manually review any `unsafe` blocks (if any exist).
- [ ] Task: Release v0.1.0-mvp
    - Tag the release.
    - Generate final SBOM.
- [ ] Task: Conductor - User Manual Verification 'Final Validation & Release' (Protocol in workflow.md)
