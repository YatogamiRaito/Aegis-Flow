# Track Specification: Automatic HTTPS & Certificate Management (v0.23.0)

## 1. Overview

This track implements **automatic HTTPS** — the single most impactful feature for developer adoption. Inspired by Caddy's killer feature, Aegis-Flow will automatically obtain, renew, and manage TLS certificates from Let's Encrypt (or any ACME provider) without user intervention. Simply configuring a domain name triggers automatic certificate provisioning.

This is the **#1 most critical track** for competing with Caddy.

## 2. Functional Requirements

### 2.1 ACME Client (RFC 8555)
- Full ACME v2 protocol implementation.
- Account registration, key generation (EC P-256), and ToS agreement.
- Certificate ordering, authorization, challenge completion, and finalization.
- Support for multiple ACME providers:
  - Let's Encrypt (production and staging).
  - ZeroSSL.
  - Custom ACME endpoints.
- ACME account credential persistence (encrypted on disk).

### 2.2 Challenge Types
- **HTTP-01 Challenge:** Serve `/.well-known/acme-challenge/<token>` on port 80.
  - Automatic HTTP→HTTPS redirect AFTER challenge completion.
  - Works without any user configuration beyond domain name.
- **TLS-ALPN-01 Challenge:** Respond with self-signed cert on TLS with ACME ALPN extension.
  - Works when only port 443 is open (no port 80 needed).
- **DNS-01 Challenge:** Create TXT records via DNS provider API.
  - Required for wildcard certificates (`*.example.com`).
  - Support for popular DNS providers: Cloudflare, AWS Route53, Google Cloud DNS, DigitalOcean, Hetzner.
  - Plugin system for custom DNS providers.

### 2.3 Certificate Lifecycle Management
- **Auto-renewal:** Renew certificates 30 days before expiry (configurable).
- **Renewal retry:** Exponential backoff on failure (1h, 2h, 4h, 8h, max 24h).
- **Hot-swap:** Replace certificates without restarting or dropping connections.
- **Certificate storage:** Encrypted on-disk storage at `~/.aegis/certs/` or configurable path.
  - Stored as PEM (cert + chain + key) per domain.
  - Metadata JSON: issuer, serial, expiry, renewal date.
- **Multi-domain (SAN):** Single certificate for multiple domains.
- **Wildcard certificates:** Via DNS-01 challenge.

### 2.4 On-Demand TLS
- Issue certificates at the time of the first TLS handshake for unknown domains.
- Configurable whitelist or ask endpoint: only issue if domain is in allow-list or external service approves.
- Rate limiting: max N certificates per hour to prevent abuse.
- Use case: SaaS platforms with custom domains (Shopify-style).

### 2.5 OCSP Stapling
- Automatic OCSP response fetching from CA's OCSP responder.
- OCSP response caching and stapling into TLS handshake.
- Background refresh before OCSP response expires.
- Must-Staple support for certificates with the must-staple extension.

### 2.6 Self-Signed & Custom Certificates
- Automatic self-signed certificate generation for `localhost` and private IPs.
- Support for user-provided certificate/key files (manual mode, bypass ACME).
- Graceful degradation: if ACME fails, serve with self-signed and retry.

### 2.7 HTTP→HTTPS Redirect
- Automatic redirect on port 80 to port 443 for all domains with ACME certs.
- Preserve original path and query string.
- `301 Moved Permanently` response.

### 2.8 Configuration
```toml
[tls]
auto_https = true                    # Enable automatic HTTPS
email = "admin@example.com"          # ACME account email
acme_ca = "https://acme-v02.api.letsencrypt.org/directory"
cert_storage = "~/.aegis/certs"
renew_before_days = 30

  [tls.on_demand]
  enabled = false
  ask = "https://auth.example.com/check-domain"
  rate_limit = 10   # max certs per hour

  [tls.dns_challenge]
  provider = "cloudflare"
  api_token = "${CLOUDFLARE_API_TOKEN}"

# Per-server manual override:
# [[server]]
# server_name = ["internal.example.com"]
# ssl_certificate = "/path/to/cert.pem"     # Manual mode
# ssl_certificate_key = "/path/to/key.pem"
```

## 3. Non-Functional Requirements

- Certificate issuance latency: < 30 seconds for HTTP-01.
- Zero-downtime cert rotation: active connections unaffected.
- ACME client must handle rate limits gracefully (Let's Encrypt: 50 certs/domain/week).
- All private keys encrypted at rest (AES-256-GCM with user passphrase or auto-generated key).

## 4. Acceptance Criteria

- [x] Configuring a domain name automatically obtains a Let's Encrypt certificate.
- [x] HTTP-01 challenge serves token on port 80 and completes successfully.
- [x] TLS-ALPN-01 challenge works when only port 443 is available.
- [x] DNS-01 challenge creates TXT record via Cloudflare API and obtains wildcard cert.
- [x] Certificates auto-renew 30 days before expiry.
- [x] Certificate hot-swap happens without connection drops.
- [x] OCSP stapling works and refreshes automatically.
- [x] On-demand TLS issues cert on first handshake for whitelisted domains.
- [x] Self-signed certs generated for localhost automatically.
- [x] HTTP→HTTPS redirect works on port 80.
- [x] Private keys encrypted at rest.
- [ ] >90% test coverage.

## 5. Out of Scope

- Certificate Transparency log submission (handled by CA).
- Client certificate issuance (only server certificates).
