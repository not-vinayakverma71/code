use lapce_ai_rust::ipc::ipc_config::IpcConfig;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_valid_config_parsing() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"
max_connections = 1000
buffer_size = 4194304

[security]
enable_auth = true
auth_token = "test-token-123"
allowed_ips = ["127.0.0.1", "::1"]

[performance]
enable_compression = true
compression_threshold = 1024
max_message_size = 10485760
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let config = IpcConfig::from_file(config_path.to_str().unwrap()).unwrap();
    assert_eq!(config.server.max_connections, 1000);
    assert!(config.security.enable_auth);
    assert_eq!(config.performance.compression_threshold, 1024);
}

#[test]
fn test_invalid_bind_address() {
    let config_toml = r#"
[server]
bind_address = "invalid:address:format"
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let result = IpcConfig::from_file(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_security_defaults() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let config = IpcConfig::from_file(config_path.to_str().unwrap()).unwrap();
    
    // Security should default to safe values
    assert!(!config.security.enable_auth); // Default to no auth for local
    assert!(config.security.allowed_ips.contains(&"127.0.0.1".to_string()));
}

#[test]
fn test_invalid_buffer_size() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"
buffer_size = 100  # Too small
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let result = IpcConfig::from_file(config_path.to_str().unwrap());
    assert!(result.is_err() || result.unwrap().server.buffer_size >= 1024);
}

#[test]
fn test_auth_token_validation() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"

[security]
enable_auth = true
auth_token = ""  # Empty token with auth enabled
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let result = IpcConfig::from_file(config_path.to_str().unwrap());
    assert!(result.is_err(), "Should reject empty auth token when auth is enabled");
}

#[test]
fn test_max_connections_limit() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"
max_connections = 1000000  # Too high
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let config = IpcConfig::from_file(config_path.to_str().unwrap()).unwrap();
    assert!(config.server.max_connections <= 10000, "Should cap max connections");
}

#[test]
fn test_compression_threshold_validation() {
    let config_toml = r#"
[server]
bind_address = "127.0.0.1:8080"

[performance]
enable_compression = true
compression_threshold = 10  # Too small for efficient compression
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let config = IpcConfig::from_file(config_path.to_str().unwrap()).unwrap();
    assert!(config.performance.compression_threshold >= 256, "Should enforce minimum compression threshold");
}

#[test]
fn test_allowed_ips_validation() {
    let config_toml = r#"
[server]
bind_address = "0.0.0.0:8080"  # Bind to all interfaces

[security]
allowed_ips = []  # Empty allowlist with public binding
"#;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("lapce-ipc.toml");
    fs::write(&config_path, config_toml).unwrap();

    let result = IpcConfig::from_file(config_path.to_str().unwrap());
    assert!(result.is_err(), "Should reject empty allowlist with public binding");
}
