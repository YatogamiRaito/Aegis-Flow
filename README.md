# Aegis-Flow

<div align="center">

**Post-Quantum Secure, Carbon-Aware Service Mesh Data Plane**

[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/YatogamiRaito/Aegis-Flow/actions/workflows/ci.yml/badge.svg)](https://github.com/YatogamiRaito/Aegis-Flow/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/YatogamiRaito/Aegis-Flow/graph/badge.svg?token=YOUR_TOKEN)](https://codecov.io/gh/YatogamiRaito/Aegis-Flow)
[![Tests](https://img.shields.io/badge/tests-1028%20passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.14.0-blue.svg)]()

</div>

## 🎯 Overview

Aegis-Flow is a high-performance service mesh data plane written in Rust, featuring:

- **🔐 Post-Quantum Cryptography**: Hybrid Kyber-768 + X25519 key exchange
- **🛡️ TEE Support**: Runs in SGX/TDX enclaves via Gramine
- **🌐 HTTP/2 Proxy**: High-performance reverse proxy with Hyper
- **🌱 Carbon-Aware Routing**: Route traffic based on grid carbon intensity
- **⏰ Green-Wait Scheduling**: Defer jobs to low-carbon time windows
- **📊 Energy Telemetry**: Per-request energy and carbon metrics
- **🔒 End-to-End Encryption**: AES-256-GCM with HKDF key derivation

## 📦 Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Aegis-Flow v0.14.0                     │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌──────────┐  ┌─────────┐ │
│  │ aegis-proxy │  │aegis-crypto │  │  common  │  │ energy  │ │
│  │  (HTTP/2)   │  │ (PQC + AES) │  │ (Types)  │  │(Carbon) │ │
│  └─────────────┘  └─────────────┘  └──────────┘  └─────────┘ │
├──────────────────────────────────────────────────────────────┤
│            Tokio Runtime + Hyper + Tower + Metrics            │
└──────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.83+ (Edition 2024)
- Cargo

### Build & Run

```bash
# Clone and build
cargo build --release

# Run the proxy
cargo run --release

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench
```

### Configuration

Copy the default configuration:

```bash
cp config/default.toml config/local.toml
```

## 🔌 Integration with Web Projects

Aegis-Flow can act as a **reverse proxy** in front of your existing web applications.

### Example: ElysiaJS/Bun Backend

```bash
# 1. Start your ElysiaJS backend (port 3000)
cd your-elysia-project && bun run dev

# 2. Start Aegis-Flow with custom upstream
cd aegis-flow
AEGIS_UPSTREAM="127.0.0.1:3000" cargo run --release

# 3. Access via proxy: http://localhost:8080
```

### Example: Node.js/Express Backend

```bash
# 1. Start Express server (port 4000)
cd express-app && npm start

# 2. Configure and run Aegis-Flow
# Edit config/local.toml:
#   [http2]
#   upstream_addr = "127.0.0.1:4000"
cargo run --release
```

### Preset Configurations

- `config/hali-saha.toml` - ElysiaJS/Bun project integration
- `config/default.toml` - Default settings (upstream: 127.0.0.1:9000)

## 📊 Performance

| Operation | Time |
|-----------|------|
| Full PQC Handshake | ~85µs |
| Key Derivation | ~7.6ns |
| AES-256-GCM Encrypt/Decrypt | <1µs |
| Carbon Router Decision | <5ms |

## 🔐 Security Features

### Post-Quantum Cryptography

- **Key Exchange**: X25519 + Kyber-768 hybrid
- **Symmetric Encryption**: AES-256-GCM / ChaCha20-Poly1305
- **Key Derivation**: HKDF-SHA256

### TEE Support

- Gramine SGX manifest included
- Docker container for deployment
- Remote attestation (DCAP) ready

## 🌱 Carbon-Aware Features

### Spatial Arbitrage
Route traffic to regions with lowest carbon intensity using real-time data from:
- WattTime API
- Electricity Maps API

### Temporal Shifting (Green-Wait)
Defer non-urgent jobs to time windows with cleaner energy:
- **Critical**: Execute immediately
- **High**: Wait up to 5 minutes
- **Normal**: Wait up to 30 minutes
- **Low**: Wait up to 2 hours
- **Background**: Wait indefinitely

### Energy Telemetry
Prometheus metrics for carbon monitoring:
- `aegis_carbon_intensity_g_kwh` - Current carbon intensity per region
- `aegis_estimated_energy_joules_total` - Energy consumed
- `aegis_estimated_carbon_grams_total` - Carbon emissions
- `aegis_deferred_jobs_current` - Jobs in Green-Wait queue

## 📁 Project Structure

```
aegis-flow/
├── crates/
│   ├── common/          # Shared types and errors
│   ├── crypto/          # PQC, cipher, TLS integration
│   ├── energy/          # Carbon API clients and cache
│   └── proxy/           # HTTP/2 proxy, carbon router, green-wait
├── config/              # Configuration files
├── docs/rfcs/           # Design documents
├── grafana/             # Dashboard templates
├── gramine/             # TEE deployment
└── .github/workflows/   # CI/CD pipelines
```

## 📈 Development Status

### ✅ Track 1: Core TEE-Native PQC Data Plane (v0.1.0)
- [x] Rust workspace setup
- [x] Hybrid PQC key exchange
- [x] Basic proxy with TLS integration
- [x] TEE (Gramine) deployment
- [x] CI/CD with SLSA L3

### ✅ Track 2: Secure Data Plane with Encryption (v0.2.0)
- [x] AES-256-GCM encryption layer
- [x] HTTP/2 reverse proxy
- [x] Encrypted streaming transport
- [x] Configuration system

### ✅ Track 3: Cloud Native Integration (v0.3.0)
- [x] Prometheus metrics
- [x] Service discovery
- [x] Health endpoints
- [x] Kubernetes deployment manifests

### ✅ Track 4: Carbon-Aware Traffic Routing (v0.4.0)
- [x] WattTime/Electricity Maps API integration
- [x] Carbon intensity caching
- [x] Spatial arbitrage routing
- [x] Green-Wait temporal shifting
- [x] Energy telemetry metrics
- [x] Grafana dashboard

### ✅ Track 5: HTTP/3 and QUIC Protocol Support (v0.5.0)
- [x] QUIC server with s2n-quic
- [x] HTTP/3 request/response handling
- [x] Dual-stack HTTP/2 + HTTP/3 server
- [x] Alt-Svc header for HTTP/3 discovery
- [x] PQC integration with QUIC TLS

### ✅ Track 6: Performance Benchmark Suite (v0.6.0)
- [x] PQC handshake benchmark (230µs)
- [x] HTTP/3 throughput benchmark
- [x] Carbon router benchmark
- [x] Load testing (43M elem/s @ 500 workers)

### ✅ Track 7: eBPF Energy Telemetry (v0.7.0)
- [x] Per-request energy measurement
- [x] EnergyEstimator with software-based estimation
- [x] EbpfLoader and EbpfMetrics
- [x] Prometheus export for energy metrics

### ✅ Track 8: Genomic Data Processing (v0.8.0)
- [x] Apache Arrow and Polars integration
- [x] VCF/BAM parser implementation
- [x] VariantAnalytics for genomic queries
- [x] Arrow Flight protocol support

### ✅ Track 9: WebAssembly Plugin System (v0.9.0)
- [x] Wasmtime 38 integration
- [x] Plugin Registry with hot reload
- [x] Fuel metering for resource limits
- [x] Host function bindings

### ✅ Track 10: PQC Migration & Security Hardening (v0.10.0)
- [x] Migrate from Kyber to ML-KEM (FIPS 203)
- [x] Wasmtime upgrade (27 → 38)
- [x] Security advisory resolution
- [x] cargo audit & cargo deny clean

### 🔜 Track 11: Advanced TEE Integration (Planned)
- [ ] Intel SGX/TDX Remote Attestation
- [ ] AMD SEV-SNP support
- [ ] DCAP quote generation/verification
- [ ] Enclave identity validation

### 🔜 Track 12: Observability Dashboard Expansion (Planned)
- [ ] Security metrics dashboard
- [ ] Performance dashboard
- [ ] Energy & carbon dashboard
- [ ] Alerting rules

### 🔜 Track 13: Production-Ready Deployment (Planned)
- [ ] HPA and PDB configuration
- [ ] Network Policies
- [ ] Secret management via External Secrets
- [ ] Multi-cloud testing (EKS, GKE, AKS)

### 🔜 Track 14: ML-DSA Digital Signatures (Planned)
- [ ] ML-DSA-44/65/87 implementation
- [ ] Hybrid signing (ML-DSA + Ed25519)
- [ ] Certificate signing with PQC
- [ ] WASM plugin signature verification

## 📜 License

Apache-2.0 - See [LICENSE](LICENSE) for details.

---

<div align="center">
  <sub>Built with ❤️ using Rust</sub>
</div>
