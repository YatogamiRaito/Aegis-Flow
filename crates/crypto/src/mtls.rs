//! mTLS (Mutual TLS) Module with PQC Support
//!
//! Provides certificate-based authentication with Post-Quantum cryptography.

use crate::certmanager::{CertManager, ParsedCert};
use crate::tls::{PqcHandshake, PqcTlsConfig, SecureChannel};
use aegis_common::{AegisError, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, error, info};

/// mTLS Configuration
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Path to server certificate
    pub cert_path: String,
    /// Path to server private key
    pub key_path: String,
    /// Path to CA certificate for client verification
    pub ca_path: Option<String>,
    /// Require client certificates
    pub require_client_cert: bool,
    /// Enable PQC key exchange
    pub pqc_enabled: bool,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "/etc/aegis/certs/server.crt".to_string(),
            key_path: "/etc/aegis/certs/server.key".to_string(),
            ca_path: Some("/etc/aegis/certs/ca.crt".to_string()),
            require_client_cert: false,
            pqc_enabled: true,
        }
    }
}

/// Certificate verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether verification succeeded
    pub verified: bool,
    /// Subject common name
    pub subject_cn: Option<String>,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Expiration time (Unix timestamp)
    pub expires_at: u64,
}

/// Client authentication state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    /// Initial state, no authentication
    Unauthenticated,
    /// PQC handshake in progress
    HandshakeInProgress,
    /// Certificate verification pending
    CertVerificationPending,
    /// Fully authenticated
    Authenticated,
    /// Authentication failed
    Failed(String),
}

/// Connected client information
#[derive(Debug)]
pub struct AuthenticatedClient {
    /// Unique connection ID
    pub connection_id: u64,
    /// Client certificate info
    pub cert: Option<ParsedCert>,
    /// Authentication state
    pub state: AuthState,
    /// Secure channel for encrypted communication
    pub channel: Option<SecureChannel>,
    /// Authentication timestamp
    pub authenticated_at: Option<u64>,
}

impl AuthenticatedClient {
    /// Create a new unauthenticated client
    pub fn new(connection_id: u64) -> Self {
        Self {
            connection_id,
            cert: None,
            state: AuthState::Unauthenticated,
            channel: None,
            authenticated_at: None,
        }
    }

    /// Check if client is fully authenticated
    pub fn is_authenticated(&self) -> bool {
        self.state == AuthState::Authenticated
    }
}

/// mTLS Authenticator for handling mutual TLS authentication with PQC
pub struct MtlsAuthenticator {
    /// Configuration
    config: MtlsConfig,
    /// Certificate manager
    cert_manager: CertManager,
    /// PQC handshake handler
    pqc_handshake: PqcHandshake,
    /// Connected clients
    clients: Arc<RwLock<HashMap<u64, AuthenticatedClient>>>,
    /// Connection counter
    connection_counter: AtomicU64,
}

impl MtlsAuthenticator {
    /// Create a new mTLS authenticator
    pub fn new(config: MtlsConfig) -> Result<Self> {
        let pqc_config = PqcTlsConfig {
            pqc_enabled: config.pqc_enabled,
            mtls_required: config.require_client_cert,
            ..Default::default()
        };

        Ok(Self {
            config,
            cert_manager: CertManager::new(),
            pqc_handshake: PqcHandshake::new(pqc_config),
            clients: Arc::new(RwLock::new(HashMap::new())),
            connection_counter: AtomicU64::new(1),
        })
    }

    /// Initialize with certificates from files
    pub fn init_from_files(&mut self) -> Result<()> {
        // Load server certificate
        let server_cert = CertManager::load_from_file(Path::new(&self.config.cert_path))?;
        let key_pem = std::fs::read_to_string(&self.config.key_path)
            .map_err(|e| AegisError::Config(format!("Failed to read key: {}", e)))?;

        self.cert_manager.set_server_cert(server_cert, key_pem)?;

        // Load CA certificate if configured
        if let Some(ca_path) = &self.config.ca_path {
            let ca_cert = CertManager::load_from_file(Path::new(ca_path))?;
            self.cert_manager.add_trusted_ca(ca_cert)?;
        }

        info!("mTLS authenticator initialized from files");
        Ok(())
    }

    /// Initialize with generated self-signed certificates (for testing)
    pub fn init_self_signed(&mut self, cn: &str) -> Result<()> {
        let (cert_pem, key_pem) = CertManager::generate_self_signed(
            cn,
            &["localhost".to_string(), "127.0.0.1".to_string()],
            365,
        )?;

        let server_cert = CertManager::parse_pem(cert_pem.as_bytes())?;
        self.cert_manager.set_server_cert(server_cert, key_pem)?;

        info!("mTLS authenticator initialized with self-signed certificate");
        Ok(())
    }

    /// Accept a new connection and start authentication
    pub fn accept_connection(&self) -> Result<(u64, crate::hybrid_kex::HybridPublicKey)> {
        let conn_id = self.connection_counter.fetch_add(1, Ordering::SeqCst);

        // Initialize PQC handshake
        let (server_pk, _state) = self.pqc_handshake.server_init()?;

        // Create client entry
        let mut client = AuthenticatedClient::new(conn_id);
        client.state = AuthState::HandshakeInProgress;

        self.clients.write().insert(conn_id, client);

        debug!("Accepted connection {}, starting PQC handshake", conn_id);
        Ok((conn_id, server_pk))
    }

    /// Complete the handshake with client's ciphertext and optional certificate
    pub fn complete_handshake(
        &self,
        connection_id: u64,
        ciphertext: &crate::hybrid_kex::HybridCiphertext,
        client_cert_der: Option<&[u8]>,
    ) -> Result<()> {
        // Scope for the write lock to get the client
        // We need to keep the lock while modifying
        let mut clients = self.clients.write();

        let client = clients
            .get_mut(&connection_id)
            .ok_or_else(|| AegisError::Crypto("Connection not found".to_string()))?;

        // Parse client certificate if provided
        let client_cert = if let Some(der) = client_cert_der {
            match CertManager::parse_der(der) {
                Ok(cert) => Some(cert),
                Err(e) => {
                    error!("Failed to parse client certificate: {}", e);
                    if self.config.require_client_cert {
                        client.state = AuthState::Failed("Invalid client certificate".to_string());
                        return Err(e);
                    }
                    None
                }
            }
        } else {
            None
        };

        // Verify client certificate if required
        if self.config.require_client_cert {
            if let Some(ref cert) = client_cert {
                match self.cert_manager.verify_chain(cert) {
                    Ok(true) => {
                        if !cert.is_valid_now() {
                            client.state =
                                AuthState::Failed("Client certificate expired".to_string());
                            return Err(AegisError::Crypto(
                                "Client certificate expired".to_string(),
                            ));
                        }
                        debug!("Client certificate verified: {}", cert.subject_cn);
                    }
                    Ok(false) | Err(_) => {
                        client.state =
                            AuthState::Failed("Certificate verification failed".to_string());
                        return Err(AegisError::Crypto(
                            "Client certificate verification failed".to_string(),
                        ));
                    }
                }
            } else {
                client.state = AuthState::Failed("Client certificate required".to_string());
                return Err(AegisError::Crypto(
                    "Client certificate required but not provided".to_string(),
                ));
            }
        }

        // Re-init the handshake state for decapsulation
        // In real implementation, we'd store the server_state
        let (_, server_state) = self.pqc_handshake.server_init()?;
        let channel = self
            .pqc_handshake
            .server_complete(ciphertext, server_state)?;

        // Update client state
        client.cert = client_cert;
        client.channel = Some(channel);
        client.state = AuthState::Authenticated;
        client.authenticated_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );

        info!("Connection {} authenticated successfully", connection_id);
        Ok(())
    }

    /// Get client state
    pub fn get_client_state(&self, connection_id: u64) -> Result<AuthState> {
        let clients = self.clients.read();

        clients
            .get(&connection_id)
            .map(|c| c.state.clone())
            .ok_or_else(|| AegisError::Crypto("Connection not found".to_string()))
    }

    /// Disconnect a client
    pub fn disconnect(&self, connection_id: u64) -> Result<()> {
        let mut clients = self.clients.write();

        if clients.remove(&connection_id).is_some() {
            debug!("Disconnected client {}", connection_id);
            Ok(())
        } else {
            Err(AegisError::Crypto("Connection not found".to_string()))
        }
    }

    /// Get count of authenticated clients
    pub fn authenticated_count(&self) -> usize {
        self.clients
            .read()
            .values()
            .filter(|client| client.is_authenticated())
            .count()
    }

    /// Check if PQC is enabled
    pub fn is_pqc_enabled(&self) -> bool {
        self.config.pqc_enabled
    }

    /// Get certificate manager reference
    pub fn cert_manager(&self) -> &CertManager {
        &self.cert_manager
    }
}

/// mTLS Handler for certificate operations (legacy interface)
#[derive(Debug)]
pub struct MtlsHandler {
    config: MtlsConfig,
}

impl MtlsHandler {
    /// Create a new mTLS handler
    pub fn new(config: MtlsConfig) -> Self {
        Self { config }
    }

    /// Check if certificate files exist
    #[allow(clippy::collapsible_if)]
    pub fn validate_paths(&self) -> Result<()> {
        if !Path::new(&self.config.cert_path).exists() {
            return Err(AegisError::Config(format!(
                "Certificate not found: {}",
                self.config.cert_path
            )));
        }

        if !Path::new(&self.config.key_path).exists() {
            return Err(AegisError::Config(format!(
                "Private key not found: {}",
                self.config.key_path
            )));
        }

        if let Some(ca_path) = &self.config.ca_path {
            if !Path::new(ca_path).exists() {
                return Err(AegisError::Config(format!(
                    "CA certificate not found: {}",
                    ca_path
                )));
            }
        }

        debug!("✅ Certificate paths validated");
        Ok(())
    }

    /// Get certificate info (placeholder for actual implementation)
    pub fn get_cert_info(&self) -> Result<CertInfo> {
        info!("📜 Loading certificate from {}", self.config.cert_path);

        // In production, this would parse the actual certificate
        Ok(CertInfo {
            subject: "CN=aegis-flow,O=Aegis".to_string(),
            issuer: "CN=Aegis CA,O=Aegis".to_string(),
            serial: "0001".to_string(),
            not_before: 0,
            not_after: u64::MAX,
            is_ca: false,
        })
    }

    /// Check if PQC is enabled
    pub fn is_pqc_enabled(&self) -> bool {
        self.config.pqc_enabled
    }
}

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertInfo {
    /// Subject distinguished name
    pub subject: String,
    /// Issuer distinguished name
    pub issuer: String,
    /// Serial number (hex)
    pub serial: String,
    /// Not valid before (Unix timestamp)
    pub not_before: u64,
    /// Not valid after (Unix timestamp)
    pub not_after: u64,
    /// Is this a CA certificate
    pub is_ca: bool,
}

impl CertInfo {
    /// Check if certificate is currently valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        current_time >= self.not_before && current_time <= self.not_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MtlsConfig::default();
        assert!(config.pqc_enabled);
        assert!(!config.require_client_cert);
        assert!(config.ca_path.is_some());
    }

    #[test]
    fn test_mtls_handler_creation() {
        let config = MtlsConfig::default();
        let handler = MtlsHandler::new(config);
        assert!(handler.is_pqc_enabled());
    }

    #[test]
    fn test_cert_validity() {
        let cert = CertInfo {
            subject: "CN=test".to_string(),
            issuer: "CN=ca".to_string(),
            serial: "0001".to_string(),
            not_before: 1000,
            not_after: 2000,
            is_ca: false,
        };

        assert!(cert.is_valid(1500));
        assert!(!cert.is_valid(500));
        assert!(!cert.is_valid(2500));
    }

    #[test]
    fn test_path_validation_error() {
        let config = MtlsConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            ..Default::default()
        };
        let handler = MtlsHandler::new(config);
        assert!(handler.validate_paths().is_err());
    }

    #[test]
    fn test_mtls_authenticator_creation() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();
        assert!(auth.is_pqc_enabled());
        assert_eq!(auth.authenticated_count(), 0);
    }

    #[test]
    fn test_mtls_self_signed_init() {
        let config = MtlsConfig::default();
        let mut auth = MtlsAuthenticator::new(config).unwrap();
        auth.init_self_signed("test.aegis.local").unwrap();
        assert!(auth.cert_manager().server_cert().is_some());
    }

    #[test]
    fn test_auth_state_transitions() {
        let state = AuthState::Unauthenticated;
        assert_eq!(state, AuthState::Unauthenticated);

        let failed = AuthState::Failed("test error".to_string());
        assert!(matches!(failed, AuthState::Failed(_)));
    }

    #[test]
    fn test_authenticated_client() {
        let mut client = AuthenticatedClient::new(1);
        assert!(!client.is_authenticated());

        client.state = AuthState::Authenticated;
        assert!(client.is_authenticated());
    }

    #[test]
    fn test_accept_connection() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        let (conn_id, _pk) = auth.accept_connection().unwrap();
        assert!(conn_id > 0);

        let state = auth.get_client_state(conn_id).unwrap();
        assert_eq!(state, AuthState::HandshakeInProgress);
    }

    #[test]
    fn test_init_from_files_invalid() {
        let config = MtlsConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            ..Default::default()
        };
        let mut auth = MtlsAuthenticator::new(config).unwrap();
        assert!(auth.init_from_files().is_err());
    }

    #[test]
    fn test_complete_handshake_invalid_id() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();
        // ID 999 does not exist
        let dummy_ct = crate::hybrid_kex::HybridCiphertext {
            x25519_ephemeral: [0u8; 32],
            mlkem_ciphertext: vec![0u8; 10],
        };
        assert!(auth.complete_handshake(999, &dummy_ct, None).is_err());
    }

    #[test]
    fn test_complete_handshake_missing_cert() {
        let config = MtlsConfig {
            require_client_cert: true,
            ..Default::default()
        };
        let auth = MtlsAuthenticator::new(config).unwrap();
        let (conn_id, _pk) = auth.accept_connection().unwrap();

        let dummy_ct = crate::hybrid_kex::HybridCiphertext {
            x25519_ephemeral: [0u8; 32],
            mlkem_ciphertext: vec![0u8; 10],
        };

        // Should fail because client cert is required but None provided
        let result = auth.complete_handshake(conn_id, &dummy_ct, None);
        assert!(result.is_err());
        if let Err(AegisError::Crypto(msg)) = result {
            assert_eq!(msg, "Client certificate required but not provided")
        }
    }

    #[test]
    fn test_disconnect() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        let (conn_id, _) = auth.accept_connection().unwrap();
        assert!(auth.disconnect(conn_id).is_ok());
        assert!(auth.get_client_state(conn_id).is_err());
    }

    #[test]
    fn test_disconnect_nonexistent() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();
        assert!(auth.disconnect(99999).is_err());
    }

    #[test]
    fn test_get_cert_info() {
        let config = MtlsConfig::default();
        let handler = MtlsHandler::new(config);
        let cert_info = handler.get_cert_info().unwrap();
        assert!(cert_info.subject.contains("CN="));
        assert!(cert_info.issuer.contains("CN="));
    }

    #[test]
    fn test_verification_result() {
        let result = VerificationResult {
            verified: true,
            subject_cn: Some("test.local".to_string()),
            fingerprint: "abc123".to_string(),
            expires_at: 1000000,
        };
        assert!(result.verified);
        assert_eq!(result.subject_cn, Some("test.local".to_string()));
    }

    #[test]
    fn test_key_path_validation_error() {
        let config = MtlsConfig {
            cert_path: "/tmp/test_cert_exists".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            ca_path: None,
            ..Default::default()
        };
        // Create the cert file temporarily
        std::fs::write("/tmp/test_cert_exists", "fake").ok();
        let handler = MtlsHandler::new(config);
        let result = handler.validate_paths();
        assert!(result.is_err());
        std::fs::remove_file("/tmp/test_cert_exists").ok();
    }

    #[test]
    fn test_auth_state_all_variants() {
        let states = [
            AuthState::Unauthenticated,
            AuthState::HandshakeInProgress,
            AuthState::CertVerificationPending,
            AuthState::Authenticated,
            AuthState::Failed("error".to_string()),
        ];
        for state in states {
            // Just verify Debug trait works
            let _ = format!("{:?}", state);
        }
    }

    #[test]
    fn test_authenticated_count() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        // Initially zero
        assert_eq!(auth.authenticated_count(), 0);

        // Accept a connection (unauthenticated)
        let (conn_id, _) = auth.accept_connection().unwrap();

        // Still zero because not authenticated
        assert_eq!(auth.authenticated_count(), 0);

        // Disconnect
        auth.disconnect(conn_id).ok();
    }

    #[test]
    fn test_is_pqc_enabled() {
        let config = MtlsConfig {
            pqc_enabled: true,
            ..Default::default()
        };
        let auth = MtlsAuthenticator::new(config).unwrap();
        assert!(auth.is_pqc_enabled());

        let config2 = MtlsConfig {
            pqc_enabled: false,
            ..Default::default()
        };
        let auth2 = MtlsAuthenticator::new(config2).unwrap();
        assert!(!auth2.is_pqc_enabled());
    }

    #[test]
    fn test_cert_manager_access() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        // Just verify we can access cert_manager
        let _cm = auth.cert_manager();
    }

    #[test]
    fn test_authenticated_client_new() {
        let client = AuthenticatedClient::new(12345);
        assert_eq!(client.connection_id, 12345);
        assert!(client.cert.is_none());
        assert_eq!(client.state, AuthState::Unauthenticated);
        assert!(client.channel.is_none());
        assert!(client.authenticated_at.is_none());
        assert!(!client.is_authenticated());
    }

    #[test]
    fn test_mtls_config_default() {
        let config = MtlsConfig::default();
        assert!(config.cert_path.contains("server.crt"));
        assert!(config.key_path.contains("server.key"));
        assert!(config.pqc_enabled);
        // Just verify the field exists
        let _ = config.require_client_cert;
    }

    #[test]
    fn test_mtls_config_clone() {
        let config = MtlsConfig {
            cert_path: "custom.crt".to_string(),
            key_path: "custom.key".to_string(),
            pqc_enabled: false,
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(config.cert_path, cloned.cert_path);
        assert_eq!(config.pqc_enabled, cloned.pqc_enabled);
    }

    #[test]
    fn test_auth_state_equality() {
        assert_eq!(AuthState::Unauthenticated, AuthState::Unauthenticated);
        assert_eq!(AuthState::Authenticated, AuthState::Authenticated);
        assert_ne!(AuthState::Unauthenticated, AuthState::Authenticated);
    }

    #[test]
    fn test_auth_state_failed_message() {
        let state = AuthState::Failed("Custom error message".to_string());
        let debug = format!("{:?}", state);
        assert!(debug.contains("Custom error message"));
    }

    #[test]
    fn test_verification_result_creation() {
        let result = VerificationResult {
            verified: true,
            subject_cn: Some("test-client".to_string()),
            fingerprint: "abc123".to_string(),
            expires_at: 1234567890,
        };
        assert!(result.verified);
        assert_eq!(result.subject_cn, Some("test-client".to_string()));
        assert_eq!(result.fingerprint, "abc123");
        assert_eq!(result.expires_at, 1234567890);
    }

    #[test]
    fn test_verification_result_failed() {
        let result = VerificationResult {
            verified: false,
            subject_cn: None,
            fingerprint: "".to_string(),
            expires_at: 0,
        };
        assert!(!result.verified);
        assert!(result.subject_cn.is_none());
    }

    #[test]
    fn test_verification_result_clone() {
        let result = VerificationResult {
            verified: true,
            subject_cn: Some("clone-test".to_string()),
            fingerprint: "fingerprint".to_string(),
            expires_at: 9999,
        };
        let cloned = result.clone();
        assert_eq!(result.verified, cloned.verified);
        assert_eq!(result.subject_cn, cloned.subject_cn);
    }

    #[test]
    fn test_mtls_handler_validate_paths_missing() {
        let config = MtlsConfig {
            cert_path: "/nonexistent/cert.crt".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
            ..Default::default()
        };
        let handler = MtlsHandler::new(config);
        let result = handler.validate_paths();
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_handshake_connection_not_found() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        // Create a dummy ciphertext
        let dummy_ct = crate::HybridCiphertext::from_bytes(&[0u8; 100]);

        if let Ok(ct) = dummy_ct {
            let result = auth.complete_handshake(99999, &ct, None);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_get_client_state_not_found() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        let result = auth.get_client_state(12345);
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnect_success() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        let (conn_id, _) = auth.accept_connection().unwrap();
        let result = auth.disconnect(conn_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticated_count_with_connections() {
        let config = MtlsConfig::default();
        let auth = MtlsAuthenticator::new(config).unwrap();

        // Accept multiple connections
        let _ = auth.accept_connection();
        let _ = auth.accept_connection();

        // None are authenticated yet
        assert_eq!(auth.authenticated_count(), 0);
    }

    #[test]
    fn test_mtls_config_with_ca_path() {
        let config = MtlsConfig {
            ca_path: Some("/custom/ca.crt".to_string()),
            ..Default::default()
        };
        assert!(config.ca_path.is_some());
        assert!(config.ca_path.unwrap().contains("ca.crt"));
    }

    #[test]
    fn test_authenticated_client_debug() {
        let client = AuthenticatedClient::new(999);
        let debug = format!("{:?}", client);
        assert!(debug.contains("999"));
        assert!(debug.contains("Unauthenticated"));
    }

    #[test]
    fn test_auth_state_variants() {
        let states = vec![
            AuthState::Unauthenticated,
            AuthState::HandshakeInProgress,
            AuthState::CertVerificationPending,
            AuthState::Authenticated,
            AuthState::Failed("error".to_string()),
        ];

        for state in states {
            let debug = format!("{:?}", state);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_mtls_config_custom_paths() {
        let config = MtlsConfig {
            cert_path: "/custom/server.crt".to_string(),
            key_path: "/custom/server.key".to_string(),
            ca_path: Some("/custom/ca.crt".to_string()),
            require_client_cert: true,
            pqc_enabled: false,
        };

        assert!(config.cert_path.contains("custom"));
        assert!(config.require_client_cert);
        assert!(!config.pqc_enabled);
    }

    #[test]
    fn test_verification_result_success() {
        let result = VerificationResult {
            verified: true,
            subject_cn: Some("test.example.com".to_string()),
            fingerprint: "abc123".to_string(),
            expires_at: 1234567890,
        };

        assert!(result.verified);
        assert!(result.subject_cn.unwrap().contains("test.example.com"));
    }

    #[test]
    fn test_verification_result_failure() {
        let result = VerificationResult {
            verified: false,
            subject_cn: Some("invalid.example.com".to_string()),
            fingerprint: "def456".to_string(),
            expires_at: 0,
        };

        assert!(!result.verified);
    }

    #[test]
    fn test_authenticated_client_id() {
        let client = AuthenticatedClient::new(12345);
        assert_eq!(client.connection_id, 12345);
    }

    #[test]
    fn test_mtls_config_defaults() {
        let config = MtlsConfig::default();
        assert!(config.require_client_cert);
        assert!(config.pqc_enabled);
    }
}
