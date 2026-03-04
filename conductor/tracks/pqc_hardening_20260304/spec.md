# Track Specification: PQC Data Plane Security Hardening (v0.30.0)

## 1. Goal

Bu track, "Core TEE-Native PQC Data Plane" audit raporunda tespit edilen **2 kritik, 4 yüksek ve 3 orta öncelikli** güvenlik ve mühendislik açığını kaparir. Hedef: `aegis-crypto` crate'ini **2026 NIST/IETF standartlarına**, Google Production kalite çıtasına ve mevcut workflow'un zorunlu kıldığı Kani formal verification seviyesine çıkarmak.

**Audit Referansı:** `conductor/tracks/pqc_hardening_20260304/` — Kaynak skor 6.3/10 → Hedef skor 10/10.

---

## 2. Functional Requirements

### FR-1: HKDF-SHA256 Tabanlı Key Derivation (P0 — Kritik)

- **FR-1.1:** `HybridSharedSecret::combine()` fonksiyonu, X25519 ve ML-KEM shared secret'lerini HKDF-SHA256 extract+expand ile tek bir PRK'ya birleştirmeli. IKM = `concat(X25519_SS || MLKEM_SS)`, info = `"aegis-flow-hybrid-kex-v1"`.
- **FR-1.2:** `HybridSharedSecret::derive_key()` fonksiyonu, mevcut XOR tabanlı implementasyonu tamamen kaldırarak HKDF-SHA256 expand kullanmalı. Info = `"aegis-flow-session-key-v1"`.
- **FR-1.3:** IETF `draft-ietf-tls-hybrid-design-10` formatına uyum sağlanmalı.
- **FR-1.4:** `combine()` çıktısı, "indistinguishable from random" özelliğe sahip olmalı; Kani harness ile bu doğrulanmalı.

### FR-2: Kriptografik Secret Zeroization (P1 — Yüksek)

- **FR-2.1:** `zeroize` crate'i workspace bağımlılığı olarak eklenmeli (`zeroize = { version = "1.8", features = ["derive"] }`).
- **FR-2.2:** `HybridSecretKey` üzerinde `ZeroizeOnDrop` derive uygulanmalı. `mlkem: Vec<u8>` alanı drop sırasında sıfırlanmalı.
- **FR-2.3:** `HybridSharedSecret` mevcut manuel `Drop` implementasyonu `zeroize` crate'inin `Zeroize` trait'i ile değiştirilmeli (compiler fence korumalı).
- **FR-2.4:** `EncryptionKey` (cipher.rs) mevcut manuel Drop'u da `zeroize`'a geçirilmeli.

### FR-3: KeyExchange Trait Uyumluluğu (P1 — Yüksek)

- **FR-3.1:** `HybridKeyExchange` struct'ı `KeyExchange` trait'ini implement etmeli.
- **FR-3.2:** Trait'in dönüş tipleri (`PublicKey`, `SharedSecret`) `HybridPublicKey` ve `HybridSharedSecret` olarak set edilmeli.
- **FR-3.3:** `generate_keypair()` signature, trait ile uyumlu hale getirilmeli — secret key serialization (bytes) desteklenmeli.
- **FR-3.4:** `dyn KeyExchange<...>` object safety test'i, `HybridKeyExchange` üzerinde doğrulanmalı.

### FR-4: TEE Attestation — Güvenli Stub Davranışı (P1 — Yüksek)

- **FR-4.1:** `verify_sgx_quote()`, `verify_tdx_quote()`, `verify_sev_snp_quote()` fonksiyonları `Ok(true)` yerine `Err(AegisError::NotImplemented(...))` döndürmeli.
- **FR-4.2:** Yalnızca `TeePlatform::None` (simülasyon) modunda `Ok(true)` geçerli kalmalı.
- **FR-4.3:** `verify_quote()` API'si, caller'a platform tipi hakkında uyarı loglamalı (WARN level).
- **FR-4.4:** `AttestationProvider` üzerinde `#[cfg(feature = "tee-real")]` feature flag hazırlanmalı (gelecekte gerçek DCAP bağlantısı için).

### FR-5: Gramine Manifest Güvenlik Sertleştirme (P1 — Yüksek)

- **FR-5.1:** `sgx.debug` sabitlenmiş `true` yerine `{{ debug_mode }}` template değişkenine dönüştürülmeli.
- **FR-5.2:** `fs.mounts` içindeki `/etc` mount'u kaldırılmalı; yalnızca `sgx.allowed_files`'ta listelenen spesifik dosyalar erişilebilir olmalı.
- **FR-5.3:** `sgx.enclave_size` ayarı `{{ enclave_size }}` template değişkenine dönüştürülmeli (varsayılan "256M").
- **FR-5.4:** `build.sh`'ye `--release` ve `--no-debug` flag'leri eklenebilir parametrik hale getirilmeli.

### FR-6: HybridPublicKey AsRef Düzeltmesi (P2 — Orta)

- **FR-6.1:** `AsRef<[u8]> for HybridPublicKey` implantasyonu kaldırılmalı veya `to_bytes()` çıktısını döndürecek şekilde düzeltilmeli.
- **FR-6.2:** Sadece X25519 public key'e erişim gereken yerler için `x25519_bytes()` accessor'ı eklenebilir.

### FR-7: Nonce Overflow Koruması (P2 — Orta)

- **FR-7.1:** `Cipher::encrypt()` fonksiyonunda nonce counter `u64::MAX - 1`'e ulaştığında `Err(AegisError::Crypto("Nonce space exhausted"))` döndürülmeli.
- **FR-7.2:** `nonce_remaining()` accessor'ı eklenmeli.

### FR-8: TeeCapabilities — Runtime CPUID Detection (P2 — Orta)

- **FR-8.1:** `raw-cpuid` crate bağımlılığı eklenip, `TeeCapabilities::detect()` fonksiyonunda gerçek CPUID leaf kontrolü yapılmalı.
- **FR-8.2:** Env var override'ları yalnızca `#[cfg(test)]` altında aktif kalmalı.
- **FR-8.3:** Fallback: CPUID erişimi mümkün değilse (container vb.) env var'a düşülmeli.

---

## 3. Non-Functional Requirements

- **NFR-1:** Tüm değişiklikler mevcut 107 crypto test'ini kırmamalı.
- **NFR-2:** Eklenen yeni testlerle birlikte `aegis-crypto` crate'inin toplam test coverage'ı >95% olmalı.
- **NFR-3:** `cargo clippy --all-targets --all-features` sıfır uyarı ile geçmeli.
- **NFR-4:** `cargo audit` ve `cargo deny check` temiz geçmeli.
- **NFR-5:** Benchmark regresyonu olmamalı: `hybrid_full_handshake` ≤ mevcut ortalamanın %5 üzeri.
- **NFR-6:** Sıfır `unsafe` blok (veya her `unsafe` blok için `// SAFETY:` justification).

---

## 4. Acceptance Criteria

1. `HybridSharedSecret::derive_key()` XOR kullanmıyor, HKDF-SHA256 kullanıyor.
2. `HybridSharedSecret::combine()` raw concat yerine HKDF extract yapıyor.
3. Tüm secret key struct'ları `zeroize` crate ile temizleniyor.
4. `HybridKeyExchange` `KeyExchange` trait'ini implemente ediyor.
5. Attestation verify stub'ları güvenli hata döndürüyor (simülasyon hariç).
6. Gramine manifest debug parametrik, `/etc` mount kaldırılmış.
7. `AsRef<[u8]>` düzeltilmiş, nonce overflow koruması eklenmiş.
8. CPUID-tabanlı TEE detection çalışıyor.
9. Tüm mevcut testler + en az 25 yeni test geçiyor.
10. `cargo bench --bench pqc_handshake` regresyon yok.

---

## 5. Out of Scope

- Gerçek Intel DCAP / AMD SEV-SNP quote verification backend'i (ayrı track).
- Veraison entegrasyonu (ayrı track).
- Custom `rustls::CryptoProvider` implementasyonu (TLS track).
- SLSA L3 pipeline tamamlama (CI/CD track).
