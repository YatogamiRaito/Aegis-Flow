# Implementation Plan: Automatic HTTPS & Certificate Management (v0.23.0)

## Phase 1: ACME Client Foundation

- [ ] Task: Create certificate management crate (`crates/certman/`)
    - [ ] Define crate structure: `lib.rs`, `acme.rs`, `challenge.rs`, `storage.rs`, `ocsp.rs`, `dns_providers/`
    - [ ] Add dependencies: reqwest, serde, ring (EC key gen), base64, pem, x509-parser
    - [ ] Add crate to workspace Cargo.toml

- [ ] Task: Implement ACME account management (`acme.rs`)
    - [ ] Write tests for EC P-256 key pair generation
    - [ ] Implement account key generation and persistence
    - [ ] Write tests for ACME directory discovery (fetch endpoints from CA)
    - [ ] Implement directory fetching and parsing
    - [ ] Write tests for new account registration with ToS agreement
    - [ ] Implement account registration (newAccount endpoint)
    - [ ] Write tests for JWS (JSON Web Signature) message signing for ACME requests
    - [ ] Implement JWS builder with nonce management

- [ ] Task: Implement certificate order flow
    - [ ] Write tests for new order creation (newOrder endpoint)
    - [ ] Implement order creation with domain identifiers
    - [ ] Write tests for authorization retrieval and challenge selection
    - [ ] Implement authorization fetching and challenge parsing
    - [ ] Write tests for order finalization with CSR
    - [ ] Implement CSR generation and order finalization
    - [ ] Write tests for certificate download (PEM chain)
    - [ ] Implement certificate download and chain assembly

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Challenge Solvers

- [ ] Task: Implement HTTP-01 challenge solver
    - [ ] Write tests for challenge token serving at /.well-known/acme-challenge/<token>
    - [ ] Implement temporary HTTP server on port 80 for challenge
    - [ ] Write tests for challenge validation notification to ACME server
    - [ ] Implement challenge response and polling for valid status
    - [ ] Write tests for integration with existing HTTP listener (serve challenge alongside normal traffic)

- [ ] Task: Implement TLS-ALPN-01 challenge solver
    - [ ] Write tests for self-signed cert generation with ACME ALPN extension
    - [ ] Implement TLS-ALPN-01 certificate builder
    - [ ] Write tests for ALPN protocol negotiation ("acme-tls/1")
    - [ ] Implement custom TLS acceptor that responds to ACME ALPN

- [ ] Task: Implement DNS-01 challenge solver framework
    - [ ] Write tests for TXT record name generation (_acme-challenge.<domain>)
    - [ ] Implement DNS challenge token computation (SHA-256 + base64url)
    - [ ] Write tests for Cloudflare DNS provider (create/delete TXT record via API)
    - [ ] Implement CloudflareDnsProvider
    - [ ] Write tests for AWS Route53 provider
    - [ ] Implement Route53DnsProvider
    - [ ] Write tests for DNS provider plugin trait (for custom providers)
    - [ ] Implement DnsProvider trait for extensibility

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Certificate Storage & Lifecycle

- [ ] Task: Implement encrypted certificate storage (`storage.rs`)
    - [ ] Write tests for PEM certificate + key writing to disk
    - [ ] Implement cert storage with configurable path
    - [ ] Write tests for private key encryption at rest (AES-256-GCM)
    - [ ] Implement key encryption/decryption
    - [ ] Write tests for certificate metadata persistence (issuer, expiry, domains)
    - [ ] Implement metadata JSON storage
    - [ ] Write tests for certificate loading from disk on startup

- [ ] Task: Implement auto-renewal
    - [ ] Write tests for renewal trigger (30 days before expiry)
    - [ ] Implement renewal check loop (run every 12 hours)
    - [ ] Write tests for renewal retry with exponential backoff
    - [ ] Implement retry logic (1h, 2h, 4h, 8h, max 24h)
    - [ ] Write tests for hot-swap (replace cert in TLS acceptor without restart)
    - [ ] Implement certificate hot-swap using Arc<RwLock<ServerConfig>>

- [ ] Task: Implement on-demand TLS
    - [ ] Write tests for unknown domain → issue cert on first handshake
    - [ ] Implement on-demand cert issuance in TLS acceptor callback
    - [ ] Write tests for whitelist/ask endpoint authorization check
    - [ ] Implement ask endpoint HTTP call for domain authorization
    - [ ] Write tests for rate limiting (max N certs/hour)
    - [ ] Implement rate limiter for on-demand issuance

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: OCSP Stapling & Integration

- [ ] Task: Implement OCSP stapling (`ocsp.rs`)
    - [ ] Write tests for OCSP request construction from certificate
    - [ ] Implement OCSP request builder
    - [ ] Write tests for OCSP response fetching from CA's responder
    - [ ] Implement HTTP-based OCSP response fetching
    - [ ] Write tests for OCSP response validation (signature, freshness)
    - [ ] Implement OCSP response validation
    - [ ] Write tests for OCSP response caching and auto-refresh
    - [ ] Implement background OCSP refresh task
    - [ ] Write tests for OCSP stapling in TLS handshake
    - [ ] Implement OCSP staple injection into rustls ServerConfig

- [ ] Task: Implement self-signed certificate generation
    - [ ] Write tests for localhost self-signed cert generation
    - [ ] Implement self-signed cert builder using rcgen crate
    - [ ] Write tests for private IP detection (10.x, 172.16.x, 192.168.x, ::1)
    - [ ] Implement auto-detection for self-signed vs ACME

- [ ] Task: Integrate with proxy server
    - [ ] Write tests for end-to-end: domain in config → auto cert → HTTPS serving
    - [ ] Implement integration between certman and proxy TLS acceptor
    - [ ] Write tests for HTTP→HTTPS automatic redirect on port 80
    - [ ] Implement auto-redirect
    - [ ] Write tests for manual cert override (user-provided cert/key files bypass ACME)

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
