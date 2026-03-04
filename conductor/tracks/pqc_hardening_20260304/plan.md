# Track Plan: PQC Data Plane Security Hardening (v0.30.0)

## Phase 1: Kriptografik KDF Düzeltmesi (P0 — Kritik)
- [ ] Task: HKDF-SHA256 ile `HybridSharedSecret::combine()` yeniden implementasyonu (TDD)
    - [ ] Test: `test_combine_uses_hkdf_not_concat` — çıktı raw concat'ten farklı olmalı
    - [ ] Test: `test_combine_deterministic` — aynı input her zaman aynı PRK üretmeli
    - [ ] Test: `test_combine_ietf_hybrid_draft_compliance` — draft-ietf-tls-hybrid-design-10 vektörleri
    - [ ] Implement: `combine()` fonksiyonunu HKDF extract ile yeniden yaz (IKM=concat, salt=None, info=label)
    - [ ] Verify: mevcut `test_full_key_exchange_roundtrip` hâlâ geçmeli (secret eşitliği korunmalı)
- [ ] Task: HKDF-SHA256 ile `HybridSharedSecret::derive_key()` yeniden implementasyonu (TDD)
    - [ ] Test: `test_derive_key_uses_hkdf` — çıktı XOR tabanlı eski çıktıdan farklı olmalı
    - [ ] Test: `test_derive_key_expansion_length` — tam 32 byte üretmeli
    - [ ] Test: `test_derive_key_different_contexts_produce_different_keys` — farklı info/context ile farklı key
    - [ ] Implement: XOR döngüsünü tamamen kaldır, `Hkdf::<Sha256>::new()` + `.expand()` kullan
    - [ ] Verify: `cargo test -p aegis-crypto` — tüm testler geçmeli
- [ ] Task: Benchmark regresyon kontrolü
    - [ ] `cargo bench --bench pqc_handshake` çalıştır
    - [ ] Sonuçları `derive_key` benchmark'ı ile önceki baseline ile karşılaştır
    - [ ] Kabul: %5'den fazla regresyon yok
- [ ] Task: Conductor - User Manual Verification 'Kriptografik KDF Düzeltmesi' (Protocol in workflow.md)

## Phase 2: Secret Zeroization & Memory Safety (P1)
- [ ] Task: `zeroize` crate'ini workspace bağımlılığına ekle
    - [ ] `Cargo.toml` workspace dependencies'e `zeroize = { version = "1.8", features = ["derive"] }` ekle
    - [ ] `crates/crypto/Cargo.toml`'e `zeroize.workspace = true` ekle
- [ ] Task: `HybridSecretKey` üzerinde `ZeroizeOnDrop` implementasyonu (TDD)
    - [ ] Test: `test_secret_key_zeroed_after_drop` — drop sonrası bellek sıfır kontrolü
    - [ ] Implement: `#[derive(ZeroizeOnDrop)]` ekle, `x25519` alanı için `#[zeroize(skip)]` (kendi zeroize eder)
    - [ ] Verify: mevcut `test_secret_key_debug_redacts` hâlâ geçmeli
- [ ] Task: `HybridSharedSecret` ve `EncryptionKey` — `Zeroize` trait'e geçiş (TDD)
    - [ ] Test: `test_shared_secret_zeroed_after_drop` — drop sonrası inner buffer sıfır
    - [ ] Test: `test_encryption_key_zeroed_after_drop` — key alanı drop sonrası sıfır
    - [ ] Implement: manuel `Drop` impl'lerini `Zeroize` + `ZeroizeOnDrop` derive ile değiştir
    - [ ] Verify: `cargo test -p aegis-crypto` — tüm testler geçmeli
- [ ] Task: Conductor - User Manual Verification 'Secret Zeroization & Memory Safety' (Protocol in workflow.md)

## Phase 2.5: Secure Data Plane Düzeltmeleri (Audit'ten)
- [ ] Task: `EncryptedStream::poll_write` — `expect()` panik yerine `io::Error` döndür (TDD)
    - [ ] Test: `test_encrypt_failure_returns_io_error` — encryption hatası panic değil error döndürmeli
    - [ ] Implement: `stream.rs:235` `.expect(...)` → `.map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Encryption failed: {}", e)))?`
    - [ ] Verify: mevcut `test_stream_roundtrip` ve `test_detect_tampering` hâlâ geçmeli
- [ ] Task: mTLS `complete_handshake` — orijinal `server_state` saklama düzeltmesi (TDD)
    - [ ] Test: `test_mtls_handshake_uses_original_server_state` — accept_connection'dan gelen state ile complete_handshake'in aynı shared secret ürettiğini doğrula
    - [ ] Implement: `MtlsAuthenticator` içindeki `clients` HashMap'ine `ServerHandshakeState` sakla, `complete_handshake`'te yeniden keypair üretme
    - [ ] Verify: `cargo test -p aegis-crypto` — tüm mTLS testleri geçmeli
- [ ] Task: `EncryptedStream` — client/server yönü için ayrı key türetme (TDD)
    - [ ] Test: `test_bidirectional_with_derived_keys` — client_key ≠ server_key ama her iki yönde doğru çalışmalı
    - [ ] Implement: `new()` constructor'a yön parametresi ekle veya HKDF ile iki farklı key türet (info=`"client"` / `"server"`)
    - [ ] Verify: mevcut `test_stream_roundtrip` ve `test_multiple_writes` geçmeli
- [ ] Task: Conductor - User Manual Verification 'Secure Data Plane Düzeltmeleri' (Protocol in workflow.md)

## Phase 3: API Tutarlılığı & Trait Uyumu (P1)
- [ ] Task: `KeyExchange` trait signature güncelleme
    - [ ] Analiz: mevcut trait signature'ını `HybridKeyExchange` API'si ile uyumlu hale getir
    - [ ] Implement: `generate_keypair()` dönüş tipi uyumlu hale getir (associated types)
- [ ] Task: `HybridKeyExchange` üzerinde `KeyExchange` trait implementasyonu (TDD)
    - [ ] Test: `test_hybrid_kex_implements_key_exchange_trait` — trait üzerinden çağrı yapılabilmeli
    - [ ] Test: `test_trait_object_safety_with_hybrid` — `dyn KeyExchange<...>` object safety
    - [ ] Test: `test_trait_roundtrip` — trait API üzerinden full handshake
    - [ ] Implement: `impl KeyExchange for HybridKeyExchange` bloğu
    - [ ] Verify: mevcut `test_trait_is_object_safe` hâlâ geçmeli
- [ ] Task: `AsRef<[u8]> for HybridPublicKey` düzeltmesi (TDD)
    - [ ] Test: `test_public_key_as_ref_returns_full_bytes` — tüm baytlar (X25519+ML-KEM) dönmeli
    - [ ] Implement: `AsRef` implantasyonunu kaldır veya `to_bytes()` çıktısıyla değiştir
    - [ ] Implement: `x25519_bytes(&self) -> &[u8; 32]` accessor ekle
    - [ ] Verify: `cargo clippy -p aegis-crypto` — sıfır uyarı
- [ ] Task: Conductor - User Manual Verification 'API Tutarlılığı & Trait Uyumu' (Protocol in workflow.md)

## Phase 4: TEE Attestation Güvenlik Sertleştirmesi (P1)
- [ ] Task: Attestation verify stub'larını güvenli hata dönen fonksiyonlara dönüştür (TDD)
    - [ ] Test: `test_verify_sgx_returns_not_implemented` — `Err(NotImplemented)` dönmeli
    - [ ] Test: `test_verify_tdx_returns_not_implemented` — aynı
    - [ ] Test: `test_verify_sevsnp_returns_not_implemented` — aynı
    - [ ] Test: `test_verify_simulation_still_passes` — `TeePlatform::None` hâlâ `Ok(true)`
    - [ ] Implement: üç platform verify fonksiyonunu `Err(AegisError::NotImplemented(...))` ile değiştir
    - [ ] Implement: `verify_quote()` fonksiyonuna WARN log ekle (real platform ama stub verify)
- [ ] Task: `tee-real` feature flag hazırlığı
    - [ ] `crates/crypto/Cargo.toml`'e `[features]` bölümü ekle: `tee-real = []`
    - [ ] `#[cfg(feature = "tee-real")]` ile gerçek verify modülü için placeholder yap
    - [ ] Verify: `cargo check -p aegis-crypto --all-features` — derleme başarılı
- [ ] Task: `TeeCapabilities` runtime CPUID detection (TDD)
    - [ ] `Cargo.toml`'e `raw-cpuid = "11"` ekle
    - [ ] Test: `test_detect_without_env_vars` — env var yokken doğru davranış
    - [ ] Test: `test_env_var_override_in_test` — `#[cfg(test)]` altında env var çalışmalı
    - [ ] Implement: `detect()` fonksiyonu gerçek CPUID leaf sorgulasın, fallback env var
    - [ ] Verify: `cargo test -p aegis-crypto` — tüm testler geçmeli
- [ ] Task: Conductor - User Manual Verification 'TEE Attestation Güvenlik Sertleştirmesi' (Protocol in workflow.md)

## Phase 5: Gramine Manifest & Enclave Sertleştirme (P1)
- [ ] Task: Gramine manifest template parametrizasyonu
    - [ ] `sgx.debug = true` → `sgx.debug = {{ debug_mode }}` dönüştür
    - [ ] `sgx.enclave_size = "512M"` → `sgx.enclave_size = "{{ enclave_size }}"` dönüştür
    - [ ] `/etc` fs.mount entry'sini kaldır
    - [ ] Verify: manifest template sözdizimsel olarak geçerli (Jinja2 lint)
- [ ] Task: `build.sh` parametrik hale getir
    - [ ] `--release` ve `--debug` modlarını argüman olarak al
    - [ ] Varsayılan değerleri belirle (debug_mode=false, enclave_size=256M)
    - [ ] Verify: `shellcheck gramine/build.sh` temiz geçmeli
- [ ] Task: Conductor - User Manual Verification 'Gramine Manifest & Enclave Sertleştirme' (Protocol in workflow.md)

## Phase 6: Cipher Nonce Güvenliği (P2)
- [ ] Task: Nonce overflow koruması eklenmesi (TDD)
    - [ ] Test: `test_nonce_overflow_returns_error` — nonce_counter u64::MAX-1'de hata döndürmeli
    - [ ] Test: `test_nonce_remaining_accessor` — kalan nonce sayısını doğru döndürmeli
    - [ ] Implement: `encrypt()` fonksiyonunda overflow kontrolü ekle
    - [ ] Implement: `nonce_remaining(&self) -> u64` accessor'ı ekle
    - [ ] Verify: cargo test -p aegis-crypto
- [ ] Task: Conductor - User Manual Verification 'Cipher Nonce Güvenliği' (Protocol in workflow.md)

## Phase 6.5: Kusursuzluk Fazı — 8-9/10 Alanları 10/10'a Çıkarma

### PQC Algoritma Seçimi (8→10)
- [ ] Task: Algoritma negotiation mekanizması (TDD)
    - [ ] Test: `test_algorithm_negotiation_selects_best` — client/server ortak en güçlü algoritmayı seçmeli
    - [ ] Test: `test_negotiation_fallback` — ML-KEM-1024 desteklenmiyorsa ML-KEM-768'e düşmeli
    - [ ] Implement: `NegotiatedAlgorithm` struct ve `negotiate()` fonksiyonu `tls.rs`'e ekle
    - [ ] Implement: `PqcAlgorithm` enum'a `supported_algorithms()` ve `strength_order()` metotları ekle
- [ ] Task: ML-KEM-1024 end-to-end doğrulama (TDD)
    - [ ] Test: `test_mlkem_1024_full_handshake` — ML-KEM-1024 ile tam handshake roundtrip
    - [ ] Test: `test_mlkem_1024_key_sizes` — doğru publickey/ciphertext boyutları
    - [ ] Implement: `HybridKeyExchange::new_with_level(SecurityLevel::High)` constructor ekle
    - [ ] Verify: benchmark ile ML-KEM-768 ve 1024 karşılaştırma

### Symmetric Encryption (8→10)
- [ ] Task: Key rotation mekanizması (TDD)
    - [ ] Test: `test_key_rotation_produces_new_key` — rotate sonrası yeni key farklı olmalı
    - [ ] Test: `test_old_key_still_decrypts_old_data` — rotation sırasında graceful transition
    - [ ] Test: `test_rotation_resets_nonce_counter` — yeni key ile nonce counter sıfırdan başlamalı
    - [ ] Implement: `Cipher::rotate_key(&mut self, new_shared_secret: &[u8])` metodu
    - [ ] Implement: `KeyRotationPolicy` struct (max_bytes, max_messages, max_duration)

### EncryptedStream Cipher Agility (9→10)
- [ ] Task: `EncryptedStream` ChaCha20-Poly1305 desteği (TDD)
    - [ ] Test: `test_encrypted_stream_chacha20_roundtrip` — ChaCha20 ile tam okuma/yazma
    - [ ] Test: `test_encrypted_stream_cipher_mismatch_fails` — farklı cipher ile decrypt başarısız
    - [ ] Implement: `EncryptedStream::new_with_cipher(stream, key, CipherAlgorithm)` constructor
    - [ ] Implement: frame header'a cipher indicator byte ekle (backward compat)
    - [ ] Verify: mevcut AES-GCM testleri hâlâ geçmeli

### Test Kapsamı (9→10)
- [ ] Task: Property-based testing (proptest)
    - [ ] `Cargo.toml`'e `proptest = "1.5"` dev-dependency ekle
    - [ ] Test: `proptest_encrypt_decrypt_roundtrip` — rastgele plaintext/key çiftleri ile roundtrip
    - [ ] Test: `proptest_key_exchange_always_agrees` — rastgele keypair'ler ile her zaman aynı shared secret
    - [ ] Test: `proptest_hkdf_output_never_zero` — HKDF çıktısı asla all-zero olmamalı
- [ ] Task: Cargo-fuzz entegrasyonu
    - [ ] `fuzz/` dizini oluştur, `cargo-fuzz` altyapısı kur
    - [ ] `fuzz_hybrid_kex_decapsulate` — rasgele ciphertext'le decapsulate crash etmemeli
    - [ ] `fuzz_cipher_decrypt` — rasgele input ile decrypt crash etmemeli (graceful error)
    - [ ] `fuzz_attestation_from_bytes` — rasgele bytes ile `AttestationQuote::from_bytes` crash yok

### Benchmark CI Otomasyonu (7→10)
- [ ] Task: Benchmark regression CI pipeline
    - [ ] `criterion`'a `--save-baseline` ve `--baseline` flag'leri ile CI script yaz
    - [ ] `.github/workflows/bench.yml` — her PR'da benchmark çalışsın
    - [ ] Threshold: %5'ten fazla regresyon varsa CI fail etsin
    - [ ] Flamegraph: `cargo flamegraph --bench pqc_handshake` profiling script ekle
    - [ ] Verify: CI pipeline'da benchmark raporu artifact olarak kaydedilmeli

- [ ] Task: Conductor - User Manual Verification 'Kusursuzluk Fazı' (Protocol in workflow.md)

## Phase 7: Final Validation & Release (v0.30.0)
- [ ] Task: Kapsamlı test suite çalıştırma
    - [ ] `cargo test -p aegis-crypto` — tüm testler geçmeli (mevcut 107 + yeni ~25)
    - [ ] `cargo clippy -p aegis-crypto --all-targets --all-features` — sıfır uyarı
    - [ ] `cargo fmt -p aegis-crypto -- --check` — formatting temiz
- [ ] Task: Coverage raporu
    - [ ] `cargo tarpaulin -p aegis-crypto` — >95% coverage hedefi
    - [ ] Coverage düşüklüğü varsa ek testler yaz
- [ ] Task: Güvenlik audit
    - [ ] `cargo audit` — sıfır güvenlik açığı
    - [ ] `cargo deny check` — sıfır ihlal
    - [ ] Manuel `unsafe` review (varsa her blok için SAFETY comment)
- [ ] Task: Benchmark final raporu
    - [ ] `cargo bench --bench pqc_handshake` — baseline ile karşılaştır
    - [ ] `cargo bench --bench mldsa_signing` — regresyon yok
    - [ ] Sonuçları `CHANGELOG.md`'ye kaydet
- [ ] Task: Release v0.30.0
    - [ ] `Cargo.toml` workspace version'ı `0.30.0` olarak güncelle
    - [ ] CHANGELOG.md'ye v0.30.0 entry'si ekle
    - [ ] `git tag -a v0.30.0 -m "PQC Data Plane Security Hardening"`
    - [ ] SBOM: `syft` ile generate et
- [ ] Task: Conductor - User Manual Verification 'Final Validation & Release' (Protocol in workflow.md)
