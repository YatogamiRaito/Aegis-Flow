# Track Plan: Performance Benchmark Hardening (v0.34.0)

## Phase 1: CI/CD Benchmark Integration (P0)
- [ ] Task: `.github/workflows/ci.yml`'e benchmark job'u ekle (`cargo bench --no-run` compile check)
- [ ] Task: Ayrı `.github/workflows/bench.yml` workflow oluştur (nightly veya PR merge sırasında)
- [ ] Task: Criterion JSON output'u CI artifact olarak upload et
- [ ] Task: Criterion HTML report'u CI artifact olarak upload et
- [ ] Task: Benchmark regression threshold gate (> %10 slowdown → CI fail)
- [ ] Task: `criterion-compare` veya `critcmp` ile baseline karşılaştırma
- [ ] Task: Conductor Verification 'CI Benchmark Integration'

## Phase 2: Missing Micro-Benchmarks (P0)
- [ ] Task: AES-256-GCM encrypt/decrypt benchmark (crates/crypto/benches/cipher.rs)
- [ ] Task: ChaCha20-Poly1305 encrypt/decrypt benchmark
- [ ] Task: EncryptedStream read/write throughput benchmark
- [ ] Task: TLS handshake benchmark (rustls)
- [ ] Task: Static file serving benchmark (çeşitli dosya boyutları)
- [ ] Task: Rate limiter (token bucket) benchmark
- [ ] Task: Caching layer (hit/miss/eviction) benchmark
- [ ] Task: Config parsing (AegisFile, TOML, YAML) benchmark
- [ ] Task: Duplicate pqc_handshake benchmark'ı kaldır (proxy'den sil, crypto'da tut)
- [ ] Task: Conductor Verification 'Missing Benchmarks'

## Phase 3: Real Network Load Testing (P0)
- [ ] Task: End-to-end HTTP load test binary oluştur (gerçek TCP bağlantısı)
- [ ] Task: End-to-end QUIC load test (gerçek UDP)
- [ ] Task: Connection concurrency limiti testi (1K, 5K, 10K concurrent connections)
- [ ] Task: Stream multiplexing efficiency benchmark
- [ ] Task: P99/P95/P50 latency ölçümü (HDR histogram ile)
- [ ] Task: Memory profiling — `dhat` veya `jemalloc-ctl` ile gerçek heap ölçümü
- [ ] Task: Conductor Verification 'Network Load Testing'

## Phase 4: Envoy Comparison (P1)
- [ ] Task: Docker Compose ile Envoy + Aegis-Flow side-by-side ortam
- [ ] Task: wrk2 veya vegeta ile standardize edilmiş load generation
- [ ] Task: Memory footprint karşılaştırması (container cgroup'dan)
- [ ] Task: Latency distribution karşılaştırması (P50/P90/P99/P999)
- [ ] Task: Throughput (RPS) karşılaştırması (eşit hardware'de)
- [ ] Task: CPU utilization karşılaştırması
- [ ] Task: Karşılaştırma sonuçlarını `docs/benchmarks/COMPARISON.md` olarak yaz
- [ ] Task: Conductor Verification 'Envoy Comparison'

## Phase 5: Containerized Benchmark Environment (P1)
- [ ] Task: `Dockerfile.bench` oluştur (deterministic benchmark ortamı)
- [ ] Task: Hardware pinning (CPU affinity, memory limits) desteği
- [ ] Task: Seed-based random generation (reproducible results)
- [ ] Task: Benchmark runner script (`scripts/run_benchmarks.sh`)
- [ ] Task: Conductor Verification 'Containerized Environment'

## Phase 6: RESULTS.md Accuracy (P1)
- [ ] Task: Tüm `~` prefix'li değerleri gerçek ölçümlerle değiştir
- [ ] Task: Envoy "estimated" satırlarını gerçek veya "N/A — comparison pending" olarak güncelle
- [ ] Task: Test environment bilgilerini CI'dan otomatik doldur
- [ ] Task: Benchmark methodology bölümü ekle
- [ ] Task: Sonuç yorumlama rehberi ekle
- [ ] Task: Conductor Verification 'RESULTS.md Accuracy'

## Phase 7: Kusursuzluk Fazı (P2)
- [ ] Task: Flamegraph entegrasyonu (`cargo flamegraph`)
- [ ] Task: Benchmark result history tracking (JSON log, grafikler)
- [ ] Task: HAProxy baseline karşılaştırması
- [ ] Task: Energy consumption benchmark (RAPL veya eBPF ile)
- [ ] Task: Cross-platform benchmark (Linux ARM64 + x86_64)
- [ ] Task: README performance badges güncelle (doğrulanmış değerlerle)
- [ ] Task: Conductor Verification 'Perfection Phase'

## Phase 8: Final Validation & Release
- [ ] Task: Tüm benchmark'lar CI'da başarıyla çalışıyor
- [ ] Task: Regression detection aktif
- [ ] Task: RESULTS.md tüm iddialar doğrulanmış
- [ ] Task: Documentation update (CHANGELOG)
- [ ] Task: Release v0.34.0
- [ ] Task: Conductor Verification 'Release v0.34.0'
