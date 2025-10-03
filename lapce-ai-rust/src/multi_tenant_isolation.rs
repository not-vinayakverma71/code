/// Multi-tenant Isolation - Day 37 AM
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub resource_limits: ResourceLimits,
    pub isolation_level: IsolationLevel,
    pub data_partition: String,
}

#[derive(Clone, Debug)]
pub struct ResourceLimits {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f32,
    pub max_connections: usize,
    pub max_storage_gb: usize,
    pub max_requests_per_second: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IsolationLevel {
    Shared,
    Dedicated,
    Isolated,
}

pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
    resource_usage: Arc<RwLock<HashMap<String, ResourceUsage>>>,
}

#[derive(Default)]
pub struct ResourceUsage {
    pub memory_mb: usize,
    pub cpu_percent: f32,
    pub active_connections: usize,
    pub storage_gb: f32,
    pub requests_count: u32,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            resource_usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register_tenant(&self, tenant: Tenant) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        tenants.insert(tenant.id.clone(), tenant);
        
        let mut usage = self.resource_usage.write().await;
        usage.insert(tenant.id, ResourceUsage::default());
        
        Ok(())
    }
    
    pub async fn check_resource_limit(&self, tenant_id: &str, resource_type: ResourceType, amount: usize) -> Result<bool> {
        let tenants = self.tenants.read().await;
        let tenant = tenants.get(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Tenant not found"))?;
        
        let usage = self.resource_usage.read().await;
        let current = usage.get(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Usage not found"))?;
        
        match resource_type {
            ResourceType::Memory => Ok(current.memory_mb + amount <= tenant.resource_limits.max_memory_mb),
            ResourceType::Connections => Ok(current.active_connections + amount <= tenant.resource_limits.max_connections),
            ResourceType::Storage => Ok((current.storage_gb + amount as f32 / 1024.0) <= tenant.resource_limits.max_storage_gb as f32),
        }
    }
    
    pub async fn allocate_resource(&self, tenant_id: &str, resource_type: ResourceType, amount: usize) -> Result<()> {
        if !self.check_resource_limit(tenant_id, resource_type, amount).await? {
            return Err(anyhow::anyhow!("Resource limit exceeded"));
        }
        
        let mut usage = self.resource_usage.write().await;
        let current = usage.get_mut(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Usage not found"))?;
        
        match resource_type {
            ResourceType::Memory => current.memory_mb += amount,
            ResourceType::Connections => current.active_connections += amount,
            ResourceType::Storage => current.storage_gb += amount as f32 / 1024.0,
        }
        
        Ok(())
    }
    
    pub async fn release_resource(&self, tenant_id: &str, resource_type: ResourceType, amount: usize) -> Result<()> {
        let mut usage = self.resource_usage.write().await;
        let current = usage.get_mut(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Usage not found"))?;
        
        match resource_type {
            ResourceType::Memory => current.memory_mb = current.memory_mb.saturating_sub(amount),
            ResourceType::Connections => current.active_connections = current.active_connections.saturating_sub(amount),
            ResourceType::Storage => current.storage_gb = (current.storage_gb - amount as f32 / 1024.0).max(0.0),
        }
        
        Ok(())
    }
    
    pub fn get_data_path(&self, tenant_id: &str) -> String {
        format!("/data/tenants/{}", tenant_id)
    }
}

pub enum ResourceType {
    Memory,
    Connections,
    Storage,
}

// Tenant context for request processing
pub struct TenantContext {
    pub tenant_id: String,
    pub isolation_level: IsolationLevel,
    pub data_path: String,
}

impl TenantContext {
    pub fn new(tenant_id: String, manager: &TenantManager) -> Self {
        Self {
            data_path: manager.get_data_path(&tenant_id),
            tenant_id,
            isolation_level: IsolationLevel::Shared,
        }
    }
}
