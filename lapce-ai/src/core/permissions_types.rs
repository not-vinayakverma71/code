/// Core permissions module
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Delete,
    Create,
    Modify,
    List,
    Admin,
    FileRead,
    FileWrite,
}

impl Permission {
    pub fn is_allowed(&self, requested: &Permission) -> bool {
        match self {
            Permission::Admin => true,
            Permission::Write => matches!(requested, Permission::Write | Permission::Create | Permission::Modify),
            Permission::Read => matches!(requested, Permission::Read | Permission::List),
            _ => self == requested,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionSet {
    permissions: Vec<Permission>,
}

impl PermissionSet {
    pub fn new(permissions: Vec<Permission>) -> Self {
        Self { permissions }
    }
    
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.iter().any(|p| p.is_allowed(permission))
    }
    
    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }
}
