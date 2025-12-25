# Rust Code Style Guide

## 1. General Formatting
*   **Rustfmt:** All code must be formatted using `cargo fmt`. This is enforced by CI.
*   **Line Length:** Maximum line length is 100 characters (default `rustfmt` setting).
*   **Indentation:** Use 4 spaces for indentation. No tabs.

## 2. Naming Conventions
*   **Types (Structs, Enums, Traits):** `UpperCamelCase`
*   **Functions, Methods, Variables, Modules:** `snake_case`
*   **Constants, Statics:** `SCREAMING_SNAKE_CASE`
*   **Lifetimes:** `'snake_case` (e.g., `'a`, `'ctx`)

## 3. Code Quality & Safety (Project Mandates)
*   **Zero Unsafe Policy:**
    *   The use of `unsafe` blocks is **strictly forbidden** by default.
    *   Exceptions are only allowed if accompanied by a formal audit trail and explicit approval from two senior maintainers.
    *   Any `unsafe` block must be documented with a `// SAFETY:` comment explaining why it is safe.
*   **Formal Verification:**
    *   Critical hot paths (specifically PQC handshake and TEE attestation logic) must be formally verified.
    *   Tools like **Kani** or **Verus** must be used to mathematically prove correctness.
*   **Linting:**
    *   All code must pass `cargo clippy --all-targets --all-features` without warnings.
    *   Treat warnings as errors in CI.

## 4. Testing & Coverage
*   **Coverage Requirement:**
    *   The project enforces a **>90% code coverage** threshold.
    *   Use `cargo-tarpaulin` to measure coverage.
*   **Test Structure:**
    *   Unit tests go in the same file as the code, in a `mod tests` module.
    *   Integration tests go in the `tests/` directory.

## 5. Documentation
*   **Public API:** All public items (functions, structs, modules) must have documentation comments (`///`).
*   **Examples:** Include usage examples in documentation where possible.
*   **README:** Each crate/module should have a `README.md` explaining its purpose.

## 6. Version Control & Contribution
*   **DCO (Developer Certificate of Origin):**
    *   All commits must be signed (`git commit -s`).
    *   Unsigned commits will be rejected by the CI pipeline.
*   **Commit Messages:** Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.
    *   Format: `<type>(<scope>): <description>`
    *   Example: `feat(proxy): implement carbon-aware routing logic`
