use aegis_proxy::config::ConfigManager;
use std::io::{Seek, Write};
use std::time::Duration;

#[test]
fn test_config_integration_full() {
    // === Part 1: Reload Flow ===

    // 1. Create a temporary config file with extension
    let mut file = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();

    let initial_config = r#"
host: "127.0.0.1"
port: 8080
pqc_enabled: false
upstream_addr: "127.0.0.1:9090"
"#;
    file.write_all(initial_config.as_bytes()).unwrap();

    // 2. Initialize ConfigManager
    let manager = ConfigManager::from_file(file.path()).expect("Failed to load config");

    {
        let config = manager.get();
        assert_eq!(config.port, 8080, "Initial port should be 8080");
        assert!(!config.pqc_enabled);
    }

    // 3. Modify the file
    std::thread::sleep(Duration::from_millis(100));

    let new_config = r#"
host: "127.0.0.1"
port: 8081
pqc_enabled: true
upstream_addr: "127.0.0.1:9090"
"#;
    file.as_file_mut().set_len(0).unwrap();
    file.as_file_mut()
        .seek(std::io::SeekFrom::Start(0))
        .unwrap();
    file.write_all(new_config.as_bytes()).unwrap();
    file.as_file().sync_all().unwrap();

    // 4. Check & Reload
    assert!(manager.check_for_changes(), "Should detect file change");
    assert!(manager.reload().expect("Failed to reload"));

    {
        let config = manager.get();
        assert_eq!(config.port, 8081, "Reloaded port should be 8081");
        assert!(config.pqc_enabled);
    }

    // === Part 2: Env Override ===
    // We reuse the same logic but set env var

    // Set env var - this should override file config on next load
    // Note: ConfigManager reads env vars in load() / apply_env_overrides()
    // ConfigManager does NOT auto-watch env vars, only file changes trigger reload logic which calls load()

    unsafe {
        std::env::set_var("AEGIS_PORT", "9000");
    }

    // Force reload (we simulate file touch to trigger reload, or just create new manager)
    // Let's create a new manager to test clean start with env var
    let manager2 = ConfigManager::from_file(file.path()).expect("Failed to load config 2");
    let config2 = manager2.get();

    assert_eq!(config2.port, 9000, "Env var should override file config");

    unsafe {
        std::env::remove_var("AEGIS_PORT");
    }
}
