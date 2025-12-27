# Track Plan: PQC Migration & Security Hardening

## Phase 1: Wasmtime Update
- [x] Task: Update wasmtime from 27 to 38+
- [x] Task: Update aegis-plugins crate for new wasmtime API
- [x] Task: Verify plugin system tests pass

## Phase 2: PQC Algorithm Migration
- [x] Task: Add pqcrypto-mlkem dependency
- [x] Task: Migrate hybrid_kex.rs from Kyber to ML-KEM
- [x] Task: Update key encapsulation/decapsulation
- [x] Task: Add pqcrypto-mldsa dependency (signing deferred to Track: ML-DSA Full Migration)
- [x] Task: Verify crypto tests pass

## Phase 3: Security Audit Cleanup
- [x] Task: Remove deprecated pqcrypto-kyber dependency
- [x] Task: Update deny.toml - remove ignore entries
- [x] Task: Update .cargo/audit.toml - remove ignore entries
- [x] Task: Verify `cargo audit` returns 0 vulnerabilities (verified 2025-12-28)
- [x] Task: Verify `cargo deny check` passes (verified 2025-12-28)

## Phase 4: Verification
- [x] Task: Run full test suite (186 tests passed 2025-12-28)
- [x] Task: Update benchmarks with new PQC algorithms
- [x] Task: Update documentation
- [x] Task: Conductor Verification 'PQC Migration Complete'

## Notes
- pqcrypto-dilithium removed from dependencies - RUSTSEC-2024-0380 resolved
- ML-DSA signing operations deferred to dedicated track: mldsa_signing_20251228
