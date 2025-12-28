# Test Coverage Tracking

## 📊 Coverage Raporu (29 Aralık 2025)

**Toplam Coverage:** 88.42% (2764/3126 satır)

---

## 🎯 Coverage Durumu

### ✅ 100% Coverage (Tam Kapsama)
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

### � Yüksek Kapsama (>90%)
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `crypto/tls.rs` | 52/53 | 1 | 98.1% |
| `crypto/attestation.rs` | 208/217 | 9 | 95.9% |
| `proxy/http3_handler.rs` | 59/61 | 2 | 96.7% |
| `proxy/config.rs` | 154/162 | 8 | 95.1% |
| `genomics/vcf_parser.rs` | 49/52 | 3 | 94.2% |
| `genomics/bam_parser.rs` | 86/92 | 6 | 93.5% |
| `proxy/discovery.rs` | 67/70 | 3 | 95.7% |
| `proxy/metrics.rs` | 55/59 | 4 | 93.2% |
| `crypto/certmanager.rs` | 129/138 | 9 | 93.5% |

### 🟠 Orta Kapsama (70-90%)
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `crypto/stream.rs` | 99/111 | 12 | 89.2% |
| `proxy/lifecycle.rs` | 88/102 | 14 | 86.3% |
| `proxy/carbon_router.rs` | 73/86 | 13 | 84.9% |
| `proxy/dual_stack_server.rs` | 56/64 | 8 | 87.5% |
| `proxy/green_wait.rs` | 108/125 | 17 | 86.4% |
| `crypto/signing.rs` | 246/296 | 50 | 83.1% |
| `crypto/mtls.rs` | 109/135 | 26 | 80.7% |
| `proxy/health_server.rs` | 53/67 | 14 | 79.1% |

### 🔴 Düşük Kapsama (<70%)
| Dosya | Satır | Eksik | Coverage |
|-------|-------|-------|----------|
| `proxy/server.rs` | 19/29 | 10 | 65.5% |
| `proxy/quic_server.rs` | 70/110 | 40 | 63.6% |
| `proxy/pqc_server.rs` | 45/77 | 32 | 58.4% |
| `proxy/http_proxy.rs` | 31/73 | 42 | 42.5% |
| `telemetry/ebpf/loader.rs` | 20/23 | 3 | 87.0% |
| `telemetry/ebpf/metrics.rs` | 59/62 | 3 | 95.2% |
| `telemetry/estimator.rs` | 58/60 | 2 | 96.7% |
| **`proxy/bootstrap.rs`** | **0/25** | **25** | **0%** ❌ |

---

## 🎯 Öncelikli Hedefler

### 1. Kritik: `proxy/bootstrap.rs` (0% → 100%)
- 25 satır hiç test edilmemiş
- Bootstrap fonksiyonu test edilmeli

### 2. Yüksek: `proxy/http_proxy.rs` (42.5% → 80%+)
- 42 satır eksik
- Request handling testleri gerekli

### 3. Yüksek: `proxy/pqc_server.rs` (58.4% → 80%+)
- 32 satır eksik
- Handshake error paths

### 4. Orta: `proxy/quic_server.rs` (63.6% → 80%+)
- 40 satır eksik
- Stream processing testleri

### 5. Orta: `crypto/signing.rs` (83.1% → 95%+)
- 50 satır eksik
- HybridVerifier ve edge cases

### 6. Orta: `crypto/mtls.rs` (80.7% → 95%+)
- 26 satır eksik
- mTLS handshake error paths

---

## 📈 Hedef Coverage

| Seviye | Hedef |
|--------|-------|
| Mevcut | 88.42% |
| Kısa Vadeli | 92%+ |
| Orta Vadeli | 95%+ |
| Uzun Vadeli | 98%+ |

---

## 🔧 Sonraki Adımlar

1. **`bootstrap.rs`** - Temel testler ekle (0% → 50%+)
2. **`http_proxy.rs`** - Request handling testleri (42% → 70%+)
3. **`pqc_server.rs`** - Error path testleri (58% → 80%+)
4. **`quic_server.rs`** - Stream testleri (64% → 80%+)
5. **`signing.rs`** - HybridVerifier testleri (83% → 95%+)

---

*Son Güncelleme: 29 Aralık 2025, 00:01 UTC+3*
