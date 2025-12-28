use aegis_proxy::{ProxyConfig, QuicConfig, QuicServer, QuicStats};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create test certificates
fn create_test_certs() -> (TempDir, PathBuf, PathBuf) {
    use aegis_crypto::certmanager::CertManager;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Generate self-signed cert
    let (cert_pem, key_pem) = CertManager::generate_self_signed(
        "localhost",
        &["127.0.0.1".to_string(), "::1".to_string()],
        365,
    )
    .expect("Failed to generate cert");

    // Write to files
    let cert_path = temp_dir.path().join("cert.pem");
    let key_path = temp_dir.path().join("key.pem");

    fs::write(&cert_path, cert_pem).expect("Failed to write cert");
    fs::write(&key_path, key_pem).expect("Failed to write key");

    (temp_dir, cert_path, key_path)
}

#[tokio::test]
async fn test_quic_server_with_real_certs() {
    let (_temp_dir, cert_path, key_path) = create_test_certs();

    let config = QuicConfig {
        bind_address: "127.0.0.1:0".to_string(),
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        enable_0rtt: true,
        max_streams: 100,
        idle_timeout_secs: 30,
        pqc_enabled: false, // Disable PQC for simplicity
    };

    let proxy_config = ProxyConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        ..Default::default()
    };

    let server = QuicServer::new(config.clone(), proxy_config);

    // Just test that server can be created with real certs
    let _ = server;

    // Verify cert files exist
    assert!(cert_path.exists());
    assert!(key_path.exists());
}

#[tokio::test]
async fn test_quic_config_with_pqc_enabled() {
    let (_temp_dir, cert_path, key_path) = create_test_certs();

    let config = QuicConfig {
        bind_address: "0.0.0.0:4433".to_string(),
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        enable_0rtt: true,
        max_streams: 100,
        idle_timeout_secs: 30,
        pqc_enabled: true,
    };

    assert!(config.pqc_enabled);
    assert_eq!(config.max_streams, 100);
}

#[tokio::test]
async fn test_quic_config_default_values() {
    let config = QuicConfig::default();

    assert_eq!(config.bind_address, "0.0.0.0:443");
    assert!(config.enable_0rtt);
    assert_eq!(config.max_streams, 100);
    assert_eq!(config.idle_timeout_secs, 30);
    assert!(config.pqc_enabled);
}

#[tokio::test]
async fn test_quic_server_creation_with_defaults() {
    let (_temp_dir, cert_path, key_path) = create_test_certs();

    let config = QuicConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        bind_address: "127.0.0.1:0".to_string(),
        ..QuicConfig::default()
    };

    let proxy_config = ProxyConfig::default();
    let server = QuicServer::new(config, proxy_config);

    // Server created successfully
    let _ = server;
}

#[tokio::test]
async fn test_quic_stats_tracking() {
    // Create stats with specific values
    let stats = QuicStats {
        connections_accepted: 5,
        streams_handled: 20,
        active_connections: 3,
        zero_rtt_connections: 2,
    };

    assert_eq!(stats.connections_accepted, 5);
    assert_eq!(stats.streams_handled, 20);
    assert_eq!(stats.active_connections, 3);
    assert_eq!(stats.zero_rtt_connections, 2);
}

#[test]
fn test_quic_config_custom_timeout() {
    let config = QuicConfig {
        bind_address: "0.0.0.0:8443".to_string(),
        cert_path: "/tmp/cert.pem".to_string(),
        key_path: "/tmp/key.pem".to_string(),
        enable_0rtt: false,
        max_streams: 200,
        idle_timeout_secs: 60,
        pqc_enabled: true,
    };

    assert_eq!(config.idle_timeout_secs, 60);
    assert_eq!(config.max_streams, 200);
    assert!(!config.enable_0rtt);
}

#[test]
fn test_quic_stats_clone() {
    let stats1 = QuicStats {
        connections_accepted: 10,
        streams_handled: 50,
        active_connections: 5,
        zero_rtt_connections: 3,
    };

    let stats2 = stats1.clone();

    assert_eq!(stats1.connections_accepted, stats2.connections_accepted);
    assert_eq!(stats1.streams_handled, stats2.streams_handled);
}
