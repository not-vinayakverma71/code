// Observability System - Structured JSON logging with metrics
// Part of Observability TODO #13

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::json;
use tracing::{info, error, warn, debug, span, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use chrono::{DateTime, Utc};
use anyhow::Result;

// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub correlation_id: String,
    pub tool_name: Option<String>,
    pub user_id: Option<String>,
    pub workspace: Option<String>,
    pub fields: HashMap<String, serde_json::Value>,
    pub duration_ms: Option<u64>,
    pub error_code: Option<u32>,
}

// In-memory metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub tool_calls: HashMap<String, ToolMetrics>,
    pub total_calls: u64,
    pub total_errors: u64,
    pub total_duration_ms: u64,
    pub start_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub call_count: u64,
    pub error_count: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub p50_duration_ms: u64,
    pub p95_duration_ms: u64,
    pub p99_duration_ms: u64,
    pub latencies: Vec<u64>,
}

impl ToolMetrics {
    fn record_call(&mut self, duration_ms: u64, success: bool) {
        self.call_count += 1;
        self.total_duration_ms += duration_ms;
        self.latencies.push(duration_ms);
        
        if !success {
            self.error_count += 1;
        }
        
        // Update min/max
        if self.min_duration_ms == 0 || duration_ms < self.min_duration_ms {
            self.min_duration_ms = duration_ms;
        }
        if duration_ms > self.max_duration_ms {
            self.max_duration_ms = duration_ms;
        }
        
        // Calculate percentiles
        self.calculate_percentiles();
    }
    
    fn calculate_percentiles(&mut self) {
        if self.latencies.is_empty() {
            return;
        }
        
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();
        
        let len = sorted.len();
        self.avg_duration_ms = self.total_duration_ms / self.call_count;
        self.p50_duration_ms = sorted[len / 2];
        self.p95_duration_ms = sorted[len * 95 / 100.min(len - 1)];
        self.p99_duration_ms = sorted[len * 99 / 100.min(len - 1)];
        
        // Keep only last 1000 samples to prevent memory growth
        if self.latencies.len() > 1000 {
            self.latencies = self.latencies.split_off(self.latencies.len() - 1000);
        }
    }
}

// Observability manager
pub struct ObservabilityManager {
    metrics: Arc<RwLock<Metrics>>,
    logs: Arc<RwLock<Vec<LogEntry>>>,
    max_logs: usize,
}

impl ObservabilityManager {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Metrics {
                start_time: Some(Utc::now()),
                ..Default::default()
            })),
            logs: Arc::new(RwLock::new(Vec::new())),
            max_logs: 10000,
        }
    }
    
    /// Initialize tracing subscriber
    pub fn init_tracing() {
        let json_layer = fmt::layer()
            // .json() // Requires json feature flag
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);
        
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_layer)
            .init();
    }
    
    /// Log tool call
    pub fn log_tool_call(
        &self,
        tool_name: &str,
        correlation_id: &str,
        args: &serde_json::Value,
        user_id: Option<String>,
        workspace: Option<String>,
    ) -> ToolCallSpan {
        let span = span!(
            Level::INFO,
            "tool_call",
            tool_name = tool_name,
            correlation_id = correlation_id,
            user_id = user_id.as_deref(),
            workspace = workspace.as_deref(),
        );
        
        let _enter = span.enter();
        
        info!(
            tool_name = tool_name,
            correlation_id = correlation_id,
            args = ?args,
            "Tool call started"
        );
        
        // Store log entry
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!("Tool call: {}", tool_name),
            correlation_id: correlation_id.to_string(),
            tool_name: Some(tool_name.to_string()),
            user_id,
            workspace,
            fields: HashMap::from([
                ("event".to_string(), json!("tool_call_start")),
                ("args".to_string(), args.clone()),
            ]),
            duration_ms: None,
            error_code: None,
        };
        
        self.store_log(entry);
        
        ToolCallSpan {
            tool_name: tool_name.to_string(),
            correlation_id: correlation_id.to_string(),
            start_time: Instant::now(),
            manager: self,
        }
    }
    
    /// Record tool result
    pub fn record_tool_result(
        &self,
        tool_name: &str,
        correlation_id: &str,
        duration_ms: u64,
        success: bool,
        error: Option<&str>,
    ) {
        // Update metrics
        let mut metrics = self.metrics.write();
        metrics.total_calls += 1;
        metrics.total_duration_ms += duration_ms;
        
        if !success {
            metrics.total_errors += 1;
        }
        
        let tool_metrics = metrics.tool_calls
            .entry(tool_name.to_string())
            .or_default();
        tool_metrics.record_call(duration_ms, success);
        
        // Log result
        let level = if success { "INFO" } else { "ERROR" };
        let message = if success {
            format!("Tool call completed: {}", tool_name)
        } else {
            format!("Tool call failed: {}", tool_name)
        };
        
        if success {
            info!(
                tool_name = tool_name,
                correlation_id = correlation_id,
                duration_ms = duration_ms,
                "Tool call completed successfully"
            );
        } else {
            error!(
                tool_name = tool_name,
                correlation_id = correlation_id,
                duration_ms = duration_ms,
                error = error,
                "Tool call failed"
            );
        }
        
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message,
            correlation_id: correlation_id.to_string(),
            tool_name: Some(tool_name.to_string()),
            user_id: None,
            workspace: None,
            fields: HashMap::from([
                ("event".to_string(), json!("tool_call_end")),
                ("success".to_string(), json!(success)),
                ("duration_ms".to_string(), json!(duration_ms)),
                ("error".to_string(), json!(error)),
            ]),
            duration_ms: Some(duration_ms),
            error_code: if success { None } else { Some(1) },
        };
        
        self.store_log(entry);
    }
    
    /// Store log entry
    fn store_log(&self, entry: LogEntry) {
        let mut logs = self.logs.write();
        
        // Maintain max size
        if logs.len() >= self.max_logs {
            let drain_count = logs.len() / 4;
            logs.drain(0..drain_count);
        }
        
        logs.push(entry);
    }
    
    /// Get metrics snapshot
    pub fn get_metrics(&self) -> Metrics {
        self.metrics.read().clone()
    }
    
    /// Get logs
    pub fn get_logs(&self, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.read();
        let limit = limit.unwrap_or(100).min(logs.len());
        
        logs.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Export metrics as JSON
    pub fn export_metrics_json(&self) -> serde_json::Value {
        let metrics = self.get_metrics();
        serde_json::to_value(metrics).unwrap_or(json!({}))
    }
    
    /// Clear all data
    pub fn clear(&self) {
        self.metrics.write().tool_calls.clear();
        self.logs.write().clear();
    }
}

// Tool call span for tracking
pub struct ToolCallSpan<'a> {
    tool_name: String,
    correlation_id: String,
    start_time: Instant,
    manager: &'a ObservabilityManager,
}

impl<'a> Drop for ToolCallSpan<'a> {
    fn drop(&mut self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.manager.record_tool_result(
            &self.tool_name,
            &self.correlation_id,
            duration_ms,
            true,
            None,
        );
    }
}

// Global observability instance
lazy_static::lazy_static! {
    pub static ref OBSERVABILITY: Arc<ObservabilityManager> = 
        Arc::new(ObservabilityManager::new());
}

// Helper macros for structured logging
#[macro_export]
macro_rules! log_tool_start {
    ($tool:expr, $corr_id:expr) => {
        $crate::core::tools::observability::OBSERVABILITY
            .log_tool_call($tool, $corr_id, &json!({}), None, None)
    };
    ($tool:expr, $corr_id:expr, $args:expr) => {
        $crate::core::tools::observability::OBSERVABILITY
            .log_tool_call($tool, $corr_id, $args, None, None)
    };
}

#[macro_export]
macro_rules! log_tool_end {
    ($tool:expr, $corr_id:expr, $duration:expr, $success:expr) => {
        $crate::core::tools::observability::OBSERVABILITY
            .record_tool_result($tool, $corr_id, $duration, $success, None)
    };
    ($tool:expr, $corr_id:expr, $duration:expr, $success:expr, $error:expr) => {
        $crate::core::tools::observability::OBSERVABILITY
            .record_tool_result($tool, $corr_id, $duration, $success, Some($error))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_recording() {
        let manager = ObservabilityManager::new();
        
        // Record some tool calls
        manager.record_tool_result("test_tool", "corr-1", 100, true, None);
        manager.record_tool_result("test_tool", "corr-2", 200, true, None);
        manager.record_tool_result("test_tool", "corr-3", 150, false, Some("error"));
        
        let metrics = manager.get_metrics();
        assert_eq!(metrics.total_calls, 3);
        assert_eq!(metrics.total_errors, 1);
        
        let tool_metrics = &metrics.tool_calls["test_tool"];
        assert_eq!(tool_metrics.call_count, 3);
        assert_eq!(tool_metrics.error_count, 1);
        assert_eq!(tool_metrics.avg_duration_ms, 150);
        assert_eq!(tool_metrics.min_duration_ms, 100);
        assert_eq!(tool_metrics.max_duration_ms, 200);
    }
    
    #[test]
    fn test_log_storage() {
        let manager = ObservabilityManager::new();
        
        let _span = manager.log_tool_call(
            "test_tool",
            "corr-123",
            &json!({"arg": "value"}),
            Some("user-1".to_string()),
            Some("/workspace".to_string()),
        );
        
        let logs = manager.get_logs(None);
        assert!(!logs.is_empty());
        
        let log = &logs[0];
        assert_eq!(log.tool_name.as_ref().unwrap(), "test_tool");
        assert_eq!(log.correlation_id, "corr-123");
        assert!(log.fields.contains_key("event"));
    }
    
    #[test]
    fn test_percentile_calculation() {
        let mut metrics = ToolMetrics::default();
        
        // Add latencies
        for i in 1..=100 {
            metrics.record_call(i as u64, true);
        }
        
        assert_eq!(metrics.p50_duration_ms, 50);
        assert_eq!(metrics.p95_duration_ms, 95);
        assert_eq!(metrics.p99_duration_ms, 99);
    }
}
