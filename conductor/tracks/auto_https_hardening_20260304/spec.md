# Track Specification: Auto HTTPS & On-Demand TLS Hardening (v0.48.0)

## 1. Overview
The current implementation of Automatic HTTPS in Aegis-Flow correctly implements the Let's Encrypt / ACME v2 protocol and handles HTTP-01 challenges. However, it fails to deliver the promised "On-Demand TLS" (Dynamic SNI issuance) because the Rustls `ResolvesServerCert` trait is strictly synchronous, whereas calling `AcmeManager::ensure_cert` to provision a certificate mid-handshake requires `async`/`await`. 

This track focuses on intercepting the TLS handshake before Rustls to allow asynchronous ACME provisioning and fully implements the "ask" authorization endpoint and DNS-01 provider modularity.

## 2. Functional Requirements

### 2.1 Async SNI Handshake Interception
- Implement a custom `Acceptor` or use `tokio-rustls::Accept` stream polling.
- Peek the `ClientHello` bytes from the raw TCP stream to extract the SNI hostname before passing the stream to `rustls`.
- Asynchronously call `acme_manager.ensure_cert(sni).await`.
- Once the certificate is ensured (loaded from disk or newly issued), pass the socket to standard `tokio-rustls::TlsAcceptor`.

### 2.2 On-Demand TLS "Ask" Endpoint & Rate Limiting
- Before issuing a certificate during the async handshake pause, the ACME manager must check the `tls.on_demand` configuration.
- If `ask` is defined (e.g. `https://auth.example.com/check-domain`), make an HTTP GET request to `ask?domain=example.com`.
- Only issue the certificate if the endpoint returns `200 OK`. Any other status implies unauthorized.
- Implement a memory-based rate limiter (sliding window or token bucket) to enforce `rate_limit = 10` certs per hour to prevent DoS attacks.

### 2.3 DNS-01 Modular Providers
- Extract the hardcoded Cloudflare API logic in `crypto_helpers` into a generic `DnsProvider` trait.
- Implement `CloudflareDnsProvider` matching the trait.
- Implement `Route53DnsProvider` (AWS Route53) via AWS HTTP API or SDK.
- Update `acme.rs` issue flow to route TXT record updates through the chosen `DnsProvider`.

## 3. Non-Functional Requirements
- **Performance:** Peeking `ClientHello` must add < 1ms of overhead per connection.
- **Resilience:** The "ask" HTTP client must have a strict timeout (e.g., 5 seconds) to prevent holding up the TLS accept loop indefinitely.

## 4. Acceptance Criteria
- [ ] Raw TCP connections to port 443 with an unknown SNI trigger a Let's Encrypt order *during* the handshake.
- [ ] If the ACME order succeeds, the connection resumes immediately and serves the requested content securely.
- [ ] The `ask` HTTP endpoint successfully rejects unauthorized domains, preventing ACME orders.
- [ ] Route53 is a valid option for DNS-01 wildcard certificates.
