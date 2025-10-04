// Metrics module - clean implementation
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub total_calls: u64,
    pub total_time_ms: u64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
    #[serde(skip)]
    pub last_execution: Option<Instant>,
}

impl Default for ToolMetrics {
    fn default() -> Self {
        Self {
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            total_execution_time_ms: 0,
            average_execution_time_ms: 0.0,
            total_calls: 0,
            total_time_ms: 0,
            min_time_ms: u64::MAX,
            max_time_ms: 0,
            avg_time_ms: 0.0,
            last_execution: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    #[serde(skip)]
    pub uptime: Duration,
    pub total_executions: u64,
    pub total_calls: u64,
    pub total_errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

impl GlobalMetrics {
    pub fn new() -> Self {
        Self {
            uptime: Duration::from_secs(0),
            total_executions: 0,
            total_calls: 0,
            total_errors: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
        }
    }
}

pub struct MetricsCollector {
    tool_metrics: Arc<DashMap<String, ToolMetrics>>,
    global_metrics: Arc<RwLock<GlobalMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            tool_metrics: Arc::new(DashMap::new()),
            global_metrics: Arc::new(RwLock::new(GlobalMetrics::new())),
        }
    }
    
    pub fn record_tool_execution(
        &self,
        tool_name: &str,
        duration: Duration,
        success: bool,
    ) {
        // Update tool-specific metrics
        let mut entry = self.tool_metrics.entry(tool_name.to_string())
            .or_insert(ToolMetrics::default());
        
        let duration_ms = duration.as_millis() as u64;
        entry.total_calls += 1;
        entry.execution_count += 1;
        entry.total_time_ms += duration_ms;
        entry.total_execution_time_ms += duration_ms;
        entry.avg_time_ms = entry.total_time_ms as f64 / entry.total_calls as f64;
        entry.average_execution_time_ms = entry.avg_time_ms;
        
        if success {
            entry.success_count += 1;
        } else {
            entry.failure_count += 1;
        }
        
        entry.min_time_ms = entry.min_time_ms.min(duration_ms);
        entry.max_time_ms = entry.max_time_ms.max(duration_ms);
        entry.last_execution = Some(Instant::now());
    }
    
    pub async fn record_cache_hit(&self, _tool_name: &str) {
        let mut global = self.global_metrics.write().await;
        global.cache_hits += 1;
        let total = global.cache_hits + global.cache_misses;
        if total > 0 {
            global.cache_hit_rate = (global.cache_hits as f64 / total as f64) * 100.0;
        }
    }
    
    pub async fn record_cache_miss(&self, _tool_name: &str) {
        let mut global = self.global_metrics.write().await;
        global.cache_misses += 1;
        let total = global.cache_hits + global.cache_misses;
        if total > 0 {
            global.cache_hit_rate = (global.cache_hits as f64 / total as f64) * 100.0;
        }
    }
    
    pub fn record_tool_registered(&self, tool_name: &str) {
        // Initialize metrics for new tool
        self.tool_metrics.entry(tool_name.to_string())
            .or_insert(ToolMetrics::default());
    }
    
    pub fn get_all_metrics(&self) -> HashMap<String, ToolMetrics> {
        let mut result = HashMap::new();
        for entry in self.tool_metrics.iter() {
            result.insert(entry.key().clone(), entry.value().clone());
        }
        result
    }
    
    pub async fn get_global_stats(&self) -> GlobalMetrics {
        let global = self.global_metrics.read().await;
        global.clone()
    }
    
    pub fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetrics> {
        self.tool_metrics.get(tool_name).map(|m| m.clone())
    }
}
