# Spec: HTTP/3 & QUIC Protocol Hardening (v0.33.0)

## Overview

Audit skoru 5.2/10 olan HTTP/3 and QUIC Protocol Support track'ini Google 2026 production standardına (10/10) çıkarmak için kapsamlı sertleştirme.

**Temel sorunlar:**
- `h3` crate dependency olarak var ama hiç kullanılmıyor — HTTP/3 binary framing yok
- `max_streams`, `idle_timeout_secs`, `enable_0rtt`, `pqc_enabled` config alanları server builder'a uygulanmıyor
- 0-RTT session resumption implementasyonu yok
- PQC (ML-KEM+X25519) QUIC TLS'e bağlanmamış
- Upstream forwarding stub (404 döndürüyor)
- Alt-Svc header üretiliyor ama HTTP/2 response'lara enjekte edilmiyor

## Functional Requirements

1. **Real HTTP/3 Framing**: `h3` + `h3-quinn` veya `h3-s2n-quic` crate'leri ile gerçek HTTP/3 binary framing
2. **QPACK Header Compression**: HTTP/3 standardına uygun header compression
3. **QUIC Transport Config**: `max_streams`, `idle_timeout`, connection limits server builder'a uygulanmalı
4. **0-RTT Session Resumption**: TLS session ticket cache, anti-replay, ticket rotation
5. **PQC-QUIC Integration**: ML-KEM+X25519 hybrid handshake ile QUIC TLS config
6. **Upstream HTTP Forwarding**: HTTP/3 isteklerini upstream'e forward edebilme
7. **Alt-Svc Injection**: HTTP/2 response'lara otomatik Alt‑Svc header ekleme
8. **QUIC Security**: Amplification attack koruması, retry token, connection flood limiti
9. **Connection Migration**: QUIC connection migration desteği
10. **Flow Control**: Per-stream ve per-connection flow control konfigürasyonu

## Non-Functional Requirements

- HTTP/3 RFC 9114 uyumluluğu
- 1-RTT connection establishment < 50ms
- 0-RTT resumption < 10ms
- Multiplexed stream throughput > 1 Gbps
- QPACK dynamic table ile header compression ratio > 60%
- Fuzz testing ile güvenlik doğrulaması
- `cargo clippy -- -W clippy::pedantic` uyumlu

## Acceptance Criteria

- [ ] `h3` crate gerçek HTTP/3 framing için kullanılıyor
- [ ] QPACK ile header encode/decode çalışıyor
- [ ] Config alanları (`max_streams`, `idle_timeout`, `0rtt`, `pqc`) server'a uygulanıyor
- [ ] 0-RTT session resumption çalışıyor ve anti-replay koruması var
- [ ] ML-KEM+X25519 ile QUIC TLS handshake çalışıyor
- [ ] HTTP/3 istekleri upstream'e forward ediliyor
- [ ] HTTP/2 response'lara Alt-Svc header enjekte ediliyor
- [ ] Amplification attack koruması aktif
- [ ] Connection migration testleri geçiyor
- [ ] Flow control konfigürasyonu çalışıyor
- [ ] Tüm benchmark hedefleri karşılanıyor
- [ ] Fuzz testing tamamlanmış
