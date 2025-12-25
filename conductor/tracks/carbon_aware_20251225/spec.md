# Spec: Carbon-Aware Traffic Routing

## Overview
Implement energy-aware traffic routing for Aegis-Flow proxy, enabling carbon-conscious request handling through integration with grid carbon intensity APIs.

## Functional Requirements
1. **Energy API Integration**: WattTime and Electricity Maps API clients
2. **Carbon Intensity Caching**: TTL-based cache for API responses
3. **Spatial Arbitrage**: Route traffic to regions with cleaner energy
4. **Temporal Shifting (Green-Wait)**: Defer non-urgent jobs to cleaner time windows
5. **Energy Telemetry**: Per-request energy metrics

## Non-Functional Requirements
- API client must handle rate limiting gracefully
- Cache TTL: 5 minutes default
- Routing decision latency < 5ms

## Acceptance Criteria
- [ ] Energy crate compiles and tests pass
- [ ] Carbon router integrates with proxy discovery
- [ ] Green-wait queue supports deferred execution
- [ ] Prometheus metrics for carbon intensity exposed
