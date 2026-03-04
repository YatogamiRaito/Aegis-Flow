# Track Plan: HTTP/3 & QUIC Protocol Hardening (v0.33.0)

## Phase 1: Real HTTP/3 Framing (P0)
- [ ] Task: `h3` crate'i `s2n-quic` adapter'ı ile entegre et (h3-s2n-quic veya custom adapter)
- [ ] Task: `process_stream()` fonksiyonunu `h3::server::Connection` kullanacak şekilde yeniden yaz
- [ ] Task: HTTP/3 HEADERS frame encode/decode (QPACK) implementasyonu
- [ ] Task: HTTP/3 DATA frame handling
- [ ] Task: HTTP/3 SETTINGS frame negotiation
- [ ] Task: HTTP/3 GOAWAY frame (graceful shutdown)
- [ ] Task: Raw text "HTTP/3 200 OK" yanıtlarını binary framing'e dönüştür
- [ ] Task: Unit testleri — frame encoding/decoding, QPACK round-trip (min 15 test)
- [ ] Task: Conductor Verification 'Real HTTP/3 Framing'

## Phase 2: QUIC Transport Configuration (P0)
- [ ] Task: `max_streams` → s2n-quic `Server::builder().with_limits()` ile uygula
- [ ] Task: `idle_timeout_secs` → s2n-quic idle timeout konfigürasyonu
- [ ] Task: Connection-level max concurrent bidirectional/unidirectional stream limitleri
- [ ] Task: Initial flow control window size konfigürasyonu
- [ ] Task: QUIC transport parameter negotiation doğrulaması
- [ ] Task: Unit testleri — her config alanının gerçek etkisi (min 8 test)
- [ ] Task: Conductor Verification 'QUIC Transport Configuration'

## Phase 3: 0-RTT Session Resumption (P0)
- [ ] Task: TLS session ticket store implementasyonu (in-memory + disk-backed)
- [ ] Task: s2n-quic session resumption API entegrasyonu
- [ ] Task: `zero_rtt_connections` stat'ını gerçek 0-RTT bağlantılarında artır
- [ ] Task: Anti-replay koruması (token-based veya timestamp-based)
- [ ] Task: Session ticket rotation (configurable TTL)
- [ ] Task: 0-RTT replay attack koruması testleri
- [ ] Task: 0-RTT latency benchmark (hedef: < 10ms)
- [ ] Task: Unit testleri (min 10 test)
- [ ] Task: Conductor Verification '0-RTT Session Resumption'

## Phase 4: PQC-QUIC Integration (P1)
- [ ] Task: `crates/crypto/src/hybrid_kex.rs` ML-KEM+X25519'u s2n-quic TLS policy olarak bağla
- [ ] Task: s2n-tls custom security policy ile PQC cipher suite konfigürasyonu
- [ ] Task: `pqc_enabled` flag'i gerçek TLS config değişikliğine dönüştür
- [ ] Task: PQC handshake overhead benchmark
- [ ] Task: PQC + 0-RTT uyumluluk testleri
- [ ] Task: Unit testleri (min 6 test)
- [ ] Task: Conductor Verification 'PQC-QUIC Integration'

## Phase 5: Upstream HTTP Forwarding (P0)
- [ ] Task: HTTP/3 request → upstream HTTP/1.1 veya HTTP/2 forwarding
- [ ] Task: Request/response body streaming (chunked transfer)
- [ ] Task: Upstream timeout ve retry konfigürasyonu
- [ ] Task: Request/response header dönüşümü (pseudo-headers → HTTP/1.1)
- [ ] Task: Upstream bağlantı havuzu (connection pooling)
- [ ] Task: Integration testleri — upstream mock server ile end-to-end (min 10 test)
- [ ] Task: Conductor Verification 'Upstream HTTP Forwarding'

## Phase 6: Alt-Svc Injection & Dual-Stack Hardening (P1)
- [ ] Task: HTTP/2 response middleware'ine Alt-Svc header enjeksiyonu ekle
- [ ] Task: Alt-Svc max-age ve versioning konfigürasyonu
- [ ] Task: Partial failure recovery — bir server fail olunca diğerini koruma
- [ ] Task: DualStackStats'ı gerçek HTTP/2 ve HTTP/3 request count'ları ile güncelle
- [ ] Task: HTTP/3 → HTTP/2 graceful degradation testleri
- [ ] Task: Unit testleri (min 6 test)
- [ ] Task: Conductor Verification 'Alt-Svc & Dual-Stack'

## Phase 7: QUIC Security Hardening (P1)
- [ ] Task: QUIC retry token mekanizması (amplification attack koruması)
- [ ] Task: Connection flood limiti (max concurrent connections per IP)
- [ ] Task: `stats.write().await` hot-path optimizasyonu (AtomicU64 veya sharded counter)
- [ ] Task: Rate limiting per-connection ve per-IP
- [ ] Task: Large request drop'unda client'a 413 Payload Too Large dönülmesi
- [ ] Task: Fuzz testing (cargo-fuzz, QUIC frame parser üzerinde)
- [ ] Task: Unit + integration testleri (min 8 test)
- [ ] Task: Conductor Verification 'QUIC Security'

## Phase 8: Performance & Benchmark Suite (P2)
- [ ] Task: Gerçek QUIC roundtrip latency benchmark (1-RTT, 0-RTT)
- [ ] Task: Multiplexed stream throughput benchmark
- [ ] Task: Head-of-line blocking comparison (HTTP/2 vs HTTP/3)
- [ ] Task: Connection migration latency benchmark
- [ ] Task: QPACK compression ratio benchmark
- [ ] Task: Spec hedefi doğrulaması: connection time < 50ms
- [ ] Task: CI'da benchmark regression gate
- [ ] Task: Conductor Verification 'Benchmarks'

## Phase 9: Kusursuzluk Fazı (P2)
- [ ] Task: QUIC connection migration implementasyonu
- [ ] Task: QUIC version negotiation (v1 + v2)
- [ ] Task: Stream priority (RFC 9218 Extensible Priorities)
- [ ] Task: QUIC datagram extension (RFC 9221) desteği
- [ ] Task: ECN (Explicit Congestion Notification) desteği
- [ ] Task: Logging — yapısal QUIC event logging (connection, stream, loss, rtt)
- [ ] Task: Documentation — migration guide, API reference, architecture diagrams
- [ ] Task: proptest — property-based testing (frame invariants)
- [ ] Task: Conductor Verification 'Perfection Phase'

## Phase 10: Final Validation & Release
- [ ] Task: `cargo clippy -- -W clippy::pedantic` clean
- [ ] Task: `cargo fmt --check` clean
- [ ] Task: `cargo audit` clean
- [ ] Task: `cargo deny check` clean  
- [ ] Task: tarpaulin coverage > 90%
- [ ] Task: Full test suite pass
- [ ] Task: Documentation update (CHANGELOG, README)
- [ ] Task: Release v0.33.0
- [ ] Task: Conductor Verification 'Release v0.33.0'
