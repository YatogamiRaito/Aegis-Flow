# Specification: ML-DSA (Dilithium) Full Migration for Digital Signatures

## Overview
Complete the post-quantum cryptography migration by implementing ML-DSA (formerly Dilithium) for digital signature operations. ML-DSA is NIST FIPS 204 standardized and provides quantum-resistant signatures for certificate signing, message authentication, and attestation.

## Background
- ML-KEM (formerly Kyber) for key encapsulation: ✅ Completed in v0.10.0
- ML-DSA (formerly Dilithium) for signatures: This track
- Together they form a complete PQC suite (FIPS 203 + FIPS 204)

## Functional Requirements

### Signature Operations
- ML-DSA-44 (security level 2) implementation
- ML-DSA-65 (security level 3) implementation
- ML-DSA-87 (security level 5) implementation
- Key generation, sign, and verify operations
- Hybrid mode: ML-DSA + Ed25519 for backwards compatibility

### Integration Points
- Self-signed certificate generation with ML-DSA
- Attestation quote signing
- Message authentication codes (MAC alternative)
- WASM plugin signature verification

### API Design
- SigningKeyPair trait with ML-DSA impl
- Signature verification in TLS handshake
- Certificate chain validation with PQC

## Non-Functional Requirements
- Signature generation < 1ms
- Signature verification < 0.5ms
- Key size documentation (larger than classical)
- Constant-time implementation (side-channel resistant)

## Acceptance Criteria
- [x] ML-DSA-65 integrated in aegis-crypto
- [x] Self-signed certs use ML-DSA by default
- [x] Hybrid mode works with Ed25519 fallback
- [x] All crypto tests pass
- [x] Benchmarks updated with signing performance

## Out of Scope
- X.509 certificate parsing with ML-DSA OIDs (pending tooling)
- HSM integration for key storage
- Certificate Authority functionality
