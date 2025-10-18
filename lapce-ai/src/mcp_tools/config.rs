/// MCP Server Configuration
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name and version
    pub name: String,
    pub version: String,
    
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    
    /// Tool permissions configuration
    pub permissions: PermissionConfig,
    
    /// Rate limiting configuration
    pub rate_limits: RateLimitConfig,
    
    /// Sandboxing configuration
    pub sandbox: SandboxConfig,
    
    /// Logging and telemetry
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// List of available tools
    pub tools: Vec<String>,
    
    /// Supported protocols
    pub protocols: Vec<String>,
    
    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,
    
    /// Supported authentication methods
    pub auth_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Default permissions for all tools
    pub default: ToolPermissions,
    
    /// Per-tool permission overrides
    pub overrides: HashMap<String, ToolPermissions>,
    
    /// Workspace-specific permissions
    pub workspace_permissions: HashMap<PathBuf, ToolPermissions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub file_read: bool,
    pub file_write: bool,
    pub file_delete: bool,
    pub process_execute: bool,
    pub network_access: bool,
    pub system_info_read: bool,
    
    /// Allowed paths for file operations
    pub allowed_paths: Vec<PathBuf>,
    
    /// Blocked paths (takes precedence over allowed)
    pub blocked_paths: Vec<PathBuf>,
    
    /// Allowed commands for execution
    pub allowed_commands: Vec<String>,
    
    /// Maximum file size for operations (bytes)
    pub max_file_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: usize,
    
    /// Maximum requests per hour
    pub requests_per_hour: usize,
    
    /// Per-tool rate limits
    pub tool_limits: HashMap<String, ToolRateLimit>,
    
    /// Enable adaptive rate limiting
    pub adaptive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRateLimit {
    pub max_per_minute: usize,
    pub max_per_hour: usize,
    pub burst_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable chroot sandboxing
    pub use_chroot: bool,
    
    /// Enable Linux namespaces
    pub use_namespaces: bool,
    
    /// Enable seccomp filters
    pub use_seccomp: bool,
    
    /// Resource limits
    pub resource_limits: ResourceLimits,
    
    /// Sandbox root directory
    pub sandbox_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU time in seconds
    pub cpu_time_limit: Option<u64>,
    
    /// Maximum memory in bytes
    pub memory_limit: Option<usize>,
    
    /// Maximum disk usage in bytes
    pub disk_limit: Option<usize>,
    
    /// Maximum number of processes
    pub process_limit: Option<usize>,
    
    /// Maximum number of open files
    pub file_descriptor_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable telemetry collection
    pub enabled: bool,
    
    /// Log level
    pub log_level: String,
    
    /// Log file path
    pub log_file: Option<PathBuf>,
    
    /// Metrics collection interval (seconds)
    pub metrics_interval: u64,
    
    /// Include tool execution details
    pub include_tool_details: bool,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "lapce-mcp-server".to_string(),
            version: "1.0.0".to_string(),
            capabilities: ServerCapabilities::default(),
            permissions: PermissionConfig::default(),
            rate_limits: RateLimitConfig::default(),
            sandbox: SandboxConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            tools: vec![
                "readFile".to_string(),
                "writeFile".to_string(),
                "executeCommand".to_string(),
                "listFiles".to_string(),
                "searchFiles".to_string(),
            ],
            protocols: vec!["json".to_string(), "xml".to_string()],
            max_concurrent_operations: 10,
            auth_methods: vec!["none".to_string()],
        }
    }
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            default: ToolPermissions::default(),
            overrides: HashMap::new(),
            workspace_permissions: HashMap::new(),
        }
    }
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            file_read: true,
            file_write: true,
            file_delete: false,
            process_execute: false,
            network_access: false,
            system_info_read: false,
            allowed_paths: vec![],
            blocked_paths: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/sys"),
                PathBuf::from("/proc"),
                PathBuf::from("/boot"),
                PathBuf::from("/dev"),
            ],
            allowed_commands: vec![],
            max_file_size: Some(10_000_000), // 10MB
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            tool_limits: HashMap::new(),
            adaptive: false,
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            use_chroot: false,
            use_namespaces: false,
            use_seccomp: false,
            resource_limits: ResourceLimits::default(),
            sandbox_root: PathBuf::from("/tmp/mcp-sandbox"),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_time_limit: Some(30), // 30 seconds
            memory_limit: Some(512 * 1024 * 1024), // 512MB
            disk_limit: Some(1024 * 1024 * 1024), // 1GB
            process_limit: Some(10),
            file_descriptor_limit: Some(100),
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            log_file: Some(PathBuf::from("/tmp/mcp-server.log")),
            metrics_interval: 60,
            include_tool_details: true,
        }
    }
}

impl McpServerConfig {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::de::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::ser::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl McpServerConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        // Check for conflicting permissions
        for path in &self.permissions.default.allowed_paths {
            if self.permissions.default.blocked_paths.contains(path) {
                return Err(anyhow::anyhow!("Path {:?} is both allowed and blocked", path));
            }
        }
        
        // Check rate limits are reasonable
        if self.rate_limits.requests_per_minute > 1000 {
            return Err(anyhow::anyhow!("Rate limit per minute too high"));
        }
        
        // Check resource limits
        if let Some(mem) = self.sandbox.resource_limits.memory_limit {
            if mem < 1024 * 1024 { // Less than 1MB
                return Err(anyhow::anyhow!("Memory limit too low"));
            }
        }
        
        Ok(())
    }
}
