# Implementation Plan: GeoIP Routing & Mail Proxy (v0.29.0)

## Phase 1: GeoIP Integration

- [ ] Task: Create GeoIP module (`crates/proxy/src/geoip.rs`)
    - [ ] Write tests for MMDB file loading (GeoLite2-Country)
    - [ ] Implement MMDB reader using maxminddb crate
    - [ ] Write tests for country lookup by IPv4 and IPv6
    - [ ] Implement country lookup → $geoip_country_code, $geoip_country_name
    - [ ] Write tests for city lookup (city, region, lat/long)
    - [ ] Implement city database lookups
    - [ ] Write tests for ASN lookup ($geoip_asn, $geoip_org)
    - [ ] Implement ASN database lookups

- [ ] Task: Integrate GeoIP variables into variable system
    - [ ] Write tests for $geoip_* variables in map directives
    - [ ] Implement GeoIP variable provider registration
    - [ ] Write tests for $geoip_* in proxy_pass / header / log format
    - [ ] Write tests for proxy_recursive (X-Forwarded-For traversal with trusted proxies)
    - [ ] Implement proxy_recursive IP resolution

- [ ] Task: Implement MMDB hot-reload
    - [ ] Write tests for file watcher on MMDB database
    - [ ] Implement hot-reload using notify crate (watch for file changes)
    - [ ] Write tests for atomic swap of MMDB reader (no lock contention during lookup)

- [ ] Task: Implement GeoIP access control
    - [ ] Write tests for country-based allow/deny (e.g., deny country = ["CN", "RU"])
    - [ ] Implement country ACL filter
    - [ ] Write tests for GeoIP + map combo (country → upstream backend)

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Geo Directive (IP Range Mapping)

- [ ] Task: Implement geo directive (`crates/proxy/src/geo_directive.rs`)
    - [ ] Write tests for CIDR range → variable mapping
    - [ ] Implement IP range tree using ip_network or custom radix tree
    - [ ] Write tests for default value when no range matches
    - [ ] Write tests for overlapping ranges (most specific wins)
    - [ ] Implement longest-prefix-match lookup
    - [ ] Write tests for proxy_recursive (resolve through X-Forwarded-For)
    - [ ] Write tests for delete option (exclude subnets from larger range)
    - [ ] Implement range exclusion

- [ ] Task: Integrate geo variables
    - [ ] Write tests for geo variable usage in proxy_pass and map
    - [ ] Implement geo variable provider

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: SMTP Mail Proxy

- [ ] Task: Implement SMTP protocol handler (`crates/mail/src/smtp.rs`)
    - [ ] Create `crates/mail/` crate with Cargo.toml
    - [ ] Write tests for SMTP greeting and EHLO handshake
    - [ ] Implement SMTP server greeting and capability advertisement
    - [ ] Write tests for SMTP AUTH (PLAIN, LOGIN mechanisms)
    - [ ] Implement AUTH command parsing and extraction
    - [ ] Write tests for STARTTLS upgrade
    - [ ] Implement TLS upgrade on SMTP connection

- [ ] Task: Implement mail auth HTTP protocol
    - [ ] Write tests for auth HTTP request construction (headers: Auth-User, Auth-Pass, Client-IP, etc.)
    - [ ] Implement auth subrequest to external HTTP auth service
    - [ ] Write tests for successful auth response parsing (Auth-Server, Auth-Port)
    - [ ] Write tests for failed auth response parsing (Auth-Status, Auth-Error-Code, Auth-Wait)
    - [ ] Implement auth response handler with wait/retry

- [ ] Task: Implement SMTP proxying
    - [ ] Write tests for client→backend SMTP relay after auth
    - [ ] Implement bidirectional SMTP forwarding
    - [ ] Write tests for XCLIENT command injection (pass real client IP to backend)
    - [ ] Implement XCLIENT support

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: IMAP & POP3 Mail Proxy

- [ ] Task: Implement IMAP protocol handler (`crates/mail/src/imap.rs`)
    - [ ] Write tests for IMAP greeting and capability response
    - [ ] Implement IMAP server greeting
    - [ ] Write tests for IMAP LOGIN/AUTHENTICATE command parsing
    - [ ] Implement IMAP auth extraction
    - [ ] Write tests for IMAP STARTTLS upgrade
    - [ ] Write tests for IMAPS (direct TLS on port 993)
    - [ ] Implement IMAP TLS handling

- [ ] Task: Implement IMAP proxying
    - [ ] Write tests for IMAP command passthrough after auth routing
    - [ ] Implement bidirectional IMAP proxying
    - [ ] Write tests for IMAP IDLE command passthrough (push notifications)
    - [ ] Implement IDLE-aware keepalive handling

- [ ] Task: Implement POP3 protocol handler (`crates/mail/src/pop3.rs`)
    - [ ] Write tests for POP3 greeting (+OK)
    - [ ] Write tests for POP3 USER/PASS authentication
    - [ ] Implement POP3 auth extraction
    - [ ] Write tests for POP3 STARTTLS and POP3S
    - [ ] Write tests for POP3 proxying after auth routing

- [ ] Task: Integrate mail modules with main server
    - [ ] Write tests for mail stream listener separate from HTTP
    - [ ] Implement mail listener configuration in main config
    - [ ] Write tests for multiple mail protocols on different ports

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
