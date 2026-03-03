# Implementation Plan: GeoIP Routing & Mail Proxy (v0.29.0)

## Phase 1: GeoIP Integration

- [x] Task: Create GeoIP module (`crates/proxy/src/geoip.rs`)
    - [x] Write tests for MMDB file loading (GeoLite2-Country)
    - [x] Implement MMDB reader using maxminddb crate
    - [x] Write tests for country lookup by IPv4 and IPv6
    - [x] Implement country lookup → $geoip_country_code, $geoip_country_name
    - [x] Write tests for city lookup (city, region, lat/long)
    - [x] Implement city database lookups
    - [x] Write tests for ASN lookup ($geoip_asn, $geoip_org)
    - [x] Implement ASN database lookups

- [x] Task: Integrate GeoIP variables into variable system
    - [x] Write tests for $geoip_* variables in map directives
    - [x] Implement GeoIP variable provider registration
    - [x] Write tests for $geoip_* in proxy_pass / header / log format
    - [x] Write tests for proxy_recursive (X-Forwarded-For traversal with trusted proxies)
    - [x] Implement proxy_recursive IP resolution

- [x] Task: Implement MMDB hot-reload
    - [x] Write tests for file watcher on MMDB database
    - [x] Implement hot-reload using notify crate (watch for file changes)
    - [x] Write tests for atomic swap of MMDB reader (no lock contention during lookup)

- [x] Task: Implement GeoIP access control
    - [x] Write tests for country-based allow/deny (e.g., deny country = ["CN", "RU"])
    - [x] Implement country ACL filter
    - [x] Write tests for GeoIP + map combo (country → upstream backend)

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Geo Directive (IP Range Mapping)

- [x] Task: Implement geo directive (`crates/proxy/src/geo_directive.rs`)
    - [x] Write tests for CIDR range → variable mapping
    - [x] Implement IP range tree using ip_network or custom radix tree
    - [x] Write tests for default value when no range matches
    - [x] Write tests for overlapping ranges (most specific wins)
    - [x] Implement longest-prefix-match lookup
    - [x] Write tests for proxy_recursive (resolve through X-Forwarded-For)
    - [x] Write tests for delete option (exclude subnets from larger range)
    - [x] Implement range exclusion

- [x] Task: Integrate geo variables
    - [x] Write tests for geo variable usage in proxy_pass and map
    - [x] Implement geo variable provider

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: SMTP Mail Proxy

- [x] Task: Implement SMTP protocol handler (`crates/mail/src/smtp.rs`)
    - [x] Create `crates/mail/` crate with Cargo.toml
    - [x] Write tests for SMTP greeting and EHLO handshake
    - [x] Implement SMTP server greeting and capability advertisement
    - [x] Write tests for SMTP AUTH (PLAIN, LOGIN mechanisms)
    - [x] Implement AUTH command parsing and extraction
    - [x] Write tests for STARTTLS upgrade
    - [x] Implement TLS upgrade on SMTP connection

- [x] Task: Implement mail auth HTTP protocol
    - [x] Write tests for auth HTTP request construction (headers: Auth-User, Auth-Pass, Client-IP, etc.)
    - [x] Implement auth subrequest to external HTTP auth service
    - [x] Write tests for successful auth response parsing (Auth-Server, Auth-Port)
    - [x] Write tests for failed auth response parsing (Auth-Status, Auth-Error-Code, Auth-Wait)
    - [x] Implement auth response handler with wait/retry

- [x] Task: Implement SMTP proxying
    - [x] Write tests for client→backend SMTP relay after auth
    - [x] Implement bidirectional SMTP forwarding
    - [x] Write tests for XCLIENT command injection (pass real client IP to backend)
    - [x] Implement XCLIENT support

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: IMAP & POP3 Mail Proxy

- [x] Task: Implement IMAP protocol handler (`crates/mail/src/imap.rs`)
    - [x] Write tests for IMAP greeting and capability response
    - [x] Implement IMAP server greeting
    - [x] Write tests for IMAP LOGIN/AUTHENTICATE command parsing
    - [x] Implement IMAP auth extraction
    - [x] Write tests for IMAP STARTTLS upgrade
    - [x] Write tests for IMAPS (direct TLS on port 993)
    - [x] Implement IMAP TLS handling

- [x] Task: Implement IMAP proxying
    - [x] Write tests for IMAP command passthrough after auth routing
    - [x] Implement bidirectional IMAP proxying
    - [x] Write tests for IMAP IDLE command passthrough (push notifications)
    - [x] Implement IDLE-aware keepalive handling

- [x] Task: Implement POP3 protocol handler (`crates/mail/src/pop3.rs`)
    - [x] Write tests for POP3 greeting (+OK)
    - [x] Write tests for POP3 USER/PASS authentication
    - [x] Implement POP3 auth extraction
    - [x] Write tests for POP3 STARTTLS and POP3S
    - [x] Write tests for POP3 proxying after auth routing

- [x] Task: Integrate mail modules with main server
    - [x] Write tests for mail stream listener separate from HTTP
    - [x] Implement mail listener configuration in main config
    - [x] Write tests for multiple mail protocols on different ports

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
