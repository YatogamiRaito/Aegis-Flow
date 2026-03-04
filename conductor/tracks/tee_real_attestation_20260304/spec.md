# Track 55: Real TEE Attestation Integration — Specification

## Overview

The `attestation.rs` module in `aegis-crypto` currently ships with **stub implementations** for all TEE platforms (Intel SGX/TDX, AMD SEV-SNP, ARM TrustZone). All `generate_*_quote()` functions return mock byte strings (e.g., `b"SGX_QUOTE_V3_MOCK_DATA"`), and all `verify_*_quote()` functions return `Err(AegisError::NotImplemented)`.

This track replaces those stubs with real hardware-backed attestation backends gated behind the `tee-real` Cargo feature flag. The goal is production-grade, cryptographically verifiable remote attestation — a foundational requirement for Aegis-Flow nodes running inside TEEs.

---

## Functional Requirements

### FR-1: Intel SGX/TDX — Gramine FS API Backend
- **Quote Generation:** Write a randomly-seeded nonce (SHA-256 of user_data) into `/dev/attestation/user_report_data`, then read `/dev/attestation/quote` to obtain the hardware-signed DCAP quote.
- **Quote Verification:** Verify the returned DCAP quote using Intel's `sgx_dcap_quoteverify` C library, wrapped behind a Rust FFI shim (or via the `dcap-ql` crate if stable).
- **Gramine Detection:** Check that `/dev/attestation/quote` exists at startup; if not, fall back to simulation mode with a `WARN` log.

### FR-2: AMD SEV-SNP — VirTEE `sev` Crate Backend
- **Quote Generation:** Use the `sev` crate (`virtee/sev`) to request an SNP attestation report via `/dev/sev-guest` ioctl.
- **Quote Verification:** Fetch AMD VCEK (Versioned Chip Endorsement Key) certificate from `https://kdsintf.amd.com` and verify the SNP report signature in Rust (no C dependency).
- **Offline Mode:** Cache the VCEK certificate to disk (`/var/cache/aegis/vcek.pem`) to avoid network round-trips on every verification.

### FR-3: ARM TrustZone — OP-TEE Backend
- **Quote Generation:** Use the `optee-teec` crate to open `/dev/tee0` and invoke a Trusted Application (TA) UUID for device identity retrieval.
- **Quote Verification:** Verify the TA response against an operator-supplied certificate embedded at compile time (`AEGIS_TRUSTZONE_ROOT_CA` env var).

### FR-4: Feature Flag Gating
- All real backends MUST be guarded by `#[cfg(feature = "tee-real")]`.
- When `tee-real` is disabled (default), behaviour must remain identical to today: stubs return mock data or `Err(NotImplemented)` with a `WARN` log. **No regression** is acceptable.
- Add `tee-real` to `Cargo.toml` as an optional feature with appropriate conditional dependencies.

### FR-5: Attestation Proxy API Endpoints
- Expose two HTTP endpoints through the Aegis-Flow proxy runtime:
  - `GET /v1/tee/quote?nonce=<hex>` — generates and returns a fresh attestation quote as JSON.
  - `POST /v1/tee/verify` — accepts a JSON body `{ "quote": "<base64>", "nonce": "<hex>", "platform": "sgx|tdx|sev|tz" }` and returns `{ "valid": true|false }`.
- Endpoints must validate incoming nonces (minimum 16 bytes, maximum 64 bytes).

### FR-6: Quote Caching & Freshness
- Generated quotes must be cached in memory for up to **60 seconds** to avoid repeated `/dev/attestation/quote` reads (which can be slow on some SGX setups).
- Expired cached quotes must be transparently regenerated.

---

## Non-Functional Requirements

- **Performance:** Quote generation must complete in < 500 ms on SGX hardware. Verification must complete in < 200 ms (excluding first-time VCEK fetch).
- **Security:** No quote bytes or signing keys must be logged at any log level. Use `#[instrument(skip_all)]` on all quote-handling functions.
- **Portability:** `tee-real` must compile cleanly on `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`. Non-Linux builds must be gated.
- **Backward Compatibility:** Existing `attestation.rs` public API (`AttestationProvider`, `AttestationQuote`, `TeeCapabilities`) must remain unchanged.

---

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-1 | `cargo build --features tee-real` succeeds on Linux x86_64 |
| AC-2 | `cargo build --features tee-real` succeeds on Linux aarch64 |
| AC-3 | `cargo build` (without `tee-real`) produces zero regressions |
| AC-4 | Unit tests for `generate_quote` pass in simulation mode |
| AC-5 | Integration test: `AttestationProvider::generate_quote` on a Gramine-hosted runner returns a non-mock quote |
| AC-6 | `verify_quote` with a tampered quote byte returns `Ok(false)` or `Err` (not a panic) |
| AC-7 | `/v1/tee/quote` endpoint returns `200 OK` with valid JSON structure |
| AC-8 | `/v1/tee/verify` with a tampered quote returns `{ "valid": false }` |
| AC-9 | No secret data appears in `tracing` output at any level |
| AC-10 | `cargo clippy --features tee-real -- -D warnings` is clean |

---

## Out of Scope

- Full PKI certificate chain rotation automation (future track).
- Multi-enclave orchestration (multiple SGX enclaves in a single Aegis-Flow process).
- Windows or macOS TEE backends.
- Production deployment of AMD VCEK certificate rotation.
