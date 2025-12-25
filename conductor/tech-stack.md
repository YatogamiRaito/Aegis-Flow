
## 1. Core Language & Runtime
*   **Language:** **Rust** (Edition 2024 - requires Rust 1.85+ / using 1.92.0 stable)
    *   *Rationale:* Provides memory safety, zero-cost abstractions, and is the industry direction for secure infrastructure (Microsoft/Google mandate).
*   **Async Runtime:** **Tokio**
    *   *Rationale:* Industry standard for async I/O, offering stability and a rich ecosystem.
*   **I/O Acceleration:** **tokio-uring**
    *   *Rationale:* Leverages Linux `io_uring` for true async I/O and zero-copy networking, critical for high-performance proxying. (Confirmed kernel compatibility with target OS).

## 2. Networking & Proxy Protocol
*   **HTTP/1.1 & HTTP/2:** **Hyper**
    *   *Rationale:* Fast, correct, and low-level HTTP implementation.
*   **HTTP/3 & QUIC:** **s2n-quic** (Primary) or **Quinn**
    *   *Rationale:* s2n-quic is AWS-backed and actively maintained, essential for modern, low-latency transport in unreliable network conditions.
*   **gRPC:** **Tonic**
    *   *Rationale:* High-performance gRPC-over-HTTP/2 implementation for the control plane API.
*   **Service Middleware:** **Tower**
    *   *Rationale:* Modular abstraction for timeouts, retries, and load balancing logic.
*   **Memory Management:** **Bytes**
    *   *Rationale:* Zero-copy buffer management to minimize allocation overhead.
*   **TLS/mTLS:** **Rustls**
    *   *Rationale:* Memory-safe TLS implementation (avoiding OpenSSL vulnerabilities).

## 3. Cryptography (Post-Quantum & Hybrid)
*   **PQC Algorithms (KEM/DS):**
    *   **KyberLib (Cryspen):** FIPS 203 compliant and formally verified implementation.
    *   **pqcrypto** (RustCrypto) & **rust-pqc/kyber**: Additional NIST-standardized algorithm support.
*   **Hybrid Key Exchange:** **x25519-dalek**
    *   *Rationale:* Combined with Kyber to ensure security against both classical and quantum adversaries (Hybrid Mode).
*   **Validated Crypto:** **aws-lc-rs**
    *   *Rationale:* AWS's formally verified crypto library as a robust alternative backend.

## 4. Confidential Computing (TEE) & Attestation
*   **Execution Environment:** **Gramine**
    *   *Rationale:* Allows running unmodified Rust binaries inside SGX/TDX enclaves. Supports simulation mode for local development on non-TEE hardware (e.g., Intel i5-12500H).
*   **Attestation Verification:** **Veraison**
    *   *Rationale:* Standardized verification of remote attestation tokens.
*   **Platform Abstraction:** **Enarx**
    *   *Rationale:* Rust-native abstraction for deploying to multiple TEE architectures (SGX, SEV, TDX).
*   **Container Integration:** **Confidential Containers (CoCo) / Kata Containers**
    *   *Rationale:* Native integration with Kubernetes for orchestrating secure enclave pods.
*   *Note:* Testing will be performed on cloud instances (Azure DCsv3 / AWS Nitro) due to local hardware limitations.

## 5. Development, Verification & Supply Chain
*   **CI/CD:** **GitHub Actions**
    *   *Rationale:* Automated pipeline for testing, linting, and building.
*   **Formal Verification:** **Kani** & **Verus**
    *   *Rationale:* Mathematically prove the correctness of critical hot paths (PQC handshake, TEE attestation logic).
*   **Performance Benchmarking:** **Criterion.rs**
    *   *Rationale:* Statistical analysis to detect performance regressions.
*   **Security & Compliance:**
    *   **cargo-deny** & **cargo-audit:** Automated dependency scanning for vulnerabilities and license compliance.
    *   **Miri:** Detection of undefined behavior in `unsafe` code blocks (if any).
    *   **DCO:** Developer Certificate of Origin enforcement for all commits.
*   **Test Coverage:** **cargo-tarpaulin**
    *   *Rationale:* Ensure >90% code coverage as per project guidelines.
*   **Supply Chain Security (SLSA L3):** **Syft** (SBOM generation) & **Cosign** (Artifact signing)
    *   *Rationale:* Guarantees the integrity and provenance of build artifacts for enterprise adoption.
