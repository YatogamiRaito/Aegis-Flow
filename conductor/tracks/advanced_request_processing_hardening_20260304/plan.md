# Implementation Plan: Advanced Request Processing Hardening (v0.51.0)

## Phase 1: Request Variable Engine
- [ ] Task: Create `crates/proxy/src/variables.rs`
    - [ ] Implement `VariableContext<'a>` holding a reference to the `hyper::Request`.
    - [ ] Implement lazy resolution for `$uri`, `$host`, `$remote_addr`, and `$http_*`.
    - [ ] Write a `interpolate(string, context)` function to replace regex variables in a string (e.g. `http://$backend:8080`).

## Phase 2: Wiring Map and Split Clients
- [ ] Task: Integrate directives into `VariableContext`
    - [ ] Pass the global `map` and `split_clients` configs into the per-request context.
    - [ ] Make `$my_map_var` implicitly look up the map definition, evaluate its source variable recursively, and return the result.
    - [ ] Refactor `http_proxy::handle_request` to run `proxy_pass` through `.interpolate()`.

## Phase 3: Traffic Mirror Execution
- [ ] Task: Spawn mirror tasks
    - [ ] In `http_proxy.rs`, detect `config.mirror` on the matched location.
    - [ ] Evaluate `should_mirror(req_id)`.
    - [ ] Clone the request (using `http_body_util::BodyExt::collect` to buffer if body is needed).
    - [ ] `tokio::spawn` a simple `reqwest` or `hyper` client call to the mirror endpoint.

## Phase 4: Auth Request Enhancements
- [ ] Task: Implement auth caching and header injection
    - [ ] Add an `moka::future::Cache` or `tokio-rs/mini-redis` caching mechanism to `auth_request.rs`.
    - [ ] When `auth_request_set` is populated, parse those headers from the auth response and append them to the main `hyper::Request` headers before proxying.
