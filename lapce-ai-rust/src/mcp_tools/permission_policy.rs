use std::collections::{HashSet, HashMap};
use dashmap::DashMap;
use anyhow::Result;

use crate::mcp_tools::system::UserId;

// Permission system from lines 477-511

#[derive(Clone)]
pub struct RateLimit {
    pub max_calls_per_minute: u32,
    pub max_calls_per_hour: u32,
}

#[derive(Clone)]
pub struct PermissionPolicy {
    pub allowed_tools: HashSet<String>,
    pub denied_operations: HashSet<(String, String)>,
    pub rate_limits: HashMap<String, RateLimit>,
}

impl PermissionPolicy {
    pub fn new() -> Self {
        let mut allowed_tools = HashSet::new();
        // Add all default tools
        allowed_tools.insert("readFile".to_string());
        allowed_tools.insert("writeFile".to_string());
        allowed_tools.insert("executeCommand".to_string());
        allowed_tools.insert("listFiles".to_string());
        allowed_tools.insert("searchFiles".to_string());
        
        Self {
            allowed_tools,
            denied_operations: HashSet::new(),
            rate_limits: HashMap::new(),
        }
    }
    
    pub fn is_allowed(&self, tool: &str, operation: &str) -> bool {
        if !self.allowed_tools.contains(tool) {
            return false;
        }
        
        !self.denied_operations.contains(&(tool.to_string(), operation.to_string()))
    }
    
    pub fn allow_tool(&mut self, tool: &str) {
        self.allowed_tools.insert(tool.to_string());
    }
    
    pub fn deny_operation(&mut self, tool: &str, operation: &str) {
        self.denied_operations.insert((tool.to_string(), operation.to_string()));
    }
}

pub struct PermissionManager {
    policies: DashMap<UserId, PermissionPolicy>,
    default_policy: PermissionPolicy,
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut default_policy = PermissionPolicy::new();
        // Default policy: allow basic read operations
        default_policy.allow_tool("readFile");
        default_policy.allow_tool("listFiles");
        default_policy.allow_tool("searchFiles");
        // Deny dangerous operations by default
        default_policy.deny_operation("executeCommand", "sudo");
        default_policy.deny_operation("writeFile", "/etc");
        default_policy.deny_operation("writeFile", "/sys");
        
        Self {
            policies: DashMap::new(),
            default_policy,
        }
    }
    
    pub fn check_permission(
        &self,
        user: &UserId,
        tool: &str,
        operation: &str,
    ) -> Result<()> {
        // REAL IMPLEMENTATION - Actually check permissions
        let policy = self.policies.get(user)
            .map(|p| p.clone())
            .unwrap_or_else(|| self.default_policy.clone());
        
        // Check if tool is allowed
        if !policy.allowed_tools.contains(tool) && !policy.allowed_tools.contains("*") {
            return Err(anyhow::anyhow!("Permission denied: tool '{}' not allowed for user '{}'", tool, user));
        }
        
        // Check if specific operation is denied
        let key = (tool.to_string(), operation.to_string());
        if policy.denied_operations.contains(&key) {
            return Err(anyhow::anyhow!("Permission denied: operation '{}' on tool '{}' not allowed", operation, tool));
        }
        
        Ok(())
    }
    
    pub fn set_user_policy(&self, user: UserId, policy: PermissionPolicy) {
        self.policies.insert(user, policy);
    }
    
    pub fn get_user_policy(&self, user: &UserId) -> Option<PermissionPolicy> {
        self.policies.get(user).map(|p| p.clone())
    }
}
