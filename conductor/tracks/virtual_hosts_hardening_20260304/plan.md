# Implementation Plan: Virtual Hosts Hardening (v0.42.0)

## Phase 1: Configuration Refactor
- [ ] Task: Integrate `vhost::ServerBlock` into `ProxyConfig`
    - [ ] Update `crates/proxy/src/config.rs`. Change `pub locations: Vec<LocationBlock>` to `pub servers: Vec<ServerBlock>`.
    - [ ] Fix configuration parsing unit tests in `config.rs`.
    - [ ] Update `bootstrap.rs` and `http_proxy.rs` to accept `Vec<ParsedServerBlock>`.

## Phase 2: SNI Certificate Binding
- [ ] Task: Populate `SniResolver` loops
    - [ ] Modify `bootstrap.rs`. Loop through `config.servers`.
    - [ ] If `ssl_cert` and `ssl_key` are provided, parse the PEM files.
    - [ ] For each `server_names` entry in the block, call `resolver.add_cert(hostname, cert)`.
    - [ ] Set the first parsed certificate as `resolver.set_default_cert`.

## Phase 3: Runtime Route Request Logic
- [ ] Task: Refactor `HttpProxy::handle_request`
    - [ ] Extract the HTTP `Host` header.
    - [ ] Call `crate::vhost::select_server(&self.servers, host)` to find the active block.
    - [ ] Run `crate::location::match_location(&selected_server.locations, uri.path())`.

## Phase 4: Directives and Variables Implementation
- [ ] Task: Apply `return` and `rewrite`
    - [ ] Instantiate `VariableResolver` in `handle_request`.
    - [ ] Process `location.rewrite` rules. If mutated, update the conceptual `$uri` and loop `match_location` if the flag is `last`.
    - [ ] Check for `location.return_directive`. If present, interpolate strings and return the HTTP response immediately (301/302/200).
    - [ ] Apply `proxy_set_header` interpolations to the upstream `reqwest` headers.

## Phase 5: Testing and Polish
- [ ] Task: End-to-End integration tests
    - [ ] Test requesting two different `Host` headers targeting two different upstreams.
    - [ ] Test a regex rewrite replacing a path segment.
    - [ ] Testing protocol in `workflow.md`.
