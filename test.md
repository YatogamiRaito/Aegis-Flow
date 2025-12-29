# Test Coverage Report
Run cargo tarpaulin --workspace

## Overall Status
**Total Coverage:** 91.56% (2885/3151 lines covered)

## Coverage by File

### 100% Coverage 🟢
- `crates/crypto/src/cipher.rs` (65/65)
- `crates/crypto/src/hybrid_kex.rs` (93/93)
- `crates/energy/src/cache.rs` (28/28)
- `crates/energy/src/types.rs` (13/13)
- `crates/genomics/src/alignment.rs` (77/77)
- `crates/genomics/src/analytics.rs` (59/59)
- `crates/genomics/src/schema.rs` (37/37)
- `crates/genomics/src/variant.rs` (67/67)
- `crates/plugins/src/engine.rs` (48/48)
- `crates/plugins/src/interface.rs` (27/27)
- `crates/plugins/src/registry.rs` (65/65)
- `crates/proxy/src/tracing_otel.rs` (59/59)
- `crates/telemetry/src/ebpf/mod.rs` (2/2)
- `crates/telemetry/src/energy.rs` (36/36)
- `crates/telemetry/src/prometheus.rs` (27/27)

### >90% Coverage 🟢
- `crates/crypto/src/tls.rs` (52/53) 98.1%
- `crates/proxy/src/carbon_router.rs` (84/86) 97.7%
- `crates/proxy/src/http3_handler.rs` (59/61) 96.7%
- `crates/telemetry/src/estimator.rs` (58/60) 96.7%
- `crates/crypto/src/attestation.rs` (209/217) 96.3%
- `crates/proxy/src/discovery.rs` (69/72) 95.8%
- `crates/proxy/src/config.rs` (154/162) 95.1%
- `crates/genomics/src/bam_parser.rs` (87/92) 94.6%
- `crates/genomics/src/vcf_parser.rs` (49/52) 94.2%
- `crates/crypto/src/signing.rs` (276/296) 93.2%
- `crates/crypto/src/certmanager.rs` (129/138) 93.5%
- `crates/proxy/src/metrics.rs` (55/59) 93.2%
- `crates/crypto/src/stream.rs` (100/111) 90.1%
- `crates/proxy/src/lifecycle.rs` (92/102) 90.2%
- `crates/crypto/src/mtls.rs` (122/135) 90.4%

### 70-90% Coverage 🟡
- `crates/energy/src/client.rs` (46/52) 88.5%
- `crates/proxy/src/green_wait.rs` (109/125) 87.2%
- `crates/telemetry/src/ebpf/loader.rs` (20/23) 87.0%
- `crates/proxy/src/dual_stack_server.rs` (55/64) 85.9%
- `crates/proxy/src/health_server.rs` (56/67) 83.6%
- `crates/proxy/src/bootstrap.rs` (29/36) 80.6%
- `crates/proxy/src/pqc_server.rs` (58/77) 75.3%

### <70% Coverage 🔴
- `crates/proxy/src/quic_server.rs` (70/110) 63.6%
- `crates/proxy/src/server.rs` (19/29) 65.5%
- `crates/telemetry/src/ebpf/metrics.rs` (68/71) 95.8%
- `crates/proxy/src/http_proxy.rs` (35/76) 46.1%

## Uncovered Lines Summary

### High Priority (Core Crypto)
- `signing.rs`: 179-181, 267-269, 355-357, 460-461, 610-611, 642-643, 653-655, 719, 733
- `stream.rs`: 79, 91, 139-140, 177, 201-203, 232, 261-262
- `certmanager.rs`: 134, 166, 194, 230, 270-271, 293, 326-327

### Medium Priority (Proxy)
- `http_proxy.rs`: 55, 80, 104-105, 130-173, 178-194
- `quic_server.rs`: 132-196, 210-261
- `pqc_server.rs`: 49, 65-67, 76-82, 100-101, 108-110, 124-126, 147, 151-152
