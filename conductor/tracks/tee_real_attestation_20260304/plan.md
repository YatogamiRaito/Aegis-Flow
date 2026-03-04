# Track 55: Real TEE Attestation Integration — Implementation Plan

## Phase 1: Foundation & Feature Gating

- [ ] Task: Audit current `attestation.rs` stubs
    - [ ] Document every stub function and its return behaviour
    - [ ] Confirm `tee-real` feature flag compiles (currently empty `mod real_tee {}`)
- [ ] Task: Extend `Cargo.toml` with conditional dependencies
    - [ ] Add `sev = { version = "4", optional = true }` under `[dependencies]`
    - [ ] Add `optee-teec = { version = "0.3", optional = true }` under `[dependencies]`
    - [ ] Gate existing `raw-cpuid` under `[target.'cfg(any(target_arch="x86", target_arch="x86_64"))'.dependencies]`
    - [ ] Define `tee-real = ["sev", "optee-teec"]` in `[features]`
- [ ] Task: Conductor - User Manual Verification 'Phase 1 Foundation' (Protocol in workflow.md)

---

## Phase 2: Intel SGX/TDX — Gramine FS API Backend

- [ ] Task: Write tests for SGX quote generation (TDD)
    - [ ] Test: `generate_sgx_quote` returns non-mock bytes when `/dev/attestation/quote` readable
    - [ ] Test: Falls back to simulation when Gramine FS not present
- [ ] Task: Implement `generate_sgx_quote` (real)
    - [ ] Detect `/dev/attestation/quote` presence at `AttestationProvider::new()`
    - [ ] Compute `SHA-256(nonce || user_data)` and write to `/dev/attestation/user_report_data`
    - [ ] Read raw bytes from `/dev/attestation/quote`
    - [ ] Return `AttestationQuote` with real `quote_bytes`
- [ ] Task: Implement `generate_tdx_quote` (real)
    - [ ] Reuse Gramine FS path (TDX uses same `/dev/attestation/` interface in Gramine)
    - [ ] Tag quote with `TeePlatform::IntelTdx`
- [ ] Task: Implement `verify_sgx_quote` and `verify_tdx_quote` (real)
    - [ ] Add `#[cfg(feature = "tee-real")]` block with FFI call to `sgx_qv_verify_quote`
    - [ ] Handle `SGX_QL_QV_RESULT_OK` mapping to `Ok(true)`
    - [ ] Handle all error codes with structured `AegisError::Crypto`
- [ ] Task: Conductor - User Manual Verification 'Phase 2 SGX/TDX' (Protocol in workflow.md)

---

## Phase 3: AMD SEV-SNP — VirTEE `sev` Crate Backend

- [ ] Task: Write tests for SEV-SNP quote generation (TDD)
    - [ ] Test: `generate_sev_snp_quote` returns attestation report bytes on SEV-SNP host
    - [ ] Test: VCEK cache path is checked before network fetch
- [ ] Task: Implement `generate_sev_snp_quote` (real)
    - [ ] Use `sev::firmware::guest::Firmware::open()?` to open `/dev/sev-guest`
    - [ ] Call `firmware.get_report(None, user_data_hash, 0)?` to obtain `AttestationReport`
    - [ ] Serialize report to bytes and store in `AttestationQuote::quote_bytes`
- [ ] Task: Implement `verify_sev_snp_quote` (real)
    - [ ] Fetch VCEK certificate from AMD KDS (`https://kdsintf.amd.com/vcek/v1/{chip_id}/{tcb}`)
    - [ ] Cache PEM to `/var/cache/aegis/vcek.pem` with a 24-hour TTL
    - [ ] Verify SNP report ECDSA P-384 signature against VCEK public key
- [ ] Task: Conductor - User Manual Verification 'Phase 3 SEV-SNP' (Protocol in workflow.md)

---

## Phase 4: ARM TrustZone — OP-TEE Backend

- [ ] Task: Write tests for TrustZone quote generation (TDD)
    - [ ] Test: Returns mock when `/dev/tee0` absent (CI environment)
    - [ ] Test: Parses TA response bytes into `AttestationQuote`
- [ ] Task: Implement `generate_trustzone_quote` (real)
    - [ ] Use `optee-teec::Context::new()`
    - [ ] Open TA session with predefined `Uuid` for Aegis identity TA
    - [ ] Invoke `TEEC_InvokeCommand(GET_ATTESTATION, ...)` and read output
- [ ] Task: Implement `verify_trustzone_quote` (real)
    - [ ] Load operator root CA from env `AEGIS_TRUSTZONE_ROOT_CA` (PEM)
    - [ ] Verify TA response signature using `ring::signature::UnparsedPublicKey`
- [ ] Task: Conductor - User Manual Verification 'Phase 4 TrustZone' (Protocol in workflow.md)

---

## Phase 5: HTTP Proxy API Endpoints

- [ ] Task: Write integration tests for attestation proxy endpoints (TDD)
    - [ ] Test: `GET /v1/tee/quote?nonce=aabbccdd...` returns `200 OK` JSON
    - [ ] Test: `POST /v1/tee/verify` with tampered quote returns `{ "valid": false }`
    - [ ] Test: Nonce shorter than 16 bytes returns `400 Bad Request`
- [ ] Task: Implement quote endpoint handler
    - [ ] Parse hex nonce from query string; validate length (16–64 bytes)
    - [ ] Call `AttestationProvider::generate_quote(nonce, user_data)`
    - [ ] Serialize `AttestationQuote` to JSON and return
- [ ] Task: Implement verify endpoint handler
    - [ ] Deserialize incoming JSON body
    - [ ] Deserialize base64 quote bytes back to `AttestationQuote`
    - [ ] Call `AttestationProvider::verify_quote(&quote, nonce)`
    - [ ] Return `{ "valid": true|false }` or `{ "error": "..." }`
- [ ] Task: Integrate endpoints into Aegis-Flow router
    - [ ] Register `/v1/tee/quote` and `/v1/tee/verify` in the admin listener
- [ ] Task: Conductor - User Manual Verification 'Phase 5 HTTP API' (Protocol in workflow.md)

---

## Phase 6: Quote Caching & Freshness

- [ ] Task: Add in-memory quote cache
    - [ ] Use `Arc<Mutex<HashMap<[u8;32], (AttestationQuote, Instant)>>>` keyed on SHA-256 of `(nonce, user_data)`
    - [ ] Cache TTL: 60 seconds
    - [ ] Add `AttestationProvider::get_or_generate_quote()` helper
- [ ] Task: Write cache eviction tests
    - [ ] Test: Expired entries are regenerated
    - [ ] Test: Concurrent requests with same nonce return identical cached bytes
- [ ] Task: Conductor - User Manual Verification 'Phase 6 Caching' (Protocol in workflow.md)

---

## Phase 7: Quality & Security Hardening

- [ ] Task: `cargo clippy --features tee-real -- -D warnings` clean
- [ ] Task: `cargo fmt --features tee-real -- --check` clean
- [ ] Task: Audit all quote-handling code paths for secret data logging
    - [ ] Add `#[instrument(skip_all)]` to `generate_quote`, `verify_quote`, all platform sub-functions
- [ ] Task: Run `cargo test -p aegis-crypto` with all stubs (no regression)
- [ ] Task: Run `cargo test -p aegis-crypto --features tee-real` (simulation mode passes)
- [ ] Task: Conductor - User Manual Verification 'Phase 7 Quality' (Protocol in workflow.md)
