/// IPC Server Configuration Module
/// Loads and manages configuration from TOML files

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use thiserror::Error;

/// Configuration validation errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid field '{field}': {reason}")]
    InvalidField { field: String, reason: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Value out of range for '{field}': {value} (expected: {range})")]
    OutOfRange { field: String, value: String, range: String },
    
    #[error("Invalid format for '{field}': {value} (expected: {expected})")]
    InvalidFormat { field: String, value: String, expected: String },
    
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// Complete IPC Server Configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcConfig {
    pub ipc: IpcSettings,
    pub shared_memory: SharedMemorySettings,
    pub metrics: MetricsSettings,
    pub monitoring: MonitoringSettings,
    pub reconnection: ReconnectionSettings,
    pub providers: ProvidersConfig,
    pub logging: LoggingSettings,
    pub performance: PerformanceSettings,
    pub security: SecuritySettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcSettings {
    pub socket_path: String,
    pub max_connections: usize,
    pub idle_timeout_secs: u64,
    pub max_message_size: usize,
    pub buffer_pool_size: usize,
    pub connection_pool_max_idle: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SharedMemorySettings {
    pub slot_size: usize,
    pub num_slots: usize,
    pub permissions: u32,
    pub ring_buffer_size: usize,  // Size per connection buffer
    pub control_buffer_size: usize,  // Size of control channel
    pub max_memory_per_connection: usize,  // Memory limit per connection
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsSettings {
    pub enable: bool,
    pub export_interval_secs: u64,
    pub export_path: String,
    pub retention_hours: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringSettings {
    pub health_check_enabled: bool,
    pub health_check_port: u16,
    pub health_check_path: String,
    pub prometheus_enabled: bool,
    pub prometheus_port: u16,
    pub grafana_dashboard_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReconnectionSettings {
    pub strategy: String,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub max_retries: u32,
    pub health_check_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub openai: ProviderSettings,
    pub anthropic: ProviderSettings,
    pub gemini: ProviderSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderSettings {
    pub enabled: bool,
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub rate_limit_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingSettings {
    pub level: String,
    pub format: String,
    pub output: String,
    pub rotation_size_mb: u64,
    pub max_backups: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceSettings {
    pub zero_copy: bool,
    pub buffer_reuse: bool,
    pub connection_pooling: bool,
    pub cache_enabled: bool,
    pub cache_size_mb: u64,
    pub cache_ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecuritySettings {
    pub tls_enabled: bool,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub auth_enabled: bool,
    pub auth_token: String,
    pub rate_limiting: bool,
    pub max_requests_per_minute: u32,
}

impl IpcConfig {
    /// Load configuration from TOML file with validation
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let mut config: IpcConfig = toml::from_str(&content)
            .context("Failed to parse TOML config")?;
        
        // Expand environment variables
        config.expand_env_vars();
        
        // Validate configuration
        config.validate()
            .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
        
        Ok(config)
    }
    
    /// Validate entire configuration with fail-closed behavior
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        self.ipc.validate()?;
        self.shared_memory.validate()?;
        self.metrics.validate()?;
        self.monitoring.validate()?;
        self.reconnection.validate()?;
        self.providers.validate()?;
        self.logging.validate()?;
        self.performance.validate()?;
        self.security.validate()?;
        Ok(())
    }
    
    /// Load with defaults and override from file if exists
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("LAPCE_AI_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());
        
        if Path::new(&config_path).exists() {
            Self::from_file(&config_path)
        } else {
            Ok(Self::default())
        }
    }
    
    /// Expand environment variables in string fields
    fn expand_env_vars(&mut self) {
        // Expand provider API keys
        if self.providers.openai.api_key.starts_with("${")
            && self.providers.openai.api_key.ends_with("}") {
            let var_name = &self.providers.openai.api_key[2..self.providers.openai.api_key.len()-1];
            if let Ok(value) = std::env::var(var_name) {
                self.providers.openai.api_key = value;
            }
        }
        
        if self.providers.anthropic.api_key.starts_with("${")
            && self.providers.anthropic.api_key.ends_with("}") {
            let var_name = &self.providers.anthropic.api_key[2..self.providers.anthropic.api_key.len()-1];
            if let Ok(value) = std::env::var(var_name) {
                self.providers.anthropic.api_key = value;
            }
        }
        
        if self.providers.gemini.api_key.starts_with("${")
            && self.providers.gemini.api_key.ends_with("}") {
            let var_name = &self.providers.gemini.api_key[2..self.providers.gemini.api_key.len()-1];
            if let Ok(value) = std::env::var(var_name) {
                self.providers.gemini.api_key = value;
            }
        }
    }
}

/// Validation implementations for each settings struct
impl IpcSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.max_connections == 0 || self.max_connections > 10000 {
            return Err(ConfigError::OutOfRange {
                field: "ipc.max_connections".to_string(),
                value: self.max_connections.to_string(),
                range: "1-10000".to_string(),
            });
        }
        
        if self.idle_timeout_secs > 3600 {
            return Err(ConfigError::OutOfRange {
                field: "ipc.idle_timeout_secs".to_string(),
                value: self.idle_timeout_secs.to_string(),
                range: "1-3600".to_string(),
            });
        }
        
        if self.max_message_size < 1024 || self.max_message_size > 100 * 1024 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "ipc.max_message_size".to_string(),
                value: self.max_message_size.to_string(),
                range: "1024-104857600 bytes".to_string(),
            });
        }
        
        if self.socket_path.is_empty() {
            return Err(ConfigError::MissingField {
                field: "ipc.socket_path".to_string(),
            });
        }
        
        Ok(())
    }
}

impl SharedMemorySettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.slot_size < 64 || self.slot_size > 64 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "shared_memory.slot_size".to_string(),
                value: self.slot_size.to_string(),
                range: "64-65536 bytes".to_string(),
            });
        }
        
        if self.num_slots < 1 || self.num_slots > 10000 {
            return Err(ConfigError::OutOfRange {
                field: "shared_memory.num_slots".to_string(),
                value: self.num_slots.to_string(),
                range: "1-10000".to_string(),
            });
        }
        
        if self.ring_buffer_size < 4096 || self.ring_buffer_size > 1024 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "shared_memory.ring_buffer_size".to_string(),
                value: self.ring_buffer_size.to_string(),
                range: "4096-1048576 bytes".to_string(),
            });
        }
        
        // Validate permissions are valid octal (0o600-0o777)
        if self.permissions < 0o600 || self.permissions > 0o777 {
            return Err(ConfigError::OutOfRange {
                field: "shared_memory.permissions".to_string(),
                value: format!("{:o}", self.permissions),
                range: "0600-0777 (octal)".to_string(),
            });
        }
        
        Ok(())
    }
}

impl MetricsSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.export_interval_secs < 5 || self.export_interval_secs > 3600 {
            return Err(ConfigError::OutOfRange {
                field: "metrics.export_interval_secs".to_string(),
                value: self.export_interval_secs.to_string(),
                range: "5-3600 seconds".to_string(),
            });
        }
        
        if self.retention_hours > 168 { // Max 1 week
            return Err(ConfigError::OutOfRange {
                field: "metrics.retention_hours".to_string(),
                value: self.retention_hours.to_string(),
                range: "1-168 hours".to_string(),
            });
        }
        
        if self.export_path.is_empty() {
            return Err(ConfigError::MissingField {
                field: "metrics.export_path".to_string(),
            });
        }
        
        Ok(())
    }
}

impl MonitoringSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.health_check_port < 1024 || self.health_check_port > 65535 {
            return Err(ConfigError::OutOfRange {
                field: "monitoring.health_check_port".to_string(),
                value: self.health_check_port.to_string(),
                range: "1024-65535".to_string(),
            });
        }
        
        if self.prometheus_port < 1024 || self.prometheus_port > 65535 {
            return Err(ConfigError::OutOfRange {
                field: "monitoring.prometheus_port".to_string(),
                value: self.prometheus_port.to_string(),
            range: "1024-65535".to_string(),
            });
        }
        
        if self.health_check_port == self.prometheus_port {
            return Err(ConfigError::InvalidField {
                field: "monitoring.ports".to_string(),
                reason: "health_check_port and prometheus_port cannot be the same".to_string(),
            });
        }
        
        Ok(())
    }
}

impl ReconnectionSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        let valid_strategies = ["exponential", "linear", "constant"];
        if !valid_strategies.contains(&self.strategy.as_str()) {
            return Err(ConfigError::InvalidFormat {
                field: "reconnection.strategy".to_string(),
                value: self.strategy.clone(),
                expected: "exponential, linear, or constant".to_string(),
            });
        }
        
        if self.initial_delay_ms == 0 || self.initial_delay_ms > 60000 {
            return Err(ConfigError::OutOfRange {
                field: "reconnection.initial_delay_ms".to_string(),
                value: self.initial_delay_ms.to_string(),
                range: "1-60000 ms".to_string(),
            });
        }
        
        if self.max_delay_ms < self.initial_delay_ms || self.max_delay_ms > 300000 {
            return Err(ConfigError::InvalidField {
                field: "reconnection.max_delay_ms".to_string(),
                reason: "must be >= initial_delay_ms and <= 300000".to_string(),
            });
        }
        
        if self.multiplier < 1.0 || self.multiplier > 10.0 {
            return Err(ConfigError::OutOfRange {
                field: "reconnection.multiplier".to_string(),
                value: self.multiplier.to_string(),
                range: "1.0-10.0".to_string(),
            });
        }
        
        if self.max_retries > 100 {
            return Err(ConfigError::OutOfRange {
                field: "reconnection.max_retries".to_string(),
                value: self.max_retries.to_string(),
                range: "0-100".to_string(),
            });
        }
        
        Ok(())
    }
}

impl ProvidersConfig {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        self.openai.validate("openai")?;
        self.anthropic.validate("anthropic")?;
        self.gemini.validate("gemini")?;
        Ok(())
    }
}

impl ProviderSettings {
    pub fn validate(&self, provider_name: &str) -> std::result::Result<(), ConfigError> {
        if self.enabled && self.api_key.is_empty() {
            return Err(ConfigError::MissingField {
                field: format!("providers.{}.api_key", provider_name),
            });
        }
        
        if self.base_url.is_empty() {
            return Err(ConfigError::MissingField {
                field: format!("providers.{}.base_url", provider_name),
            });
        }
        
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(ConfigError::InvalidFormat {
                field: format!("providers.{}.base_url", provider_name),
                value: self.base_url.clone(),
                expected: "URL starting with http:// or https://".to_string(),
            });
        }
        
        if self.timeout_secs == 0 || self.timeout_secs > 300 {
            return Err(ConfigError::OutOfRange {
                field: format!("providers.{}.timeout_secs", provider_name),
                value: self.timeout_secs.to_string(),
                range: "1-300 seconds".to_string(),
            });
        }
        
        if self.max_retries > 10 {
            return Err(ConfigError::OutOfRange {
                field: format!("providers.{}.max_retries", provider_name),
                value: self.max_retries.to_string(),
                range: "0-10".to_string(),
            });
        }
        
        Ok(())
    }
}

impl LoggingSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            return Err(ConfigError::InvalidFormat {
                field: "logging.level".to_string(),
                value: self.level.clone(),
                expected: "trace, debug, info, warn, or error".to_string(),
            });
        }
        
        let valid_formats = ["json", "plain", "compact"];
        if !valid_formats.contains(&self.format.as_str()) {
            return Err(ConfigError::InvalidFormat {
                field: "logging.format".to_string(),
                value: self.format.clone(),
                expected: "json, plain, or compact".to_string(),
            });
        }
        
        let valid_outputs = ["stdout", "stderr", "file"];
        if !valid_outputs.contains(&self.output.as_str()) {
            return Err(ConfigError::InvalidFormat {
                field: "logging.output".to_string(),
                value: self.output.clone(),
                expected: "stdout, stderr, or file".to_string(),
            });
        }
        
        if self.rotation_size_mb > 1000 {
            return Err(ConfigError::OutOfRange {
                field: "logging.rotation_size_mb".to_string(),
                value: self.rotation_size_mb.to_string(),
                range: "1-1000 MB".to_string(),
            });
        }
        
        if self.max_backups > 100 {
            return Err(ConfigError::OutOfRange {
                field: "logging.max_backups".to_string(),
                value: self.max_backups.to_string(),
                range: "1-100".to_string(),
            });
        }
        
        Ok(())
    }
}

impl PerformanceSettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.cache_size_mb > 1000 {
            return Err(ConfigError::OutOfRange {
                field: "performance.cache_size_mb".to_string(),
                value: self.cache_size_mb.to_string(),
                range: "1-1000 MB".to_string(),
            });
        }
        
        if self.cache_ttl_secs > 86400 { // Max 24 hours
            return Err(ConfigError::OutOfRange {
                field: "performance.cache_ttl_secs".to_string(),
                value: self.cache_ttl_secs.to_string(),
                range: "1-86400 seconds".to_string(),
            });
        }
        
        Ok(())
    }
}

impl SecuritySettings {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.tls_enabled {
            if self.tls_cert_path.is_empty() {
                return Err(ConfigError::MissingField {
                    field: "security.tls_cert_path".to_string(),
                });
            }
            
            if self.tls_key_path.is_empty() {
                return Err(ConfigError::MissingField {
                    field: "security.tls_key_path".to_string(),
                });
            }
            
            // Validate cert and key files exist
            if !Path::new(&self.tls_cert_path).exists() {
                return Err(ConfigError::InvalidField {
                    field: "security.tls_cert_path".to_string(),
                    reason: "certificate file does not exist".to_string(),
                });
            }
            
            if !Path::new(&self.tls_key_path).exists() {
                return Err(ConfigError::InvalidField {
                    field: "security.tls_key_path".to_string(),
                    reason: "key file does not exist".to_string(),
                });
            }
        }
        
        if self.auth_enabled && self.auth_token.is_empty() {
            return Err(ConfigError::MissingField {
                field: "security.auth_token".to_string(),
            });
        }
        
        if self.max_requests_per_minute == 0 || self.max_requests_per_minute > 100000 {
            return Err(ConfigError::OutOfRange {
                field: "security.max_requests_per_minute".to_string(),
                value: self.max_requests_per_minute.to_string(),
                range: "1-100000".to_string(),
            });
        }
        
        Ok(())
    }
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            ipc: IpcSettings {
                socket_path: "/tmp/lapce-ai.sock".to_string(),
                max_connections: 1000,
                idle_timeout_secs: 300,
                max_message_size: 10 * 1024 * 1024,
                buffer_pool_size: 100,
                connection_pool_max_idle: 500,
            },
            shared_memory: SharedMemorySettings {
                slot_size: 1024,
                num_slots: 1024,
                permissions: 0o600,
                ring_buffer_size: 32 * 1024,  // 32KB per connection (100 conns = 3.2MB)
                control_buffer_size: 4096,  // 4KB control channel
                max_memory_per_connection: 30 * 1024,  // 30KB max to stay under 3MB for 100 conns
            },
            metrics: MetricsSettings {
                enable: true,
                export_interval_secs: 60,
                export_path: "/metrics".to_string(),
                retention_hours: 24,
            },
            monitoring: MonitoringSettings {
                health_check_enabled: true,
                health_check_port: 9090,
                health_check_path: "/health".to_string(),
                prometheus_enabled: true,
                prometheus_port: 9091,
                grafana_dashboard_path: "./dashboards".to_string(),
            },
            reconnection: ReconnectionSettings {
                strategy: "exponential".to_string(),
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                multiplier: 2.0,
                max_retries: 10,
                health_check_interval_secs: 30,
            },
            providers: ProvidersConfig {
                openai: ProviderSettings {
                    enabled: true,
                    api_key: String::new(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4-turbo-preview".to_string(),
                    max_retries: 3,
                    timeout_secs: 30,
                    rate_limit_per_minute: Some(500),
                },
                anthropic: ProviderSettings {
                    enabled: true,
                    api_key: String::new(),
                    base_url: "https://api.anthropic.com".to_string(),
                    default_model: "claude-3-opus".to_string(),
                    max_retries: 3,
                    timeout_secs: 30,
                    rate_limit_per_minute: Some(100),
                },
                gemini: ProviderSettings {
                    enabled: true,
                    api_key: String::new(),
                    base_url: "https://generativelanguage.googleapis.com".to_string(),
                    default_model: "gemini-pro".to_string(),
                    max_retries: 3,
                    timeout_secs: 30,
                    rate_limit_per_minute: Some(200),
                },
            },
            logging: LoggingSettings {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stdout".to_string(),
                rotation_size_mb: 100,
                max_backups: 10,
            },
            performance: PerformanceSettings {
                zero_copy: true,
                buffer_reuse: true,
                connection_pooling: true,
                cache_enabled: true,
                cache_size_mb: 100,
                cache_ttl_secs: 3600,
            },
            security: SecuritySettings {
                tls_enabled: false,
                tls_cert_path: String::new(),
                tls_key_path: String::new(),
                auth_enabled: false,
                auth_token: String::new(),
                rate_limiting: true,
                max_requests_per_minute: 10000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = IpcConfig::default();
        assert_eq!(config.ipc.socket_path, "/tmp/lapce-ai.sock");
        assert_eq!(config.ipc.max_connections, 1000);
        assert_eq!(config.providers.openai.default_model, "gpt-4-turbo-preview");
    }
    
    #[test]
    fn test_load_config() {
        // Test with non-existent file returns defaults
        std::env::remove_var("LAPCE_AI_CONFIG");
        let config = IpcConfig::load().unwrap();
        assert_eq!(config.ipc.max_connections, 1000);
    }
    
    #[test]
    fn test_env_var_expansion() {
        std::env::set_var("TEST_API_KEY", "test-key-123");
        let mut config = IpcConfig::default();
        config.providers.openai.api_key = "${TEST_API_KEY}".to_string();
        config.expand_env_vars();
        assert_eq!(config.providers.openai.api_key, "test-key-123");
        std::env::remove_var("TEST_API_KEY");
    }
}
