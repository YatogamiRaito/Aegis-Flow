# RFC-001: Hybrid Kyber+X25519 Integration Strategy

**Status:** Approved  
**Date:** 2025-12-25  
**Author:** Aegis-Flow Team

## Summary

This RFC defines the hybrid key exchange strategy combining classical X25519 with post-quantum Kyber-768 for "Harvest Now, Decrypt Later" (HNDL) protection.

## Motivation

Quantum computers threaten current asymmetric cryptography. By combining:
- **X25519**: Proven classical security
- **Kyber-768**: NIST FIPS 203 post-quantum KEM

We achieve hybrid security: even if one algorithm is broken, the other provides protection.

## Design

### Key Exchange Flow

```
Client                                    Server
  |                                         |
  |  1. Generate hybrid keypair             |
  |     (x25519_sk, kyber_sk)               |
  |     (x25519_pk, kyber_pk)               |
  |                                         |
  |  2. ClientHello + hybrid_pk ----------> |
  |                                         |
  |                    3. Encapsulate       |
  |                       x25519_ss = DH    |
  |                       kyber_ss = Encap  |
  |                                         |
  |  <-------- 4. ServerHello + ciphertext  |
  |                                         |
  |  5. Decapsulate                         |
  |     x25519_ss = DH(sk, ephemeral_pk)    |
  |     kyber_ss = Decap(ct, kyber_sk)      |
  |                                         |
  |  6. shared_secret = HKDF(x25519_ss || kyber_ss)
  |                                         |
```

### Shared Secret Derivation

```rust
fn derive_shared_secret(x25519_ss: &[u8], kyber_ss: &[u8]) -> [u8; 32] {
    let mut ikm = Vec::with_capacity(64);
    ikm.extend_from_slice(x25519_ss);  // 32 bytes
    ikm.extend_from_slice(kyber_ss);   // 32 bytes
    
    // HKDF-SHA256 extract and expand
    hkdf_sha256(&ikm, b"aegis-flow-hybrid-kex-v1")
}
```

### State Machine

```
┌─────────────┐
│    Init     │
└──────┬──────┘
       │ generate_keypair()
       ▼
┌─────────────┐
│  KeysReady  │
└──────┬──────┘
       │ receive_peer_public_key()
       ▼
┌─────────────┐
│ Encapsulate │ ◄── Client path
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Complete   │
└─────────────┘
```

## Security Considerations

1. **Algorithm Independence**: If Kyber is broken, X25519 still protects. If X25519 is broken, Kyber still protects.
2. **No Downgrade**: Hybrid mode is mandatory; pure classical not allowed.
3. **Constant Time**: All operations must be constant-time to prevent timing attacks.
4. **Zeroization**: Secret keys must be zeroized after use.

## Test Plan

1. Unit tests for keypair generation
2. Unit tests for encapsulate/decapsulate round-trip
3. Test shared secret consistency between client/server
4. Benchmark: Target <2ms overhead for handshake

## Implementation Status

- [x] Basic HybridKeyExchange struct
- [x] Keypair generation (X25519 + Kyber-768)
- [x] Encapsulation
- [ ] Full decapsulation with X25519 (needs StaticSecret)
- [ ] HKDF integration
- [ ] Formal verification with Kani
