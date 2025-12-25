# Project Workflow

## 1. Development Loop
The following loop is executed for every task in a track.

1.  **Understand:** Read the task description and relevant code.
2.  **Plan:** Break down the task into smaller steps if necessary.
3.  **RFC/Design (For Major Changes):**
    *   If the task involves a major architectural change or critical security boundary, draft an **RFC (Request for Comments)**.
    *   Wait for approval from maintainers before proceeding.
4.  **Implement (TDD):**
    *   **Write Tests First:** Create unit/integration tests that fail.
    *   **Implement Code:** Write the code to make the tests pass.
    *   **Formal Verification:** For PQC handshake and TEE attestation logic, use **Kani** or **Verus** to prove correctness.
5.  **Verify:**
    *   Run tests: `cargo test`
    *   Run linting: `cargo clippy --all-targets --all-features`
    *   Run formatting: `cargo fmt -- --check`
    *   Check coverage: `cargo tarpaulin` (Ensure >90% coverage)
6.  **Commit:**
    *   Stage changes: `git add .`
    *   Commit with DCO sign-off: `git commit -s -m "type(scope): description"`
    *   *Note:* Using `-s` is mandatory for DCO compliance.

## 2. CI/CD Pipeline Requirements
Every Pull Request (PR) must pass the following checks:
*   **Formatting:** `cargo fmt`
*   **Linting:** `cargo clippy` (Warnings as errors)
*   **Security Audit:** `cargo audit` & `cargo deny check`
*   **Test Coverage:** `cargo tarpaulin` (>90% required)
*   **SLSA Level 3:**
    *   Generate SBOM using `syft`.
    *   Sign build artifacts using `cosign`.

## 3. Phase Completion Protocol
At the end of each development phase:
1.  **Review:** Review all completed tasks and artifacts.
2.  **Documentation:** Ensure all ADRs and RFCs are up to date.
3.  **Release:** Tag the release using Semantic Versioning (e.g., `v0.1.0`).
