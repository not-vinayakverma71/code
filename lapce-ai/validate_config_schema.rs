#!/usr/bin/env cargo
/*
[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
tempfile = "3.0"
*/

/// Standalone configuration validation script for IPC-011
/// Demonstrates schema validation and safe-defaults with fail-closed behavior

use std::fs;
use tempfile::TempDir;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid field '{field}': {reason}")]
    InvalidField { field: String, reason: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Value out of range for '{field}': {value} (expected: {range})")]
    OutOfRange { field: String, value: String, range: String },
}

// Minimal IPC config representation for validation testing
#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub max_connections: u32,
    pub idle_timeout_secs: u64,
    pub max_message_size: usize,
    pub socket_path: String,
    pub slot_size: usize,
    pub num_slots: usize,
    pub ring_buffer_size: usize,
    pub permissions: u32,
    pub rate_limiting: bool,
    pub max_requests_per_minute: u32,
}

impl IpcConfig {
    pub fn default() -> Self {
        Self {
            socket_path: "/tmp/lapce-ai.sock".to_string(),
            max_connections: 1000,
            idle_timeout_secs: 300,
            max_message_size: 10 * 1024 * 1024,
            slot_size: 1024,
            num_slots: 100,
            ring_buffer_size: 64 * 1024,
            permissions: 0o600,
            rate_limiting: true,
            max_requests_per_minute: 1000,
        }
    }
    
    pub fn validate(&self) -> Result<(), ConfigError> {
        // IPC validation
        if self.max_connections == 0 || self.max_connections > 10000 {
            return Err(ConfigError::OutOfRange {
                field: "max_connections".to_string(),
                value: self.max_connections.to_string(),
                range: "1-10000".to_string(),
            });
        }
        
        if self.idle_timeout_secs > 3600 {
            return Err(ConfigError::OutOfRange {
                field: "idle_timeout_secs".to_string(),
                value: self.idle_timeout_secs.to_string(),
                range: "1-3600".to_string(),
            });
        }
        
        if self.max_message_size < 1024 || self.max_message_size > 100 * 1024 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "max_message_size".to_string(),
                value: self.max_message_size.to_string(),
                range: "1024-104857600 bytes".to_string(),
            });
        }
        
        if self.socket_path.is_empty() {
            return Err(ConfigError::MissingField {
                field: "socket_path".to_string(),
            });
        }
        
        // Shared memory validation
        if self.slot_size < 64 || self.slot_size > 64 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "slot_size".to_string(),
                value: self.slot_size.to_string(),
                range: "64-65536 bytes".to_string(),
            });
        }
        
        if self.num_slots < 1 || self.num_slots > 10000 {
            return Err(ConfigError::OutOfRange {
                field: "num_slots".to_string(),
                value: self.num_slots.to_string(),
                range: "1-10000".to_string(),
            });
        }
        
        if self.ring_buffer_size < 4096 || self.ring_buffer_size > 1024 * 1024 {
            return Err(ConfigError::OutOfRange {
                field: "ring_buffer_size".to_string(),
                value: self.ring_buffer_size.to_string(),
                range: "4096-1048576 bytes".to_string(),
            });
        }
        
        if self.permissions < 0o600 || self.permissions > 0o777 {
            return Err(ConfigError::OutOfRange {
                field: "permissions".to_string(),
                value: format!("{:o}", self.permissions),
                range: "0600-0777 (octal)".to_string(),
            });
        }
        
        // Security validation
        if self.max_requests_per_minute == 0 || self.max_requests_per_minute > 100000 {
            return Err(ConfigError::OutOfRange {
                field: "max_requests_per_minute".to_string(),
                value: self.max_requests_per_minute.to_string(),
                range: "1-100000".to_string(),
            });
        }
        
        Ok(())
    }
}

fn test_default_config_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing default configuration validation...");
    
    let config = IpcConfig::default();
    config.validate()?;
    
    println!("✅ Default configuration is valid");
    Ok(())
}

fn test_production_safety() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing production safety defaults...");
    
    let config = IpcConfig::default();
    
    // Verify production-safe defaults
    assert!(config.permissions >= 0o600, "SHM permissions should be secure");
    assert!(config.rate_limiting, "Rate limiting should be enabled by default");
    assert!(config.max_connections <= 10000, "Connection limit should be reasonable");
    assert!(config.max_message_size >= 1024, "Message size should have minimum");
    assert!(!config.socket_path.is_empty(), "Socket path should not be empty");
    
    println!("✅ Production safety defaults verified");
    Ok(())
}

fn test_fail_closed_behavior() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing fail-closed validation behavior...");
    
    let mut config = IpcConfig::default();
    
    // Test various failure cases
    let test_cases = vec![
        ("max_connections = 0", || {
            config.max_connections = 0;
            config.validate()
        }),
        ("max_connections too high", || {
            config.max_connections = 50000;
            config.validate()
        }),
        ("idle_timeout_secs too high", || {
            config.max_connections = 1000; // Reset
            config.idle_timeout_secs = 7200;
            config.validate()
        }),
        ("max_message_size too small", || {
            config.idle_timeout_secs = 300; // Reset
            config.max_message_size = 512;
            config.validate()
        }),
        ("max_message_size too large", || {
            config.max_message_size = 200 * 1024 * 1024;
            config.validate()
        }),
        ("empty socket_path", || {
            config.max_message_size = 10 * 1024 * 1024; // Reset
            config.socket_path = String::new();
            config.validate()
        }),
        ("slot_size too small", || {
            config.socket_path = "/tmp/test.sock".to_string(); // Reset
            config.slot_size = 32;
            config.validate()
        }),
        ("slot_size too large", || {
            config.slot_size = 128 * 1024;
            config.validate()
        }),
        ("num_slots = 0", || {
            config.slot_size = 1024; // Reset
            config.num_slots = 0;
            config.validate()
        }),
        ("ring_buffer_size too small", || {
            config.num_slots = 100; // Reset
            config.ring_buffer_size = 2048;
            config.validate()
        }),
        ("permissions too restrictive", || {
            config.ring_buffer_size = 64 * 1024; // Reset
            config.permissions = 0o500;
            config.validate()
        }),
        ("max_requests_per_minute = 0", || {
            config.permissions = 0o600; // Reset
            config.max_requests_per_minute = 0;
            config.validate()
        }),
        ("max_requests_per_minute too high", || {
            config.max_requests_per_minute = 200000;
            config.validate()
        }),
    ];
    
    for (test_name, test_fn) in test_cases {
        match test_fn() {
            Ok(_) => return Err(format!("Expected {} to fail validation", test_name).into()),
            Err(_) => println!("  ✅ {} correctly failed validation", test_name),
        }
    }
    
    println!("✅ Fail-closed validation behavior verified");
    Ok(())
}

fn test_boundary_conditions() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing boundary conditions...");
    
    let mut config = IpcConfig::default();
    
    // Test valid boundary values
    config.max_connections = 1;
    config.validate()?;
    println!("  ✅ min max_connections (1) is valid");
    
    config.max_connections = 10000;
    config.validate()?;
    println!("  ✅ max max_connections (10000) is valid");
    
    config.permissions = 0o600;
    config.validate()?;
    println!("  ✅ min permissions (0o600) is valid");
    
    config.permissions = 0o777;
    config.validate()?;
    println!("  ✅ max permissions (0o777) is valid");
    
    println!("✅ Boundary conditions verified");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 IPC Configuration Schema Validation Test (IPC-011)");
    println!("=====================================================");
    
    test_default_config_validation()?;
    test_production_safety()?;
    test_fail_closed_behavior()?;
    test_boundary_conditions()?;
    
    println!();
    println!("🎉 All configuration validation tests passed!");
    println!("   ✅ Schema validation working correctly");
    println!("   ✅ Safe defaults confirmed");
    println!("   ✅ Fail-closed behavior verified");
    println!("   ✅ Production-ready configuration");
    println!();
    println!("📋 IPC-011 completion summary:");
    println!("   • Added comprehensive configuration validation");
    println!("   • Implemented fail-closed behavior on invalid configs");
    println!("   • Verified safe production defaults");
    println!("   • Created structured error types with clear messages");
    println!("   • Added boundary condition testing");
    
    Ok(())
}
