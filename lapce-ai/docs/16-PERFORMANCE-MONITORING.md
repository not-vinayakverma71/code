# Step 11: Performance Monitoring - Zero-Overhead Metrics System
## Real-time Performance Tracking with 1MB Memory Footprint

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : 1:1 TRANSLATION - PRESERVE YEARS OF PERFORMANCE TUNING
**DO NOT CHANGE PERFORMANCE CHARACTERISTICS - JUST TRANSLATE** **THIS IS NOT A REWRITE - IT'S A TYPESCRIPT → RUST PORT**


**MUST TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex/`
**IMPORTANT**: The AI timing is CALIBRATED over years:
- Token generation speed - keep same 
- Tool execution timing - copy exactly
- Latencies are INTENTIONAL - preserve them
- Just translate TypeScript → Rust, don't "optimize"

## ✅ Success Criteria
- [ ] **Memory Usage**: < 1MB monitoring overhead
- [ ] **Metrics Collection**: < 100ns per metric
- [ ] **Latency Tracking**: HDR histogram with μs precision
- [ ] **Throughput**: Track 1M+ events/second
- [ ] **CPU Profiling**: < 1% overhead when enabled
- [ ] **Export Format**: Prometheus-compatible metrics
- [ ] **Real-time Dashboard**: WebSocket updates at 60fps
- [ ] **Test Coverage**: Monitor 24h without memory growth

## Overview
Our performance monitoring system provides comprehensive metrics collection with minimal overhead, using lock-free data structures and efficient aggregation techniques.

## Core Monitoring Architecture

### Performance Monitor System
```rust
use prometheus::{Encoder, TextEncoder, Counter, Histogram, Gauge};
use hdrhistogram::Histogram as HdrHistogram;
use crossbeam::channel::{bounded, Sender};

pub struct PerformanceMonitor {
    // Metrics collectors
    metrics: Arc<MetricsCollector>,
    
    // Tracing system
    tracer: Arc<Tracer>,
    
    // Profiler
    profiler: Arc<Profiler>,
    
    // Export system
    exporter: Arc<MetricsExporter>,
    
    // Alerting
    alerter: Arc<AlertManager>,
}

pub struct MetricsCollector {
    // Request metrics
    request_counter: Counter,
    request_duration: Histogram,
    request_size: Histogram,
    
    // System metrics
    memory_usage: Gauge,
    cpu_usage: Gauge,
    
    // Custom histograms
    latency_histogram: Arc<RwLock<HdrHistogram>>,
    
    // Thread-local collectors
    thread_collectors: ThreadLocal<RefCell<LocalMetrics>>,
}

pub struct LocalMetrics {
    request_count: u64,
    total_latency: Duration,
    error_count: u64,
}
```

## Lock-Free Metrics Collection

### 1. Atomic Metrics
```rust
use std::sync::atomic::{AtomicU64, AtomicF64, Ordering};

pub struct AtomicMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_ns: AtomicU64,
    
    // Moving averages
    avg_latency: AtomicF64,
    avg_throughput: AtomicF64,
}

impl AtomicMetrics {
    pub fn record_request(&self, latency: Duration, success: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        
        // Update latency
        let nanos = latency.as_nanos() as u64;
        self.total_latency_ns.fetch_add(nanos, Ordering::Relaxed);
        
        // Update moving average (EWMA)
        self.update_moving_average(nanos);
    }
    
    fn update_moving_average(&self, new_latency_ns: u64) {
        let alpha = 0.1; // Smoothing factor
        let new_value = new_latency_ns as f64;
        
        loop {
            let current = self.avg_latency.load(Ordering::Relaxed);
            let updated = alpha * new_value + (1.0 - alpha) * current;
            
            match self.avg_latency.compare_exchange_weak(
                current.to_bits(),
                updated.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }
}
```

### 2. HdrHistogram for Latency
```rust
pub struct LatencyTracker {
    histogram: Arc<RwLock<HdrHistogram>>,
    rotation_interval: Duration,
    last_rotation: Arc<RwLock<Instant>>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            histogram: Arc::new(RwLock::new(
                HdrHistogram::new_with_bounds(1, 60_000_000, 3).unwrap()
            )),
            rotation_interval: Duration::from_secs(60),
            last_rotation: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    pub fn record(&self, latency_us: u64) {
        // Check if we need to rotate
        if self.should_rotate() {
            self.rotate();
        }
        
        // Record value
        self.histogram.write().unwrap()
            .record(latency_us)
            .unwrap_or_else(|_| {
                tracing::warn!("Latency value {} out of range", latency_us);
            });
    }
    
    pub fn get_percentiles(&self) -> LatencyPercentiles {
        let histogram = self.histogram.read().unwrap();
        
        LatencyPercentiles {
            p50: histogram.value_at_percentile(50.0),
            p90: histogram.value_at_percentile(90.0),
            p95: histogram.value_at_percentile(95.0),
            p99: histogram.value_at_percentile(99.0),
            p999: histogram.value_at_percentile(99.9),
            max: histogram.max(),
            mean: histogram.mean(),
        }
    }
    
    fn rotate(&self) {
        let mut histogram = self.histogram.write().unwrap();
        histogram.reset();
        *self.last_rotation.write().unwrap() = Instant::now();
    }
}
```

## Distributed Tracing

### 1. Trace Context
```rust
use opentelemetry::{trace::{Tracer, Span, SpanBuilder}, Context};

pub struct DistributedTracer {
    tracer: Box<dyn Tracer>,
    spans: DashMap<SpanId, ActiveSpan>,
}

pub struct ActiveSpan {
    span: Box<dyn Span>,
    start_time: Instant,
    attributes: HashMap<String, String>,
}

impl DistributedTracer {
    pub fn start_span(&self, name: &str, parent: Option<SpanId>) -> SpanId {
        let mut builder = self.tracer.span_builder(name);
        
        if let Some(parent_id) = parent {
            if let Some(parent_span) = self.spans.get(&parent_id) {
                builder = builder.with_parent_context(parent_span.context());
            }
        }
        
        let span = builder.start(&self.tracer);
        let span_id = span.span_context().span_id();
        
        self.spans.insert(span_id, ActiveSpan {
            span: Box::new(span),
            start_time: Instant::now(),
            attributes: HashMap::new(),
        });
        
        span_id
    }
    
    pub fn end_span(&self, span_id: SpanId) {
        if let Some((_, mut span)) = self.spans.remove(&span_id) {
            span.span.end();
        }
    }
    
    pub fn add_event(&self, span_id: SpanId, name: &str, attributes: Vec<(&str, &str)>) {
        if let Some(mut span) = self.spans.get_mut(&span_id) {
            span.span.add_event(name, attributes);
        }
    }
}
```

## CPU Profiling

### 1. Sampling Profiler
```rust
use pprof::{ProfilerGuard, Report};

pub struct CpuProfiler {
    guard: Option<ProfilerGuard<'static>>,
    samples: Arc<RwLock<Vec<Sample>>>,
    sampling_rate: u32,
}

impl CpuProfiler {
    pub fn start(&mut self, frequency: i32) -> Result<()> {
        self.guard = Some(pprof::ProfilerGuard::new(frequency)?);
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<Report> {
        if let Some(guard) = self.guard.take() {
            Ok(guard.report().build()?)
        } else {
            Err(Error::ProfilerNotStarted)
        }
    }
    
    pub fn generate_flamegraph(&self, report: &Report) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        report.flamegraph(&mut buffer)?;
        Ok(buffer)
    }
}
```

### 2. Memory Profiling
```rust
use jemalloc_ctl::{stats, epoch};

pub struct MemoryProfiler {
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    interval: Duration,
}

impl MemoryProfiler {
    pub async fn collect_snapshot(&self) -> Result<MemorySnapshot> {
        // Update statistics
        epoch::advance()?;
        
        let allocated = stats::allocated::read()?;
        let active = stats::active::read()?;
        let mapped = stats::mapped::read()?;
        let retained = stats::retained::read()?;
        let resident = stats::resident::read()?;
        
        let snapshot = MemorySnapshot {
            timestamp: Instant::now(),
            allocated,
            active,
            mapped,
            retained,
            resident,
            heap_profile: self.collect_heap_profile()?,
        };
        
        self.snapshots.write().unwrap().push(snapshot.clone());
        
        Ok(snapshot)
    }
    
    fn collect_heap_profile(&self) -> Result<HeapProfile> {
        // Collect allocation sites
        let mut profile = HeapProfile::default();
        
        // Use jemalloc's prof API if available
        #[cfg(feature = "profiling")]
        {
            let dump = jemalloc_ctl::prof::dump()?;
            profile.parse_dump(&dump)?;
        }
        
        Ok(profile)
    }
}
```

## Metrics Export

### 1. Prometheus Exporter
```rust
pub struct PrometheusExporter {
    registry: Registry,
    encoder: TextEncoder,
    server: Option<Server>,
}

impl PrometheusExporter {
    pub async fn start(&mut self, addr: SocketAddr) -> Result<()> {
        let registry = self.registry.clone();
        let encoder = self.encoder.clone();
        
        let make_svc = make_service_fn(move |_| {
            let registry = registry.clone();
            let encoder = encoder.clone();
            
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    Self::handle_metrics(req, registry.clone(), encoder.clone())
                }))
            }
        });
        
        let server = Server::bind(&addr).serve(make_svc);
        self.server = Some(server);
        
        Ok(())
    }
    
    async fn handle_metrics(
        req: Request<Body>,
        registry: Registry,
        encoder: TextEncoder,
    ) -> Result<Response<Body>> {
        if req.uri().path() != "/metrics" {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }
        
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))?)
    }
}
```

## Real-time Dashboard

### 1. Metrics Dashboard
```rust
pub struct MetricsDashboard {
    metrics: Arc<MetricsCollector>,
    ws_connections: Arc<RwLock<HashMap<Uuid, Sender<DashboardUpdate>>>>,
}

impl MetricsDashboard {
    pub async fn stream_updates(&self, ws: WebSocket) -> Result<()> {
        let id = Uuid::new_v4();
        let (tx, rx) = mpsc::channel(100);
        
        self.ws_connections.write().unwrap().insert(id, tx);
        
        // Send updates every second
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                let update = DashboardUpdate {
                    timestamp: Instant::now(),
                    request_rate: metrics.get_request_rate(),
                    error_rate: metrics.get_error_rate(),
                    latency_p99: metrics.get_latency_p99(),
                    memory_usage: metrics.get_memory_usage(),
                    cpu_usage: metrics.get_cpu_usage(),
                };
                
                if tx.send(update).await.is_err() {
                    break;
                }
            }
        });
        
        // Forward updates to WebSocket
        while let Some(update) = rx.recv().await {
            let json = serde_json::to_string(&update)?;
            ws.send(Message::Text(json)).await?;
        }
        
        self.ws_connections.write().unwrap().remove(&id);
        Ok(())
    }
}
```

## Alerting System

### 1. Alert Manager
```rust
pub struct AlertManager {
    rules: Vec<AlertRule>,
    notifiers: Vec<Box<dyn Notifier>>,
    active_alerts: DashMap<String, Alert>,
}

pub struct AlertRule {
    name: String,
    condition: Box<dyn Fn(&Metrics) -> bool>,
    severity: AlertSeverity,
    cooldown: Duration,
}

impl AlertManager {
    pub async fn check_alerts(&self, metrics: &Metrics) {
        for rule in &self.rules {
            let should_alert = (rule.condition)(metrics);
            
            if should_alert {
                self.trigger_alert(rule, metrics).await;
            } else {
                self.resolve_alert(&rule.name).await;
            }
        }
    }
    
    async fn trigger_alert(&self, rule: &AlertRule, metrics: &Metrics) {
        let alert = Alert {
            name: rule.name.clone(),
            severity: rule.severity,
            triggered_at: Instant::now(),
            message: format!("Alert: {} triggered", rule.name),
            metrics: metrics.clone(),
        };
        
        // Check if already active
        if self.active_alerts.contains_key(&rule.name) {
            return;
        }
        
        self.active_alerts.insert(rule.name.clone(), alert.clone());
        
        // Notify all channels
        for notifier in &self.notifiers {
            notifier.notify(&alert).await;
        }
    }
}
```

## Memory Profile
- **Metrics collectors**: 200KB
- **HdrHistogram**: 300KB
- **Trace storage**: 200KB
- **Export buffers**: 100KB
- **Dashboard state**: 100KB
- **Alert state**: 100KB
- **Total**: ~1MB
