/// MCP Cache Management Tool
/// Provides cache inspection, management, and statistics

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use async_trait::async_trait;

use crate::cache_integration::{get_cache, init_global_cache, CacheConfig};
use crate::mcp_tools::{Tool, ToolContext, ToolResult};

pub struct CacheTool {
    name: String,
    description: String,
}

impl CacheTool {
    pub fn new() -> Self {
        Self {
            name: "cache_management".to_string(),
            description: "Manage and inspect the cache system".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheArgs {
    operation: String, // "get", "put", "delete", "clear", "stats", "warm"
    key: Option<String>,
    value: Option<String>,
    pattern: Option<String>,
}

#[async_trait]
impl Tool for CacheTool {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["get", "set", "delete", "clear", "list"]
                },
                "key": {"type": "string"},
                "value": {}
            },
            "required": ["operation"]
        })
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let cache_args: CacheArgs = serde_json::from_value(args)?;
        
        // Ensure cache is initialized
        if get_cache().is_none() {
            init_global_cache(CacheConfig::default()).await?;
        }
        
        let cache = get_cache()
            .ok_or_else(|| anyhow::anyhow!("Cache not initialized"))?;
        
        let result = match cache_args.operation.as_str() {
            "get" => {
                let key = cache_args.key.ok_or_else(|| anyhow::anyhow!("Key required for get operation"))?;
                
                if let Some(value) = cache.get(&key).await {
                    serde_json::json!({
                        "found": true,
                        "key": key,
                        "size": value.len(),
                        "value": String::from_utf8_lossy(&value).to_string()
                    })
                } else {
                    serde_json::json!({
                        "found": false,
                        "key": key
                    })
                }
            }
            
            "put" => {
                let key = cache_args.key.ok_or_else(|| anyhow::anyhow!("Key required for put operation"))?;
                let value = cache_args.value.ok_or_else(|| anyhow::anyhow!("Value required for put operation"))?;
                
                cache.put(key.clone(), value.as_bytes().to_vec()).await?;
                
                serde_json::json!({
                    "success": true,
                    "operation": "put",
                    "key": key,
                    "size": value.len()
                })
            }
            
            "delete" => {
                let key = cache_args.key.ok_or_else(|| anyhow::anyhow!("Key required for delete operation"))?;
                
                cache.invalidate(&key).await?;
                
                serde_json::json!({
                    "success": true,
                    "operation": "delete",
                    "key": key
                })
            }
            
            "clear" => {
                let pattern = cache_args.pattern.unwrap_or_else(|| "*".to_string());
                
                let count = cache.invalidate(&pattern).await?;
                
                serde_json::json!({
                    "success": true,
                    "operation": "clear",
                    "pattern": pattern,
                    "cleared": count
                })
            }
            
            "stats" => {
                let stats = cache.get_statistics().await?;
                
                // stats is already a serde_json::Value, just return it
                stats
            }
            
            "warm" => {
                cache.warm_cache().await?;
                
                serde_json::json!({
                    "success": true,
                    "operation": "warm",
                    "message": "Cache warming initiated"
                })
            }
            
            _ => {
                return Err(anyhow::anyhow!("Unknown operation: {}. Valid operations are: get, put, delete, clear, stats, warm", cache_args.operation));
            }
        };
        
        Ok(ToolResult {
            data: Some(serde_json::json!(result)),
            success: true,
            error: None,
            metadata: Some(serde_json::json!({
                "tool": "cache_management",
                "operation": cache_args.operation
            })),
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        let cache_args: CacheArgs = serde_json::from_value(args.clone())?;
        
        // Validate operation
        let valid_ops = ["get", "put", "delete", "clear", "stats", "warm"];
        if !valid_ops.contains(&cache_args.operation.as_str()) {
            return Err(anyhow::anyhow!("Invalid operation: {}", cache_args.operation));
        }
        
        // Validate required fields based on operation
        match cache_args.operation.as_str() {
            "get" | "delete" => {
                if cache_args.key.is_none() {
                    return Err(anyhow::anyhow!("Key required for {} operation", cache_args.operation));
                }
            }
            "put" => {
                if cache_args.key.is_none() || cache_args.value.is_none() {
                    return Err(anyhow::anyhow!("Both key and value required for put operation"));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

/// Cache metrics tool
pub struct CacheMetricsTool {
    name: String,
    description: String,
}

impl CacheMetricsTool {
    pub fn new() -> Self {
        Self {
            name: "cache_metrics".to_string(),
            description: "Get detailed cache performance metrics".to_string(),
        }
    }
}

#[async_trait]
impl Tool for CacheMetricsTool {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    
    async fn execute(&self, _args: Value, _context: ToolContext) -> Result<ToolResult> {
        let cache = get_cache()
            .ok_or_else(|| anyhow::anyhow!("Cache not initialized"))?;
        
        let metrics = cache.get_detailed_metrics().await?;
        
        Ok(ToolResult {
            data: Some(serde_json::to_value(&metrics)?),
            success: true,
            error: None,
            metadata: Some(serde_json::json!({
                "tool": "cache_metrics"
            })),
        })
    }
    
    async fn validate(&self, _args: &Value) -> Result<()> {
        Ok(())
    }
}

/// Register cache tools with MCP
pub fn register_cache_tools() -> Vec<Box<dyn Tool + Send + Sync>> {
    vec![
        Box::new(CacheTool::new()),
        Box::new(CacheMetricsTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_tool_operations() {
        let tool = CacheTool::new();
        let context = ToolContext::default();
        
        // Test put operation
        let put_args = serde_json::json!({
            "operation": "put",
            "key": "test_key",
            "value": "test_value"
        });
        
        let result = tool.execute(put_args, context.clone()).await;
        assert!(result.is_ok());
        
        // Test get operation
        let get_args = serde_json::json!({
            "operation": "get",
            "key": "test_key"
        });
        
        let result = tool.execute(get_args, context.clone()).await;
        assert!(result.is_ok());
        
        // Test stats operation
        let stats_args = serde_json::json!({
            "operation": "stats"
        });
        
        let result = tool.execute(stats_args, context).await;
        assert!(result.is_ok());
    }
}