# Track Specification: eBPF Energy Telemetry Hardening

## Overview
Transform the current mock-based eBPF telemetry implementation into a production-ready, kernel-level energy measurement system capable of operating within the expected <1% CPU overhead constraint, and ensuring proper Prometheus metrics and API integration.

## Functional Requirements

### FR-1: Actual eBPF Program Implementation
- Add `aya`, `aya-bpf`, and `aya-log` dependencies.
- Implement real C or Rust eBPF programs for `kprobe` / `raw_tracepoint`.
- Hook into kernel context switch and network I/O functions.

### FR-2: Zero-Copy Ring Buffer
- Replace the mock `RwLock<HashMap>` implementation.
- Use `aya::RingBuf` for efficient, lock-free transmission of metrics from kernel space to user space.

### FR-3: Shared Telemetry State & Global Exporter
- Refactor `/energy` endpoint so it does not recreate `EnergyEstimator` per-request.
- Bind the real-time aggregated metrics to `EnergyPrometheusExporter` in the proxy crate flow.
- Record `aegis_request_energy_joules` (Histogram) and related counters instead of transient gauges.

### FR-4: Hardware Backed Fallback (RAPL)
- Provide a fallback that reads from `/sys/class/powercap/intel-rapl` when available.
- Integrate RAPL readings into the estimation model for more accurate base coefficients.

### FR-5: Advanced CO-RE Support
- Use `libbpf-rs` or `aya`'s built-in BTF parsing to ensure Compile Once - Run Everywhere portability.
- Remove simple version-string parsing in favor of actual BPF map / BTF existence checks.

## Non-Functional Requirements

### NFR-1: Performance & Overhead
- The eBPF overhead strictly must not exceed 1% of total request latency.
- Utilize batching and ring-buffers for transferring stats to user space.

### NFR-2: CI/CD Testing
- Add conditional CI steps (where kernel permits) or mock tests to enforce API correctness.
- Establish baseline benchmarks to prove < 1% overhead claim.

## Acceptance Criteria
1. `crates/telemetry/Cargo.toml` contains actual eBPF crates.
2. The proxy utilizes a global `SharedEnergyEstimator` or `EbpfMetrics` instance.
3. `/energy` endpoint returns continuously accumulating valid production data.
4. eBPF hooks can successfully compile and run on Kernel 5.8+ with BTF.
5. CPU overhead is empirically benchmarked and confirmed < 1%.
