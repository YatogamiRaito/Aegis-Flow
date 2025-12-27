# Specification: Advanced TEE Integration with Remote Attestation

## Overview
Implement Remote Attestation protocol for Intel SGX/TDX and AMD SEV-SNP Trusted Execution Environments. This enables clients to cryptographically verify that Aegis-Flow is running inside a genuine, unmodified TEE enclave before sending sensitive data.

## Functional Requirements

### Remote Attestation Protocol
- Implement DCAP (Data Center Attestation Primitives) for Intel SGX
- Support Intel TDX quote generation and verification
- AMD SEV-SNP attestation report support
- Quote verification against collateral (TCB info, QE identity)

### Attestation Service
- `/attestation/quote` endpoint to generate attestation quote
- `/attestation/verify` endpoint for quote verification
- Challenge-response nonce support for freshness
- Integration with Intel Attestation Service (IAS) or DCAP

### Enclave Identity
- MRENCLAVE and MRSIGNER measurement extraction
- TCB level verification
- Product ID and security version validation
- Enclave mode detection (debug/production)

## Non-Functional Requirements
- Quote generation < 50ms latency
- Support for both ECDSA and EPID attestation
- Graceful fallback when running outside TEE
- Comprehensive logging for audit trails

## Acceptance Criteria
- [ ] Client can request and verify TEE attestation before establishing connection
- [ ] Quote includes application-specific data (nonce, session info)
- [ ] Verification validates enclave is in production mode
- [ ] Integration tests with mock attestation service

## Out of Scope
- Hardware procurement (assumes TEE-capable hardware)
- Key provisioning from external KMS
- Multi-enclave communication
