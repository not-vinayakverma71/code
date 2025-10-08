/// IPC Server Configuration Module
/// Loads and manages configuration from TOML files

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

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
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let mut config: IpcConfig = toml::from_str(&content)
            .context("Failed to parse TOML config")?;
        
        // Expand environment variables
        config.expand_env_vars();
        
        Ok(config)
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
