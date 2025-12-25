# Track Plan: Carbon-Aware Traffic Routing

## Phase 1: Energy API Integration
- [x] Task: Create energy crate structure
- [x] Task: Implement WattTime API client
- [x] Task: Implement Electricity Maps API client
- [x] Task: Implement carbon intensity caching
- [x] Task: Unit tests for API clients and cache
- [x] Task: Conductor Verification 'Energy API Integration'

## Phase 2: Spatial Arbitrage
- [ ] Task: Create carbon_router.rs in proxy crate
- [ ] Task: Implement region-based routing logic
- [ ] Task: Integration with discovery module
- [ ] Task: Unit tests for carbon routing
- [ ] Task: Conductor Verification 'Spatial Arbitrage'

## Phase 3: Temporal Shifting (Green-Wait)
- [ ] Task: Create green_wait.rs in proxy crate
- [ ] Task: Implement deferred job queue
- [ ] Task: Energy forecast integration
- [ ] Task: Integration tests
- [ ] Task: Conductor Verification 'Temporal Shifting'

## Phase 4: Energy Telemetry
- [ ] Task: Add carbon intensity metrics
- [ ] Task: Per-request energy tracking
- [ ] Task: Grafana dashboard template
- [ ] Task: Conductor Verification 'Energy Telemetry'

## Phase 5: Release v0.4.0
- [ ] Task: Documentation update
- [ ] Task: Performance benchmarks
- [ ] Task: Release v0.4.0
- [ ] Task: Conductor Verification 'Release v0.4.0'
