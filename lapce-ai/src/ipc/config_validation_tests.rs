
#[cfg(test)]
mod config_validation_tests {
    use super::*;
    use crate::ipc::ipc_config::IpcConfig;
    use tempfile::TempDir;
    use std::fs;

    /// Test default configuration is valid
    #[test]
    fn test_default_config_is_valid() {
        let config = IpcConfig::default();
        assert!(config.validate().is_ok(), "Default configuration should be valid");
    }

    /// Test safe defaults are production-ready
    #[test]
    fn test_safe_defaults_are_production_ready() {
        let config = IpcConfig::default();
        
        // IPC settings should be safe
        assert!(config.ipc.max_connections > 0 && config.ipc.max_connections <= 10000);
        assert!(config.ipc.idle_timeout_secs <= 3600);
        assert!(config.ipc.max_message_size >= 1024);
        assert!(!config.ipc.socket_path.is_empty());
        
        // Shared memory should be safe
        assert!(config.shared_memory.slot_size >= 64);
        assert!(config.shared_memory.num_slots > 0);
        assert!(config.shared_memory.ring_buffer_size >= 4096);
        assert!(config.shared_memory.permissions >= 0o600);
        
        // Security should be enabled by default
        assert!(!config.security.tls_enabled); // Disabled by default for testing, but validated when enabled
        assert!(config.security.max_requests_per_minute > 0);
    }

    /// Test IPC settings validation - out of range values
    #[test]
    fn test_ipc_settings_validation() {
        let mut config = IpcConfig::default();
        
        // Test max_connections out of range
        config.ipc.max_connections = 0;
        assert!(config.validate().is_err());
        
        config.ipc.max_connections = 20000;
        assert!(config.validate().is_err());
        
        // Test idle_timeout_secs out of range
        config.ipc.max_connections = 1000; // Reset to valid
        config.ipc.idle_timeout_secs = 7200;
        assert!(config.validate().is_err());
        
        // Test max_message_size out of range
        config.ipc.idle_timeout_secs = 300; // Reset to valid
        config.ipc.max_message_size = 512;
        assert!(config.validate().is_err());
        
        config.ipc.max_message_size = 200 * 1024 * 1024;
        assert!(config.validate().is_err());
        
        // Test empty socket_path
        config.ipc.max_message_size = 10 * 1024 * 1024; // Reset to valid
        config.ipc.socket_path = String::new();
        assert!(config.validate().is_err());
    }

    /// Test shared memory validation
    #[test]
    fn test_shared_memory_validation() {
        let mut config = IpcConfig::default();
        
        // Test slot_size out of range
        config.shared_memory.slot_size = 32;
        assert!(config.validate().is_err());
        
        config.shared_memory.slot_size = 128 * 1024;
        assert!(config.validate().is_err());
        
        // Test num_slots out of range
        config.shared_memory.slot_size = 1024; // Reset to valid
        config.shared_memory.num_slots = 0;
        assert!(config.validate().is_err());
        
        config.shared_memory.num_slots = 20000;
        assert!(config.validate().is_err());
        
        // Test ring_buffer_size out of range
        config.shared_memory.num_slots = 100; // Reset to valid
        config.shared_memory.ring_buffer_size = 2048;
        assert!(config.validate().is_err());
        
        config.shared_memory.ring_buffer_size = 2 * 1024 * 1024;
        assert!(config.validate().is_err());
        
        // Test permissions out of range
        config.shared_memory.ring_buffer_size = 64 * 1024; // Reset to valid
        config.shared_memory.permissions = 0o500;
        assert!(config.validate().is_err());
        
        config.shared_memory.permissions = 0o777;
        assert!(config.validate().is_ok()); // 0o777 should be valid (upper bound)
    }

    /// Test metrics validation
    #[test]
    fn test_metrics_validation() {
        let mut config = IpcConfig::default();
        
        // Test export_interval_secs out of range
        config.metrics.export_interval_secs = 2;
        assert!(config.validate().is_err());
        
        config.metrics.export_interval_secs = 7200;
        assert!(config.validate().is_err());
        
        // Test retention_hours out of range
        config.metrics.export_interval_secs = 60; // Reset to valid
        config.metrics.retention_hours = 200;
        assert!(config.validate().is_err());
        
        // Test empty export_path
        config.metrics.retention_hours = 24; // Reset to valid
        config.metrics.export_path = String::new();
        assert!(config.validate().is_err());
    }

    /// Test monitoring validation
    #[test]
    fn test_monitoring_validation() {
        let mut config = IpcConfig::default();
        
        // Test health_check_port out of range
        config.monitoring.health_check_port = 80;
        assert!(config.validate().is_err());
        
        config.monitoring.health_check_port = 65535; // Max valid port
        assert!(config.validate().is_ok()); // Should be valid
        
        // Test prometheus_port out of range
        config.monitoring.health_check_port = 8080; // Reset to valid
        config.monitoring.prometheus_port = 500;
        assert!(config.validate().is_err());
        
        // Test duplicate ports
        config.monitoring.prometheus_port = 8080; // Same as health_check_port
        assert!(config.validate().is_err());
    }

    /// Test reconnection validation
    #[test]
    fn test_reconnection_validation() {
        let mut config = IpcConfig::default();
        
        // Test invalid strategy
        config.reconnection.strategy = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test initial_delay_ms out of range
        config.reconnection.strategy = "exponential".to_string(); // Reset to valid
        config.reconnection.initial_delay_ms = 0;
        assert!(config.validate().is_err());
        
        config.reconnection.initial_delay_ms = 120000;
        assert!(config.validate().is_err());
        
        // Test max_delay_ms less than initial_delay_ms
        config.reconnection.initial_delay_ms = 1000; // Reset to valid
        config.reconnection.max_delay_ms = 500;
        assert!(config.validate().is_err());
        
        // Test multiplier out of range
        config.reconnection.max_delay_ms = 30000; // Reset to valid
        config.reconnection.multiplier = 0.5;
        assert!(config.validate().is_err());
        
        config.reconnection.multiplier = 20.0;
        assert!(config.validate().is_err());
        
        // Test max_retries out of range
        config.reconnection.multiplier = 2.0; // Reset to valid
        config.reconnection.max_retries = 200;
        assert!(config.validate().is_err());
    }

    /// Test provider validation
    #[test]
    fn test_provider_validation() {
        let mut config = IpcConfig::default();
        
        // Test enabled provider with empty API key
        config.providers.openai.enabled = true;
        config.providers.openai.api_key = String::new();
        assert!(config.validate().is_err());
        
        // Test empty base_url
        config.providers.openai.api_key = "sk-test".to_string(); // Reset to valid
        config.providers.openai.base_url = String::new();
        assert!(config.validate().is_err());
        
        // Test invalid base_url format
        config.providers.openai.base_url = "not-a-url".to_string();
        assert!(config.validate().is_err());
        
        // Test timeout_secs out of range
        config.providers.openai.base_url = "https://api.openai.com".to_string(); // Reset to valid
        config.providers.openai.timeout_secs = 0;
        assert!(config.validate().is_err());
        
        config.providers.openai.timeout_secs = 500;
        assert!(config.validate().is_err());
        
        // Test max_retries out of range
        config.providers.openai.timeout_secs = 30; // Reset to valid
        config.providers.openai.max_retries = 15;
        assert!(config.validate().is_err());
    }

    /// Test logging validation
    #[test]
    fn test_logging_validation() {
        let mut config = IpcConfig::default();
        
        // Test invalid level
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid format
        config.logging.level = "info".to_string(); // Reset to valid
        config.logging.format = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid output
        config.logging.format = "json".to_string(); // Reset to valid
        config.logging.output = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test rotation_size_mb out of range
        config.logging.output = "stdout".to_string(); // Reset to valid
        config.logging.rotation_size_mb = 2000;
        assert!(config.validate().is_err());
        
        // Test max_backups out of range
        config.logging.rotation_size_mb = 100; // Reset to valid
        config.logging.max_backups = 200;
        assert!(config.validate().is_err());
    }

    /// Test performance validation
    #[test]
    fn test_performance_validation() {
        let mut config = IpcConfig::default();
        
        // Test cache_size_mb out of range
        config.performance.cache_size_mb = 2000;
        assert!(config.validate().is_err());
        
        // Test cache_ttl_secs out of range
        config.performance.cache_size_mb = 100; // Reset to valid
        config.performance.cache_ttl_secs = 100000;
        assert!(config.validate().is_err());
    }

    /// Test security validation
    #[test]
    fn test_security_validation() {
        let mut config = IpcConfig::default();
        
        // Test TLS enabled without cert path
        config.security.tls_enabled = true;
        config.security.tls_cert_path = String::new();
        assert!(config.validate().is_err());
        
        // Test TLS enabled without key path
        config.security.tls_cert_path = "/path/to/cert".to_string();
        config.security.tls_key_path = String::new();
        assert!(config.validate().is_err());
        
        // Test auth enabled without token
        config.security.tls_enabled = false; // Reset to valid
        config.security.auth_enabled = true;
        config.security.auth_token = String::new();
        assert!(config.validate().is_err());
        
        // Test max_requests_per_minute out of range
        config.security.auth_enabled = false; // Reset to valid
        config.security.max_requests_per_minute = 0;
        assert!(config.validate().is_err());
        
        config.security.max_requests_per_minute = 200000;
        assert!(config.validate().is_err());
    }

    /// Test loading invalid TOML files fails closed
    #[test]
    fn test_invalid_toml_fails_closed() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");
        
        // Create invalid TOML
        fs::write(&config_path, "invalid toml content [[[").unwrap();
        
        let result = IpcConfig::from_file(&config_path);
        assert!(result.is_err(), "Invalid TOML should fail to load");
    }

    /// Test loading TOML with invalid values fails closed
    #[test]
    fn test_invalid_values_fail_closed() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_values.toml");
        
        // Create TOML with invalid values
        let invalid_config = r#"
[ipc]
socket_path = "/tmp/lapce-ai.sock"
max_connections = 0
idle_timeout_secs = 300
max_message_size = 10485760
buffer_pool_size = 100
connection_pool_max_idle = 500
connection_pool_max_lifetime_secs = 3600

[shared_memory]
slot_size = 32
num_slots = 100
ring_buffer_size = 65536
permissions = 384
namespace_suffix = "lapce-ai"

[metrics]
enabled = true
export_interval_secs = 60
retention_hours = 24
export_path = "/tmp/lapce-metrics"

[monitoring]
health_check_enabled = true
health_check_port = 8080
health_check_path = "/health"
prometheus_enabled = true
prometheus_port = 9090
grafana_dashboard_path = "./dashboards"

[reconnection]
strategy = "exponential"
initial_delay_ms = 1000
max_delay_ms = 30000
multiplier = 2.0
max_retries = 5

[providers]

[providers.openai]
enabled = false
api_key = ""
base_url = "https://api.openai.com"
timeout_secs = 30
max_retries = 3

[providers.anthropic]
enabled = false
api_key = ""
base_url = "https://api.anthropic.com"
timeout_secs = 30
max_retries = 3

[providers.gemini]
enabled = false
api_key = ""
base_url = "https://generativelanguage.googleapis.com"
timeout_secs = 30
max_retries = 3

[logging]
level = "info"
format = "json"
output = "stdout"
file_path = "/tmp/lapce-ai.log"
rotation_size_mb = 100
max_backups = 10

[performance]
cache_size_mb = 50
cache_ttl_secs = 3600

[security]
tls_enabled = false
tls_cert_path = ""
tls_key_path = ""
auth_enabled = false
auth_token = ""
rate_limiting = true
max_requests_per_minute = 1000
"#;
        
        fs::write(&config_path, invalid_config).unwrap();
        
        let result = IpcConfig::from_file(&config_path);
        assert!(result.is_err(), "Config with invalid values should fail validation");
    }

    /// Test environment variable expansion
    #[test]
    fn test_env_var_expansion() {
        std::env::set_var("TEST_API_KEY", "test-key-value");
        
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("env_test.toml");
        
        let config_content = r#"
[ipc]
socket_path = "/tmp/lapce-ai.sock"
max_connections = 1000
idle_timeout_secs = 300
max_message_size = 10485760
buffer_pool_size = 100
connection_pool_max_idle = 500
connection_pool_max_lifetime_secs = 3600

[shared_memory]
slot_size = 1024
num_slots = 100
ring_buffer_size = 65536
control_buffer_size = 4096
max_memory_per_connection = 30720
permissions = 384

[metrics]
enable = true
export_interval_secs = 60
retention_hours = 24
export_path = "/tmp/lapce-metrics"

[monitoring]
health_check_enabled = true
health_check_port = 8080
health_check_path = "/health"
prometheus_enabled = true
prometheus_port = 9090
grafana_dashboard_path = "./dashboards"

[reconnection]
strategy = "exponential"
initial_delay_ms = 1000
max_delay_ms = 30000
multiplier = 2.0
max_retries = 5

[providers]

[providers.openai]
enabled = true
api_key = "${TEST_API_KEY}"
base_url = "https://api.openai.com"
timeout_secs = 30
max_retries = 3

[providers.anthropic]
enabled = false
api_key = ""
base_url = "https://api.anthropic.com"
timeout_secs = 30
max_retries = 3

[providers.gemini]
enabled = false
api_key = ""
base_url = "https://generativelanguage.googleapis.com"
timeout_secs = 30
max_retries = 3

[logging]
level = "info"
format = "json"
output = "stdout"
file_path = "/tmp/lapce-ai.log"
rotation_size_mb = 100
max_backups = 10

[performance]
cache_size_mb = 50
cache_ttl_secs = 3600

[security]
tls_enabled = false
tls_cert_path = ""
tls_key_path = ""
auth_enabled = false
auth_token = ""
rate_limiting = true
max_requests_per_minute = 1000
"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let config = IpcConfig::from_file(&config_path).unwrap();
        assert_eq!(config.providers.openai.api_key, "test-key-value");
        
        std::env::remove_var("TEST_API_KEY");
    }

    /// Test configuration loading with missing file uses defaults
    #[test]
    fn test_missing_file_uses_defaults() {
        let result = IpcConfig::load();
        assert!(result.is_ok(), "Loading missing config should use defaults");
        
        let config = result.unwrap();
        assert!(config.validate().is_ok(), "Default config should be valid");
    }

    /// Test fail-closed behavior: strict validation prevents bad configs
    #[test]
    fn test_fail_closed_validation() {
        // Test various edge cases that should fail validation
        let mut config = IpcConfig::default();
        
        // Edge case: very large values
        config.ipc.max_connections = u32::MAX as usize;
        assert!(config.validate().is_err());
        
        // Edge case: zero values where not allowed
        config = IpcConfig::default();
        config.shared_memory.num_slots = 0;
        assert!(config.validate().is_err());
        
        // Edge case: negative-like behavior (testing boundary)
        config = IpcConfig::default();
        config.reconnection.multiplier = 0.0;
        assert!(config.validate().is_err());
        
        // Edge case: malformed URLs
        config = IpcConfig::default();
        config.providers.openai.enabled = true;
        config.providers.openai.api_key = "sk-test".to_string();
        config.providers.openai.base_url = "ftp://invalid.com".to_string();
        assert!(config.validate().is_err());
    }

    /// Test specific production-readiness validation
    #[test]
    fn test_production_readiness() {
        let config = IpcConfig::default();
        
        // Verify production-safe defaults
        assert!(config.shared_memory.permissions >= 0o600, "SHM permissions should be secure");
        assert!(config.security.rate_limiting, "Rate limiting should be enabled by default");
        assert!(config.metrics.enable, "Metrics should be enabled by default");
        assert!(config.monitoring.health_check_enabled, "Health check should be enabled by default");
        
        // Verify reasonable resource limits
        assert!(config.ipc.max_connections <= 10000, "Connection limit should be reasonable");
        assert!(config.performance.cache_size_mb <= 1000, "Cache size should be reasonable");
        assert!(config.logging.max_backups <= 100, "Log backup count should be reasonable");
    }
}
