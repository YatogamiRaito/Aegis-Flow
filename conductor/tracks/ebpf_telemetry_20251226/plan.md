# Track Plan: eBPF Energy Telemetry

## Phase 1: Energy Metrics Foundation
- [x] Task: Create aegis-telemetry crate
- [x] Task: Define EnergyMetrics struct (CPU, memory, network, storage)
- [x] Task: Implement software-based energy estimation (no eBPF fallback)
- [x] Task: Add Prometheus metrics exporter
- [x] Task: Conductor Verification 'Energy Metrics Foundation'

## Phase 2: eBPF Integration
- [ ] Task: Add aya/libbpf-rs dependency
- [ ] Task: Create eBPF program for CPU cycle tracking
- [ ] Task: Implement ring buffer consumer
- [ ] Task: CO-RE support for kernel compatibility
- [ ] Task: Conductor Verification 'eBPF Integration'

## Phase 3: Per-Request Tracking
- [ ] Task: Request ID propagation through eBPF
- [ ] Task: Per-endpoint energy aggregation
- [ ] Task: Energy cost calculation (joules per request)
- [ ] Task: Integration with Http3Handler
- [ ] Task: Conductor Verification 'Per-Request Tracking'

## Phase 4: Release v0.7.0
- [ ] Task: Documentation update
- [ ] Task: Live energy dashboard data endpoint
- [ ] Task: Release v0.7.0
- [ ] Task: Conductor Verification 'Release v0.7.0'
