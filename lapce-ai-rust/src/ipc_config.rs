/// IPC Server Configuration Module
/// Loads and manages configuration from TOML files

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

/// Complete IPC Server Configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcConfig {
    pub server: ServerConfig,
    pub providers: ProvidersConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub socket_path: String,
    pub max_connections: usize,
    pub idle_timeout_secs: u64,
    pub max_message_size: usize,
    pub buffer_pool_size: usize,
    pub enable_auto_reconnect: bool,
    pub reconnect_delay_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub enabled_providers: Vec<String>,
    pub default_provider: String,
    pub fallback_enabled: bool,
    pub fallback_order: Vec<String>,
    pub load_balance: bool,
    pub circuit_breaker_enabled: bool,
    pub circuit_breaker_threshold: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub enable_compression: bool,
    pub compression_threshold: usize,
    pub enable_binary_protocol: bool,
    pub worker_threads: usize,
    pub max_concurrent_requests: usize,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub allowed_origins: Vec<String>,
    pub rate_limit_per_second: Option<u32>,
    pub max_request_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub metrics_endpoint: String,
    pub enable_tracing: bool,
    pub log_level: String,
    pub health_check_interval_secs: u64,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                socket_path: "/tmp/lapce-ai.sock".to_string(),
                max_connections: 1000,
                idle_timeout_secs: 300,
                max_message_size: 10 * 1024 * 1024, // 10MB
                buffer_pool_size: 100,
                enable_auto_reconnect: true,
                reconnect_delay_ms: 50,
            },
            providers: ProvidersConfig {
                enabled_providers: vec!["openai".to_string(), "anthropic".to_string()],
                default_provider: "openai".to_string(),
                fallback_enabled: true,
                fallback_order: vec!["openai".to_string(), "anthropic".to_string()],
                load_balance: false,
                circuit_breaker_enabled: true,
                circuit_breaker_threshold: 5,
            },
            performance: PerformanceConfig {
                enable_compression: false,
                compression_threshold: 1024,
                enable_binary_protocol: true,
                worker_threads: 4,
                max_concurrent_requests: 100,
                request_timeout_secs: 30,
            },
            security: SecurityConfig {
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
                allowed_origins: vec!["*".to_string()],
                rate_limit_per_second: Some(1000),
                max_request_size: 10 * 1024 * 1024,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_port: 9090,
                metrics_endpoint: "/metrics".to_string(),
                enable_tracing: false,
                log_level: "info".to_string(),
                health_check_interval_secs: 5,
            },
        }
    }
}

impl IpcConfig {
    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        
        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path))
    }
    
    /// Save configuration to TOML file
    pub fn save(&self, path: &str) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {}", path))?;
        
        Ok(())
    }
    
    /// Create default config file if it doesn't exist
    pub fn create_default_if_missing(path: &str) -> Result<Self> {
        if Path::new(path).exists() {
            Self::from_file(path)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.max_connections == 0 {
            anyhow::bail!("max_connections must be greater than 0");
        }
        
        if self.server.max_message_size == 0 {
            anyhow::bail!("max_message_size must be greater than 0");
        }
        
        // Validate TLS config
        if self.security.enable_tls {
            if self.security.tls_cert_path.is_none() || self.security.tls_key_path.is_none() {
                anyhow::bail!("TLS enabled but cert/key paths not provided");
            }
        }
        
        // Validate provider config
        if self.providers.enabled_providers.is_empty() {
            anyhow::bail!("No providers enabled");
        }
        
        if !self.providers.enabled_providers.contains(&self.providers.default_provider) {
            anyhow::bail!("Default provider not in enabled providers list");
        }
        
        Ok(())
    }
}

/// Environment variable overrides
impl IpcConfig {
    pub fn apply_env_overrides(mut self) -> Self {
        // Override socket path
        if let Ok(path) = std::env::var("LAPCE_IPC_SOCKET") {
            self.server.socket_path = path;
        }
        
        // Override max connections
        if let Ok(max) = std::env::var("LAPCE_IPC_MAX_CONNECTIONS") {
            if let Ok(max) = max.parse() {
                self.server.max_connections = max;
            }
        }
        
        // Override default provider
        if let Ok(provider) = std::env::var("LAPCE_DEFAULT_PROVIDER") {
            self.providers.default_provider = provider;
        }
        
        // Override log level
        if let Ok(level) = std::env::var("LAPCE_LOG_LEVEL") {
            self.monitoring.log_level = level;
        }
        
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = IpcConfig::default();
        assert_eq!(config.server.socket_path, "/tmp/lapce-ai.sock");
        assert_eq!(config.server.max_connections, 1000);
        assert_eq!(config.providers.default_provider, "openai");
    }
    
    #[test]
    fn test_save_and_load() {
        let config = IpcConfig::default();
        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        config.save(path).unwrap();
        let loaded = IpcConfig::from_file(path).unwrap();
        
        assert_eq!(config.server.socket_path, loaded.server.socket_path);
        assert_eq!(config.server.max_connections, loaded.server.max_connections);
    }
    
    #[test]
    fn test_validation() {
        let mut config = IpcConfig::default();
        assert!(config.validate().is_ok());
        
        config.server.max_connections = 0;
        assert!(config.validate().is_err());
        
        config.server.max_connections = 100;
        config.security.enable_tls = true;
        assert!(config.validate().is_err()); // No cert/key paths
        
        config.security.tls_cert_path = Some("/path/to/cert".to_string());
        config.security.tls_key_path = Some("/path/to/key".to_string());
        assert!(config.validate().is_ok());
    }
}
