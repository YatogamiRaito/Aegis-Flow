# Contributing to Aegis-Flow

Thank you for your interest in contributing to Aegis-Flow! This document provides guidelines for contributing.

## Development Setup

### Prerequisites

- Rust 1.83+ (Edition 2024)
- Cargo
- Docker (for container builds)

### Building

```bash
# Clone the repository
git clone https://github.com/YatogamiRaito/Aegis-Flow.git
cd Aegis-Flow

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench
```

## Code Style

### Formatting
All code must pass `cargo fmt`:
```bash
cargo fmt --all -- --check
```

### Linting
All code must pass `cargo clippy` without warnings:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Formatting, no code change
- `refactor:` Code restructuring
- `perf:` Performance improvements
- `test:` Adding/updating tests
- `chore:` Maintenance tasks

## Pull Request Process

1. **Fork & Branch**: Create a feature branch from `main`
2. **Implement**: Make your changes with tests
3. **Verify**: Run `cargo test --workspace` and `cargo clippy`
4. **Document**: Update CHANGELOG.md if needed
5. **Submit**: Open a PR with a clear description

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `aegis-common` | Shared types and errors |
| `aegis-crypto` | PQC key exchange, encryption |
| `aegis-energy` | Carbon API clients |
| `aegis-genomics` | Genomic data processing |
| `aegis-plugins` | WASM plugin system |
| `aegis-proxy` | HTTP/2/3 proxy server |
| `aegis-telemetry` | Energy metrics |

## Testing

- Unit tests: `cargo test -p <crate>`
- Integration tests: `cargo test --test <test_name>`
- All tests: `cargo test --workspace`

## Questions?

Open an issue with the `question` label.
