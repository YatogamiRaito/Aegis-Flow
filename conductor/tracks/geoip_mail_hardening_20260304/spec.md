# Track Specification: GeoIP Routing & Mail Proxy Hardening (v0.54.0)

## 1. Overview
The `aegis-mail` workspace crate successfully encapsulates SMTP, IMAP, and POP3 protocol parsing along with HTTP authentication routing. In the `proxy` crate, `geoip.rs` correctly interacts with MaxMind databases.

However, neither of these features are wired into the main Aegis-Flow proxy. The engine currently only listens on HTTP/HTTPS/QUIC and has no awareness of the mail crate or GeoIP resolution during the HTTP request cycle.

This track aims to expose the Mail proxies by dynamically binding TCP listeners based on configuration, and to inject GeoIP database results into the Nginx-style variable engine.

## 2. Functional Requirements

### 2.1 Mail Proxy Integration
- In `config.rs`, parse a new `[[mail]]` block that includes: `protocol` (smtp/imap/pop3), `listen` address, `auth_http` URL, and `starttls` boolean.
- Add `aegis-mail` as a dependency in `crates/proxy/Cargo.toml`.
- In `bootstrap.rs`, loop over the `[mail]` configs. For each:
    - Bind a `tokio::net::TcpListener` (with `SO_REUSEPORT` enabled via Track 53 mechanisms).
    - Spawn a background task (`tokio::spawn`) that accepts incoming mail connections.
    - Dispatch each connection to the appropriate handler in the `aegis_mail` crate (`smtp::handle_client`, `imap::handle_client`, `pop3::handle_client`).

### 2.2 GeoIP & Geo Directive Integration
- Update `config.rs` to parse the `[geoip]` block.
- In `bootstrap.rs`, if `geoip` is configured, eagerly load the `.mmdb` files via the logic in `geoip.rs`. Store the loaded DBs in a shared `Arc` state accessible by the HTTP workers.
- Add an extension to `http_proxy.rs` (or `variables.rs`) so that when the `VariableContext` is building:
    1. Extract the client's actual remote IP (respecting `X-Forwarded-For` and `proxy_recursive` if configured).
    2. Query the GeoIP DB.
    3. Insert `$geoip_country_code`, `$geoip_country_name`, etc., into the request's Variable Context.
- Similar implementation for the `geo` directive: parse the CIDR ranges, evaluate the remote IP against the tree, and inject the mapped variable into the Context.

## 3. Non-Functional Requirements
- **Performance:** Ensure MMDB reads during the HTTP request lifecycle are fast and do not block the tokio executor (e.g., use memory-mapped DBs properly, which `maxminddb` generally does).
- **Graceful Shutdown:** Mail `TcpListener` tasks must respect the global shutdown cancellation tokens and finish in-flight mail connections before exiting.

## 4. Acceptance Criteria
- [ ] Attempting to connect via `nc localhost 25` returns the `220 ... ESMTP` Aegis-Flow banner.
- [ ] Mail auth subrequests fire successfully and can be seen in external mock HTTP servers.
- [ ] An incoming HTTP request's IP is successfully mapped to `$geoip_country_code` and can be utilized in a `sub_filter` or `proxy_pass` destination.
