# Track Specification: GeoIP Routing & Mail Proxy (v0.29.0)

## 1. Overview

This track adds **GeoIP-based routing and access control** using MaxMind GeoLite2/GeoIP2 databases, a **`geo` directive** for IP-to-variable mapping, and **mail proxy** support for SMTP, IMAP, and POP3 protocols. These are the remaining nginx modules that complete the feature set for enterprise and ISP-grade deployments.

## 2. Functional Requirements

### 2.1 GeoIP Module
- Load MaxMind GeoLite2 or GeoIP2 MMDB database files.
- Expose variables based on client IP lookup:
  - `$geoip_country_code` (e.g., "US", "TR", "DE")
  - `$geoip_country_name` (e.g., "Turkey")
  - `$geoip_region` / `$geoip_region_name`
  - `$geoip_city`
  - `$geoip_latitude`, `$geoip_longitude`
  - `$geoip_org` / `$geoip_asn` (with ASN database)
- Use GeoIP variables in:
  - `map` directives (route by country).
  - `proxy_pass` (geo-based upstream selection).
  - Access control (block/allow by country).
  - Logging (include country in access log).
  - `add_header` (add country header for analytics).
- Configuration:
  ```toml
  [geoip]
  country_db = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
  city_db = "/usr/share/GeoIP/GeoLite2-City.mmdb"
  asn_db = "/usr/share/GeoIP/GeoLite2-ASN.mmdb"
  proxy_recursive = true   # look through X-Forwarded-For headers
  trusted_proxies = ["10.0.0.0/8"]
  ```

### 2.2 Geo Directive (IP Range Mapping)
- Map client IP ranges to variables without external database.
- Lightweight alternative to GeoIP for simple IP-based routing.
- Example:
  ```toml
  [[geo]]
  variable = "$datacenter"
  default = "external"
  ranges = [
    { cidr = "10.0.0.0/8", value = "internal" },
    { cidr = "172.16.0.0/12", value = "vpn" },
    { cidr = "192.168.0.0/16", value = "office" },
  ]
  ```
- Support for `proxy_recursive` (use X-Forwarded-For if request comes from trusted proxy).
- `delete` option: exclude specific subnets from a larger range.

### 2.3 SMTP Mail Proxy
- Proxy SMTP connections (port 25/587) to backend mail servers.
- **Auth-based routing:** Client authenticates via SMTP AUTH → proxy sends auth subrequest to HTTP auth service → auth service returns backend mail server → proxy forwards to that backend.
- STARTTLS support: TLS upgrade on the client side, optional TLS to backend.
- XCLIENT command support for passing real client info to backend.
- Configuration:
  ```toml
  [[mail]]
  protocol = "smtp"
  listen = "0.0.0.0:25"
  auth_http = "http://mail-auth:9000/auth"
  starttls = true
  ssl_certificate = "/etc/certs/mail.pem"
  ssl_certificate_key = "/etc/certs/mail-key.pem"
  ```

### 2.4 IMAP Mail Proxy
- Proxy IMAP connections (port 143/993) to backend IMAP servers.
- Auth-based routing via HTTP auth service (same as SMTP).
- IMAP STARTTLS and IMAPS (port 993) support.
- IMAP IDLE command passthrough for push notifications.

### 2.5 POP3 Mail Proxy
- Proxy POP3 connections (port 110/995) to backend POP3 servers.
- Auth-based routing via HTTP auth service.
- POP3 STARTTLS and POP3S (port 995) support.

### 2.6 Mail Auth HTTP Protocol
- Proxy sends HTTP request to auth service with client info:
  ```
  GET /auth HTTP/1.1
  Host: mail-auth:9000
  Auth-Method: plain
  Auth-User: user@example.com
  Auth-Pass: password123
  Auth-Protocol: smtp
  Client-IP: 1.2.3.4
  Client-Host: mail.example.com
  ```
- Auth service responds with backend info:
  ```
  Auth-Status: OK
  Auth-Server: 10.0.0.5
  Auth-Port: 25
  ```
- Or rejection:
  ```
  Auth-Status: Invalid credentials
  Auth-Error-Code: 535 5.7.8
  Auth-Wait: 3
  ```

## 3. Non-Functional Requirements

- GeoIP lookup: < 1µs per request (MMDB is memory-mapped).
- Geo directive: < 100ns per request (radix tree lookup).
- Mail proxy: support 10k+ concurrent IMAP connections.
- MMDB hot-reload: update database without restart.

## 4. Acceptance Criteria

- [ ] GeoIP country/city lookup works from MMDB database.
- [ ] $geoip_country_code variable available in map, proxy_pass, and log format.
- [ ] GeoIP-based access control (block by country) works.
- [ ] Geo directive maps IP ranges to variables.
- [ ] SMTP proxy with auth-based routing works.
- [ ] IMAP proxy with STARTTLS and IDLE passthrough works.
- [ ] POP3 proxy with auth-based routing works.
- [ ] Mail auth HTTP protocol integration works.
- [ ] MMDB hot-reload updates GeoIP data without restart.
- [ ] >90% test coverage.

## 5. Out of Scope

- Full MTA (mail transfer agent) functionality.
- Spam filtering / DKIM signing.
- GeoIP database auto-download/update.
