# Track Plan: TEE Attestation Security Hardening (v0.38.0)

## Phase 1: Native Platform Bindings
- [ ] Task: Evaluate and integrate `intel-tee-quote-provider` or equivalent Rust packages for DCAP SGX/TDX.
- [ ] Task: Integrate `sev` crate for SEV-SNP.
- [ ] Task: Replace mock byte sequences in `attestation.rs` with real hardware calls (protecting with `cfg` attributes for local development fallback).
- [ ] Task: Conductor Verification 'Native Bindings'

## Phase 2: Collateral & Verification Service
- [ ] Task: Implement Intel PCS HTTP Client to fetch TCB Info and PCK Certs.
- [ ] Task: Integrate DCAP Quote Verification library (`sgx_dcap_quoteverify`).
- [ ] Task: Implement AMD SEV VCEK certificate fetching and verification.
- [ ] Task: Conductor Verification 'Collateral Verification'

## Phase 3: Proxy HTTP Endpoint Integration
- [ ] Task: Add `/attestation/quote` endpoint logic in `aegis-proxy` routers.
- [ ] Task: Add `/attestation/verify` endpoint logic.
- [ ] Task: Write end-to-end integration tests mimicking a client fetching and verifying a quote.
- [ ] Task: Conductor Verification 'Proxy Integration'

## Phase 4: Release & Perfection
- [ ] Task: Performance optimization (caching collateral, asynchronous verifications).
- [ ] Task: Documentation updates (How to deploy Aegis-Flow on SGX-enabled Kubernetes nodes).
- [ ] Task: Conductor Verification 'Release'
