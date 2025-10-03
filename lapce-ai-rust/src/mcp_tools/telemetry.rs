/// Telemetry and Logging System for MCP Tools
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

pub struct TelemetrySystem {
    metrics: Arc<RwLock<MetricsStore>>,
    config: TelemetryConfig,
    writer: Arc<RwLock<Box<dyn MetricsWriter + Send + Sync>>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub log_level: String,
    pub flush_interval: Duration,
    pub include_tool_details: bool,
    pub export_format: ExportFormat,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Prometheus,
    OpenTelemetry,
}

struct MetricsStore {
    tool_metrics: HashMap<String, ToolMetrics>,
    system_metrics: SystemMetrics,
    events: Vec<TelemetryEvent>,
}

#[derive(Clone, Default, Serialize)]
pub struct ToolMetrics {
    pub invocations: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub avg_duration_ms: u64,
    pub last_invoked: Option<SystemTime>,
}

#[derive(Clone, Default, Serialize)]
pub struct SystemMetrics {
    pub total_requests: u64,
    pub active_requests: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f32,
    pub uptime_seconds: u64,
    pub error_rate: f32,
}

#[derive(Clone, Serialize)]
pub struct TelemetryEvent {
    pub timestamp: SystemTime,
    pub event_type: EventType,
    pub tool_name: Option<String>,
    pub session_id: Option<String>,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum EventType {
    ToolInvocation,
    ToolSuccess,
    ToolFailure,
    PermissionDenied,
    RateLimitExceeded,
    SecurityViolation,
    SystemError,
}

#[async_trait::async_trait]
pub trait MetricsWriter {
    async fn write(&mut self, metrics: &MetricsSnapshot) -> Result<(), Box<dyn std::error::Error>>;
    async fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub tool_metrics: HashMap<String, ToolMetrics>,
    pub system_metrics: SystemMetrics,
    pub recent_events: Vec<TelemetryEvent>,
}

impl TelemetrySystem {
    pub fn new(config: TelemetryConfig) -> Self {
        let writer: Box<dyn MetricsWriter + Send + Sync> = match config.export_format {
            ExportFormat::Json => Box::new(JsonMetricsWriter::new()),
            ExportFormat::Prometheus => Box::new(PrometheusMetricsWriter::new()),
            ExportFormat::OpenTelemetry => Box::new(OpenTelemetryMetricsWriter::new()),
        };
        
        Self {
            metrics: Arc::new(RwLock::new(MetricsStore {
                tool_metrics: HashMap::new(),
                system_metrics: SystemMetrics::default(),
                events: Vec::new(),
            })),
            config,
            writer: Arc::new(RwLock::new(writer)),
        }
    }
    
    pub async fn record_tool_invocation(
        &self,
        tool_name: &str,
        duration: Duration,
        success: bool,
        session_id: Option<String>,
    ) {
        if !self.config.enabled {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        let tool_metrics = metrics.tool_metrics.entry(tool_name.to_string())
            .or_insert_with(ToolMetrics::default);
        
        tool_metrics.invocations += 1;
        if success {
            tool_metrics.successes += 1;
        } else {
            tool_metrics.failures += 1;
        }
        
        let duration_ms = duration.as_millis() as u64;
        tool_metrics.total_duration_ms += duration_ms;
        tool_metrics.min_duration_ms = tool_metrics.min_duration_ms.min(duration_ms);
        tool_metrics.max_duration_ms = tool_metrics.max_duration_ms.max(duration_ms);
        tool_metrics.avg_duration_ms = tool_metrics.total_duration_ms / tool_metrics.invocations;
        tool_metrics.last_invoked = Some(SystemTime::now());
        
        // Record event
        let event = TelemetryEvent {
            timestamp: SystemTime::now(),
            event_type: if success { EventType::ToolSuccess } else { EventType::ToolFailure },
            tool_name: Some(tool_name.to_string()),
            session_id,
            details: HashMap::new(),
        };
        metrics.events.push(event);
        
        // Log
        if success {
            info!("Tool {} executed successfully in {}ms", tool_name, duration_ms);
        } else {
            warn!("Tool {} failed after {}ms", tool_name, duration_ms);
        }
    }
    
    pub async fn record_security_event(&self, event_type: EventType, details: HashMap<String, String>) {
        if !self.config.enabled {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        let event = TelemetryEvent {
            timestamp: SystemTime::now(),
            event_type: event_type.clone(),
            tool_name: None,
            session_id: None,
            details,
        };
        metrics.events.push(event);
        
        error!("Security event: {:?}", event_type);
    }
    
    pub async fn update_system_metrics(&self, memory_bytes: u64, cpu_percent: f32) {
        if !self.config.enabled {
            return;
        }
        
        let mut metrics = self.metrics.write().await;
        metrics.system_metrics.memory_usage_bytes = memory_bytes;
        metrics.system_metrics.cpu_usage_percent = cpu_percent;
    }
    
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let metrics = self.metrics.read().await;
        MetricsSnapshot {
            timestamp: SystemTime::now(),
            tool_metrics: metrics.tool_metrics.clone(),
            system_metrics: metrics.system_metrics.clone(),
            recent_events: metrics.events.clone(),
        }
    }
    
    pub async fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = self.get_snapshot().await;
        let mut writer = self.writer.write().await;
        writer.write(&snapshot).await?;
        writer.flush().await
    }
}

// JSON metrics writer
struct JsonMetricsWriter {
    buffer: Vec<MetricsSnapshot>,
}

impl JsonMetricsWriter {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

#[async_trait::async_trait]
impl MetricsWriter for JsonMetricsWriter {
    async fn write(&mut self, metrics: &MetricsSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.push(metrics.clone());
        Ok(())
    }
    
    async fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.buffer)?;
        tokio::fs::write("metrics.json", json).await?;
        self.buffer.clear();
        Ok(())
    }
}

// Prometheus metrics writer
struct PrometheusMetricsWriter;

impl PrometheusMetricsWriter {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MetricsWriter for PrometheusMetricsWriter {
    async fn write(&mut self, metrics: &MetricsSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        // In production, export to Prometheus
        Ok(())
    }
    
    async fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// OpenTelemetry metrics writer  
struct OpenTelemetryMetricsWriter;

impl OpenTelemetryMetricsWriter {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MetricsWriter for OpenTelemetryMetricsWriter {
    async fn write(&mut self, metrics: &MetricsSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        // In production, export to OpenTelemetry
        Ok(())
    }
    
    async fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

struct ConsoleMetricsWriter {
    buffer: Vec<MetricsSnapshot>,
}

impl ConsoleMetricsWriter {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

#[async_trait::async_trait]
impl MetricsWriter for ConsoleMetricsWriter {
    async fn write(&mut self, metrics: &MetricsSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.push(metrics.clone());
        Ok(())
    }
    
    async fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.buffer.is_empty() {
            let json = serde_json::to_string_pretty(&self.buffer)?;
            debug!("Metrics: {}", json);
            self.buffer.clear();
        }
        Ok(())
    }
}
