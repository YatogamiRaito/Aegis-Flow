# Spec: Performance Benchmark Hardening (v0.34.0)

## Overview

Audit skoru 6.1/10 olan Performance Benchmark Suite track'ini Google 2026 production standardına (10/10) çıkarmak için kapsamlı sertleştirme.

**Temel sorunlar:**
- CI workflow'da benchmark step'i tamamen yok — regression detection çalışmıyor
- Envoy karşılaştırması "estimated" — gerçek ölçüm değil
- Load test'ler in-memory handler çağrıyor, gerçek network I/O yok
- Encryption/decryption, TLS handshake, EncryptedStream benchmark'ları eksik
- P99 latency ölçümü yok (spec istiyor)
- Containerized benchmark environment yok
- RESULTS.md iddialarının doğrulanabilirliği zayıf

## Functional Requirements

1. **CI/CD Benchmark Step**: `cargo bench` CI pipeline'ına eklenmeli, threshold-based regression gate
2. **Real Network Load Testing**: Gerçek TCP/QUIC bağlantısı ile end-to-end RPS ölçümü
3. **Envoy Comparison**: Real Envoy instance ile side-by-side benchmark
4. **Missing Benchmarks**: Encryption, TLS, EncryptedStream, static file, rate limiter, caching
5. **P99/P95 Latency**: HDR histogram ile latency distribution ölçümü
6. **Containerized Environment**: Docker-based reproducible benchmark ortamı
7. **Regression Detection**: Criterion baseline karşılaştırması, threshold alerting
8. **Visualization**: HTML rapor ve CI artifact upload

## Non-Functional Requirements

- Tüm benchmark sonuçları reproducible ve verifiable olmalı
- Claims doğrulanabilir kaynaklara dayalı olmalı
- CI'da benchmark report artifact olarak upload edilmeli
- Seed-based deterministic benchmark'lar tercih edilmeli

## Acceptance Criteria

- [ ] CI'da `cargo bench` minimum bir job olarak çalışıyor
- [ ] Regression detection aktif (criterion baseline threshold)
- [ ] Envoy ile gerçek karşılaştırma (Docker Compose ile side-by-side)
- [ ] Encryption/decryption benchmark var (AES-256-GCM, ChaCha20-Poly1305)
- [ ] TLS handshake benchmark var
- [ ] P99 latency ölçümü var ve < 10ms doğrulanmış
- [ ] Gerçek network I/O ile end-to-end RPS testi
- [ ] Containerized benchmark Dockerfile var
- [ ] RESULTS.md tüm değerler gerçek ölçümleri yansıtıyor (~ prefix yok)
- [ ] Duplicate pqc_handshake benchmark kaldırılmış (tek crate'de)
