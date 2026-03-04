# Implementation Plan: Auto HTTPS & On-Demand TLS Hardening (v0.48.0)

## Phase 1: Custom ClientHello Peeker
- [ ] Task: Extract SNI asynchronously
    - [ ] Create `tls_peeker.rs` utility.
    - [ ] Read the first TLS record from the TCP stream without consuming it, parse the Server Name Indication (SNI) extension.
    - [ ] Once SNI is extracted, invoke `acme_manager.ensure_cert(sni).await`.

## Phase 2: Async TLS Acceptor Refactor
- [ ] Task: Replace `ResolvesServerCert` with Pre-Handshake Loading
    - [ ] In `crates/proxy/src/bootstrap.rs` and the HTTP server runner, modify the loop that accepts TCP streams.
    - [ ] Route the accepted stream through the custom peeker, wait for ACME module, then build a `rustls::ServerConfig` on the fly for that connection OR fetch from an updated thread-safe `RwLock<Arc<ServerConfig>>`.

## Phase 3: Implement Ask Endpoint and Rate Limiting
- [ ] Task: Restrict On-Demand Issuance
    - [ ] In `acme.rs`, before initiating an `issue_cert` call, check the `tokio` rate limiter `max certs per hour`.
    - [ ] If an `ask` URL is provided, perform an HTTP GET using `reqwest`.
    - [ ] Abort the TLS handshake if the ask endpoint does not return 200 OK or rate limit is exceeded.

## Phase 4: DNS-01 Provider Refactor
- [ ] Task: Abstract DNS-01 logic
    - [ ] Define `pub trait DnsProvider { async fn create_txt_record(...); fn delete_txt_record(...); }`
    - [ ] Move Cloudflare logic to `cloudflare_dns.rs`.
    - [ ] Add `route53_dns.rs` communicating via AWS API.
    - [ ] Wire the selected provider into `issue_cert` dynamically based on configuration.
