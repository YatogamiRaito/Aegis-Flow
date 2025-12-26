# Changelog

All notable changes to Aegis-Flow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] - 2025-12-26

### Added
- **PQC Migration & Security Hardening (Track 10)**
  - Migrate from pqcrypto-kyber to pqcrypto-mlkem (NIST FIPS 203)
  - Add ML-KEM variants to KeyExchangeType and PqcAlgorithm enums
  - New hybrid key exchange: X25519-MLKEM768-Hybrid
- **Test Coverage Expansion**
  - New integration tests for `proxy` config hot-reload and env overrides
  - Edge case tests for `genomics` analytics (empty data, null qualities)
- **CI/CD Improvements**
  - Docker build & push job to GitHub Container Registry
  - Buildx cache for faster builds
  - Fixed Dockerfile Rust version (1.83)
  - Synced Helm chart `appVersion` to 0.10.0

### Changed
- **wasmtime** upgraded from 27 to 38.0.4
  - Fixes RUSTSEC-2025-0046 (fd_renumber vulnerability)
  - Fixes RUSTSEC-2025-0118 (shared memory vulnerability)
  - Fixes RUSTSEC-2025-0057 (fxhash unmaintained)
  - Fixes RUSTSEC-2024-0436 (paste unmaintained)
- Default PQC algorithm changed to HybridMlKem768
- Release profile: added `opt-level = 3` for maximum performance
- `certmanager.rs`: Improved SAN parsing with graceful error handling

### Deprecated
- `KeyExchangeType::Kyber768` - Use `MlKem768` instead
- `KeyExchangeType::HybridX25519Kyber768` - Use `HybridX25519MlKem768` instead
- `PqcAlgorithm::Kyber768Only` - Use `MlKem768Only` instead
- `PqcAlgorithm::HybridKyber768` - Use `HybridMlKem768` instead

### Security
- Resolved 5 security advisories (RUSTSEC-2024-0381, RUSTSEC-2025-*)
- Only 1 remaining: RUSTSEC-2024-0380 (pqcrypto-dilithium, future migration planned)

## [0.9.0] - 2025-12-26

### Added
- **WebAssembly Plugin System (Track 9)**
  - New aegis-plugins crate with Wasmtime 27
  - WasmEngine: module caching, fuel metering
  - PluginRequest/PluginResponse for plugin communication
  - PluginRegistry: load/unload, hot reload support
  - 12 new tests in plugins crate

## [0.8.0] - 2025-12-26

### Added
- **Genomic Data Processing (Track 8)**
  - New aegis-genomics crate with Apache Arrow and Polars
  - GenomicSchema for VCF variants, BAM alignments, sequences
  - VariantBatchBuilder and AlignmentBatchBuilder for Arrow RecordBatch
  - VcfParser for VCF to Arrow conversion
  - BamHeader parser for SAM/BAM header parsing
  - VariantAnalytics: count_by_chromosome, filter_by_quality/region
  - 22 new tests in genomics crate

### Fixed
- deny.toml: Added polars-arrow-format license exception
- deny.toml: Skip duplicate bitflags/hashbrown/indexmap versions

## [0.7.0] - 2025-12-26

### Added
- **eBPF Energy Telemetry (Track 7)**
  - New aegis-telemetry crate for energy measurement
  - EnergyMetrics and EnergyBreakdown structs (CPU, memory, network, storage)
  - EnergyEstimator with software-based estimation
  - EbpfLoader and EbpfMetrics for per-request tracking
  - Prometheus export: aegis_request_energy_joules, carbon metrics
  - /energy endpoint in Http3Handler for real-time stats
  - 22 new tests in telemetry crate

### Changed
- Added aegis-telemetry as dependency to aegis-proxy
- Updated test count to 121 passing

## [0.6.0] - 2025-12-26

### Added
- **Performance Benchmark Suite**
  - PQC handshake benchmark (230µs - 21x faster than target)
  - HTTP/3 throughput benchmark
  - Carbon router benchmark
  - Load testing (43M elem/s @ 500 workers)
  - docs/benchmarks/RESULTS.md comparison report

### Changed
- Converted aegis-proxy to lib+bin structure for benchmark access
- Updated test count to 99 passing tests

## [0.5.0] - 2025-12-26

### Added
- **HTTP/3 and QUIC Protocol Support**
  - `QuicServer` with s2n-quic for QUIC connections
  - HTTP/3 request/response handling via `Http3Handler`
  - `DualStackServer` running HTTP/2 and HTTP/3 simultaneously
  - Alt-Svc header for HTTP/3 discovery
  - 0-RTT session resumption support
  - PQC (Kyber+X25519) integration with QUIC TLS

### Changed
- Updated test count to 99 passing tests
- Added `pqc_enabled` field to `QuicConfig`

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
