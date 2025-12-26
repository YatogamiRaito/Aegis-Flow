# Track Specification: PQC Migration & Security Hardening

## Overview
Migrate to NIST-standardized ML-KEM/ML-DSA algorithms and update wasmtime to fix security vulnerabilities.

## Goals
1. **Zero security vulnerabilities** - Clean `cargo audit` and `cargo deny`
2. **NIST compliance** - Use ML-KEM-768 (formerly Kyber-768) and ML-DSA (formerly Dilithium)
3. **Maintainable dependencies** - No unmaintained crates

## Current State
- wasmtime: 27 → needs 38+ for security fixes
- pqcrypto-kyber: 0.8 → migrate to pqcrypto-mlkem 0.1.1
- pqcrypto-dilithium: 0.5 → migrate to pqcrypto-mldsa

## Security Issues to Fix
- RUSTSEC-2025-0046: wasmtime fd_renumber vulnerability
- RUSTSEC-2025-0118: wasmtime shared memory vulnerability
- RUSTSEC-2025-0057: fxhash unmaintained (wasmtime dependency)
- RUSTSEC-2024-0436: paste unmaintained (wasmtime dependency)
- RUSTSEC-2024-0380: pqcrypto-dilithium deprecated
- RUSTSEC-2024-0381: pqcrypto-kyber deprecated

## Success Criteria
- `cargo audit` returns 0 vulnerabilities
- `cargo deny check` passes
- All tests pass with new dependencies
- Hybrid key exchange works with ML-KEM
