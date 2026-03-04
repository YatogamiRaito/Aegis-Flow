# Track Plan: Carbon-Aware Hardening (v0.32.0)

## Phase 1: Energy API Retry/Backoff (P0)
- [ ] Task: Exponential backoff middleware (TDD)
    - [ ] Test: `test_retry_on_rate_limit` — rate limit sonrası otomatik retry
    - [ ] Test: `test_retry_respects_retry_after` — retry_after_seconds bekleme süresi
    - [ ] Test: `test_max_retries_exceeded` — 3 denemeden sonra hata dönmeli
    - [ ] Test: `test_no_retry_on_auth_error` — auth hatalarında retry yapılmamalı
    - [ ] Implement: `RetryPolicy` struct (max_retries, base_delay_ms, max_delay_ms)
    - [ ] Implement: `EnergyApiClient` wrapper veya `tower::retry::Retry` layer
- [ ] Task: WattTime + ElectricityMaps client'larına retry entegrasyonu
    - [ ] Verify: wiremock testlerini rate limit retry ile güncelle
- [ ] Task: Conductor - User Manual Verification 'Retry/Backoff' (Protocol in workflow.md)

## Phase 2: Carbon Forecast API (P0)
- [ ] Task: `EnergyApiClient` trait genişletme (TDD)
    - [ ] Test: `test_forecast_trait_method_exists` — trait'te `get_carbon_forecast` mevcut
    - [ ] Implement: `get_carbon_forecast(&self, region: &Region, hours: u32) -> Result<Vec<ForecastPoint>, EnergyApiError>`
    - [ ] Implement: `ForecastPoint` struct (timestamp, predicted_intensity, confidence)
- [ ] Task: WattTime forecast endpoint implementasyonu (TDD)
    - [ ] Test: `test_watttime_forecast_24h` — 24 saat forecast döndürmeli
    - [ ] Implement: `/v3/forecast` endpoint çağrısı ve response parsing
- [ ] Task: ElectricityMaps forecast endpoint implementasyonu (TDD)
    - [ ] Test: `test_electricitymaps_forecast` — forecast array döndürmeli
    - [ ] Implement: `/v3/carbon-intensity/forecast` endpoint çağrısı
- [ ] Task: GreenWait predictive scheduling (TDD)
    - [ ] Test: `test_predictive_schedule_selects_greenest_window` — forecast'e göre en iyi pencere seçilmeli
    - [ ] Test: `test_predictive_schedule_respects_max_wait` — priority max_wait'i aşmayacak pencere seçilmeli
    - [ ] Implement: `GreenWaitScheduler::estimate_green_window()` — forecast data ile tahmini execution zamanı
- [ ] Task: Forecast → Cache entegrasyonu
    - [ ] Implement: `CarbonIntensityCache`'e forecast point'leri TTL ile sakla
- [ ] Task: Conductor - User Manual Verification 'Carbon Forecast API' (Protocol in workflow.md)

## Phase 3: Queue Persistence (P1)
- [ ] Task: Disk-backed queue seçimi ve implementasyonu (TDD)
    - [ ] `Cargo.toml`'e `redb = "2"` ekle (embedded key-value store, pure Rust)
    - [ ] Test: `test_persist_job` — job disk'e yazılıp geri okunabilmeli
    - [ ] Test: `test_recover_after_restart` — queue clear + disk'ten recover
    - [ ] Test: `test_memory_and_disk_consistent` — in-memory ve disk senkron olmalı
    - [ ] Implement: `PersistentQueue` struct — redb backend + in-memory VecDeque hot cache
    - [ ] Implement: `DeferredJob` için `Serialize`/`Deserialize` derive
- [ ] Task: GreenWaitScheduler persistence entegrasyonu
    - [ ] Implement: `submit()` → disk'e yaz, `process_ready_jobs()` → disk'ten sil
    - [ ] Verify: mevcut tüm green_wait testleri hâlâ geçmeli
- [ ] Task: Conductor - User Manual Verification 'Queue Persistence' (Protocol in workflow.md)

## Phase 4: Integration Test & Benchmark (P1-P2)
- [ ] Task: CarbonRouter ↔ GreenWait integration test (TDD)
    - [ ] Test: `test_e2e_dirty_region_defers_job` — CarbonRouter kirli region tespit → GreenWait'e defer
    - [ ] Test: `test_e2e_green_window_executes_deferred` — intensity düşünce deferred job execute
    - [ ] Test: `test_e2e_forecast_driven_scheduling` — forecast verisiyle predictive execution
- [ ] Task: Benchmark genişletme
    - [ ] `benches/carbon_router.rs`'e `select_greenest_region_100` benchmark ekle
    - [ ] `benches/green_wait.rs` oluştur: `process_ready_jobs_1000` throughput
    - [ ] `get_routing_weight` latency benchmark
    - [ ] Verify: tüm benchmark'lar compile ve çalışmalı
- [ ] Task: Conductor - User Manual Verification 'Integration Test & Benchmark' (Protocol in workflow.md)

## Phase 5: Final Validation & Release (v0.32.0)
- [ ] Task: Kapsamlı test suite
    - [ ] `cargo test -p aegis-energy -p aegis-proxy` — tüm testler geçmeli
    - [ ] `cargo clippy --all-targets --all-features` — sıfır uyarı
- [ ] Task: Coverage raporu
    - [ ] `cargo tarpaulin -p aegis-energy -p aegis-proxy` — >95% coverage
- [ ] Task: Release v0.32.0
    - [ ] Version bump, CHANGELOG, git tag
- [ ] Task: Conductor - User Manual Verification 'Final Validation & Release' (Protocol in workflow.md)
