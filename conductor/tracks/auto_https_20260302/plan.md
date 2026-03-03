# Implementation Plan: Automatic HTTPS & Certificate Management (v0.23.0)

## Phase 1: ACME Client Foundation

- [x] Task: Create certificate management crate (`crates/certman/`)
    - [x] Define crate structure: `lib.rs`, `acme.rs`, `challenge.rs`, `storage.rs`, `ocsp.rs`, `dns_providers/`
    - [x] Add dependencies: reqwest, serde, ring (EC key gen), base64, pem, x509-parser
    - [x] Add crate to workspace Cargo.toml

- [x] Task: Implement ACME account management (`acme.rs`)
    - [x] Write tests for EC P-256 key pair generation
    - [x] Implement account key generation and persistence
    - [x] Write tests for ACME directory discovery (fetch endpoints from CA)
    - [x] Implement directory fetching and parsing
    - [x] Write tests for new account registration with ToS agreement
    - [x] Implement account registration (newAccount endpoint)
    - [x] Write tests for JWS (JSON Web Signature) message signing for ACME requests
    - [x] Implement JWS builder with nonce management

- [x] Task: Implement certificate order flow
    - [x] Write tests for new order creation (newOrder endpoint)
    - [x] Implement order creation with domain identifiers
    - [x] Write tests for authorization retrieval and challenge selection
    - [x] Implement authorization fetching and challenge parsing
    - [x] Write tests for order finalization with CSR
    - [x] Implement CSR generation and order finalization
    - [x] Write tests for certificate download (PEM chain)
    - [x] Implement certificate download and chain assembly

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Challenge Solvers

- [x] Task: Implement HTTP-01 challenge solver
    - [x] Write tests for challenge token serving at /.well-known/acme-challenge/<token>
    - [x] Implement temporary HTTP server on port 80 for challenge
    - [x] Write tests for challenge validation notification to ACME server
    - [x] Implement challenge response and polling for valid status
    - [x] Write tests for integration with existing HTTP listener (serve challenge alongside normal traffic)

- [x] Task: Implement TLS-ALPN-01 challenge solver
    - [x] Write tests for self-signed cert generation with ACME ALPN extension
    - [x] Implement TLS-ALPN-01 certificate builder
    - [x] Write tests for ALPN protocol negotiation ("acme-tls/1")
    - [x] Implement custom TLS acceptor that responds to ACME ALPN

- [x] Task: Implement DNS-01 challenge solver framework
    - [x] Write tests for TXT record name generation (_acme-challenge.<domain>)
    - [x] Implement DNS challenge token computation (SHA-256 + base64url)
    - [x] Write tests for Cloudflare DNS provider (create/delete TXT record via API)
    - [x] Implement CloudflareDnsProvider
    - [x] Write tests for AWS Route53 provider
    - [x] Implement Route53DnsProvider
    - [x] Write tests for DNS provider plugin trait (for custom providers)
    - [x] Implement DnsProvider trait for extensibility

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Certificate Storage & Lifecycle

- [x] Task: Implement encrypted certificate storage (`storage.rs`)
    - [x] Write tests for PEM certificate + key writing to disk
    - [x] Implement cert storage with configurable path
    - [x] Write tests for private key encryption at rest (AES-256-GCM)
    - [x] Implement key encryption/decryption
    - [x] Write tests for certificate metadata persistence (issuer, expiry, domains)
    - [x] Implement metadata JSON storage
    - [x] Write tests for certificate loading from disk on startup

- [x] Task: Implement auto-renewal
    - [x] Write tests for renewal trigger (30 days before expiry)
    - [x] Implement renewal check loop (run every 12 hours)
    - [x] Write tests for renewal retry with exponential backoff
    - [x] Implement retry logic (1h, 2h, 4h, 8h, max 24h)
    - [x] Write tests for hot-swap (replace cert in TLS acceptor without restart)
    - [x] Implement certificate hot-swap using Arc<RwLock<ServerConfig>>

- [x] Task: Implement on-demand TLS
    - [x] Write tests for unknown domain → issue cert on first handshake
    - [x] Implement on-demand cert issuance in TLS acceptor callback
    - [x] Write tests for whitelist/ask endpoint authorization check
    - [x] Implement ask endpoint HTTP call for domain authorization
    - [x] Write tests for rate limiting (max N certs/hour)
    - [x] Implement rate limiter for on-demand issuance

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: OCSP Stapling & Integration

- [x] Task: Implement OCSP stapling (`ocsp.rs`)
    - [x] Write tests for OCSP request construction from certificate
    - [x] Implement OCSP request builder
    - [x] Write tests for OCSP response fetching from CA's responder
    - [x] Implement HTTP-based OCSP response fetching
    - [x] Write tests for OCSP response validation (signature, freshness)
    - [x] Implement OCSP response validation
    - [x] Write tests for OCSP response caching and auto-refresh
    - [x] Implement background OCSP refresh task
    - [x] Write tests for OCSP stapling in TLS handshake
    - [x] Implement OCSP staple injection into rustls ServerConfig

- [x] Task: Implement self-signed certificate generation
    - [x] Write tests for localhost self-signed cert generation
    - [x] Implement self-signed cert builder using rcgen crate
    - [x] Write tests for private IP detection (10.x, 172.16.x, 192.168.x, ::1)
    - [x] Implement auto-detection for self-signed vs ACME

- [x] Task: Integrate with proxy server
    - [x] Write tests for end-to-end: domain in config → auto cert → HTTPS serving
    - [x] Implement integration between certman and proxy TLS acceptor
    - [x] Write tests for HTTP→HTTPS automatic redirect on port 80
    - [x] Implement auto-redirect
    - [x] Write tests for manual cert override (user-provided cert/key files bypass ACME)

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
