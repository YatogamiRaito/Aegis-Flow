# Track Plan: ML-DSA (Dilithium) Full Migration for Digital Signatures

## Phase 1: ML-DSA Foundation
- [x] Task: Add pqcrypto-mldsa signing functionality
- [x] Task: Create SigningKeyPair trait in aegis-crypto
- [x] Task: Implement ML-DSA-44 wrapper
- [x] Task: Implement ML-DSA-65 wrapper
- [x] Task: Implement ML-DSA-87 wrapper
- [x] Task: Conductor Verification 'ML-DSA Foundation' (10 tests passed)

## Phase 2: Signature Operations
- [x] Task: Key generation implementation
- [x] Task: Sign operation with constant-time guarantees
- [x] Task: Verify operation
- [x] Task: Serialization/deserialization for keys and sigs
- [x] Task: Conductor Verification 'Signature Operations'

## Phase 3: Hybrid Mode
- [x] Task: Create HybridSigner (ML-DSA + Ed25519)
- [x] Task: Signature format for hybrid mode
- [x] Task: Verification accepts both pure and hybrid
- [x] Task: Configuration for algorithm selection
- [x] Task: Conductor Verification 'Hybrid Mode' (17 tests passed)

## Phase 4: Integration
- [ ] Task: Update CertManager for ML-DSA signing
- [ ] Task: Self-signed certificate with ML-DSA
- [ ] Task: Attestation quote signing integration
- [ ] Task: WASM plugin signature verification
- [ ] Task: Conductor Verification 'Integration'

## Phase 5: Testing & Release
- [ ] Task: Unit tests for all ML-DSA variants
- [ ] Task: Benchmark signing/verification performance
- [ ] Task: Update documentation
- [ ] Task: Security advisory cleanup confirmation
- [ ] Task: Release v0.11.0
- [ ] Task: Conductor Verification 'Release'
