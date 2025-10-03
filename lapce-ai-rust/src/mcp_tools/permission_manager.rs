/// Permission Manager for MCP Tools
/// Manages and enforces permissions for tool execution

use crate::mcp_tools::{
    config::{ToolPermissions, McpServerConfig},
    permissions::Permission,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, bail};

/// Permission manager for enforcing tool permissions
pub struct PermissionManager {
    config: Arc<RwLock<McpServerConfig>>,
    permission_cache: Arc<RwLock<HashMap<String, PermissionSet>>>,
}

#[derive(Clone, Debug)]
pub struct PermissionSet {
    pub tool_name: String,
    pub permissions: ToolPermissions,
    pub cached_at: std::time::Instant,
}

impl PermissionManager {
    pub fn new(config: Arc<RwLock<McpServerConfig>>) -> Self {
        Self {
            config,
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if a specific permission is allowed
    pub async fn check_permission(
        &self,
        tool_name: &str,
        permission: &Permission,
        path: Option<&Path>,
    ) -> Result<bool> {
        let permissions = self.get_tool_permissions(tool_name).await?;
        
        match permission {
            Permission::FileRead(_) => {
                if !permissions.file_read {
                    return Ok(false);
                }
                if let Some(p) = path {
                    self.check_path_permission(p, &permissions).await
                } else {
                    Ok(true)
                }
            },
            Permission::FileWrite(_) => {
                if !permissions.file_write {
                    return Ok(false);
                }
                if let Some(p) = path {
                    self.check_path_permission(p, &permissions).await
                } else {
                    Ok(true)
                }
            },
            Permission::FileDelete(_) => {
                if !permissions.file_delete {
                    return Ok(false);
                }
                if let Some(p) = path {
                    self.check_path_permission(p, &permissions).await
                } else {
                    Ok(true)
                }
            },
            Permission::ProcessExecute(cmd) => {
                if !permissions.process_execute {
                    return Ok(false);
                }
                self.check_command_permission(cmd, &permissions).await
            },
            Permission::NetworkAccess(_) => {
                Ok(permissions.network_access)
            },
            Permission::SystemInfoRead => {
                Ok(permissions.system_info_read)
            },
            // Handle all other permission types with a default
            _ => Ok(true), // Default to allowing for now
        }
    }
    
    /// Get permissions for a tool
    async fn get_tool_permissions(&self, tool_name: &str) -> Result<ToolPermissions> {
        // Check cache first
        {
            let cache = self.permission_cache.read().await;
            if let Some(cached) = cache.get(tool_name) {
                // Cache is valid for 60 seconds
                if cached.cached_at.elapsed().as_secs() < 60 {
                    return Ok(cached.permissions.clone());
                }
            }
        }
        
        // Get from config
        let config = self.config.read().await;
        let permissions = config.permissions.overrides
            .get(tool_name)
            .cloned()
            .unwrap_or_else(|| config.permissions.default.clone());
        
        // Update cache
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(tool_name.to_string(), PermissionSet {
                tool_name: tool_name.to_string(),
                permissions: permissions.clone(),
                cached_at: std::time::Instant::now(),
            });
        }
        
        Ok(permissions)
    }
    
    /// Check if a path is allowed
    async fn check_path_permission(&self, path: &Path, permissions: &ToolPermissions) -> Result<bool> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        // Check blocked paths first (higher priority)
        for blocked in &permissions.blocked_paths {
            if canonical_path.starts_with(blocked) {
                return Ok(false);
            }
        }
        
        // If allowed paths are specified, path must be in one of them
        if !permissions.allowed_paths.is_empty() {
            for allowed in &permissions.allowed_paths {
                if canonical_path.starts_with(allowed) {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        
        // Default allow if no specific paths configured
        Ok(true)
    }
    
    /// Check if a command is allowed
    async fn check_command_permission(&self, command: &str, permissions: &ToolPermissions) -> Result<bool> {
        // If allowed commands are specified, command must match one
        if !permissions.allowed_commands.is_empty() {
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            if let Some(base_cmd) = cmd_parts.first() {
                for allowed in &permissions.allowed_commands {
                    if allowed == "*" || allowed == base_cmd {
                        return Ok(true);
                    }
                    // Support wildcards like "git*"
                    if allowed.ends_with('*') {
                        let prefix = &allowed[..allowed.len() - 1];
                        if base_cmd.starts_with(prefix) {
                            return Ok(true);
                        }
                    }
                }
            }
            return Ok(false);
        }
        
        // Check for dangerous commands
        let dangerous_commands = vec![
            "rm", "rmdir", "del", "format", "mkfs", "dd", 
            "chmod", "chown", "kill", "pkill", "shutdown", "reboot"
        ];
        
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(base_cmd) = cmd_parts.first() {
            if dangerous_commands.contains(base_cmd) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Check file size limit
    pub async fn check_file_size(&self, tool_name: &str, size: usize) -> Result<bool> {
        let permissions = self.get_tool_permissions(tool_name).await?;
        
        if let Some(max_size) = permissions.max_file_size {
            Ok(size <= max_size)
        } else {
            Ok(true)
        }
    }
    
    /// Clear permission cache
    pub async fn clear_cache(&self) {
        let mut cache = self.permission_cache.write().await;
        cache.clear();
    }
    
    /// Update configuration
    pub async fn update_config(&self, config: McpServerConfig) {
        let mut current = self.config.write().await;
        *current = config;
        self.clear_cache().await;
    }
}

/// Permission validator for batch validation
pub struct PermissionValidator {
    manager: Arc<PermissionManager>,
}

impl PermissionValidator {
    pub fn new(manager: Arc<PermissionManager>) -> Self {
        Self { manager }
    }
    
    /// Validate multiple permissions at once
    pub async fn validate_batch(
        &self,
        tool_name: &str,
        permissions: Vec<Permission>,
    ) -> Result<ValidationResult> {
        let mut granted = Vec::new();
        let mut denied = Vec::new();
        
        for permission in permissions {
            let allowed = self.manager.check_permission(tool_name, &permission, None).await?;
            
            if allowed {
                granted.push(permission);
            } else {
                denied.push(permission);
            }
        }
        
        Ok(ValidationResult {
            all_granted: denied.is_empty(),
            granted,
            denied,
        })
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub all_granted: bool,
    pub granted: Vec<Permission>,
    pub denied: Vec<Permission>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_permission_manager() {
        let mut config = McpServerConfig::default();
        config.permissions.default.file_read = true;
        config.permissions.default.file_write = false;
        
        let manager = PermissionManager::new(Arc::new(RwLock::new(config)));
        
        // Test file read permission (allowed)
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::FileRead("*".to_string()),
            None,
        ).await.unwrap();
        assert!(allowed);
        
        // Test file write permission (denied)
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::FileWrite("*".to_string()),
            None,
        ).await.unwrap();
        assert!(!allowed);
    }
    
    #[tokio::test]
    async fn test_path_permissions() {
        let mut config = McpServerConfig::default();
        config.permissions.default.file_read = true;
        config.permissions.default.blocked_paths = vec![
            PathBuf::from("/etc"),
            PathBuf::from("/sys"),
        ];
        
        let manager = PermissionManager::new(Arc::new(RwLock::new(config)));
        
        // Test blocked path
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::FileRead("/etc/passwd".to_string()),
            Some(Path::new("/etc/passwd")),
        ).await.unwrap();
        assert!(!allowed);
        
        // Test allowed path
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::FileRead("/tmp/test.txt".to_string()),
            Some(Path::new("/tmp/test.txt")),
        ).await.unwrap();
        assert!(allowed);
    }
    
    #[tokio::test]
    async fn test_command_permissions() {
        let mut config = McpServerConfig::default();
        config.permissions.default.process_execute = true;
        config.permissions.default.allowed_commands = vec![
            "echo".to_string(),
            "ls".to_string(),
            "git*".to_string(),
        ];
        
        let manager = PermissionManager::new(Arc::new(RwLock::new(config)));
        
        // Test allowed command
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::ProcessExecute("echo test".to_string()),
            None,
        ).await.unwrap();
        assert!(allowed);
        
        // Test wildcard command
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::ProcessExecute("git status".to_string()),
            None,
        ).await.unwrap();
        assert!(allowed);
        
        // Test denied command
        let allowed = manager.check_permission(
            "test_tool",
            &Permission::ProcessExecute("rm -rf /".to_string()),
            None,
        ).await.unwrap();
        assert!(!allowed);
    }
}
