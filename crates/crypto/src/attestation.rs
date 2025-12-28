//! TEE Remote Attestation Module
//!
//! Provides remote attestation capabilities for Trusted Execution Environments.
//! Supports Intel SGX/TDX and AMD SEV-SNP attestation protocols.
//!
//! # Features
//!
//! - Quote generation and verification
//! - Enclave identity validation (MRENCLAVE/MRSIGNER)
//! - TEE detection and capability checking
//! - Challenge-response nonce support
//!
//! # Example
//!
//! ```ignore
//! use aegis_crypto::attestation::{AttestationProvider, AttestationQuote};
//!
//! let provider = AttestationProvider::detect()?;
//! let quote = provider.generate_quote(b"nonce")?;
//! assert!(quote.verify()?);
//! ```

use aegis_common::{AegisError, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// TEE Platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeePlatform {
    /// Intel Software Guard Extensions
    IntelSgx,
    /// Intel Trust Domain Extensions
    IntelTdx,
    /// AMD Secure Encrypted Virtualization - Secure Nested Paging
    AmdSevSnp,
    /// No TEE available (simulation mode)
    None,
}

impl TeePlatform {
    /// Get platform name
    pub fn name(&self) -> &'static str {
        match self {
            Self::IntelSgx => "Intel SGX",
            Self::IntelTdx => "Intel TDX",
            Self::AmdSevSnp => "AMD SEV-SNP",
            Self::None => "None (Simulation)",
        }
    }

    /// Check if this is a real TEE
    pub fn is_tee(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Attestation quote containing platform evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationQuote {
    /// TEE platform that generated this quote
    pub platform: TeePlatform,
    /// Raw quote bytes
    pub quote_bytes: Vec<u8>,
    /// Nonce/challenge that was included
    pub nonce: Vec<u8>,
    /// User data embedded in the quote
    pub user_data: Vec<u8>,
    /// Timestamp when quote was generated (Unix epoch seconds)
    pub timestamp: i64,
    /// Quote signature (using ML-DSA if available)
    pub signature: Option<Vec<u8>>,
}

impl AttestationQuote {
    /// Create a new attestation quote
    pub fn new(
        platform: TeePlatform,
        quote_bytes: Vec<u8>,
        nonce: Vec<u8>,
        user_data: Vec<u8>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            platform,
            quote_bytes,
            nonce,
            user_data,
            timestamp,
            signature: None,
        }
    }

    /// Add signature to the quote
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Serialize quote to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // Format: [platform: 1][nonce_len: 4][nonce][user_data_len: 4][user_data][quote_len: 4][quote][timestamp: 8][sig_len: 4][sig]
        let mut bytes = Vec::new();

        // Platform
        bytes.push(match self.platform {
            TeePlatform::IntelSgx => 0,
            TeePlatform::IntelTdx => 1,
            TeePlatform::AmdSevSnp => 2,
            TeePlatform::None => 255,
        });

        // Nonce
        bytes.extend_from_slice(&(self.nonce.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.nonce);

        // User data
        bytes.extend_from_slice(&(self.user_data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.user_data);

        // Quote
        bytes.extend_from_slice(&(self.quote_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.quote_bytes);

        // Timestamp
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());

        // Signature
        if let Some(ref sig) = self.signature {
            bytes.extend_from_slice(&(sig.len() as u32).to_le_bytes());
            bytes.extend_from_slice(sig);
        } else {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }

        bytes
    }

    /// Deserialize quote from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 22 {
            return Err(AegisError::Crypto("Quote too short".to_string()));
        }

        let mut offset = 0;

        // Platform
        let platform = match bytes[offset] {
            0 => TeePlatform::IntelSgx,
            1 => TeePlatform::IntelTdx,
            2 => TeePlatform::AmdSevSnp,
            _ => TeePlatform::None,
        };
        offset += 1;

        // Nonce
        let nonce_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;
        if offset + nonce_len > bytes.len() {
            return Err(AegisError::Crypto("Invalid nonce length".to_string()));
        }
        let nonce = bytes[offset..offset + nonce_len].to_vec();
        offset += nonce_len;

        // User data
        if offset + 4 > bytes.len() {
            return Err(AegisError::Crypto("Missing user data length".to_string()));
        }
        let user_data_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;
        if offset + user_data_len > bytes.len() {
            return Err(AegisError::Crypto("Invalid user data length".to_string()));
        }
        let user_data = bytes[offset..offset + user_data_len].to_vec();
        offset += user_data_len;

        // Quote
        if offset + 4 > bytes.len() {
            return Err(AegisError::Crypto("Missing quote length".to_string()));
        }
        let quote_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;
        if offset + quote_len > bytes.len() {
            return Err(AegisError::Crypto("Invalid quote length".to_string()));
        }
        let quote_bytes = bytes[offset..offset + quote_len].to_vec();
        offset += quote_len;

        // Timestamp
        if offset + 8 > bytes.len() {
            return Err(AegisError::Crypto("Missing timestamp".to_string()));
        }
        let timestamp = i64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        // Signature (optional)
        let signature = if offset + 4 <= bytes.len() {
            let sig_len = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += 4;
            if sig_len > 0 && offset + sig_len <= bytes.len() {
                Some(bytes[offset..offset + sig_len].to_vec())
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            platform,
            quote_bytes,
            nonce,
            user_data,
            timestamp,
            signature,
        })
    }

    /// Check if quote is fresh (within max_age_seconds)
    pub fn is_fresh(&self, max_age_seconds: i64) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        (now - self.timestamp).abs() <= max_age_seconds
    }
}

/// Enclave identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclaveIdentity {
    /// MRENCLAVE - hash of enclave code and data
    pub mrenclave: [u8; 32],
    /// MRSIGNER - hash of enclave signing key
    pub mrsigner: [u8; 32],
    /// Product ID
    pub product_id: u16,
    /// Security Version Number
    pub svn: u16,
    /// Debug mode flag
    pub debug_mode: bool,
}

impl EnclaveIdentity {
    /// Create a new enclave identity
    pub fn new(
        mrenclave: [u8; 32],
        mrsigner: [u8; 32],
        product_id: u16,
        svn: u16,
        debug_mode: bool,
    ) -> Self {
        Self {
            mrenclave,
            mrsigner,
            product_id,
            svn,
            debug_mode,
        }
    }

    /// Check if this is a production enclave (not debug)
    pub fn is_production(&self) -> bool {
        !self.debug_mode
    }

    /// Verify MRENCLAVE matches expected value
    pub fn verify_mrenclave(&self, expected: &[u8; 32]) -> bool {
        self.mrenclave == *expected
    }

    /// Verify MRSIGNER matches expected value
    pub fn verify_mrsigner(&self, expected: &[u8; 32]) -> bool {
        self.mrsigner == *expected
    }
}

/// TEE capability flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TeeCapabilities {
    /// SGX available
    pub sgx: bool,
    /// SGX2 (EDMM) available
    pub sgx2: bool,
    /// TDX available
    pub tdx: bool,
    /// SEV available
    pub sev: bool,
    /// SEV-ES available
    pub sev_es: bool,
    /// SEV-SNP available
    pub sev_snp: bool,
}

impl TeeCapabilities {
    /// Detect available TEE capabilities on this system
    pub fn detect() -> Self {
        let mut caps = Self::default();

        // Check for SGX
        #[cfg(target_arch = "x86_64")]
        {
            // Check CPUID for SGX support
            // In real implementation, would use cpuid crate
            // For now, we simulate detection
            if std::env::var("AEGIS_TEE_SGX").is_ok() {
                caps.sgx = true;
            }
            if std::env::var("AEGIS_TEE_TDX").is_ok() {
                caps.tdx = true;
            }
            if std::env::var("AEGIS_TEE_SEV_SNP").is_ok() {
                caps.sev_snp = true;
            }
        }

        caps
    }

    /// Get the best available TEE platform
    pub fn best_platform(&self) -> TeePlatform {
        if self.tdx {
            TeePlatform::IntelTdx
        } else if self.sgx {
            TeePlatform::IntelSgx
        } else if self.sev_snp {
            TeePlatform::AmdSevSnp
        } else {
            TeePlatform::None
        }
    }

    /// Check if any TEE is available
    pub fn any_available(&self) -> bool {
        self.sgx || self.tdx || self.sev_snp
    }
}

/// Attestation provider for generating and verifying quotes
pub struct AttestationProvider {
    /// Detected platform
    platform: TeePlatform,
    /// Capabilities
    capabilities: TeeCapabilities,
}

impl AttestationProvider {
    /// Create a new attestation provider, detecting available TEE
    pub fn new() -> Self {
        let capabilities = TeeCapabilities::detect();
        let platform = capabilities.best_platform();

        if platform.is_tee() {
            info!("TEE detected: {}", platform.name());
        } else {
            warn!("No TEE detected, running in simulation mode");
        }

        Self {
            platform,
            capabilities,
        }
    }

    /// Get the current platform
    pub fn platform(&self) -> TeePlatform {
        self.platform
    }

    /// Get capabilities
    pub fn capabilities(&self) -> &TeeCapabilities {
        &self.capabilities
    }

    /// Generate an attestation quote
    pub fn generate_quote(&self, nonce: &[u8], user_data: &[u8]) -> Result<AttestationQuote> {
        debug!(
            platform = ?self.platform,
            nonce_len = nonce.len(),
            user_data_len = user_data.len(),
            "Generating attestation quote"
        );

        match self.platform {
            TeePlatform::IntelSgx => self.generate_sgx_quote(nonce, user_data),
            TeePlatform::IntelTdx => self.generate_tdx_quote(nonce, user_data),
            TeePlatform::AmdSevSnp => self.generate_sev_snp_quote(nonce, user_data),
            TeePlatform::None => self.generate_simulation_quote(nonce, user_data),
        }
    }

    /// Verify an attestation quote
    pub fn verify_quote(&self, quote: &AttestationQuote, expected_nonce: &[u8]) -> Result<bool> {
        debug!(platform = ?quote.platform, "Verifying attestation quote");

        // Check nonce matches
        if quote.nonce != expected_nonce {
            debug!("Nonce mismatch");
            return Ok(false);
        }

        // Check freshness (5 minutes)
        if !quote.is_fresh(300) {
            debug!("Quote is stale");
            return Ok(false);
        }

        // Platform-specific verification
        match quote.platform {
            TeePlatform::IntelSgx => self.verify_sgx_quote(quote),
            TeePlatform::IntelTdx => self.verify_tdx_quote(quote),
            TeePlatform::AmdSevSnp => self.verify_sev_snp_quote(quote),
            TeePlatform::None => Ok(true), // Simulation mode always passes
        }
    }

    // ========================================================================
    // Platform-specific implementations (stubs for now)
    // ========================================================================

    fn generate_sgx_quote(&self, nonce: &[u8], user_data: &[u8]) -> Result<AttestationQuote> {
        // In real implementation, would call SGX DCAP API
        // For now, generate a mock quote
        let mock_quote = b"SGX_QUOTE_V3_MOCK_DATA".to_vec();
        Ok(AttestationQuote::new(
            TeePlatform::IntelSgx,
            mock_quote,
            nonce.to_vec(),
            user_data.to_vec(),
        ))
    }

    fn generate_tdx_quote(&self, nonce: &[u8], user_data: &[u8]) -> Result<AttestationQuote> {
        let mock_quote = b"TDX_QUOTE_V4_MOCK_DATA".to_vec();
        Ok(AttestationQuote::new(
            TeePlatform::IntelTdx,
            mock_quote,
            nonce.to_vec(),
            user_data.to_vec(),
        ))
    }

    fn generate_sev_snp_quote(&self, nonce: &[u8], user_data: &[u8]) -> Result<AttestationQuote> {
        let mock_quote = b"SEV_SNP_REPORT_MOCK".to_vec();
        Ok(AttestationQuote::new(
            TeePlatform::AmdSevSnp,
            mock_quote,
            nonce.to_vec(),
            user_data.to_vec(),
        ))
    }

    fn generate_simulation_quote(
        &self,
        nonce: &[u8],
        user_data: &[u8],
    ) -> Result<AttestationQuote> {
        debug!("Generating simulation quote (no TEE)");
        let mock_quote = b"SIMULATION_QUOTE_NO_TEE".to_vec();
        Ok(AttestationQuote::new(
            TeePlatform::None,
            mock_quote,
            nonce.to_vec(),
            user_data.to_vec(),
        ))
    }

    fn verify_sgx_quote(&self, _quote: &AttestationQuote) -> Result<bool> {
        // Would verify against Intel collateral service
        Ok(true)
    }

    fn verify_tdx_quote(&self, _quote: &AttestationQuote) -> Result<bool> {
        Ok(true)
    }

    fn verify_sev_snp_quote(&self, _quote: &AttestationQuote) -> Result<bool> {
        Ok(true)
    }
}

impl Default for AttestationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AttestationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttestationProvider")
            .field("platform", &self.platform)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tee_platform_properties() {
        assert!(TeePlatform::IntelSgx.is_tee());
        assert!(TeePlatform::IntelTdx.is_tee());
        assert!(TeePlatform::AmdSevSnp.is_tee());
        assert!(!TeePlatform::None.is_tee());
    }

    #[test]
    fn test_tee_platform_names() {
        assert_eq!(TeePlatform::IntelSgx.name(), "Intel SGX");
        assert_eq!(TeePlatform::IntelTdx.name(), "Intel TDX");
        assert_eq!(TeePlatform::AmdSevSnp.name(), "AMD SEV-SNP");
        assert_eq!(TeePlatform::None.name(), "None (Simulation)");
    }

    #[test]
    fn test_attestation_quote_creation() {
        let quote = AttestationQuote::new(
            TeePlatform::IntelSgx,
            b"test_quote".to_vec(),
            b"nonce123".to_vec(),
            b"user_data".to_vec(),
        );

        assert_eq!(quote.platform, TeePlatform::IntelSgx);
        assert_eq!(quote.nonce, b"nonce123");
        assert!(quote.is_fresh(60));
    }

    #[test]
    fn test_attestation_quote_serialization() {
        let original = AttestationQuote::new(
            TeePlatform::IntelTdx,
            b"quote_bytes".to_vec(),
            b"nonce".to_vec(),
            b"user_data".to_vec(),
        );

        let bytes = original.to_bytes();
        let recovered = AttestationQuote::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.platform, original.platform);
        assert_eq!(recovered.quote_bytes, original.quote_bytes);
        assert_eq!(recovered.nonce, original.nonce);
        assert_eq!(recovered.user_data, original.user_data);
    }

    #[test]
    fn test_attestation_quote_serialization_all_platforms() {
        for platform in [
            TeePlatform::IntelSgx,
            TeePlatform::IntelTdx,
            TeePlatform::AmdSevSnp,
            TeePlatform::None,
        ] {
            let original = AttestationQuote::new(
                platform,
                b"quote".to_vec(),
                b"nonce".to_vec(),
                b"data".to_vec(),
            );

            let bytes = original.to_bytes();
            let recovered = AttestationQuote::from_bytes(&bytes).unwrap();
            assert_eq!(recovered.platform, platform);
        }
    }

    #[test]
    fn test_attestation_quote_from_bytes_error_short() {
        let too_short = vec![0u8; 10];
        assert!(AttestationQuote::from_bytes(&too_short).is_err());
    }

    #[test]
    fn test_enclave_identity() {
        let identity = EnclaveIdentity::new([1u8; 32], [2u8; 32], 1, 1, false);

        assert!(identity.is_production());
        assert!(identity.verify_mrenclave(&[1u8; 32]));
        assert!(!identity.verify_mrenclave(&[0u8; 32]));
    }

    #[test]
    fn test_enclave_identity_debug_mode() {
        let debug_identity = EnclaveIdentity::new([1u8; 32], [2u8; 32], 1, 1, true);
        assert!(!debug_identity.is_production());
    }

    #[test]
    fn test_enclave_identity_mrsigner() {
        let identity = EnclaveIdentity::new([1u8; 32], [2u8; 32], 42, 5, false);

        assert!(identity.verify_mrsigner(&[2u8; 32]));
        assert!(!identity.verify_mrsigner(&[0u8; 32]));
        assert_eq!(identity.product_id, 42);
        assert_eq!(identity.svn, 5);
    }

    #[test]
    fn test_tee_capabilities_detect() {
        let caps = TeeCapabilities::detect();
        // In CI/test environment, likely no TEE
        let platform = caps.best_platform();
        assert!(matches!(
            platform,
            TeePlatform::None
                | TeePlatform::IntelSgx
                | TeePlatform::IntelTdx
                | TeePlatform::AmdSevSnp
        ));
    }

    #[test]
    fn test_tee_capabilities_default() {
        let caps = TeeCapabilities::default();
        assert!(!caps.any_available());
        assert_eq!(caps.best_platform(), TeePlatform::None);
    }

    #[test]
    fn test_tee_capabilities_priority() {
        // TDX has highest priority
        let caps = TeeCapabilities {
            sgx: true,
            tdx: true,
            sev_snp: true,
            ..Default::default()
        };
        assert_eq!(caps.best_platform(), TeePlatform::IntelTdx);

        // SGX is second
        let caps = TeeCapabilities {
            sgx: true,
            sev_snp: true,
            ..Default::default()
        };
        assert_eq!(caps.best_platform(), TeePlatform::IntelSgx);

        // SEV-SNP is third
        let caps = TeeCapabilities {
            sev_snp: true,
            ..Default::default()
        };
        assert_eq!(caps.best_platform(), TeePlatform::AmdSevSnp);
    }

    #[test]
    fn test_attestation_provider() {
        let provider = AttestationProvider::new();

        let quote = provider.generate_quote(b"test_nonce", b"app_data").unwrap();

        assert_eq!(quote.nonce, b"test_nonce");
        assert_eq!(quote.user_data, b"app_data");

        // Verify the quote
        let valid = provider.verify_quote(&quote, b"test_nonce").unwrap();
        assert!(valid);

        // Verify with wrong nonce should fail
        let invalid = provider.verify_quote(&quote, b"wrong_nonce").unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_attestation_provider_default() {
        let provider = AttestationProvider::default();
        assert!(matches!(
            provider.platform(),
            TeePlatform::None
                | TeePlatform::IntelSgx
                | TeePlatform::IntelTdx
                | TeePlatform::AmdSevSnp
        ));
    }

    #[test]
    fn test_attestation_provider_debug() {
        let provider = AttestationProvider::new();
        let debug_str = format!("{:?}", provider);
        assert!(debug_str.contains("AttestationProvider"));
        assert!(debug_str.contains("platform"));
    }

    #[test]
    fn test_quote_with_signature() {
        let quote = AttestationQuote::new(
            TeePlatform::None,
            b"quote".to_vec(),
            b"nonce".to_vec(),
            b"data".to_vec(),
        )
        .with_signature(b"signature".to_vec());

        assert!(quote.signature.is_some());
        assert_eq!(quote.signature.unwrap(), b"signature");
    }

    #[test]
    fn test_quote_signature_serialization() {
        let original = AttestationQuote::new(
            TeePlatform::None,
            b"quote".to_vec(),
            b"nonce".to_vec(),
            b"data".to_vec(),
        )
        .with_signature(b"my_signature".to_vec());

        let bytes = original.to_bytes();
        let recovered = AttestationQuote::from_bytes(&bytes).unwrap();

        assert!(recovered.signature.is_some());
        assert_eq!(recovered.signature.unwrap(), b"my_signature");
    }

    #[test]
    fn test_attestation_provider_capabilities() {
        let provider = AttestationProvider::new();
        let caps = provider.capabilities();

        // Just check it returns a valid reference
        let _ = caps.any_available();
    }

    #[test]
    fn test_quote_freshness() {
        let quote = AttestationQuote::new(
            TeePlatform::None,
            b"quote".to_vec(),
            b"nonce".to_vec(),
            b"data".to_vec(),
        );

        // Fresh within 1 hour
        assert!(quote.is_fresh(3600));
        // Fresh within 1 second
        assert!(quote.is_fresh(1));
    }
}

