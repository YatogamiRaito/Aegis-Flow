# Specification: TEE Attestation Security Hardening (v0.38.0)

## Overview
Replaces the stubbed (mock) TEE attestation implementations with real bindings for Intel SGX/TDX and AMD SEV-SNP. Integrates with the actual Proxy layer by adding the `/attestation/quote` and `/attestation/verify` HTTP endpoints, allowing clients to establish trust before interacting with Aegis-Flow.

## Functional Requirements

### 1. Real TEE Bindings
- Integrate `intel-tee-quote-provider` (or similar native bindings) for Intel SGX DCAP / TDX quote generation.
- Integrate `sev` crate for AMD SEV-SNP report generation.
- Implement conditional compilation (`#[cfg(target_os = "linux")]` and hardware feature flags) so the project can still build on non-TEE systems (falling back to simulation mode ONLY when explicitly requested or no hardware is available).

### 2. Collateral Verification
- Connect to Intel Provisioning Certification Service (PCS) to fetch TCB (Trusted Computing Base) Info, QE Identity, and Revocation Lists.
- Implement cryptographic verification of the DCAP quotes using fetched collateral.

### 3. Proxy API Endpoints
- Implement `GET /attestation/quote?nonce=<challenge>` in the `aegis-proxy` or `process_manager`/`server` modules.
- Implement `POST /attestation/verify` that accepts a quote and a nonce, returning the validation result.
- Connect these endpoints to the `AttestationProvider`.

## Acceptance Criteria
- [ ] `generate_sgx_quote` and `verify_sgx_quote` execute real DCAP operations (or appropriately designed mock structures that interact with a test PCS server).
- [ ] `generate_sev_snp_quote` produces a real SEV-SNP report layout.
- [ ] HTTP endpoint `/attestation/quote` is reachable and returns a valid `AttestationQuote` payload.
- [ ] HTTP endpoint `/attestation/verify` correctly validates the quote against local/remote collateral.
- [ ] Unit and Integration tests cover both successful validations and stale/invalid nonce scenarios on the new endpoints.
