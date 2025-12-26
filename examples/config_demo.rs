//! Configuration Loading Example
//!
//! Demonstrates loading proxy configuration from YAML with environment overrides.
//!
//! Run with: cargo run --example config_demo

use aegis_proxy::config::ConfigManager;
use std::io::Write;

fn main() {
    println!("⚙️  Aegis-Flow Configuration Demo\n");

    // Create a temporary config file
    let config_content = r#"
host: "127.0.0.1"
port: 8443
pqc_enabled: true
upstream_addr: "backend.example.com:443"

tls:
  cert_path: "/etc/aegis/cert.pem"
  key_path: "/etc/aegis/key.pem"

logging:
  level: "info"
  format: "json"
"#;

    let mut file = tempfile::Builder::new()
        .suffix(".yaml")
        .tempfile()
        .expect("Failed to create temp file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    println!("1. Loading configuration from YAML file...");
    let manager = ConfigManager::from_file(file.path()).expect("Failed to load config");

    let config = manager.get();
    println!("   ✅ Host: {}", config.host);
    println!("   ✅ Port: {}", config.port);
    println!("   ✅ PQC Enabled: {}", config.pqc_enabled);
    println!("   ✅ Upstream: {}", config.upstream_addr);

    // Demonstrate environment variable override
    println!("\n2. Environment variables override file values:");
    println!("   Set AEGIS_PORT=9000 to override port");
    println!("   Set AEGIS_PQC_ENABLED=false to disable PQC");

    // Demonstrate hot-reload capability
    println!("\n3. Hot-reload capability:");
    println!("   manager.check_for_changes() - detects file modifications");
    println!("   manager.reload() - reloads configuration");

    println!("\n🎉 Configuration system ready!");
}
