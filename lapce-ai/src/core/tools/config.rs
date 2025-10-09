// Centralized tool configuration - P0-config

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::HashMap;

/// Central configuration for tool execution system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Approval policy settings
    pub approval: ApprovalPolicy,
    
    /// Timeout settings
    pub timeouts: TimeoutConfig,
    
    /// Security settings
    pub security: SecurityConfig,
    
    /// Performance settings
    pub performance: PerformanceConfig,
    
    /// Tool-specific overrides
    pub tool_overrides: HashMap<String, ToolOverride>,
}

/// Approval policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    /// Require approval for all destructive operations
    pub require_for_destructive: bool,
    
    /// Require approval for file writes
    pub require_for_writes: bool,
    
    /// Require approval for command execution
    pub require_for_commands: bool,
    
    /// Auto-approve read operations
    pub auto_approve_reads: bool,
    
    /// Approval timeout in seconds
    pub timeout_seconds: u64,
    
    /// Default action on timeout (approve/deny)
    pub default_on_timeout: ApprovalDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDefault {
    Approve,
    Deny,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default tool execution timeout
    pub default_execution_seconds: u64,
    
    /// Command execution timeout
    pub command_execution_seconds: u64,
    
    /// File operation timeout
    pub file_operation_seconds: u64,
    
    /// Network operation timeout
    pub network_operation_seconds: u64,
    
    /// Maximum allowed timeout
    pub max_timeout_seconds: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable workspace bounds checking
    pub enforce_workspace_bounds: bool,
    
    /// Enable .rooignore enforcement
    pub enforce_rooignore: bool,
    
    /// Enable dangerous command blocking
    pub block_dangerous_commands: bool,
    
    /// Additional blocked commands
    pub additional_blocked_commands: Vec<String>,
    
    /// Maximum file size for operations (bytes)
    pub max_file_size_bytes: u64,
    
    /// Maximum command output size (bytes)
    pub max_output_size_bytes: u64,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable caching for .rooignore
    pub cache_rooignore: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Maximum concurrent tool executions
    pub max_concurrent_tools: usize,
    
    /// Enable performance metrics collection
    pub collect_metrics: bool,
    
    /// Stream command output
    pub stream_command_output: bool,
}

/// Tool-specific override configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOverride {
    /// Override approval requirement
    pub require_approval: Option<bool>,
    
    /// Override timeout
    pub timeout_seconds: Option<u64>,
    
    /// Tool-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            approval: ApprovalPolicy::default(),
            timeouts: TimeoutConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            tool_overrides: HashMap::new(),
        }
    }
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self {
            require_for_destructive: true,
            require_for_writes: true,
            require_for_commands: true,
            auto_approve_reads: true,
            timeout_seconds: 30,
            default_on_timeout: ApprovalDefault::Deny,
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_execution_seconds: 30,
            command_execution_seconds: 30,
            file_operation_seconds: 10,
            network_operation_seconds: 60,
            max_timeout_seconds: 300, // 5 minutes
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enforce_workspace_bounds: true,
            enforce_rooignore: true,
            block_dangerous_commands: true,
            additional_blocked_commands: vec![
                "dd".to_string(),
                "mkfs".to_string(),
                "fdisk".to_string(),
                "parted".to_string(),
            ],
            max_file_size_bytes: 100 * 1024 * 1024, // 100MB
            max_output_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_rooignore: true,
            cache_ttl_seconds: 300, // 5 minutes
            max_concurrent_tools: 10,
            collect_metrics: true,
            stream_command_output: true,
        }
    }
}

impl ToolConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Get timeout for a specific tool
    pub fn get_timeout(&self, tool_name: &str) -> Duration {
        if let Some(override_config) = self.tool_overrides.get(tool_name) {
            if let Some(timeout) = override_config.timeout_seconds {
                return Duration::from_secs(timeout.min(self.timeouts.max_timeout_seconds));
            }
        }
        Duration::from_secs(self.timeouts.default_execution_seconds)
    }
    
    /// Check if approval is required for a tool
    pub fn requires_approval(&self, tool_name: &str, operation: &str) -> bool {
        // Check tool-specific override first
        if let Some(override_config) = self.tool_overrides.get(tool_name) {
            if let Some(require) = override_config.require_approval {
                return require;
            }
        }
        
        // Check operation type
        match operation {
            "read" => !self.approval.auto_approve_reads,
            "write" | "create" | "delete" => self.approval.require_for_writes,
            "execute" => self.approval.require_for_commands,
            _ => self.approval.require_for_destructive,
        }
    }
    
    /// Check if a command is blocked
    pub fn is_command_blocked(&self, command: &str) -> bool {
        if !self.security.block_dangerous_commands {
            return false;
        }
        
        // Check additional blocked commands
        let cmd_lower = command.to_lowercase();
        self.security.additional_blocked_commands.iter()
            .any(|blocked| cmd_lower.contains(blocked))
    }
    
    /// Get maximum file size
    pub fn max_file_size(&self) -> usize {
        self.security.max_file_size_bytes as usize
    }
    
    /// Get maximum output size
    pub fn max_output_size(&self) -> usize {
        self.security.max_output_size_bytes as usize
    }
}

/// Global configuration instance
static mut GLOBAL_CONFIG: Option<ToolConfig> = None;
static CONFIG_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global configuration
pub fn get_config() -> &'static ToolConfig {
    unsafe {
        CONFIG_INIT.call_once(|| {
            GLOBAL_CONFIG = Some(ToolConfig::default());
        });
        GLOBAL_CONFIG.as_ref().unwrap()
    }
}

/// Set the global configuration
pub fn set_config(config: ToolConfig) {
    unsafe {
        CONFIG_INIT.call_once(|| {});
        GLOBAL_CONFIG = Some(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ToolConfig::default();
        
        assert!(config.approval.require_for_destructive);
        assert!(config.approval.require_for_writes);
        assert!(config.security.enforce_workspace_bounds);
        assert_eq!(config.timeouts.default_execution_seconds, 30);
    }
    
    #[test]
    fn test_timeout_override() {
        let mut config = ToolConfig::default();
        
        // Add tool override
        config.tool_overrides.insert(
            "slow_tool".to_string(),
            ToolOverride {
                require_approval: None,
                timeout_seconds: Some(120),
                settings: HashMap::new(),
            },
        );
        
        assert_eq!(config.get_timeout("slow_tool").as_secs(), 120);
        assert_eq!(config.get_timeout("normal_tool").as_secs(), 30);
    }
    
    #[test]
    fn test_approval_requirements() {
        let config = ToolConfig::default();
        
        assert!(!config.requires_approval("read_file", "read"));
        assert!(config.requires_approval("write_file", "write"));
        assert!(config.requires_approval("execute_command", "execute"));
    }
    
    #[test]
    fn test_command_blocking() {
        let config = ToolConfig::default();
        
        assert!(config.is_command_blocked("dd if=/dev/zero"));
        assert!(config.is_command_blocked("mkfs.ext4"));
        assert!(!config.is_command_blocked("ls -la"));
    }
    
    #[test]
    fn test_config_with_dangerous_command_override() {
        let mut config = ToolConfig::default();
        
        // Add additional dangerous commands
        config.security.additional_blocked_commands.push("format".to_string());
        config.security.additional_blocked_commands.push("fdisk".to_string());
        
        // Test that both default and additional commands are blocked
        assert!(config.is_command_blocked("dd if=/dev/zero"));
        assert!(config.is_command_blocked("format c:"));
        assert!(config.is_command_blocked("fdisk /dev/sda"));
        assert!(!config.is_command_blocked("ls -la"));
    }
    
    #[test]
    fn test_timeout_max_enforcement() {
        let mut config = ToolConfig::default();
        config.timeouts.max_timeout_seconds = 60;
        
        // Add tool override with timeout exceeding max
        config.tool_overrides.insert(
            "long_tool".to_string(),
            ToolOverride {
                require_approval: None,
                timeout_seconds: Some(120),
                settings: HashMap::new(),
            },
        );
        
        // Should cap at max_timeout_seconds
        assert_eq!(config.get_timeout("long_tool").as_secs(), 60);
    }
    
    #[test]
    fn test_approval_policy_override() {
        let mut config = ToolConfig::default();
        
        // Set default policies
        config.approval.auto_approve_reads = true;
        config.approval.require_for_writes = true;
        
        // Add tool that doesn't require approval
        config.tool_overrides.insert(
            "trusted_tool".to_string(),
            ToolOverride {
                require_approval: Some(false),
                timeout_seconds: None,
                settings: HashMap::new(),
            },
        );
        
        // Check various scenarios
        assert!(!config.requires_approval("any_tool", "read"));
        assert!(config.requires_approval("any_tool", "write"));
        assert!(!config.requires_approval("trusted_tool", "write"));
    }
    
    #[test]
    fn test_config_serialization() {
        let config = ToolConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        
        assert!(toml_str.contains("[approval]"));
        assert!(toml_str.contains("[timeouts]"));
        assert!(toml_str.contains("[security]"));
        assert!(toml_str.contains("[performance]"));
    }
}
