// Permission Management
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::path::Path;
use parking_lot::RwLock;
use dashmap::DashMap;
use anyhow::{Result, bail};
use serde::{Serialize, Deserialize};

use crate::mcp_tools::system::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    FileRead(String),
    FileWrite(String),
    FileDelete(String),
    ProcessExecute(String),
    NetworkAccess(String),
    SystemInfo,
    Execute(String),
    GitAccess(String),
    SystemInfoRead,
    CommandExecute(String),
    WorkspaceWrite,
    WorkspaceRead,
    ToolExecute(String),
    ReadFile,
    WriteFile,
    ExecuteCommand,
    ListFiles,
    SearchFiles,
    EditFile,
    GitOperations,
    All,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Role {
    Admin,
    User,
    ReadOnly,
}

pub struct PermissionManager {
    granted: HashSet<Permission>,
    workspace: Option<String>,
    role: Role,
    denied_operations: HashSet<String>,
    role_permissions: std::collections::HashMap<Role, Vec<Permission>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            granted: HashSet::new(),
            workspace: None,
            role: Role::User,
            denied_operations: HashSet::new(),
            role_permissions: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_workspace(workspace: String) -> Self {
        Self {
            granted: HashSet::new(),
            workspace: Some(workspace),
            role: Role::User,
            denied_operations: HashSet::new(),
            role_permissions: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self.apply_role_permissions();
        self
    }
    
    fn apply_role_permissions(&mut self) {
        // Apply admin permissions
        self.role_permissions.insert(
            Role::Admin,
            vec![
                Permission::FileRead("*".to_string()),
                Permission::FileWrite("*".to_string()),
                Permission::CommandExecute("*".to_string()),
                Permission::NetworkAccess("*".to_string()),
                Permission::SystemInfo,
            ]
        );
        
        // Apply standard permissions
        self.role_permissions.insert(
            Role::User,
            vec![
                Permission::FileRead("*".to_string()),
                Permission::SystemInfo,
            ]
        );
        
        self.role_permissions.insert(
            Role::ReadOnly,
            vec![
                Permission::FileRead("*".to_string()),
            ]
        );
        
        match self.role {
            Role::Admin => {
                // Admin has all permissions
                self.granted.insert(Permission::FileRead("*".to_string()));
                self.granted.insert(Permission::FileWrite("*".to_string()));
                self.granted.insert(Permission::CommandExecute("*".to_string()));
                self.granted.insert(Permission::NetworkAccess("*".to_string()));
                self.granted.insert(Permission::SystemInfo);
            }
            Role::User => {
                // User has limited permissions
                self.granted.insert(Permission::FileRead("*".to_string()));
                self.granted.insert(Permission::SystemInfo);
            }
            Role::ReadOnly => {
                // ReadOnly has limited permissions
                self.granted.insert(Permission::FileRead("*".to_string()));
            }
        }
        
        Ok(())
    }
    
    pub fn check_permission(&self, user: &str, permission: &Permission, operation: &str) -> Result<()> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            bail!("Permission denied for user {}: {:?} (operation: {})", user, permission, operation)
        }
    }
    
    pub fn check(&self, permission: &Permission) -> Result<()> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            bail!("Permission denied: {:?}", permission)
        }
    }
    
    /// Check if a path-based permission is granted (with workspace restrictions)
    pub fn check_path_permission(&self, path: &str, write: bool) -> Result<()> {
        // Ensure path is within workspace if workspace is set
        if let Some(ref workspace) = self.workspace {
            let abs_path = if Path::new(path).is_absolute() {
                path.to_string()
            } else {
                format!("{}/{}", workspace, path)
            };
            
            if !abs_path.starts_with(workspace) {
                bail!("Access denied: path '{}' is outside workspace", path);
            }
        }
        
        let permission = if write {
            Permission::FileWrite(path.to_string())
        } else {
            Permission::FileRead(path.to_string())
        };
        
        self.check_permission("system", &permission, "execute")
    }
    
    pub fn has_permission(&self, permission: &Permission) -> bool {
        // Check exact match
        if self.granted.contains(permission) {
            return true;
        }
        
        // Check wildcard permissions
        match permission {
            Permission::FileRead(_) => {
                self.granted.contains(&Permission::FileRead("*".to_string()))
            }
            Permission::FileWrite(_) => {
                self.granted.contains(&Permission::FileWrite("*".to_string()))
            }
            Permission::CommandExecute(_) => {
                self.granted.contains(&Permission::CommandExecute("*".to_string()))
            }
            Permission::NetworkAccess(_) => {
                self.granted.contains(&Permission::NetworkAccess("*".to_string()))
            }
            Permission::SystemInfo => self.granted.contains(&Permission::SystemInfo),
            Permission::ProcessExecute(_) => {
                self.granted.contains(&Permission::ProcessExecute("*".to_string()))
            }
            Permission::WorkspaceWrite => self.granted.contains(&Permission::WorkspaceWrite),
            Permission::WorkspaceRead => self.granted.contains(&Permission::WorkspaceRead),
            Permission::ToolExecute(_) => {
                self.granted.contains(&Permission::ToolExecute("*".to_string()))
            }
            _ => false  // Handle remaining cases
        }
    }
    
    /// Grant a permission
    pub fn grant(&mut self, permission: Permission) {
        self.granted.insert(permission);
    }
    
    /// Grant default permissions for a workspace
    pub fn grant_workspace_defaults(&mut self) {
        self.grant(Permission::FileRead("*".to_string()));
        self.grant(Permission::FileWrite("*".to_string()));
        self.grant(Permission::CommandExecute("git".to_string()));
        self.grant(Permission::CommandExecute("npm".to_string()));
        self.grant(Permission::CommandExecute("cargo".to_string()));
    }
}
