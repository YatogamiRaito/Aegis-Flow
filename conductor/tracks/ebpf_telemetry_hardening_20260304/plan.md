# Track Plan: eBPF Energy Telemetry Hardening

## Phase 1: eBPF Infrastructure setup
- [ ] Task: Add `aya` ecosystem dependencies to `telemetry` crate
- [ ] Task: Create `kernel/` workspace or sub-crate for BPF programs
- [ ] Task: Conductor Verification 'eBPF Infrastructure setup'

## Phase 2: Kernel Space Implementation
- [ ] Task: Write BPF maps (`RingBuf`, `HashMap`)
- [ ] Task: Implement `raw_tracepoint/sched_switch` for CPU cycle accounting
- [ ] Task: Implement `kprobe/tcp_sendmsg` and `tcp_recvmsg` for Network I/O
- [ ] Task: Conductor Verification 'Kernel Space Implementation'

## Phase 3: User Space State & RingBuf Consumer
- [ ] Task: Remove mock in-memory HashMap in `ebpf/metrics.rs`
- [ ] Task: Implement `aya::RingBuf` consumer polling loop
- [ ] Task: Aggregate incoming kernel events correctly into user-space model
- [ ] Task: Conductor Verification 'User Space State & RingBuf Consumer'

## Phase 4: Hardware Measurement (RAPL) Fallback
- [ ] Task: Implement `/sys/class/powercap/intel-rapl` reader
- [ ] Task: Integrate fallback into `EnergyEstimator` if eBPF fails
- [ ] Task: Conductor Verification 'Hardware Measurement (RAPL) Fallback'

## Phase 5: Proxy Integration & /energy Refactor
- [ ] Task: Initialize a global `SharedEnergyEstimator` in Proxy server boot
- [ ] Task: Pass global estimator to `Http3Handler`
- [ ] Task: Fix Prometheus exporter to connect to the global estimator
- [ ] Task: Conductor Verification 'Proxy Integration & /energy Refactor'

## Phase 6: Overhead Benchmarks
- [ ] Task: Add load testing criterion benchmarks with eBPF enabled vs disabled
- [ ] Task: Measure and record exact P99 / CPU overhead differences
- [ ] Task: Update telemetry documentation
- [ ] Task: Conductor Verification 'Overhead Benchmarks'

## Phase 6.5: Kusursuzluk Fazı
- [ ] Task: BTF vmlinux generation mechanism for kernels without built-in BTF
- [ ] Task: eBPF verifier limitations / memory leak handling in BPF kernel maps
- [ ] Task: Lock-free concurrency (Atomics) refinement for SharedEnergyEstimator
- [ ] Task: Full property-based testing setup for network bytes -> joules math conversions
- [ ] Task: Conductor Verification 'Kusursuzluk Fazı'

## Phase 7: Finalization
- [ ] Task: Release v0.35.0
- [ ] Task: Conductor Verification 'Finalization'
