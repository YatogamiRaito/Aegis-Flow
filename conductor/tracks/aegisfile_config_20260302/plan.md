# Implementation Plan: Aegisfile — Simple Configuration Format (v0.24.0)

## Phase 1: Aegisfile Parser

- [ ] Task: Create Aegisfile parser (`crates/config/src/aegisfile.rs`)
    - [ ] Write tests for lexer: tokenize braces, strings, identifiers, comments, newlines
    - [ ] Implement lexer using logos or hand-written scanner
    - [ ] Write tests for parser: top-level domain blocks
    - [ ] Implement recursive-descent parser for `domain { directives }`
    - [ ] Write tests for nested directive blocks (`reverse_proxy { ... }`)
    - [ ] Implement nested block parsing
    - [ ] Write tests for quoted strings, unquoted arguments, multi-word values
    - [ ] Write tests for comment handling (# inline, # full line)

- [ ] Task: Implement Aegisfile AST
    - [ ] Write tests for AST node types (Site, Directive, Matcher, Block, Argument)
    - [ ] Implement AST data structures
    - [ ] Write tests for AST-to-ProxyConfig conversion
    - [ ] Implement AST → internal ProxyConfig mapper
    - [ ] Write tests for error reporting with line/column numbers
    - [ ] Implement error messages with context and suggestions

- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Directive Implementations

- [ ] Task: Implement core directives
    - [ ] Write tests for `reverse_proxy` directive parsing (path matcher + upstream)
    - [ ] Implement reverse_proxy → ProxyConfig location mapping
    - [ ] Write tests for `root` and `file_server` directives
    - [ ] Implement static file config generation
    - [ ] Write tests for `redirect` directive (URL + status code)
    - [ ] Write tests for `rewrite` directive (pattern + replacement)
    - [ ] Write tests for `respond` directive (body + status)

- [ ] Task: Implement security directives
    - [ ] Write tests for `rate_limit` directive parsing
    - [ ] Write tests for `basicauth` directive with user block
    - [ ] Write tests for `jwt_auth` directive with JWKS config
    - [ ] Write tests for `header` directive (add/remove/set)
    - [ ] Implement security directive → config mapping

- [ ] Task: Implement matcher syntax
    - [ ] Write tests for path matchers (/api/*, *.php)
    - [ ] Write tests for named matchers (@api { path /api/* })
    - [ ] Write tests for method matchers (@get { method GET })
    - [ ] Write tests for remote_ip matchers
    - [ ] Implement matcher resolution engine

- [ ] Task: Implement `process` blocks for PM2 integration
    - [ ] Write tests for process block parsing (command, instances, env, max_memory)
    - [ ] Implement process block → EcosystemConfig mapping

- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Config Conversion & Import Tools

- [ ] Task: Implement `aegis adapt` (Aegisfile → TOML/YAML)
    - [ ] Write tests for Aegisfile → TOML conversion
    - [ ] Implement adapter that serializes internal config to TOML
    - [ ] Write tests for Aegisfile → YAML conversion
    - [ ] Implement YAML serializer

- [ ] Task: Implement `aegis import --from nginx`
    - [ ] Write tests for basic nginx.conf parsing (server blocks, locations)
    - [ ] Implement nginx config lexer/parser (subset: server, location, proxy_pass, root, listen)
    - [ ] Write tests for nginx → Aegisfile conversion
    - [ ] Implement nginx-to-Aegisfile translator
    - [ ] Write tests for unsupported directive warnings

- [ ] Task: Implement `aegis fmt` (auto-formatter)
    - [ ] Write tests for consistent indentation (4 spaces inside blocks)
    - [ ] Write tests for directive ordering (standard order within blocks)
    - [ ] Implement formatter that parses and re-emits Aegisfile

- [ ] Task: Implement `aegis validate`
    - [ ] Write tests for syntax error detection
    - [ ] Write tests for semantic validation (unknown directives, invalid ports, etc.)
    - [ ] Implement validator with human-readable error output

- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Editor Support & Integration

- [ ] Task: Create VS Code/TextMate syntax grammar
    - [ ] Write Aegisfile.tmLanguage.json for syntax highlighting
    - [ ] Package as VS Code extension skeleton (aegisfile-vscode)
    - [ ] Test highlighting for all directive types

- [ ] Task: Integrate Aegisfile into CLI
    - [ ] Write tests for auto-detection: Aegisfile > aegis.toml > aegis.yaml
    - [ ] Implement config file discovery chain in CLI startup
    - [ ] Write tests for --config flag override
    - [ ] Write tests for mixed mode (Aegisfile for proxy, ecosystem.toml for processes)

- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)
