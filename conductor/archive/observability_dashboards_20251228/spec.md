# Specification: Prometheus/Grafana Dashboard Expansion

## Overview
Expand the observability stack with comprehensive Grafana dashboards for monitoring Aegis-Flow in production. Create pre-built dashboards for security, performance, energy, and operational metrics.

## Functional Requirements

### Dashboard Categories

#### 1. Security Dashboard
- PQC handshake success/failure rates
- Key exchange algorithm distribution
- Certificate expiration warnings
- mTLS authentication metrics
- Failed authentication attempts (geo-mapped)

#### 2. Performance Dashboard
- Request latency percentiles (p50, p95, p99)
- Throughput (RPS) per endpoint
- HTTP/2 vs HTTP/3 protocol distribution
- Connection pool utilization
- Memory and CPU usage

#### 3. Energy & Carbon Dashboard
- Real-time carbon intensity by region
- Energy consumption per request
- Green-Wait queue depth and wait times
- Deferred jobs count
- Carbon savings estimation

#### 4. Operational Dashboard
- Service discovery status
- Health check pass/fail rates
- Active connections count
- Error rates by type
- Plugin execution metrics

### Alerting Rules
- High latency alerts (p99 > threshold)
- Certificate expiration < 30 days
- Carbon intensity spike
- Error rate > 5%

## Non-Functional Requirements
- Dashboards loadable via Grafana provisioning
- JSON export for version control
- Light/dark theme support
- Mobile-responsive panels

## Acceptance Criteria
- [x] All 4 dashboard categories created
- [x] Alerting rules documented
- [x] Dashboards work with provided Docker Compose
- [x] README with screenshots

## Out of Scope
- Custom Grafana plugins
- Log aggregation (Loki integration)
- Distributed tracing UI (Jaeger/Tempo)
