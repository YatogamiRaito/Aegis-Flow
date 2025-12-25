# Aegis-Flow

<div align="center">

**Post-Quantum Secure, Carbon-Aware Service Mesh Data Plane**

[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/YatogamiRaito/Aegis-Flow/actions/workflows/ci.yml/badge.svg)](https://github.com/YatogamiRaito/Aegis-Flow/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/YatogamiRaito/Aegis-Flow/graph/badge.svg?token=YOUR_TOKEN)](https://codecov.io/gh/YatogamiRaito/Aegis-Flow)
[![Tests](https://img.shields.io/badge/tests-99%20passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.6.0-blue.svg)]()

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
│                        Aegis-Flow v0.4.0                      │
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

- Rust 1.92+ (Edition 2024)
- Cargo

### Build & Run

```bash
# Clone and build
cargo build --release

# Run the proxy
cargo run

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

## 📜 License

Apache-2.0 - See [LICENSE](LICENSE) for details.

---

<div align="center">
  <sub>Built with ❤️ using Rust</sub>
</div>
