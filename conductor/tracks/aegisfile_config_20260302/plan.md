# Implementation Plan: Aegisfile — Simple Configuration Format (v0.24.0)

## Phase 1: Aegisfile Parser

- [x] Task: Create Aegisfile parser (`crates/config/src/aegisfile.rs`)
    - [x] Write tests for lexer: tokenize braces, strings, identifiers, comments, newlines
    - [x] Implement lexer using logos or hand-written scanner
    - [x] Write tests for parser: top-level domain blocks
    - [x] Implement recursive-descent parser for `domain { directives }`
    - [x] Write tests for nested directive blocks (`reverse_proxy { ... }`)
    - [x] Implement nested block parsing
    - [x] Write tests for quoted strings, unquoted arguments, multi-word values
    - [x] Write tests for comment handling (# inline, # full line)

- [x] Task: Implement Aegisfile AST
    - [x] Write tests for AST node types (Site, Directive, Matcher, Block, Argument)
    - [x] Implement AST data structures
    - [x] Write tests for AST-to-ProxyConfig conversion
    - [x] Implement AST → internal ProxyConfig mapper
    - [x] Write tests for error reporting with line/column numbers
    - [x] Implement error messages with context and suggestions

- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Directive Implementations

- [x] Task: Implement core directives
    - [x] Write tests for `reverse_proxy` directive parsing (path matcher + upstream)
    - [x] Implement reverse_proxy → ProxyConfig location mapping
    - [x] Write tests for `root` and `file_server` directives
    - [x] Implement static file config generation
    - [x] Write tests for `redirect` directive (URL + status code)
    - [x] Write tests for `rewrite` directive (pattern + replacement)
    - [x] Write tests for `respond` directive (body + status)

- [x] Task: Implement security directives
    - [x] Write tests for `rate_limit` directive parsing
    - [x] Write tests for `basicauth` directive with user block
    - [x] Write tests for `jwt_auth` directive with JWKS config
    - [x] Write tests for `header` directive (add/remove/set)
    - [x] Implement security directive → config mapping

- [x] Task: Implement matcher syntax
    - [x] Write tests for path matchers (/api/*, *.php)
    - [x] Write tests for named matchers (@api { path /api/* })
    - [x] Write tests for method matchers (@get { method GET })
    - [x] Write tests for remote_ip matchers
    - [x] Implement matcher resolution engine

- [x] Task: Implement `process` blocks for PM2 integration
    - [x] Write tests for process block parsing (command, instances, env, max_memory)
    - [x] Implement process block → EcosystemConfig mapping

- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Config Conversion & Import Tools

- [x] Task: Implement `aegis adapt` (Aegisfile → TOML/YAML)
    - [x] Write tests for Aegisfile → TOML conversion
    - [x] Implement adapter that serializes internal config to TOML
    - [x] Write tests for Aegisfile → YAML conversion
    - [x] Implement YAML serializer

- [x] Task: Implement `aegis import --from nginx`
    - [x] Write tests for basic nginx.conf parsing (server blocks, locations)
    - [x] Implement nginx config lexer/parser (subset: server, location, proxy_pass, root, listen)
    - [x] Write tests for nginx → Aegisfile conversion
    - [x] Implement nginx-to-Aegisfile translator
    - [x] Write tests for unsupported directive warnings

- [x] Task: Implement `aegis fmt` (auto-formatter)
    - [x] Write tests for consistent indentation (4 spaces inside blocks)
    - [x] Write tests for directive ordering (standard order within blocks)
    - [x] Implement formatter that parses and re-emits Aegisfile

- [x] Task: Implement `aegis validate`
    - [x] Write tests for syntax error detection
    - [x] Write tests for semantic validation (unknown directives, invalid ports, etc.)
    - [x] Implement validator with human-readable error output

- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Editor Support & Integration

- [x] Task: Create VS Code/TextMate syntax grammar
    - [x] Write Aegisfile.tmLanguage.json for syntax highlighting
    - [x] Package as VS Code extension skeleton (aegisfile-vscode)
    - [x] Test highlighting for all directive types

- [x] Task: Integrate Aegisfile into CLI
    - [x] Write tests for auto-detection: Aegisfile > aegis.toml > aegis.yaml
    - [x] Implement config file discovery chain in CLI startup
    - [x] Write tests for --config flag override
    - [x] Write tests for mixed mode (Aegisfile for proxy, ecosystem.toml for processes)

- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
