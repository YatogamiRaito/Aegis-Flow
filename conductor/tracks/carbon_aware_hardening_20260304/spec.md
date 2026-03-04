# Track Specification: Carbon-Aware Hardening (v0.32.0)

## Goal
Carbon-Aware Traffic Routing skorunu 8.8/10 → 10/10'a çıkarmak. Retry, forecast, persistence, integration test ve benchmark hatalarını düzeltmek.

---

## Functional Requirements

### FR-1: Retry/Backoff Stratejisi (P0)
- **FR-1.1:** Energy API client'larına `tower::retry` veya custom exponential backoff ekle.
- **FR-1.2:** `RateLimitExceeded.retry_after_seconds` değeri otomatik bekleme süresi olarak kullanılmalı.
- **FR-1.3:** Max retry: 3, base delay: 100ms, max delay: 30s.

### FR-2: Carbon Forecast API Desteği (P0)
- **FR-2.1:** `EnergyApiClient` trait'ine `get_carbon_forecast()` metodu ekle.
- **FR-2.2:** WattTime ve ElectricityMaps forecast endpoint'leri implement edilmeli.
- **FR-2.3:** `GreenWaitScheduler` predictive scheduling: "en yakın yeşil pencere ne zaman?" hesaplaması.
- **FR-2.4:** Forecast data `CarbonIntensityCache`'e entegre edilmeli.

### FR-3: Queue Persistence (P1)
- **FR-3.1:** `GreenWaitScheduler` queue'su disk-backed olmalı (sled/redb veya SQLite).
- **FR-3.2:** Process restart sonrası deferred job'lar recover edilebilmeli.
- **FR-3.3:** In-memory fast path korunmalı (hot cache + disk fallback).

### FR-4: Cross-Module Integration Test (P1)
- **FR-4.1:** `CarbonRouter` → `GreenWait` end-to-end akışı test edilmeli.
- **FR-4.2:** Senaryo: region kirli → job defer → intensity düşer → job execute.

### FR-5: Benchmark Genişletme (P2)
- **FR-5.1:** `select_greenest_region` 100 region ile benchmark.
- **FR-5.2:** `process_ready_jobs` 1000 job throughput benchmark.
- **FR-5.3:** `get_routing_weight` latency benchmark.

---

## Non-Functional Requirements
- NFR-1: Retry backoff toplam timeout <60s.
- NFR-2: Forecast API coverage: en az 24 saat ilerisine tahmin.
- NFR-3: Queue persistence seek latency <1ms.
- NFR-4: >95% test coverage.
- NFR-5: `cargo clippy` sıfır uyarı.

## Acceptance Criteria
1. Rate limit sonrası otomatik retry çalışıyor.
2. Forecast API cevabıyla GreenWait predictive scheduling yapıyor.
3. Process kill → restart sonrası queue recover ediliyor.
4. CarbonRouter→GreenWait integration test geçiyor.
5. Tüm yeni benchmark'lar çalışıyor.
