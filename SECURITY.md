# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.10.x  | :white_check_mark: |
| < 0.10  | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: security@aegis-flow.io

Include:
- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Any potential mitigations

We will respond within 48 hours and work with you to understand and address the issue.

## Security Features

Aegis-Flow implements several security measures:

- **Post-Quantum Cryptography**: ML-KEM-768 + X25519 hybrid key exchange
- **Memory Safety**: Written in Rust with no unsafe code in core paths
- **Dependency Auditing**: Regular `cargo audit` and `cargo deny` checks
- **TEE Support**: Gramine SGX manifests for enclave deployment

## Known Issues

See [RUSTSEC advisories](https://rustsec.org/) for any known issues in dependencies.
