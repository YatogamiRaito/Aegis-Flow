# Track Specification: Cloud Native Integration Hardening (v0.31.0)

## 1. Goal

Bu track, "Cloud Native Integration" audit raporunda tespit edilen **3 büyük boşluğu** kapatır: xDS protocol desteği, OpenTelemetry SDK entegrasyonu ve Grafana dashboard provisioning. Ek olarak Helm chart'taki port tutarsızlığını düzeltir. Hedef: Cloud Native skorunu 7.5/10 → 10/10'a çıkarmak.

---

## 2. Functional Requirements

### FR-1: xDS Protocol Desteği (P0 — Yüksek)

- **FR-1.1:** gRPC server altyapısı (`tonic` crate) eklenmeli.
- **FR-1.2:** Listener Discovery Service (LDS) — listener config değişikliklerini yayınlayan endpoint.
- **FR-1.3:** Cluster Discovery Service (CDS) — upstream cluster tanımlarını yayınlayan endpoint.
- **FR-1.4:** Route Discovery Service (RDS) — routing kurallarını yayınlayan endpoint.
- **FR-1.5:** Envoy ADS (Aggregated Discovery Service) desteği — tek gRPC stream üzerinden tüm xDS kaynakları.
- **FR-1.6:** Snapshot-based config versioning — Envoy `go-control-plane` tarzı snapshot cache.
- **FR-1.7:** `envoy.api.v3` protobuf tanımları derlenmiş ve API uyumlu olmalı.

### FR-2: OpenTelemetry SDK Entegrasyonu (P0 — Yüksek)

- **FR-2.1:** `opentelemetry` ve `opentelemetry-otlp` crate'leri eklenmeli.
- **FR-2.2:** `tracing-opentelemetry` ile mevcut `tracing` crate span'larından OTel export yapılmalı.
- **FR-2.3:** Jaeger/OTLP exporter: environment variable (`OTEL_EXPORTER_OTLP_ENDPOINT`) ile konfigüre edilebilmeli.
- **FR-2.4:** W3C Trace Context propagation, mevcut `tracing_otel.rs` custom impl yerine OTel SDK ile yapılmalı.
- **FR-2.5:** B3 propagation desteği eklenebilmeli (spec'te var, mevcut kodda yok).
- **FR-2.6:** Sampling konfigürasyonu: `OTEL_TRACES_SAMPLER` env var desteği.
- **FR-2.7:** Mevcut `tracing_otel.rs` backward-compatible kalmalı veya deprecated olarak işaretlenmeli.

### FR-3: Grafana Dashboard Provisioning (P1 — Orta)

- **FR-3.1:** `deploy/grafana/` dizini altında JSON dashboard dosyaları oluşturulmalı.
- **FR-3.2:** Dashboard'lar: Request Rate, Latency P50/P95/P99, Error Rate, PQC Handshake Duration, Active Connections, Bytes In/Out.
- **FR-3.3:** Helm chart'a Grafana ConfigMap provisioning template'i eklenmeli.
- **FR-3.4:** Dashboard'lar `aegis_` metric namespace'iyle uyumlu olmalı.

### FR-4: Helm Chart Düzeltmeleri (P2 — Düşük)

- **FR-4.1:** `values.yaml`'da `service.metricsPort` (8080) ile `podAnnotations.prometheus.io/port` (9090) tutarsızlığı düzeltilmeli.
- **FR-4.2:** Helm chart'a `helm lint` ve `helm template` CI kontrolü eklenmeli.
- **FR-4.3:** `values.yaml`'daki `image.tag` dinamik hale getirilmeli (Chart.yaml appVersion ile senkron).

---

## 3. Non-Functional Requirements

- **NFR-1:** xDS gRPC endpoint <10ms response latency.
- **NFR-2:** OTel span export <1% CPU overhead (sampling rate %10'da).
- **NFR-3:** Tüm yeni modüller >90% test coverage.
- **NFR-4:** `cargo clippy --all-targets --all-features` sıfır uyarı.
- **NFR-5:** Helm chart `helm lint` temiz geçmeli.
- **NFR-6:** Grafana dashboard'lar import edildiğinde hatasız yüklenmeli.

---

## 4. Acceptance Criteria

1. xDS gRPC server LDS/CDS/RDS endpoint'leri çalışıyor.
2. `opentelemetry-otlp` ile OTLP/Jaeger export çalışıyor.
3. W3C Trace Context ve B3 propagation OTel SDK ile sağlanıyor.
4. Grafana JSON dashboard'lar `deploy/grafana/` altında mevcut.
5. Helm chart port tutarsızlığı düzeltilmiş.
6. Tüm yeni testler geçiyor.

---

## 5. Out of Scope

- Envoy sidecar injection / Istio entegrasyonu (ayrı track).
- Custom Prometheus Operator kurulumu.
- Real Kubernetes cluster integration test (unit + Helm lint yeterli).
