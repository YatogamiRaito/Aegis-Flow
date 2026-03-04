# Implementation Plan: GeoIP Routing & Mail Proxy Hardening (v0.54.0)

## Phase 1: Dependency & Config Wiring
- [ ] Task: Project Configuration
    - [ ] Add `aegis-mail = { path = "../mail" }` to `crates/proxy/Cargo.toml`.
    - [ ] Update `ProxyConfig` in `config.rs` to parse `[[mail]]`, `[geoip]`, and `[[geo]]` tables.
    - [ ] Write tests ensuring a comprehensive TOML config validates.

## Phase 2: Mail Server Listeners
- [ ] Task: Mail Bootstrap Loop
    - [ ] In `bootstrap.rs`, implement `start_mail_listeners(config, shutdown_rx)`.
    - [ ] Bind `TcpListener` per config, respecting `SO_REUSEPORT`.
    - [ ] Loop and `accept()`.
    - [ ] Pass the accepted stream to `aegis_mail::smtp::handle_client` (and IMAP/POP3 variants).

## Phase 3: GeoIP Variable Injection
- [ ] Task: Middleware Context Initialization
    - [ ] In `bootstrap.rs`, load MMDBs (Country, City, ASN) into a global `Arc` state.
    - [ ] In `http_proxy::handle_request`, before executing location rules or building upstream requests, resolve the true Client IP.
    - [ ] Call `geoip::lookup_country(ip, db)` and inject the results into the `VariableContext` (to be created in Track 51).

## Phase 4: Geo Directive Logic
- [ ] Task: Evaluate `geo` CIDR ranges
    - [ ] In config struct, parse the ranges into a radix tree or list of `ipnet::IpNet`.
    - [ ] During the same variable context initialization, attempt to match the Client IP against the tree.
    - [ ] Inject the resulting value into the `VariableContext`.

## Phase 5: Verification & Tests
- [ ] Task: Write Integration Tests
    - [ ] Spawn the full `aegis-proxy` dynamically via tests, configure a mail port.
    - [ ] Use a standard TCP client to verify the SMTP protocol handshakes work end-to-end through the proxy.
