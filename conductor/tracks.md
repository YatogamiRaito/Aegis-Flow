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

---

## [x] Track 13: Production-Ready Deployment with Helm Chart Improvements (v0.13.0)
*Link: [./conductor/tracks/production_deployment_20251228/](./conductor/tracks/production_deployment_20251228/)*
*Priority: 3 - Infrastructure after core features*

---

## [x] Track 14: Prometheus/Grafana Dashboard Expansion (v0.14.0)
*Link: [./conductor/tracks/observability_dashboards_20251228/](./conductor/tracks/observability_dashboards_20251228/)*
*Priority: 4 - LAST - Monitors all features*

---

## [x] Track 15: Process Manager Core (v0.15.0)
*Link: [./tracks/process_manager_20260302/](./tracks/process_manager_20260302/)*
*Completion: 2026-03-03*
*Priority: 1 - Foundation for nginx+PM2 unification*

---

## [x] Track 16: Static File Server & Compression (v0.16.0)
*Link: [./tracks/static_server_20260302/](./tracks/static_server_20260302/)*
*Priority: 2 - Core nginx feature: static content with Gzip/Brotli*

---

## [x] Track 17: Virtual Hosts & Routing Engine (v0.17.0)
*Link: [./tracks/virtual_hosts_20260302/](./tracks/virtual_hosts_20260302/)*
*Priority: 3 - Server blocks, location matching, URL rewriting*

---

## [x] Track 18: Upstream Groups & Advanced Load Balancing (v0.18.0)
*Link: [./tracks/upstream_lb_20260302/](./tracks/upstream_lb_20260302/)*
*Priority: 4 - Health checks, sticky sessions, circuit breaker*

---

## [x] Track 19: Rate Limiting & Security (v0.19.0)
*Link: [./tracks/rate_limiting_20260302/](./tracks/rate_limiting_20260302/)*
*Priority: 5 - Token bucket, IP ACL, HTTP auth, WAF basics*

---

## [~] Track 20: Proxy Caching & Response Optimization (v0.20.0)
*Link: [./tracks/caching_20260302/](./tracks/caching_20260302/)*
*Priority: 6 - Two-tier cache, stale serving, purge API*
*Status: Missing - Missing proxy-level cache.*

---

## [x] Track 21: Log Management & CLI Interface (v0.21.0)
*Link: [./tracks/logging_cli_20260302/](./tracks/logging_cli_20260302/)*
*Priority: 7 - Access/error logs, TUI monitor, startup scripts*

---

## [ ] Track 22: WebSocket, TCP/UDP Stream & Protocol Support (v0.22.0)
*Link: [./tracks/stream_proxy_20260302/](./tracks/stream_proxy_20260302/)*
*Priority: 8 - L4 proxy, WebSocket, PROXY Protocol, FastCGI, gRPC*
*Status: Missing - Missing L4 logic.*

---

## [ ] Track 23: Automatic HTTPS & Certificate Management (v0.23.0)
*Link: [./tracks/auto_https_20260302/](./tracks/auto_https_20260302/)*
*Priority: 9 - ACME/Let's Encrypt, OCSP stapling, on-demand TLS*
*Status: Missing - Missing ACME logic.*

---

## [x] Track 24: Aegisfile — Simple Configuration Format (v0.24.0)
*Link: [./tracks/aegisfile_config_20260302/](./tracks/aegisfile_config_20260302/)*
*Priority: 10 - Caddyfile-like DX, nginx config import, formatter*

---

## [x] Track 25: Dynamic Configuration API (v0.25.0)
*Link: [./tracks/dynamic_config_api_20260302/](./tracks/dynamic_config_api_20260302/)*
*Priority: 11 - REST admin API, runtime config CRUD, versioning & rollback*

---

## [~] Track 26: Advanced Request Processing (v0.26.0)
*Link: [./tracks/advanced_request_processing_20260302/](./tracks/advanced_request_processing_20260302/)*
*Priority: 12 - map directive, split_clients A/B, auth_request, traffic mirroring*

---

## [~] Track 27: Response Transformation & Logging Extensions (v0.27.0)
*Link: [./tracks/response_transform_20260302/](./tracks/response_transform_20260302/)*
*Priority: 13 - sub_filter body rewriting, syslog, SSI, image filter*

---

## [x] Track 28: Multi-Worker Architecture (v0.28.0)
*Link: [./tracks/multi_worker_20260302/](./tracks/multi_worker_20260302/)*
*Priority: 14 - Master + N workers, SO_REUSEPORT, hot binary upgrade*

---

## [~] Track 29: GeoIP Routing & Mail Proxy (v0.29.0)
*Link: [./tracks/geoip_mail_20260302/](./tracks/geoip_mail_20260302/)*
*Priority: 15 - MaxMind GeoIP2, geo directive, SMTP/IMAP/POP3 proxy*


