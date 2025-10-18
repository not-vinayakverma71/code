// Observability debug tool - exposes metrics and logs
use async_trait::async_trait;
use serde_json::{json, Value};
use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput};
use crate::core::tools::observability::OBSERVABILITY;

pub struct ObservabilityTool;

impl ObservabilityTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ObservabilityTool {
    fn name(&self) -> &'static str {
        "observability"
    }
    
    fn description(&self) -> &'static str {
        "Get observability metrics and recent logs for debugging"
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> ToolResult {
        let command = args["command"].as_str().unwrap_or("metrics");
        
        let result = match command {
            "metrics" => {
                // Export metrics
                let metrics = OBSERVABILITY.get_metrics();
                json!({
                    "command": "metrics",
                    "metrics": serde_json::to_value(metrics).unwrap_or(json!({}))
                })
            }
            "logs" => {
                // Get recent logs
                let limit = args["limit"].as_u64().unwrap_or(100) as usize;
                let logs = OBSERVABILITY.get_logs(Some(limit));
                json!({
                    "command": "logs",
                    "count": logs.len(),
                    "logs": serde_json::to_value(logs).unwrap_or(json!([]))
                })
            }
            "clear" => {
                // Clear all data
                OBSERVABILITY.clear();
                json!({
                    "command": "clear",
                    "status": "cleared"
                })
            }
            "summary" => {
                // Get summary statistics
                let metrics = OBSERVABILITY.get_metrics();
                json!({
                    "command": "summary",
                    "total_calls": metrics.total_calls,
                    "total_errors": metrics.total_errors,
                    "error_rate": if metrics.total_calls > 0 {
                        (metrics.total_errors as f64 / metrics.total_calls as f64) * 100.0
                    } else {
                        0.0
                    },
                    "avg_duration_ms": if metrics.total_calls > 0 {
                        metrics.total_duration_ms / metrics.total_calls
                    } else {
                        0
                    },
                    "tool_count": metrics.tool_calls.len(),
                    "uptime_seconds": metrics.start_time.map(|start| {
                        (chrono::Utc::now() - start).num_seconds()
                    })
                })
            }
            _ => {
                json!({
                    "error": format!("Unknown command: {}", command),
                    "available_commands": ["metrics", "logs", "clear", "summary"]
                })
            }
        };
        
        Ok(ToolOutput::success(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_observability_summary() {
        let tool = ObservabilityTool::new();
        let context = ToolContext::default();
        
        let args = json!({
            "command": "summary"
        });
        
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["total_calls"].is_number());
    }
    
    #[tokio::test]
    async fn test_observability_metrics() {
        let tool = ObservabilityTool::new();
        let context = ToolContext::default();
        
        let args = json!({
            "command": "metrics"
        });
        
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
    }
}
