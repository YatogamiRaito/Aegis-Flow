# Test Coverage Tracking

## Eklenen Testler (Bu Oturum)

### ✅ Tamamlanan
| Dosya | Önceki | Eklenen Test | Durum |
|-------|--------|--------------|-------|
| `registry.rs` | 64/65 | `test_load_all_plugins_with_invalid_wasm` | ✅ |
| `http3_handler.rs` | 59/61 | `test_readiness_endpoint`, `test_healthz_endpoint` | ✅ |
| `estimator.rs` | 58/60 | `test_shared_energy_estimator_type`, `test_total_energy_joules_precision` | ✅ |
| `vcf_parser.rs` | 49/52 | `test_parse_variant_with_info`, `test_parse_variant_all_fields_present`, `test_parse_default_impl` | ✅ |
| `discovery.rs` | 67/70 | `test_mark_failed_nonexistent_endpoint`, `test_weighted_round_robin_single_endpoint` | ✅ |
| `metrics.rs` | 55/59 | `test_record_handshake_*`, `test_carbon_intensity_*`, `test_record_energy_impact_precision`, `test_update_deferred_jobs_*` | ✅ |
| `ebpf/loader.rs` | 20/23 | `test_unload_without_load`, `test_loader_full_cycle`, `test_shared_loader_operations` | ✅ |
| `ebpf/metrics.rs` | 59/62 | `test_request_data_with_all_fields`, `test_finish_request_with_block_and_memory`, `test_metrics_concurrent_requests` | ✅ |

---

## Mevcut Coverage Durumu

|| Tested/Total Lines:
|| crates/crypto/src/attestation.rs: 171/217
|| crates/crypto/src/certmanager.rs: 125/138
|| crates/crypto/src/cipher.rs: 65/65
|| crates/crypto/src/hybrid_kex.rs: 93/93
|| crates/crypto/src/mtls.rs: 99/134
|| crates/crypto/src/signing.rs: 246/296
|| crates/crypto/src/stream.rs: 99/111
|| crates/crypto/src/tls.rs: 48/53
|| crates/energy/src/cache.rs: 28/28
|| crates/energy/src/client.rs: 36/52
|| crates/energy/src/types.rs: 13/13
|| crates/genomics/src/alignment.rs: 77/77
|| crates/genomics/src/analytics.rs: 59/59
|| crates/genomics/src/bam_parser.rs: 85/92
|| crates/genomics/src/schema.rs: 37/37
|| crates/genomics/src/variant.rs: 67/67
|| crates/genomics/src/vcf_parser.rs: 49/52 → 52/52 (hedef)
|| crates/plugins/src/engine.rs: 48/48
|| crates/plugins/src/interface.rs: 27/27
|| crates/plugins/src/registry.rs: 64/65 → 65/65 (hedef)
|| crates/proxy/src/bootstrap.rs: 0/25
|| crates/proxy/src/carbon_router.rs: 73/86
|| crates/proxy/src/config.rs: 154/162
|| crates/proxy/src/discovery.rs: 67/70 → 70/70 (hedef)
|| crates/proxy/src/dual_stack_server.rs: 56/64
|| crates/proxy/src/green_wait.rs: 108/125
|| crates/proxy/src/health_server.rs: 53/67
|| crates/proxy/src/http3_handler.rs: 59/61 → 61/61 (hedef)
|| crates/proxy/src/http_proxy.rs: 31/73
|| crates/proxy/src/lifecycle.rs: 88/102
|| crates/proxy/src/metrics.rs: 55/59 → 59/59 (hedef)
|| crates/proxy/src/pqc_server.rs: 45/77
|| crates/proxy/src/quic_server.rs: 70/110
|| crates/proxy/src/server.rs: 19/29
|| crates/proxy/src/tracing_otel.rs: 59/59
|| crates/proxy/tests/health_server_integration.rs: 3/3
|| crates/proxy/tests/pqc_server_integration.rs: 3/3
|| crates/proxy/tests/proxy_flow.rs: 5/5
|| crates/proxy/tests/quic_server_integration.rs: 8/8
|| crates/proxy/tests/server_integration.rs: 3/3
|| crates/telemetry/src/ebpf/loader.rs: 20/23 → 23/23 (hedef)
|| crates/telemetry/src/ebpf/metrics.rs: 59/62 → 62/62 (hedef)
|| crates/telemetry/src/ebpf/mod.rs: 2/2
|| crates/telemetry/src/energy.rs: 36/36
|| crates/telemetry/src/estimator.rs: 58/60 → 60/60 (hedef)
|| crates/telemetry/src/prometheus.rs: 27/27
|| 86.30% coverage, 2697/3125 lines covered

## Sonraki Hedefler
- [ ] `crates/crypto/src/attestation.rs` - 46 satır eksik
- [ ] `crates/crypto/src/signing.rs` - 50 satır eksik
- [ ] `crates/crypto/src/mtls.rs` - 35 satır eksik
- [ ] `crates/proxy/src/http_proxy.rs` - 42 satır eksik
- [ ] `crates/proxy/src/quic_server.rs` - 40 satır eksik
