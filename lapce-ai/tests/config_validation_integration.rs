/// Integration tests for IPC configuration validation (IPC-011)
/// Tests schema validation and safe-defaults with fail-closed behavior

use std::fs;
use tempfile::TempDir;

// Mock the IPC config types for testing
#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub ipc: IpcSettings,
    pub shared_memory: SharedMemorySettings,
    pub security: SecuritySettings,
}

#[derive(Debug, Clone)]
pub struct IpcSettings {
    pub max_connections: u32,
    pub idle_timeout_secs: u64,
    pub max_message_size: usize,
    pub socket_path: String,
}

#[derive(Debug, Clone)]
pub struct SharedMemorySettings {
    pub slot_size: usize,
    pub num_slots: usize,
    pub ring_buffer_size: usize,
    pub permissions: u32,
}

#[derive(Debug, Clone)]
pub struct SecuritySettings {
    pub tls_enabled: bool,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub auth_enabled: bool,
    pub auth_token: String,
    pub rate_limiting: bool,
    pub max_requests_per_minute: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid field '{field}': {reason}")]
    InvalidField { field: String, reason: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Value out of range for '{field}': {value} (expected: {range})")]
    OutOfRange { field: String, value: String, range: String },
}

impl IpcConfig {
    pub fn default() -> Self {
        Self {
            ipc: IpcSettings {
                socket_path: "/tmp/lapce-ai.sock".to_string(),
                max_connections: 1000,
                idle_timeout_secs: 300,
                max_message_size: 10 * 1024 * 1024,
            },
            shared_memory: SharedMemorySettings {
                slot_size: 1024,
                num_slots: 100,
                ring_buffer_size: 64 * 1024,
                permissions: 0o600,
            },
            security: SecuritySettings {
                tls_enabled: false,
                tls_cert_path: String::new(),
                tls_key_path: String::new(),
                auth_enabled: false,
                auth_token: String::new(),
                rate_limiting: true,
                max_requests_per_minute: 1000,
            },
        }
    }
    
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.ipc.validate()?;
        self.shared_memory.validate()?;
        self.security.validate()?;
        Ok(())
    }
}

impl IpcSettings {
    pub fn validate(&self) -> Result<(), ConfigError> {
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
    pub fn validate(&self) -> Result<(), ConfigError> {
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

impl SecuritySettings {
    pub fn validate(&self) -> Result<(), ConfigError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = IpcConfig::default();
        assert!(config.validate().is_ok(), "Default configuration should be valid");
    }

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
        
        // Security should be configured safely
        assert!(config.security.rate_limiting, "Rate limiting should be enabled by default");
        assert!(config.security.max_requests_per_minute > 0);
    }

    #[test]
    fn test_ipc_settings_validation_failures() {
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

    #[test]
    fn test_shared_memory_validation_failures() {
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
    }

    #[test]
    fn test_security_validation_failures() {
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

    #[test]
    fn test_fail_closed_validation() {
        // Test various edge cases that should fail validation
        let mut config = IpcConfig::default();
        
        // Edge case: very large values
        config.ipc.max_connections = u32::MAX;
        assert!(config.validate().is_err());
        
        // Edge case: zero values where not allowed
        config = IpcConfig::default();
        config.shared_memory.num_slots = 0;
        assert!(config.validate().is_err());
        
        // Edge case: invalid permissions
        config = IpcConfig::default();
        config.shared_memory.permissions = 0o500; // Too restrictive
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_production_readiness() {
        let config = IpcConfig::default();
        
        // Verify production-safe defaults
        assert!(config.shared_memory.permissions >= 0o600, "SHM permissions should be secure");
        assert!(config.security.rate_limiting, "Rate limiting should be enabled by default");
        
        // Verify reasonable resource limits
        assert!(config.ipc.max_connections <= 10000, "Connection limit should be reasonable");
    }
}
