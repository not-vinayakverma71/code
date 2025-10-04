use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, bail};
use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::RwLock;
use async_trait::async_trait;

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult},
    permissions::Permission,
    rate_limiter::GovernorRateLimiter,
};

// DashMap-based tool registry for concurrent access
pub struct ToolRegistry {
    // Tool name -> Tool implementation
    tools: DashMap<String, Arc<dyn Tool>>,
    
    // Tool metadata for fast lookup
    metadata: DashMap<String, ToolMetadata>,
    
    // Rate limiter
    rate_limiter: Arc<GovernorRateLimiter>,
    
    // Metrics
    metrics: Arc<RwLock<ToolMetrics>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            metadata: DashMap::new(),
            rate_limiter: Arc::new(GovernorRateLimiter::new()),
            metrics: Arc::new(RwLock::new(ToolMetrics::new())),
        }
    }
    
    pub fn register(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        let metadata = ToolMetadata {
            name: name.clone(),
            description: tool.description().to_string(),
            permissions: tool.required_permissions(),
            registered_at: Instant::now(),
        };
        
        self.tools.insert(name.clone(), tool);
        self.metadata.insert(name, metadata);
    }
    
    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.metadata.remove(name);
        self.tools.remove(name).map(|(_, tool)| tool)
    }
    
    pub async fn dispatch(
        &self,
        tool_name: &str,
        args: Value,
        context: ToolContext,
    ) -> Result<ToolResult> {
        let start = Instant::now();
        
        // Check if tool exists
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?
            .clone();
        
        // Check rate limits
        self.rate_limiter.check_rate_limit(&context.user, tool_name).await?;
        
        // Validate arguments
        tool.validate(&args).await?;
        
        // Execute tool
        let result = tool.execute(args, context).await;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.record_execution(tool_name, start.elapsed(), result.is_ok());
        
        result
    }
    
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.metadata
            .iter()
            .map(|entry| {
                let metadata = entry.value();
                ToolInfo {
                    name: metadata.name.clone(),
                    description: metadata.description.clone(),
                    permissions: metadata.permissions.clone(),
                }
            })
            .collect()
    }
    
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|entry| entry.clone())
    }
    
    pub async fn get_metrics(&self) -> ToolMetrics {
        self.metrics.read().await.clone()
    }
    
    pub fn bulk_register(&self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register(tool);
        }
    }
    
    pub fn clear(&self) {
        self.tools.clear();
        self.metadata.clear();
    }
}

#[derive(Debug, Clone)]
struct ToolMetadata {
    name: String,
    description: String,
    permissions: Vec<Permission>,
    registered_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone)]
pub struct ToolMetrics {
    executions: DashMap<String, ExecutionStats>,
    total_executions: u64,
    total_errors: u64,
}

impl ToolMetrics {
    fn new() -> Self {
        Self {
            executions: DashMap::new(),
            total_executions: 0,
            total_errors: 0,
        }
    }
    
    fn record_execution(&mut self, tool_name: &str, duration: Duration, success: bool) {
        self.total_executions += 1;
        if !success {
            self.total_errors += 1;
        }
        
        self.executions
            .entry(tool_name.to_string())
            .and_modify(|stats| {
                stats.count += 1;
                stats.total_duration += duration;
                if !success {
                    stats.errors += 1;
                }
                if duration < stats.min_duration {
                    stats.min_duration = duration;
                }
                if duration > stats.max_duration {
                    stats.max_duration = duration;
                }
            })
            .or_insert(ExecutionStats {
                count: 1,
                errors: if success { 0 } else { 1 },
                total_duration: duration,
                min_duration: duration,
                max_duration: duration,
            });
    }
    
    pub fn get_stats(&self, tool_name: &str) -> Option<ExecutionStats> {
        self.executions.get(tool_name).map(|entry| entry.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub count: u64,
    pub errors: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
}

impl ExecutionStats {
    pub fn average_duration(&self) -> Duration {
        if self.count > 0 {
            self.total_duration / self.count as u32
        } else {
            Duration::ZERO
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.count > 0 {
            ((self.count - self.errors) as f64) / (self.count as f64) * 100.0
        } else {
            100.0
        }
    }
}
