# Changelog

All notable changes to Aegis-Flow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-12-26

### Added
- **Carbon-Aware Traffic Routing**
  - `aegis-energy` crate with WattTime and Electricity Maps API clients
  - Carbon intensity caching with TTL-based invalidation
  - `CarbonRouter` for spatial arbitrage routing
  - `GreenWaitScheduler` for temporal shifting (defer jobs to green windows)
  - Energy telemetry metrics (carbon intensity, estimated energy, deferred jobs)
  - Grafana dashboard template for carbon monitoring

### Changed
- Updated architecture to include energy crate
- Added carbon router decision latency (<5ms) to performance benchmarks

## [0.3.0] - 2025-12-25

### Added
- **Cloud Native Integration**
  - Prometheus metrics with `metrics-exporter-prometheus`
  - Service discovery module with round-robin load balancing
  - Health check endpoints (`/healthz`, `/ready`)
  - Kubernetes deployment manifests
  - OpenTelemetry tracing support

## [0.2.0] - 2025-12-25

### Added
- **Secure Data Plane**
  - AES-256-GCM and ChaCha20-Poly1305 encryption
  - `EncryptedStream` for transparent stream encryption
  - HTTP/2 reverse proxy with Hyper
  - TOML-based configuration system

### Changed
- Integrated encrypted transport with PQC handshake

## [0.1.0] - 2025-12-24

### Added
- **Core TEE-Native PQC Data Plane**
  - Rust workspace with modular crate structure
  - Hybrid PQC key exchange (X25519 + Kyber-768)
  - HKDF-SHA256 key derivation
  - Basic TLS integration with Rustls
  - Gramine SGX manifest for TEE deployment
  - GitHub Actions CI/CD pipeline
  - Criterion benchmarks for cryptographic operations
