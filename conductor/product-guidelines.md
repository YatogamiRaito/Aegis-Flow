
## 1. Tone & Voice: Sovereign Deep Tech
The documentation and messaging for Aegis-Flow must embody "Sovereign Deep Tech."
*   **Authoritative & Precise:** We speak with the mathematical rigor required for Post-Quantum Cryptography (PQC) and Trusted Execution Environments (TEEs).
*   **Inevitable:** The tone should convey that the shift to quantum-resistant, carbon-aware, and privacy-preserving infrastructure is not just an option, but an architectural inevitability.
*   **High-Velocity:** While rigorous, we also reflect the speed and modern ergonomics of the Rust ecosystem. We are building the "blueprint for the next generation of the internet."
*   **Avoid:** Marketing fluff, vague promises, or overly "playful" language.

## 2. Code Quality & Engineering Standards
Aegis-Flow requires an uncompromising standard of engineering excellence to meet its strategic goals.
*   **Zero Unsafe (with Audit):** The use of `unsafe` blocks is strictly forbidden unless accompanied by a formal audit trail and explicit approval from two senior maintainers.
*   **Formally Verified Hot Paths:** Critical security boundaries—specifically the PQC handshake state machine and TEE attestation logic—must be mathematically proven correct using verification tools like Kani or Verus, going beyond standard unit testing.
*   **Async-First:** All I/O operations must be non-blocking and leverage the `tokio` runtime to ensure maximum concurrency and throughput.
*   **Test-Driven:** No feature is merged without comprehensive unit and integration tests, aiming for >90% coverage.

## 3. Visual Identity
The visual language should reflect the convergence of high-tech security, industrial reliability, and sustainability.
*   **Core Aesthetic:** A blend of **Cyberpunk/Futuristic** (neon accents, geometric quantum shapes) and **Industrial/Minimalist** (clean lines, technical schematics).
*   **Sustainability:** Subtle integration of **Organic/Green** hues to visually represent the "Carbon-Aware" traffic routing capabilities.
*   **Demo Milestone:** Visuals should highlight the capability to span from cloud to edge (e.g., schematics showing connection between a Cloud Node and a RISC-V ESP32-C3).

## 4. Documentation Strategy: Trust-First Architecture
Our documentation prioritizes establishing trust and proving correctness over simple "getting started" ease.
*   **Threat Model First:** Documentation must begin with a detailed Threat Model (e.g., "Attack vectors in a post-quantum world") to set the context.
*   **Architectural Decision Records (ADRs):** We justify every major decision (Rust, TEEs, specific PQC algorithms) with formal ADRs.
*   **Proofs & Benchmarks:** Correctness proofs, security guarantees, and rigorous performance benchmarks take precedence over API references. Operational guides are secondary to establishing the system's foundational integrity.

## 5. Governance & Contribution
To maintain high strategic value and readiness for acquisition:
*   **RFC-Driven Governance:** All major architectural changes must pass through a formal Request for Comments (RFC) process, mirroring the Rust project's own governance.
*   **SLSA Level 3 Compliance:** The CI/CD pipeline must enforce Supply-chain Levels for Software Artifacts (SLSA) Level 3 standards. This includes provenance generation, signed binaries, and Software Bill of Materials (SBOMs) to ensure a tamper-proof supply chain.
*   **Legal & Stability:**
    *   **DCO:** All commits must be signed (Developer Certificate of Origin).
    *   **Strict SemVer:** Rigid adherence to Semantic Versioning.
    *   **Security:** A defined `SECURITY.md` with PGP keys for responsible disclosure.
