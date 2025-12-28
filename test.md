# Test Coverage Tracking

## 📊 Coverage Raporu (29 Aralık 2025)

**Toplam Coverage:** 88.42% (2764/3126 satır)

---

## 🎯 Coverage Durumu

### ✅ 100% Coverage (Tam Kapsama) - 15 Dosya
| Dosya | Satır |
|-------|-------|
| `crypto/cipher.rs` | 65/65 ✅ |
| `crypto/hybrid_kex.rs` | 93/93 ✅ |
| `energy/cache.rs` | 28/28 ✅ |
| `energy/types.rs` | 13/13 ✅ |
| `genomics/alignment.rs` | 77/77 ✅ |
| `genomics/analytics.rs` | 59/59 ✅ |
| `genomics/schema.rs` | 37/37 ✅ |
| `genomics/variant.rs` | 67/67 ✅ |
| `plugins/engine.rs` | 48/48 ✅ |
| `plugins/interface.rs` | 27/27 ✅ |
| `plugins/registry.rs` | 65/65 ✅ |
| `proxy/tracing_otel.rs` | 59/59 ✅ |
| `telemetry/energy.rs` | 36/36 ✅ |
| `telemetry/prometheus.rs` | 27/27 ✅ |
| `telemetry/ebpf/mod.rs` | 2/2 ✅ |

### 🟢 Yüksek Kapsama (>90%) - 9 Dosya
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `crypto/tls.rs` | 52/53 | 1 | 98.1% |
| `proxy/http3_handler.rs` | 59/61 | 2 | 96.7% |
| `telemetry/estimator.rs` | 58/60 | 2 | 96.7% |
| `telemetry/ebpf/metrics.rs` | 59/62 | 3 | 95.2% |
| `crypto/attestation.rs` | 208/217 | 9 | 95.9% |
| `proxy/config.rs` | 154/162 | 8 | 95.1% |
| `proxy/discovery.rs` | 67/70 | 3 | 95.7% |
| `genomics/vcf_parser.rs` | 49/52 | 3 | 94.2% |
| `proxy/metrics.rs` | 55/59 | 4 | 93.2% |

### � Orta Kapsama (70-90%) - 11 Dosya
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `genomics/bam_parser.rs` | 86/92 | 6 | 93.5% |
| `crypto/certmanager.rs` | 129/138 | 9 | 93.5% |
| `crypto/stream.rs` | 99/111 | 12 | 89.2% |
| `telemetry/ebpf/loader.rs` | 20/23 | 3 | 87.0% |
| `proxy/dual_stack_server.rs` | 56/64 | 8 | 87.5% |
| `proxy/lifecycle.rs` | 88/102 | 14 | 86.3% |
| `proxy/green_wait.rs` | 108/125 | 17 | 86.4% |
| `proxy/carbon_router.rs` | 73/86 | 13 | 84.9% |
| `crypto/signing.rs` | 246/296 | 50 | 83.1% |
| `crypto/mtls.rs` | 109/135 | 26 | 80.7% |
| `proxy/health_server.rs` | 53/67 | 14 | 79.1% |

### 🔴 Düşük Kapsama (<70%) - 6 Dosya
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `proxy/server.rs` | 19/29 | 10 | 65.5% |
| `proxy/quic_server.rs` | 70/110 | 40 | 63.6% |
| `proxy/pqc_server.rs` | 45/77 | 32 | 58.4% |
| `energy/client.rs` | 46/52 | 6 | 88.5% |
| `proxy/http_proxy.rs` | 31/73 | 42 | 42.5% |
| **`proxy/bootstrap.rs`** | **0/25** | **25** | **0%** ❌ |

---

## 📈 Özet

- **Toplam Dosya:** 41  
- **100% Coverage:** 15 dosya  
- **>90% Coverage:** 24 dosya  
- **<70% Coverage:** 6 dosya

---

## 🔧 100% Yapılabilecek Dosyalar

1. **`crypto/tls.rs`** (1 satır eksik) - En kolay
2. **`proxy/http3_handler.rs`** (2 satır eksik)
3. **`telemetry/estimator.rs`** (2 satır eksik)
4. **`telemetry/ebpf/metrics.rs`** (3 satır eksik)
5. **`telemetry/ebpf/loader.rs`** (3 satır eksik)
6. **`proxy/discovery.rs`** (3 satır eksik)

---

*Son Güncelleme: 29 Aralık 2025, 00:15 UTC+3*
