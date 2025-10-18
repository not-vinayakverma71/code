// Task Orchestrator Metrics - CHUNK-03: T17
// Metrics and structured logging for task orchestration

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

/// Metrics for task orchestration
#[derive(Debug)]
pub struct TaskOrchestratorMetrics {
    // Task lifecycle metrics
    tasks_created: AtomicU64,
    tasks_started: AtomicU64,
    tasks_completed: AtomicU64,
    tasks_aborted: AtomicU64,
    tasks_paused: AtomicU64,
    
    // Active state
    active_tasks: AtomicUsize,
    
    // Message metrics
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    
    // Token usage
    total_tokens_in: AtomicU64,
    total_tokens_out: AtomicU64,
    
    // Event bus metrics
    events_published: AtomicU64,
    event_publish_errors: AtomicU64,
    
    // Persistence metrics
    tasks_persisted: AtomicU64,
    tasks_restored: AtomicU64,
    persistence_errors: AtomicU64,
    
    // Mistake tracking
    total_mistakes: AtomicU64,
    mistake_threshold_exceeded: AtomicU64,
    
    // Performance metrics
    avg_task_duration_ms: Arc<RwLock<MovingAverage>>,
    avg_message_latency_ms: Arc<RwLock<MovingAverage>>,
    
    // Per-tool metrics
    tool_invocations: Arc<RwLock<HashMap<String, u64>>>,
    tool_failures: Arc<RwLock<HashMap<String, u64>>>,
    
    // Queue sizes (for backpressure monitoring)
    event_queue_size: AtomicUsize,
    task_queue_size: AtomicUsize,
}

impl TaskOrchestratorMetrics {
    pub fn new() -> Self {
        Self {
            tasks_created: AtomicU64::new(0),
            tasks_started: AtomicU64::new(0),
            tasks_completed: AtomicU64::new(0),
            tasks_aborted: AtomicU64::new(0),
            tasks_paused: AtomicU64::new(0),
            active_tasks: AtomicUsize::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            total_tokens_in: AtomicU64::new(0),
            total_tokens_out: AtomicU64::new(0),
            events_published: AtomicU64::new(0),
            event_publish_errors: AtomicU64::new(0),
            tasks_persisted: AtomicU64::new(0),
            tasks_restored: AtomicU64::new(0),
            persistence_errors: AtomicU64::new(0),
            total_mistakes: AtomicU64::new(0),
            mistake_threshold_exceeded: AtomicU64::new(0),
            avg_task_duration_ms: Arc::new(RwLock::new(MovingAverage::new(100))),
            avg_message_latency_ms: Arc::new(RwLock::new(MovingAverage::new(100))),
            tool_invocations: Arc::new(RwLock::new(HashMap::new())),
            tool_failures: Arc::new(RwLock::new(HashMap::new())),
            event_queue_size: AtomicUsize::new(0),
            task_queue_size: AtomicUsize::new(0),
        }
    }
    
    // Task lifecycle
    pub fn record_task_created(&self) {
        self.tasks_created.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_task_started(&self) {
        self.tasks_started.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_task_completed(&self, duration: Duration) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.avg_task_duration_ms.write().add(duration.as_millis() as f64);
    }
    
    pub fn record_task_aborted(&self) {
        self.tasks_aborted.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_task_paused(&self) {
        self.tasks_paused.fetch_add(1, Ordering::Relaxed);
    }
    
    // Messages
    pub fn record_message_sent(&self, latency: Duration) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.avg_message_latency_ms.write().add(latency.as_millis() as f64);
    }
    
    pub fn record_message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }
    
    // Tokens
    pub fn record_tokens(&self, tokens_in: u64, tokens_out: u64) {
        self.total_tokens_in.fetch_add(tokens_in, Ordering::Relaxed);
        self.total_tokens_out.fetch_add(tokens_out, Ordering::Relaxed);
    }
    
    // Events
    pub fn record_event_published(&self) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_event_publish_error(&self) {
        self.event_publish_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    // Persistence
    pub fn record_task_persisted(&self) {
        self.tasks_persisted.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_task_restored(&self) {
        self.tasks_restored.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_persistence_error(&self) {
        self.persistence_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    // Mistakes
    pub fn record_mistake(&self) {
        self.total_mistakes.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_mistake_threshold_exceeded(&self) {
        self.mistake_threshold_exceeded.fetch_add(1, Ordering::Relaxed);
    }
    
    // Tools
    pub fn record_tool_success(&self, tool_name: &str) {
        let mut invocations = self.tool_invocations.write();
        *invocations.entry(tool_name.to_string()).or_insert(0) += 1;
    }
    
    pub fn record_tool_invocation(&self, tool_name: &str) {
        let mut invocations = self.tool_invocations.write();
        *invocations.entry(tool_name.to_string()).or_insert(0) += 1;
    }
    
    pub fn record_tool_failure(&self, tool_name: &str) {
        let mut failures = self.tool_failures.write();
        *failures.entry(tool_name.to_string()).or_insert(0) += 1;
    }
    
    // Queue sizes
    pub fn set_event_queue_size(&self, size: usize) {
        self.event_queue_size.store(size, Ordering::Relaxed);
    }
    
    pub fn set_task_queue_size(&self, size: usize) {
        self.task_queue_size.store(size, Ordering::Relaxed);
    }
    
    // Getters
    pub fn get_tasks_created(&self) -> u64 {
        self.tasks_created.load(Ordering::Relaxed)
    }
    
    pub fn get_active_tasks(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }
    
    pub fn get_tasks_completed(&self) -> u64 {
        self.tasks_completed.load(Ordering::Relaxed)
    }
    
    pub fn get_total_tokens(&self) -> (u64, u64) {
        (
            self.total_tokens_in.load(Ordering::Relaxed),
            self.total_tokens_out.load(Ordering::Relaxed),
        )
    }
    
    pub fn get_avg_task_duration_ms(&self) -> f64 {
        self.avg_task_duration_ms.read().get()
    }
    
    pub fn get_tool_stats(&self) -> HashMap<String, (u64, u64)> {
        let invocations = self.tool_invocations.read();
        let failures = self.tool_failures.read();
        
        let mut stats = HashMap::new();
        for (tool, count) in invocations.iter() {
            let failures = *failures.get(tool).unwrap_or(&0);
            stats.insert(tool.clone(), (*count, failures));
        }
        stats
    }
    
    /// Get snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            tasks_created: self.tasks_created.load(Ordering::Relaxed),
            tasks_started: self.tasks_started.load(Ordering::Relaxed),
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            tasks_aborted: self.tasks_aborted.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            total_tokens_in: self.total_tokens_in.load(Ordering::Relaxed),
            total_tokens_out: self.total_tokens_out.load(Ordering::Relaxed),
            events_published: self.events_published.load(Ordering::Relaxed),
            avg_task_duration_ms: self.avg_task_duration_ms.read().get(),
            tool_stats: self.get_tool_stats(),
            timestamp: SystemTime::now(),
        }
    }
}

impl Default for TaskOrchestratorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub tasks_created: u64,
    pub tasks_started: u64,
    pub tasks_completed: u64,
    pub tasks_aborted: u64,
    pub active_tasks: usize,
    pub messages_sent: u64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub events_published: u64,
    pub avg_task_duration_ms: f64,
    pub tool_stats: HashMap<String, (u64, u64)>,
    pub timestamp: SystemTime,
}

/// Moving average calculator
#[derive(Debug)]
struct MovingAverage {
    values: Vec<f64>,
    capacity: usize,
    sum: f64,
}

impl MovingAverage {
    fn new(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            capacity,
            sum: 0.0,
        }
    }
    
    fn add(&mut self, value: f64) {
        if self.values.len() >= self.capacity {
            let old = self.values.remove(0);
            self.sum -= old;
        }
        self.values.push(value);
        self.sum += value;
    }
    
    fn get(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum / self.values.len() as f64
        }
    }
}

/// Global metrics instance
static METRICS: once_cell::sync::Lazy<TaskOrchestratorMetrics> = 
    once_cell::sync::Lazy::new(|| TaskOrchestratorMetrics::new());

/// Get global metrics instance
pub fn global_metrics() -> &'static TaskOrchestratorMetrics {
    &METRICS
}

/// Structured logging helpers
pub mod logging {
    use tracing::{info, warn, error, debug};
    
    pub fn log_task_created(task_id: &str) {
        info!(
            task_id = task_id,
            event = "task_created",
            "Task created"
        );
    }
    
    pub fn log_task_started(task_id: &str) {
        info!(
            task_id = task_id,
            event = "task_started",
            "Task started"
        );
    }
    
    pub fn log_task_completed(task_id: &str, duration_ms: u64) {
        info!(
            task_id = task_id,
            event = "task_completed",
            duration_ms = duration_ms,
            "Task completed"
        );
    }
    
    pub fn log_task_aborted(task_id: &str, reason: Option<&str>) {
        warn!(
            task_id = task_id,
            event = "task_aborted",
            reason = reason.unwrap_or("unknown"),
            "Task aborted"
        );
    }
    
    pub fn log_state_transition(task_id: &str, from: &str, to: &str) {
        debug!(
            task_id = task_id,
            event = "state_transition",
            from_state = from,
            to_state = to,
            "Task state transition"
        );
    }
    
    pub fn log_mistake_detected(task_id: &str, count: u32) {
        warn!(
            task_id = task_id,
            event = "mistake_detected",
            mistake_count = count,
            "Consecutive mistake detected"
        );
    }
    
    pub fn log_persistence_error(task_id: &str, error: &str) {
        error!(
            task_id = task_id,
            event = "persistence_error",
            error = error,
            "Failed to persist task"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = TaskOrchestratorMetrics::new();
        assert_eq!(metrics.get_tasks_created(), 0);
        assert_eq!(metrics.get_active_tasks(), 0);
    }
    
    #[test]
    fn test_task_lifecycle_metrics() {
        let metrics = TaskOrchestratorMetrics::new();
        
        metrics.record_task_created();
        assert_eq!(metrics.get_tasks_created(), 1);
        assert_eq!(metrics.get_active_tasks(), 1);
        
        metrics.record_task_started();
        assert_eq!(metrics.get_tasks_created(), 1);
        
        metrics.record_task_completed(Duration::from_secs(5));
        assert_eq!(metrics.get_tasks_completed(), 1);
        assert_eq!(metrics.get_active_tasks(), 0);
    }
    
    #[test]
    fn test_token_tracking() {
        let metrics = TaskOrchestratorMetrics::new();
        
        metrics.record_tokens(100, 200);
        metrics.record_tokens(50, 75);
        
        let (tokens_in, tokens_out) = metrics.get_total_tokens();
        assert_eq!(tokens_in, 150);
        assert_eq!(tokens_out, 275);
    }
    
    #[test]
    fn test_tool_metrics() {
        let metrics = TaskOrchestratorMetrics::new();
        
        metrics.record_tool_invocation("read_file");
        metrics.record_tool_invocation("read_file");
        metrics.record_tool_invocation("write_file");
        metrics.record_tool_failure("read_file");
        
        let stats = metrics.get_tool_stats();
        assert_eq!(stats.get("read_file"), Some(&(2, 1)));
        assert_eq!(stats.get("write_file"), Some(&(1, 0)));
    }
    
    #[test]
    fn test_moving_average() {
        let mut avg = MovingAverage::new(3);
        
        avg.add(10.0);
        assert_eq!(avg.get(), 10.0);
        
        avg.add(20.0);
        assert_eq!(avg.get(), 15.0);
        
        avg.add(30.0);
        assert_eq!(avg.get(), 20.0);
        
        // Should evict 10.0
        avg.add(40.0);
        assert_eq!(avg.get(), 30.0);
    }
    
    #[test]
    fn test_snapshot() {
        let metrics = TaskOrchestratorMetrics::new();
        
        metrics.record_task_created();
        metrics.record_task_started();
        metrics.record_tokens(100, 200);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.tasks_created, 1);
        assert_eq!(snapshot.tasks_started, 1);
        assert_eq!(snapshot.total_tokens_in, 100);
        assert_eq!(snapshot.total_tokens_out, 200);
    }
    
    #[test]
    fn test_global_metrics() {
        let m1 = global_metrics();
        let m2 = global_metrics();
        
        // Should be same instance
        assert_eq!(m1 as *const _, m2 as *const _);
    }
}
