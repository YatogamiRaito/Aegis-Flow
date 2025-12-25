# Aegis-Flow

<div align="center">

**Post-Quantum Secure, Carbon-Aware Service Mesh Data Plane**

[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-28%20passing-brightgreen.svg)]()

</div>

## 🎯 Overview

Aegis-Flow is a high-performance service mesh data plane written in Rust, featuring:

- **🔐 Post-Quantum Cryptography**: Hybrid Kyber-768 + X25519 key exchange
- **🛡️ TEE Support**: Runs in SGX/TDX enclaves via Gramine
- **🌐 HTTP/2 Proxy**: High-performance reverse proxy with Hyper
- **📊 Observability**: Built-in metrics, tracing, and health endpoints
- **🔒 End-to-End Encryption**: AES-256-GCM with HKDF key derivation

## 📦 Architecture

```
┌─────────────────────────────────────────────────┐
│                  Aegis-Flow                      │
├─────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │
│  │ aegis-proxy │  │aegis-crypto │  │  common  │ │
│  │  (HTTP/2)   │  │ (PQC + AES) │  │ (Types)  │ │
│  └─────────────┘  └─────────────┘  └──────────┘ │
├─────────────────────────────────────────────────┤
│        Tokio Runtime + Hyper + Tower            │
└─────────────────────────────────────────────────┘
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

## 🔐 Security Features

### Post-Quantum Cryptography

- **Key Exchange**: X25519 + Kyber-768 hybrid
- **Symmetric Encryption**: AES-256-GCM / ChaCha20-Poly1305
- **Key Derivation**: HKDF-SHA256

### TEE Support

- Gramine SGX manifest included
- Docker container for deployment
- Remote attestation (DCAP) ready

## 📁 Project Structure

```
aegis-flow/
├── crates/
│   ├── common/          # Shared types and errors
│   ├── crypto/          # PQC, cipher, TLS integration
│   └── proxy/           # HTTP/2 proxy, PQC server
├── config/              # Configuration files
├── docs/rfcs/           # Design documents
├── gramine/             # TEE deployment
└── .github/workflows/   # CI/CD pipelines
```

## 📈 Development Status

### ✅ Track 1: Core TEE-Native PQC Data Plane (v0.1.0-mvp)
- [x] Rust workspace setup
- [x] Hybrid PQC key exchange
- [x] Basic proxy with TLS integration
- [x] TEE (Gramine) deployment
- [x] CI/CD with SLSA L3

### 🔄 Track 2: Secure Data Plane with Encryption (v0.2.0)
- [x] AES-256-GCM encryption layer
- [x] HTTP/2 reverse proxy
- [x] mTLS with PQC
- [x] Configuration system

### 🔄 Track 3: Cloud Native Integration (v0.3.0)
- [x] Prometheus Metrics & Grafana Dashboard
- [x] Kubernetes Helm Chart
- [x] Service Discovery (DNS/Static) & Load Balancing
- [x] Distributed Tracing (OpenTelemetry)

## 📊 Observability

Aegis-Flow provides a full observability stack:

- **Metrics**: Prometheus endpoint at `:9090/metrics`
- **Tracing**: OpenTelemetry (W3C Trace Context)
- **Logging**: Structured JSON logging via `tracing`
- **Dashboards**: Grafana dashboard included in `deploy/grafana`

## 📜 License

Apache-2.0 - See [LICENSE](LICENSE) for details.

---

<div align="center">
  <sub>Built with ❤️ using Rust</sub>
</div>
