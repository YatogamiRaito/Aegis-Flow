# Track Plan: ML-DSA (Dilithium) Full Migration for Digital Signatures

## Phase 1: ML-DSA Foundation
- [ ] Task: Add pqcrypto-mldsa signing functionality
- [ ] Task: Create SigningKeyPair trait in aegis-crypto
- [ ] Task: Implement ML-DSA-44 wrapper
- [ ] Task: Implement ML-DSA-65 wrapper
- [ ] Task: Implement ML-DSA-87 wrapper
- [ ] Task: Conductor Verification 'ML-DSA Foundation'

## Phase 2: Signature Operations
- [ ] Task: Key generation implementation
- [ ] Task: Sign operation with constant-time guarantees
- [ ] Task: Verify operation
- [ ] Task: Serialization/deserialization for keys and sigs
- [ ] Task: Conductor Verification 'Signature Operations'

## Phase 3: Hybrid Mode
- [ ] Task: Create HybridSigner (ML-DSA + Ed25519)
- [ ] Task: Signature format for hybrid mode
- [ ] Task: Verification accepts both pure and hybrid
- [ ] Task: Configuration for algorithm selection
- [ ] Task: Conductor Verification 'Hybrid Mode'

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
- [ ] Task: Release v0.14.0
- [ ] Task: Conductor Verification 'Release'
