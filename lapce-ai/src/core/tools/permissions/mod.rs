// Permissions module for core tools

pub mod rooignore;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct PermissionManager {
    allowed_paths: Vec<PathBuf>,
    denied_paths: Vec<PathBuf>,
    permissions: HashMap<String, Vec<crate::core::permissions::Permission>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            allowed_paths: vec![],
            denied_paths: vec![],
            permissions: HashMap::new(),
        }
    }
    
    pub fn check_permission(&self, path: &Path, permission: &crate::core::permissions::Permission) -> bool {
        // Check if path is denied
        if self.denied_paths.iter().any(|p| path.starts_with(p)) {
            return false;
        }
        
        // Check if path is allowed
        if !self.allowed_paths.is_empty() && !self.allowed_paths.iter().any(|p| path.starts_with(p)) {
            return false;
        }
        
        // Check specific permissions
        if let Some(path_str) = path.to_str() {
            if let Some(perms) = self.permissions.get(path_str) {
                return perms.iter().any(|p| p == permission);
            }
        }
        
        true
    }
    
    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.allowed_paths.push(path);
    }
    
    pub fn add_denied_path(&mut self, path: PathBuf) {
        self.denied_paths.push(path);
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
