# Track Specification: Aegisfile CLI & Integration Hardening (v0.49.0)

## 1. Overview
The basic parser for the Aegisfile syntax (in `crates/aegisfile`) successfully lexes and parses blocks, providing a dummy `SiteConfig` tree. However, it is not wired into Aegis-Flow's configuration core, its output AST is incomplete, and none of the CLI management tools (`adapt`, `import`, `fmt`, `validate`) exist. 

This track aims to finalize the Aegisfile feature by bridging the AST to the primary `ProxyConfig`, intercepting the startup lifecycle to load `Aegisfile`, and injecting the missing CLI subcommands into the `crates/cli` package established in Track 46.

## 2. Functional Requirements

### 2.1 Complete the AST
- Update `crates/aegisfile/src/parser.rs` and `ast.rs`.
- Add parsing for the missing security directives: `jwt_auth`, `rate_limit`, `header`, `basicauth`.
- Add parsing for matchers (`@methods`, `@remote_ip`).
- Add parsing for the `process` blocks (Process manager / PM2 targets).

### 2.2 AST to ProxyConfig Translator
- Create `crates/proxy/src/config/aegisfile_bridge.rs`.
- Implement a parser that takes `crates::aegisfile::ast::SiteConfig` and translates it into `crates::proxy::config::ProxyConfig`.
- Map `reverse_proxy` to `UpstreamPool` and `LocationBlock`.
- Map domain names to `ServerBlock` and enable `auto_https` automatically.
- Map `process` blocks into `EcosystemConfig` formats.

### 2.3 Daemon Startup Integration
- Modify `crates/proxy/src/config.rs::load_config`.
- Add an auto-discovery step: If `Aegisfile` exists in the local directory, load it first.
- If it exists alongside `aegis.yaml`, prioritize `Aegisfile` or throw a conflict warning based on a CLI flag.

### 2.4 CLI Integration (Relies on Track 46 CLI)
- Expand the CLI application (`aegis`) with the following subcommands:
  - `aegis validate [file]`: Runs parser and prints syntax errors with colorized line references.
  - `aegis fmt [file]`: Re-writes the file with standardized indentation (using `ast::format_sites`).
  - `aegis adapt [file]`: Converts Aegisfile to TOML or YAML and prints it to stdout.
  - `aegis import --from nginx <nginx.conf>`: Translates simple Nginx location/server blocks into an Aegisfile format using regular expressions.

## 3. Non-Functional Requirements
- **Helpful Errors:** Any Aegisfile parsing error during server startup MUST print gracefully, pointing to the exact line number, rather than a Rust panic.
- **Conversion Fidelity:** The `adapt` output in TOML format must produce a functionally identical proxy server behavior as directly booting the Aegisfile.

## 4. Acceptance Criteria
- [ ] Running `aegis start` auto-detects `Aegisfile` in the CWD and starts the server.
- [ ] `aegis adapt Aegisfile > aegis.toml` generates a valid configuration file.
- [ ] Advanced blocks like `jwt_auth`, `rate_limit`, and `header` are successfully mapped to `ProxyConfig`.
