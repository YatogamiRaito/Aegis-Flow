# Track Plan: PQC Migration & Security Hardening

## Phase 1: Wasmtime Update
- [x] Task: Update wasmtime from 27 to 38+
- [x] Task: Update aegis-plugins crate for new wasmtime API
- [x] Task: Verify plugin system tests pass

## Phase 2: PQC Algorithm Migration
- [x] Task: Add pqcrypto-mlkem dependency
- [x] Task: Migrate hybrid_kex.rs from Kyber to ML-KEM
- [x] Task: Update key encapsulation/decapsulation
- [ ] Task: Add pqcrypto-mldsa dependency (optional, signing)
- [x] Task: Verify crypto tests pass

## Phase 3: Security Audit Cleanup
- [x] Task: Remove deprecated pqcrypto-kyber dependency
- [x] Task: Update deny.toml - remove ignore entries
- [x] Task: Update .cargo/audit.toml - remove ignore entries
- [ ] Task: Verify `cargo audit` returns 0 vulnerabilities
- [ ] Task: Verify `cargo deny check` passes

## Phase 4: Verification
- [ ] Task: Run full test suite
- [ ] Task: Update benchmarks with new PQC algorithms
- [ ] Task: Update documentation
- [ ] Task: Conductor Verification 'PQC Migration Complete'
