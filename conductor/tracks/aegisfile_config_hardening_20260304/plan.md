# Implementation Plan: Aegisfile CLI & Integration Hardening (v0.49.0)

## Phase 1: Complete AST parsing
- [ ] Task: Support all `aegisfile` directives
    - [ ] Modify `crates/aegisfile/src/parser.rs` to ingest complex blocks (`process`, `jwt_auth`, `header`).
    - [ ] Map these nested properties into struct expansions in `ast.rs`.

## Phase 2: Aegisfile to ProxyConfig Bridge
- [ ] Task: Map the simplified AST to the core configuration schema.
    - [ ] Create `crates/proxy/src/config/aegisfile_bridge.rs` (or add to `config.rs`).
    - [ ] Transform `ast::SiteConfig` lists into `ProxyConfig` Server blocks.
    - [ ] Assign default locations and instantiate the `AutoHttpsConfig` correctly.

## Phase 3: Daemon Loading
- [ ] Task: Integrate `Aegisfile` into Proxy Startup
    - [ ] In `crates/proxy/src/config.rs`, adapt `load_config()`. Check if `Aegisfile` exists.
    - [ ] Read the string, call `aegisfile::parser::parse()`, bridge to `ProxyConfig`, and return.

## Phase 4: CLI Commands
- [ ] Task: Implement configuration manager subcommands in `crates/cli`
    - [ ] Implement `aegis validate` using `aegisfile::ast::validate_sites()`.
    - [ ] Implement `aegis fmt` using `aegisfile::ast::format_sites()`.
    - [ ] Implement `aegis adapt` utilizing the bridge backwards or via generic serializing of `ProxyConfig`.
    - [ ] Add basic Nginx to Aegisfile translation logic.
