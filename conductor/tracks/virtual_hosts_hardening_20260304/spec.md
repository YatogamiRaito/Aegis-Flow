# Track Specification: Virtual Hosts Hardening (v0.42.0)

## 1. Overview
The current proxy architecture contains fully functioning libraries for Virtual Hosts (`vhost.rs`), location matching (`location.rs`), variable interpolation (`variables.rs`), and URL rewriting (`rewrite.rs`). However, `http_proxy.rs` and `config.rs` do not integrate these libraries. This hardening track resolves these omissions, officially bringing multi-domain Virtual Host support and dynamic routing to Aegis-Flow.

## 2. Functional Requirements

### 2.1 Configuration Integration
- Replace the flat `locations: Vec<LocationBlock>` inside `ProxyConfig` with an array of `servers: Vec<ServerBlock>`.
- The configuration loader must parse multiple `[[server]]` virtual hosts correctly.

### 2.2 SNI & TLS Certificates
- During `bootstrap.rs`, iterate through `ProxyConfig::servers`.
- For each server requiring TLS, load its specified `ssl_cert` and `ssl_key`.
- Populate `SniResolver::add_cert(domain, cert)` so that the TLS acceptor can dynamically serve the correct certificate based on the ClientHello SNI extension.

### 2.3 HTTP Request Routing
- `HttpProxy` must evaluate the `Host` header (or SNI context) using `vhost::select_server` to locate the correct `ServerBlock` for every incoming request.
- The `match_location` algorithm must be executed against the targeted `ServerBlock`'s location list, not a global list.

### 2.4 Variables and Directives
- Instantiate `VariableResolver` in `handle_request` with context from the incoming HTTP request.
- If a matching location contains `return_directive`, evaluate it (interpolating variables) and return the HTTP response immediately.
- If a matching location contains `rewrite` rules, evaluate them sequentially. Handle `last`, `break`, `redirect`, and `permanent` flags strictly according to standard NGINX behaviors.
- Apply `proxy_set_header`, `add_header`, and `proxy_hide_header` to the request and response contexts.

## 3. Non-Functional Requirements
- **Performance:** `ServerBlock` matching should cache `ServerNameMatcher`s upon initialization to ensure quick O(1) or fast-regex evaluation during request handling.
- **Stability:** Misconfigured TLS certificates in a `[[server]]` block should log an error but not necessarily crash the entire proxy if other healthy server blocks exist.

## 4. Acceptance Criteria
- [ ] `config.rs` successfully parses `[[server]]` arrays from TOML configs.
- [ ] Multiple domains (`curl -H "Host: a.com"`, `curl -H "Host: b.com"`) get routed to distinct upstreams or static folders based on the server blocks.
- [ ] `rewrite` patterns successfully mutate the `$uri` before upstream connection.
- [ ] `return` directives instantly return HTTP 301/302 redirects without connecting to the upstream backend.
- [ ] Variables like `$remote_addr` are correctly injected into `proxy_set_header` outputs.
