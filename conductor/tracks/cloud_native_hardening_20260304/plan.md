# Track Plan: Cloud Native Integration Hardening (v0.31.0)

## Phase 1: xDS Protocol Altyapısı (P0)
- [ ] Task: gRPC server altyapısı kurulumu
    - [ ] `Cargo.toml`'e `tonic`, `prost`, `prost-types` bağımlılıklarını ekle
    - [ ] proto dosyaları: `proto/envoy/api/v3/` altında CDS/LDS/RDS .proto dosyaları
    - [ ] `build.rs` ile protobuf derleme pipeline'ı
    - [ ] Verify: `cargo build -p aegis-proxy` — proto derleme başarılı
- [ ] Task: Snapshot Cache implementasyonu (TDD)
    - [ ] Test: `test_snapshot_cache_set_get` — config snapshot kaydedip alabilmeli
    - [ ] Test: `test_snapshot_versioning` — yeni snapshot eski versiyonu geçersiz kılmalı
    - [ ] Implement: `xds/snapshot.rs` — version-based config snapshot cache
    - [ ] Verify: `cargo test -p aegis-proxy` — snapshot testleri geçmeli
- [ ] Task: LDS (Listener Discovery Service) implementasyonu (TDD)
    - [ ] Test: `test_lds_stream_response` — stream bağlantısında doğru listener config dönmeli
    - [ ] Test: `test_lds_version_update` — versiyon değişikliğinde güncelleme gönderilmeli
    - [ ] Implement: `xds/lds.rs` — gRPC stream handler
- [ ] Task: CDS (Cluster Discovery Service) implementasyonu (TDD)
    - [ ] Test: `test_cds_cluster_list` — registered cluster'ları listeleme
    - [ ] Implement: `xds/cds.rs` — cluster endpoint tanımlama
- [ ] Task: RDS (Route Discovery Service) implementasyonu (TDD)
    - [ ] Test: `test_rds_route_matching` — route konfigürasyonu doğru dönmeli
    - [ ] Implement: `xds/rds.rs` — routing kuralları
- [ ] Task: ADS (Aggregated Discovery Service) entegrasyonu
    - [ ] Test: `test_ads_multiplex` — tek stream üzerinden LDS+CDS+RDS
    - [ ] Implement: `xds/ads.rs` — aggregated stream multiplexer
- [ ] Task: Conductor - User Manual Verification 'xDS Protocol Altyapısı' (Protocol in workflow.md)

## Phase 2: OpenTelemetry SDK Entegrasyonu (P0)
- [ ] Task: OTel crate'lerini workspace'e ekle
    - [ ] `Cargo.toml` workspace deps: `opentelemetry = "0.27"`, `opentelemetry-otlp`, `tracing-opentelemetry`
    - [ ] `crates/proxy/Cargo.toml`'e optional `otel` feature flag ekle
- [ ] Task: OTLP Exporter setup (TDD)
    - [ ] Test: `test_otel_tracer_init` — tracer başarıyla başlatılıyor
    - [ ] Test: `test_otel_span_creation` — span oluşturulup export queue'ya giriyor
    - [ ] Implement: `otel.rs` — `init_tracer()` fonksiyonu, OTLP gRPC/HTTP exporter
    - [ ] Implement: `OTEL_EXPORTER_OTLP_ENDPOINT` env var desteği
- [ ] Task: tracing-opentelemetry Layer entegrasyonu (TDD)
    - [ ] Test: `test_tracing_spans_exported_as_otel` — `tracing::info_span!()` otomatik OTel span oluşturmalı
    - [ ] Implement: `tracing_subscriber` stack'ine `OpenTelemetryLayer` ekle
    - [ ] Verify: mevcut `tracing` span'ları OTel'e bridge ediliyor
- [ ] Task: W3C + B3 Propagation (TDD)
    - [ ] Test: `test_w3c_propagation_roundtrip` — traceparent header'dan context extract/inject
    - [ ] Test: `test_b3_propagation_roundtrip` — X-B3-TraceId header desteği
    - [ ] Implement: `opentelemetry::propagation::TextMapCompositePropagator` kullan
- [ ] Task: Sampling konfigürasyonu
    - [ ] Test: `test_sampling_always_on` — %100 sampling
    - [ ] Test: `test_sampling_ratio` — `OTEL_TRACES_SAMPLER=traceidratio` desteği
    - [ ] Implement: sampler config env var'dan okunmalı
- [ ] Task: Mevcut `tracing_otel.rs` backward compat
    - [ ] `tracing_otel.rs`'deki custom `TraceContext` struct'ını `#[deprecated]` yap
    - [ ] Migration notları ekle
- [ ] Task: Conductor - User Manual Verification 'OpenTelemetry SDK Entegrasyonu' (Protocol in workflow.md)

## Phase 3: Grafana Dashboard Provisioning (P1)
- [ ] Task: Grafana dashboard JSON tanımları oluştur
    - [ ] `deploy/grafana/aegis-overview.json` — Request Rate, Error Rate, P95 Latency
    - [ ] `deploy/grafana/aegis-pqc.json` — PQC Handshake Duration, Success Rate
    - [ ] `deploy/grafana/aegis-connections.json` — Active Connections, Bytes In/Out
    - [ ] Dashboard'ların `aegis_` metric namespace'i ile uyumlu olduğunu doğrula
- [ ] Task: Helm chart'a Grafana ConfigMap template ekle
    - [ ] `templates/grafana-dashboards-configmap.yaml` oluştur
    - [ ] `values.yaml`'a `grafana.dashboards.enabled` flag ekle
    - [ ] Verify: `helm template` output'unda ConfigMap görünmeli
- [ ] Task: Conductor - User Manual Verification 'Grafana Dashboard Provisioning' (Protocol in workflow.md)

## Phase 4: Helm Chart Düzeltmeleri (P2)
- [ ] Task: Port tutarsızlığı düzeltmesi
    - [ ] `values.yaml`: `podAnnotations.prometheus.io/port` → `{{ .Values.service.metricsPort }}` template'e taşı
    - [ ] veya sabit değeri `metricsPort` ile eşitle
    - [ ] Verify: `helm template` ile rendered annotation doğru port'u içermeli
- [ ] Task: `image.tag` / `appVersion` senkronizasyonu
    - [ ] `deployment.yaml`'da `image.tag` default olarak `.Chart.AppVersion` kullansın
    - [ ] `Chart.yaml`'da `appVersion` güncelle
- [ ] Task: CI Helm lint
    - [ ] `.github/workflows/` veya CI config'e `helm lint deploy/helm/aegis-flow` step'i ekle
    - [ ] Verify: `helm lint deploy/helm/aegis-flow` temiz geçmeli
- [ ] Task: Conductor - User Manual Verification 'Helm Chart Düzeltmeleri' (Protocol in workflow.md)

## Phase 4.5: Kusursuzluk Fazı — 8-9/10 Alanları 10/10'a Çıkarma

### Prometheus Metrics (9→10)
- [ ] Task: Custom histogram bucket'lar (TDD)
    - [ ] Test: `test_latency_histogram_buckets` — bucket sınırları [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0] olmalı
    - [ ] Implement: `PrometheusBuilder::set_buckets_for_metric()` ile her histogram'a özel bucket tanımla
    - [ ] PQC handshake histogram'ı: [0.0001, 0.0005, 0.001, 0.002, 0.005, 0.01] (µs hassasiyetinde)
- [ ] Task: `/metrics` endpoint integration test (TDD)
    - [ ] Test: `test_metrics_endpoint_returns_prometheus_format` — HTTP GET `/metrics` çağrısı text/plain döndürmeli
    - [ ] Test: `test_metrics_endpoint_contains_all_registered_metrics` — 18 metric adının tamamı çıktıda olmalı
    - [ ] Test: `test_metrics_scrape_after_request` — request sonrası `aegis_requests_total` artmış olmalı
    - [ ] Implement: integration test helper — küçük HTTP server başlatıp scrape et

### Service Discovery (8→10)
- [ ] Task: Async DNS Resolver entegrasyonu (TDD)
    - [ ] `Cargo.toml`'e `hickory-resolver = "0.24"` ekle
    - [ ] Test: `test_dns_resolve_localhost` — `localhost` çözümlemesi başarılı
    - [ ] Test: `test_dns_resolve_with_ttl_cache` — TTL süresi dolmadan cache'ten dönmeli
    - [ ] Test: `test_dns_resolve_failure_graceful` — çözümlenemez domain hatasız None döndürmeli
    - [ ] Implement: `DnsResolver` struct — `hickory-resolver` async API wrapper
    - [ ] Implement: TTL-based caching (`DnsCache` struct, `HashMap<String, (Vec<SocketAddr>, Instant)>`)
- [ ] Task: Background health check task (TDD)
    - [ ] Test: `test_health_checker_marks_unhealthy` — TCP connect başarısız olursa endpoint unhealthy
    - [ ] Test: `test_health_checker_restores_healthy` — düzelince tekrar healthy
    - [ ] Test: `test_health_check_interval_configurable` — interval parametresi çalışmalı
    - [ ] Implement: `HealthChecker` struct — `tokio::spawn` ile periyodik TCP/HTTP health check
    - [ ] Implement: `ServiceRegistry::with_health_checker(interval: Duration)` entegrasyonu

### Helm Chart (8→10)
- [ ] Task: Helm test hook (TDD)
    - [ ] `templates/tests/test-connection.yaml` oluştur — `helm test` ile k8s'de connectivity kontrolü
    - [ ] Verify: `helm template --show-only templates/tests/test-connection.yaml` geçerli YAML üretmeli
- [ ] Task: RBAC least-privilege audit
    - [ ] `templates/role.yaml` ve `rolebinding.yaml` oluştur (henüz yoksa)
    - [ ] Minimum izinler: `get/list/watch` sadece `configmaps`, `services`, `endpoints` 
    - [ ] Verify: `helm template` output'unda Role doğru izinlerle görünmeli
- [ ] Task: `values.schema.json` — Helm values validation
    - [ ] JSON Schema dosyası oluştur, tüm values alanlarının tiplerini ve required'larını tanımla
    - [ ] Verify: `helm lint --strict deploy/helm/aegis-flow` geçmeli

- [ ] Task: Conductor - User Manual Verification 'Kusursuzluk Fazı' (Protocol in workflow.md)

## Phase 5: Final Validation & Release (v0.31.0)
- [ ] Task: Kapsamlı test suite
    - [ ] `cargo test -p aegis-proxy` — tüm testler geçmeli
    - [ ] `cargo clippy -p aegis-proxy --all-targets --all-features` — sıfır uyarı
    - [ ] `helm lint deploy/helm/aegis-flow` — temiz
- [ ] Task: Coverage raporu
    - [ ] `cargo tarpaulin -p aegis-proxy` — >90% coverage hedefi
- [ ] Task: Release v0.31.0
    - [ ] Version bump, CHANGELOG, git tag
    - [ ] SBOM generate
- [ ] Task: Conductor - User Manual Verification 'Final Validation & Release' (Protocol in workflow.md)
