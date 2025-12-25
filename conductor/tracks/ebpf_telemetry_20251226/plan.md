# Track Plan: eBPF Energy Telemetry

## Phase 1: Energy Metrics Foundation
- [x] Task: Create aegis-telemetry crate
- [x] Task: Define EnergyMetrics struct (CPU, memory, network, storage)
- [x] Task: Implement software-based energy estimation (no eBPF fallback)
- [x] Task: Add Prometheus metrics exporter
- [x] Task: Conductor Verification 'Energy Metrics Foundation'

## Phase 2: eBPF Integration
- [x] Task: Add aya/libbpf-rs dependency (feature-gated)
- [x] Task: Create eBPF program for CPU cycle tracking (mock impl)
- [x] Task: Implement ring buffer consumer (EbpfMetrics)
- [x] Task: CO-RE support for kernel compatibility (version check)
- [x] Task: Conductor Verification 'eBPF Integration'

## Phase 3: Per-Request Tracking
- [x] Task: Request ID propagation through eBPF
- [x] Task: Per-endpoint energy aggregation
- [x] Task: Energy cost calculation (joules per request)
- [x] Task: Integration with Http3Handler (/energy endpoint)
- [x] Task: Conductor Verification 'Per-Request Tracking'

## Phase 4: Release v0.7.0
- [x] Task: Documentation update
- [x] Task: Live energy dashboard data endpoint
- [x] Task: Release v0.7.0
- [x] Task: Conductor Verification 'Release v0.7.0'
