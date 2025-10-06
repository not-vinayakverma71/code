use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::path::PathBuf;
use dashmap::DashMap;
use serde_json::Value;
use anyhow::Result;
use parking_lot::Mutex;

use crate::mcp_tools::core::{Tool, ToolResult};
use crate::mcp_tools::permissions::PermissionManager;
use crate::mcp_tools::rate_limiter::RateLimiter;

// Core MCP Architecture from lines 61-96

pub type UserId = String;
pub type SessionId = String;

#[derive(Clone)]
pub struct ToolContext {
    pub workspace: PathBuf,
    pub user: UserId,
    pub session: SessionId,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

#[derive(Default)]
pub struct ToolMetrics {
    pub total_calls: std::sync::atomic::AtomicU64,
    pub total_errors: std::sync::atomic::AtomicU64,
    pub total_time_ms: std::sync::atomic::AtomicU64,
    pub per_tool_metrics: DashMap<String, ToolStats>,
}

#[derive(Clone, Default)]
pub struct ToolStats {
    pub calls: u64,
    pub errors: u64,
    pub avg_time_ms: f64,
    pub last_called: Option<SystemTime>,
}

pub struct ResourceLimits {
    pub memory_limit: usize,
    pub cpu_limit: Duration,
    pub timeout: Duration,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024, // 100MB
            cpu_limit: Duration::from_secs(10),
            timeout: Duration::from_secs(30),
        }
    }
}

pub struct ProcessSandbox {
    pub cgroup_manager: Arc<CGroupManager>,
    pub namespace_manager: Arc<NamespaceManager>,
    pub temp_dirs: Arc<Mutex<Vec<PathBuf>>>,
}

pub struct CGroupManager {
    cgroup_path: PathBuf,
}

impl CGroupManager {
    pub fn new() -> Self {
        Self {
            cgroup_path: PathBuf::from("/sys/fs/cgroup"),
        }
    }
}

pub struct NamespaceManager {
    namespaces: HashMap<String, u32>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::new(),
        }
    }
}

pub struct McpToolSystem {
    // Tool registry
    pub tools: DashMap<String, Arc<Box<dyn Tool>>>,
    
    // Execution sandbox
    pub sandbox: Arc<ProcessSandbox>,
    
    // Rate limiting
    pub rate_limiter: Arc<RateLimiter>,
    
    // Permission system
    pub permissions: Arc<PermissionManager>,
    
    // Metrics
    pub metrics: Arc<ToolMetrics>,
}

impl McpToolSystem {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            sandbox: Arc::new(ProcessSandbox {
                cgroup_manager: Arc::new(CGroupManager::new()),
                namespace_manager: Arc::new(NamespaceManager::new()),
                temp_dirs: Arc::new(Mutex::new(Vec::new())),
            }),
            rate_limiter: Arc::new(RateLimiter::new()),
            permissions: Arc::new(PermissionManager::new()),
            metrics: Arc::new(ToolMetrics::default()),
        }
    }

    pub fn register_tool(&self, name: String, tool: Box<dyn Tool>) {
        self.tools.insert(name, Arc::new(tool));
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: Value,
        context: ToolContext,
    ) -> Result<ToolResult> {
        // Check if tool exists
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?
            .clone();

        // Check permissions
        let permission = crate::mcp_tools::permissions::Permission::ToolExecute(tool_name.to_string());
        self.permissions.check_permission("user", &permission, tool_name)?;

        // Check rate limit
        self.rate_limiter.check_rate_limit(&context.user, tool_name).await?;

        // Record metrics
        let start = std::time::Instant::now();
        self.metrics.total_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Validate args
        tool.validate(&args).await?;

        // Execute tool - convert context to core::ToolContext
        let core_context = crate::mcp_tools::core::ToolContext {
            workspace: context.workspace.clone(),
            user: context.user.clone(),
            user_id: context.user.clone(),
            session_id: context.session.clone(),
            request_id: uuid::Uuid::new_v4().to_string(),
            cancellation_token: context.cancellation_token.clone(),
            metadata: None,
        };
        let result = match tool.execute(args.clone(), core_context).await {
            Ok(r) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                self.metrics.total_time_ms.fetch_add(elapsed_ms, std::sync::atomic::Ordering::Relaxed);
                
                // Update per-tool metrics
                let mut entry = self.metrics.per_tool_metrics.entry(tool_name.to_string())
                    .or_default();
                entry.calls += 1;
                entry.last_called = Some(SystemTime::now());
                entry.avg_time_ms = (entry.avg_time_ms * (entry.calls - 1) as f64 + elapsed_ms as f64) / entry.calls as f64;
                
                r
            }
            Err(e) => {
                self.metrics.total_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                let mut entry = self.metrics.per_tool_metrics.entry(tool_name.to_string())
                    .or_default();
                entry.errors += 1;
                
                return Err(e);
            }
        };

        Ok(result)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let mut metrics = HashMap::new();
        
        metrics.insert("total_calls".to_string(), 
            serde_json::json!(self.metrics.total_calls.load(std::sync::atomic::Ordering::Relaxed)));
        metrics.insert("total_errors".to_string(),
            serde_json::json!(self.metrics.total_errors.load(std::sync::atomic::Ordering::Relaxed)));
        metrics.insert("total_time_ms".to_string(),
            serde_json::json!(self.metrics.total_time_ms.load(std::sync::atomic::Ordering::Relaxed)));
        
        let per_tool: HashMap<String, Value> = self.metrics.per_tool_metrics.iter()
            .map(|entry| {
                let stats = entry.value();
                (entry.key().clone(), serde_json::json!({
                    "calls": stats.calls,
                    "errors": stats.errors,
                    "avg_time_ms": stats.avg_time_ms,
                    "last_called": stats.last_called.map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })
                }))
            })
            .collect();
        
        metrics.insert("per_tool".to_string(), serde_json::json!(per_tool));
        
        metrics
    }
}
