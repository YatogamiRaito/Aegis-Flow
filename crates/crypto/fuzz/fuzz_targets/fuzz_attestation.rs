#![no_main]
use libfuzzer_sys::fuzz_target;
use aegis_crypto::attestation::AttestationQuote;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize the random data into an AttestationQuote
    let _ = AttestationQuote::from_bytes(data);
});
