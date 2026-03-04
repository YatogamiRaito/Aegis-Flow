# Project Tracks

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

---

## [x] Track: Core TEE-Native PQC Data Plane
*Link: [./conductor/tracks/core_tee_pqc_20251224/](./conductor/tracks/core_tee_pqc_20251224/)*

## [x] Track: Secure Data Plane with Encryption
*Link: [./conductor/tracks/secure_data_plane_20251225/](./conductor/tracks/secure_data_plane_20251225/)*

## [x] Track: Cloud Native Integration
*Link: [./conductor/tracks/cloud_native_20251225/](./conductor/tracks/cloud_native_20251225/)*

---

## [x] Track: Carbon-Aware Traffic Routing
*Link: [./conductor/tracks/carbon_aware_20251225/](./conductor/tracks/carbon_aware_20251225/)*

---

## [x] Track: HTTP/3 and QUIC Protocol Support
*Link: [./conductor/tracks/http3_quic_20251226/](./conductor/tracks/http3_quic_20251226/)*

---

## [x] Track: Performance Benchmark Suite
*Link: [./conductor/tracks/benchmarks_20251226/](./conductor/tracks/benchmarks_20251226/)*

---

## [x] Track: eBPF Energy Telemetry
*Link: [./conductor/tracks/ebpf_telemetry_20251226/](./conductor/tracks/ebpf_telemetry_20251226/)*

---

## [x] Track: Genomic Data Processing
*Link: [./conductor/tracks/genomics_20251226/](./conductor/tracks/genomics_20251226/)*
---

## [x] Track: WebAssembly Plugin System
*Link: [./conductor/tracks/wasm_plugins_20251226/](./conductor/tracks/wasm_plugins_20251226/)*

---

## [x] Track: PQC Migration & Security Hardening
*Link: [./conductor/tracks/pqc_migration_20251226/](./conductor/tracks/pqc_migration_20251226/)*

---

## [x] Track 11: ML-DSA (Dilithium) Full Migration for Digital Signatures (v0.11.0)
*Link: [./conductor/tracks/mldsa_signing_20251228/](./conductor/tracks/mldsa_signing_20251228/)*
*Priority: 1 - Core cryptography foundation*

---

## [x] Track 12: Advanced TEE Integration with Remote Attestation (v0.12.0)
*Link: [./conductor/tracks/tee_attestation_20251228/](./conductor/tracks/tee_attestation_20251228/)*
*Priority: 2 - Depends on ML-DSA for quote signing*
*Audit Score: 4.8/10 → Hardening track created*

---

## [x] Track 13: Production-Ready Deployment with Helm Chart Improvements (v0.13.0)
*Link: [./conductor/tracks/production_deployment_20251228/](./conductor/tracks/production_deployment_20251228/)*
*Priority: 3 - Infrastructure after core features*
*Audit Score: 6.2/10 → Hardening track created*

---

## [x] Track 14: Prometheus/Grafana Dashboard Expansion (v0.14.0)
*Link: (Spec/Plan files missing, validated via `grafana/` and `deploy/helm`)*
*Priority: 3 - Observability layer for operational readiness*
*Audit Score: 9.8/10 → Perfect implementation, no hardening needed*

---

## [x] Track 15: Process Manager Core (v0.15.0)
*Link: [./conductor/tracks/process_manager_20260302/](./conductor/tracks/process_manager_20260302/)*
*Priority: 1 - Foundational for clustered multi-worker execution*
*Audit Score: 6.5/10 → Hardening track created*

---

## [x] Track 16: Static File Server & Compression (v0.16.0)
*Link: [./conductor/tracks/static_server_20260302/](./conductor/tracks/static_server_20260302/)*
*Priority: 2 - Core nginx feature: static content with Gzip/Brotli*
*Audit Score: 7.2/10 → Hardening track created*

---

## [x] Track 17: Virtual Hosts & Routing Engine (v0.17.0)
*Link: [./conductor/tracks/virtual_hosts_20260302/](./conductor/tracks/virtual_hosts_20260302/)*
*Priority: 3 - Server blocks, location matching, URL rewriting*
*Audit Score: 4.5/10 → Hardening track created*

---

## [x] Track 18: Upstream Groups & Advanced Load Balancing (v0.18.0)
*Link: [./conductor/tracks/upstream_lb_20260302/](./conductor/tracks/upstream_lb_20260302/)*
*Priority: 4 - Health checks, sticky sessions, circuit breaker*
*Audit Score: 4.5/10 → Hardening track created*

---

## [x] Track 19: Rate Limiting & Security (v0.19.0)
*Link: [./conductor/tracks/rate_limiting_20260302/](./conductor/tracks/rate_limiting_20260302/)*
*Priority: 5 - Token bucket, IP ACL, HTTP auth, WAF basics*
*Audit Score: 4.5/10 → Hardening track created*

---

## [x] Track 20: Proxy Caching & Response Optimization (v0.20.0)
*Link: [./conductor/tracks/caching_20260302/](./conductor/tracks/caching_20260302/)*
*Priority: 6 - Two-tier cache, stale serving, purge API*
*Audit Score: 3.5/10 → Hardening track created*

---

## [x] Track 21: Log Management & CLI Interface (v0.21.0)
*Link: [./conductor/tracks/logging_cli_20260302/](./conductor/tracks/logging_cli_20260302/)*
*Priority: 7 - Access/error logs, TUI monitor, startup scripts*
*Audit Score: 1.5/10 → Hardening track created*

---

## [x] Track 22: WebSocket, TCP/UDP Stream & Protocol Support (v0.22.0)
*Link: [./conductor/tracks/stream_proxy_20260302/](./conductor/tracks/stream_proxy_20260302/)*
*Priority: 8 - L4 proxy, WebSocket, PROXY Protocol, FastCGI, gRPC*
*Audit Score: 4.0/10 → Hardening track created*

---

## [x] Track 23: Automatic HTTPS & Certificate Management (v0.23.0)
*Link: [./conductor/tracks/auto_https_20260302/](./conductor/tracks/auto_https_20260302/)*
*Priority: 9 - Zero-config Let's Encrypt provisioning*
*Audit Score: 7.5/10 → Hardening track created* via HTTP-01/ALPN-01/DNS-01, On-Demand provisioning, and natively integrated x509-ocsp Stapling.*

---

## [x] Track 24: Aegisfile — Simple Configuration Format (v0.24.0)
*Link: [./conductor/tracks/aegisfile_config_20260302/](./conductor/tracks/aegisfile_config_20260302/)*
*Priority: 6 - Caddy-like human-readable config format*
*Audit Score: 4.0/10 → Hardening track created*

---

## [x] Track 25: Dynamic Configuration API (v0.25.0)
*Link: [./tracks/dynamic_config_api_20260302/](./tracks/dynamic_config_api_20260302/)*
*Priority: 11 - REST admin API, runtime config CRUD, versioning & rollback*

---

## [x] Track 26: Advanced Request Processing (v0.26.0)
*Link: [./tracks/advanced_request_processing_20260302/](./tracks/advanced_request_processing_20260302/)*
*Priority: 12 - map directive, split_clients A/B, auth_request, traffic mirroring*
*Status: Complete — split_clients MurmurHash3 A/B, auth_request subrequest, limit_except method ACL, stub_status metrics endpoint. All 634 tests pass.*

---

## [x] Track 27: Response Transformation & Logging Extensions (v0.27.0)
*Link: [./tracks/response_transform_20260302/](./tracks/response_transform_20260302/)*
*Priority: 13 - sub_filter body rewriting, syslog, SSI, image filter*
*Status: Complete — regex sub_filter, RFC 5424 syslog (UDP/TCP), SSI directive parsing, image_filter passthrough. 641 tests pass.*

---

## [x] Track 28: Multi-Worker Architecture (v0.28.0)
*Link: [./tracks/multi_worker_20260302/](./tracks/multi_worker_20260302/)*
*Priority: 14 - Master + N workers, SO_REUSEPORT, hot binary upgrade*

---

## [x] Track 29: GeoIP Routing & Mail Proxy (v0.29.0)
*Link: [./tracks/geoip_mail_20260302/](./tracks/geoip_mail_20260302/)*
*Priority: 14 - GeoIP MMDB lookup, geo directive, SMTP/IMAP/POP3 mail proxy*
*Status: Complete — geoip.rs enhanced (city/region/lat/lon/ASN/org, MMDB hot-reload, proxy_recursive), aegis-mail crate (smtp, imap, pop3, mail_auth). 29 mail + 650 proxy tests pass.*

---

## [ ] Track 30: PQC Data Plane Security Hardening (v0.30.0)
*Link: [./tracks/pqc_hardening_20260304/](./tracks/pqc_hardening_20260304/)*
*Priority: 1 - Critical security fixes: HKDF KDF, zeroize, trait compat, attestation stubs, Gramine hardening*
*Audit Score: 6.3/10 → Target 10/10*

---

## [ ] Track 31: Cloud Native Integration Hardening (v0.31.0)
*Link: [./tracks/cloud_native_hardening_20260304/](./tracks/cloud_native_hardening_20260304/)*
*Priority: 2 - xDS protocol (LDS/CDS/RDS/ADS), OpenTelemetry SDK, Grafana dashboards, Helm fixes*
*Audit Score: 7.5/10 → Target 10/10*

---

## [ ] Track 32: Carbon-Aware Traffic Routing Hardening (v0.32.0)
*Link: [./tracks/carbon_aware_hardening_20260304/](./tracks/carbon_aware_hardening_20260304/)*
*Priority: 3 - Retry/backoff, forecast API, queue persistence, integration test, benchmark*
*Audit Score: 8.8/10 → Target 10/10*

---

## [ ] Track 33: HTTP/3 & QUIC Protocol Hardening (v0.33.0)
*Link: [./conductor/tracks/http3_quic_hardening_20260304/](./conductor/tracks/http3_quic_hardening_20260304/)*
*Priority: 1 - Real h3 framing, QUIC transport config, 0-RTT, PQC integration, upstream forwarding, security hardening*
*Audit Score: 5.2/10 → Target 10/10*

---

## [ ] Track 34: Performance Benchmark Hardening (v0.34.0)
*Link: [./conductor/tracks/benchmark_hardening_20260304/](./conductor/tracks/benchmark_hardening_20260304/)*
*Priority: 2 - CI benchmark step, real network load test, Envoy comparison, missing benchmarks, regression detection*
*Audit Score: 6.1/10 → Target 10/10*

---

## [ ] Track 35: eBPF Energy Telemetry Hardening (v0.35.0)
*Link: [./conductor/tracks/ebpf_telemetry_hardening_20260304/](./conductor/tracks/ebpf_telemetry_hardening_20260304/)*
*Priority: 3 - Real eBPF architecture (Aya), RingBuf, global Prometheus exporter, RAPL fallback, <1% overhead benchmark*
*Audit Score: 3.8/10 → Target 10/10*

---

## [ ] Track 36: Genomic Data Processing Hardening (v0.36.0)
*Link: [./conductor/tracks/genomics_hardening_20260304/](./conductor/tracks/genomics_hardening_20260304/)*
*Priority: 3 - Arrow Flight Server implementation, Proxy integration, FASTA parser, and <5s 1GB streaming benchmark*
*Audit Score: 6.5/10 → Target 10/10*

---

## [ ] Track 37: WASM Plugin Engine Proxy Integration (v0.37.0)
*Link: [./conductor/tracks/wasm_plugins_hardening_20260304/](./conductor/tracks/wasm_plugins_hardening_20260304/)*
*Priority: 3 - WASM host ABI functions, real plugin compilation (wasm32), proxy request pipeline hooks, <100µs limit*
*Audit Score: 8.5/10 → Target 10/10*

---

## [ ] Track 38: TEE Attestation Security Hardening (v0.38.0)
*Link: [./conductor/tracks/tee_attestation_hardening_20260304/](./conductor/tracks/tee_attestation_hardening_20260304/)*
*Priority: 1 - Real SGX/TDX/SEV bindings, PCS collateral fetch, /attestation proxy APIs*
*Audit Score: 4.8/10 → Target 10/10*

---

## [ ] Track 39: Production Deployment Hardening (v0.39.0)
*Link: [./conductor/tracks/production_deployment_hardening_20260304/](./conductor/tracks/production_deployment_hardening_20260304/)*
*Priority: 3 - External Secrets Operator CRDs, Multi-Cluster and Service Mesh routing configurations*
*Audit Score: 6.2/10 → Target 10/10*

---

## [ ] Track 40: Process Manager CLI & Daemon Bootstrapping (v0.40.0)
*Link: [./conductor/tracks/process_manager_hardening_20260304/](./conductor/tracks/process_manager_hardening_20260304/)*
*Priority: 1 - Integrate procman crate with Aegis CLI (start/stop) and background daemon loop*
*Audit Score: 6.5/10 → Target 10/10*

---

## [ ] Track 41: Static File Server Hardening (v0.41.0)
*Link: [./conductor/tracks/static_server_hardening_20260304/](./conductor/tracks/static_server_hardening_20260304/)*
*Priority: 2 - Asynchronous streaming and caching header implementations*
*Audit Score: 7.2/10 → Target 10/10*

---

## [ ] Track 42: Virtual Hosts Hardening (v0.42.0)
*Link: [./conductor/tracks/virtual_hosts_hardening_20260304/](./conductor/tracks/virtual_hosts_hardening_20260304/)*
*Priority: 2 - Bind routing models into `config.rs` and `http_proxy.rs`*
*Audit Score: 4.5/10 → Target 10/10*

---

## [ ] Track 43: Upstream Groups Hardening (v0.43.0)
*Link: [./conductor/tracks/upstream_lb_hardening_20260304/](./conductor/tracks/upstream_lb_hardening_20260304/)*
*Priority: 2 - Bind load balancing and circuit breakers to proxy lifecycle*
*Audit Score: 4.5/10 → Target 10/10*

---

## [ ] Track 44: Rate Limiting & Security Hardening (v0.44.0)
*Link: [./conductor/tracks/rate_limiting_hardening_20260304/](./conductor/tracks/rate_limiting_hardening_20260304/)*
*Priority: 2 - Bind security models to proxy lifecycle*
*Audit Score: 4.5/10 → Target 10/10*

---

## [ ] Track 45: Proxy Caching Hardening (v0.45.0)
*Link: [./conductor/tracks/caching_hardening_20260304/](./conductor/tracks/caching_hardening_20260304/)*
*Priority: 2 - Implement Disk Caching, Stale Updates and PURGE API*
*Audit Score: 3.5/10 → Target 10/10*

---

## [ ] Track 46: Log Management & CLI Hardening (v0.46.0)
*Link: [./conductor/tracks/logging_cli_hardening_20260304/](./conductor/tracks/logging_cli_hardening_20260304/)*
*Priority: 2 - Build the missing CLI and async log writers*
*Audit Score: 1.5/10 → Target 10/10*

---

## [ ] Track 47: Protocol & Stream Proxy Hardening (v0.47.0)
*Link: [./conductor/tracks/stream_proxy_hardening_20260304/](./conductor/tracks/stream_proxy_hardening_20260304/)*
*Priority: 2 - Inject the proxy loops and HTTP Upgrades into lifecycle*
*Audit Score: 4.0/10 → Target 10/10*

---

## [ ] Track 48: Auto HTTPS & On-Demand TLS Hardening (v0.48.0)
*Link: [./conductor/tracks/auto_https_hardening_20260304/](./conductor/tracks/auto_https_hardening_20260304/)*
*Priority: 3 - Custom async ClientHello Peeker for true on-demand issuance*
*Audit Score: 7.5/10 → Target 10/10*

---

## [ ] Track 49: Aegisfile CLI & Integration Hardening (v0.49.0)
*Link: [./conductor/tracks/aegisfile_config_hardening_20260304/](./conductor/tracks/aegisfile_config_hardening_20260304/)*
*Priority: 4 - Integrate Aegisfile parsing into main lifecycle and CLI*
*Audit Score: 4.0/10 → Target 10/10*
