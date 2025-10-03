# CHUNK-29 Step 6: Error Recovery & Benchmark Specifications

**Generated:** 2025-10-02  
**Status:** Complete

## Executive Summary

Comprehensive error recovery strategies and performance benchmark specifications for the VS Code to Lapce API bridge, targeting 99.9% reliability and <10μs latency per operation.

---

## 1. ERROR RECOVERY ARCHITECTURE

### 1.1 Error Classification

```rust
// lapce-ai-rust/src/error/classification.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,    // System failure, must abort
    Recoverable, // Can retry or fallback
    Warning,     // Degraded but functional
    Info,        // Informational only
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Terminal,       // PTY/shell integration failures
    FileSystem,     // I/O operations
    Network,        // IPC/RPC communication
    Parser,         // Escape sequence/protocol parsing
    Resource,       // Memory/CPU limits
    Timeout,        // Operation timeouts
    UserCancelled,  // User-initiated cancellation
}

pub struct ClassifiedError {
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
    pub retryable: bool,
    pub fallback_available: bool,
    pub error: Box<dyn Error>,
}
```

### 1.2 Recovery Strategy Matrix

| Error Category | Severity | Primary Strategy | Fallback Strategy | Max Retries |
|---------------|----------|------------------|-------------------|-------------|
| Terminal | Critical | Recreate terminal | Alert user | 1 |
| Terminal | Recoverable | Retry command | Raw output mode | 3 |
| FileSystem | Critical | Alert user | None | 0 |
| FileSystem | Recoverable | Retry with backoff | Cache previous | 3 |
| Network | Critical | Reconnect IPC | Local mode | 5 |
| Network | Recoverable | Retry request | Queue for later | 3 |
| Parser | Critical | Reset parser state | Skip markers | 0 |
| Parser | Recoverable | Buffer & retry | Raw passthrough | 1 |
| Resource | Critical | Free resources | Graceful shutdown | 0 |
| Resource | Recoverable | Throttle operations | Reduce quality | N/A |
| Timeout | Recoverable | Increase timeout | Cancel operation | 2 |

---

## 2. TERMINAL ERROR RECOVERY

### 2.1 Shell Integration Failures

```rust
// lapce-ai-rust/src/recovery/terminal.rs

pub struct TerminalRecovery {
    max_retries: u32,
    fallback_mode: FallbackMode,
    health_monitor: HealthMonitor,
}

#[derive(Debug, Clone)]
pub enum FallbackMode {
    NoShellIntegration,  // Send commands without tracking
    RawOutput,           // No escape sequence parsing
    AlternativeShell,    // Try different shell
    MinimalPty,          // Basic PTY without features
}

impl TerminalRecovery {
    pub async fn execute_with_recovery(
        &mut self,
        terminal: &mut dyn Terminal,
        command: &str,
    ) -> Result<CommandOutput> {
        // Try with shell integration first
        match self.try_shell_integration(terminal, command).await {
            Ok(output) => {
                self.health_monitor.record_success();
                Ok(output)
            }
            Err(e) if e.is_recoverable() => {
                self.health_monitor.record_failure();
                
                // Progressive fallback
                match self.fallback_mode {
                    FallbackMode::NoShellIntegration => {
                        self.execute_without_integration(terminal, command).await
                    }
                    FallbackMode::RawOutput => {
                        self.execute_raw_mode(terminal, command).await
                    }
                    FallbackMode::AlternativeShell => {
                        self.try_alternative_shell(terminal, command).await
                    }
                    FallbackMode::MinimalPty => {
                        self.create_minimal_pty(command).await
                    }
                }
            }
            Err(e) => Err(e),
        }
    }
    
    async fn try_shell_integration(
        &self,
        terminal: &mut dyn Terminal,
        command: &str,
    ) -> Result<CommandOutput> {
        let timeout = Duration::from_secs(5);
        
        tokio::time::timeout(timeout, async {
            terminal.execute_command(command).await
        })
        .await
        .map_err(|_| ErrorKind::Timeout)?
    }
    
    async fn execute_without_integration(
        &self,
        terminal: &mut dyn Terminal,
        command: &str,
    ) -> Result<CommandOutput> {
        // Send command without expecting markers
        terminal.send_text(&format!("{}\n", command)).await?;
        
        // Collect output for fixed duration
        let output = self.collect_raw_output(terminal, Duration::from_secs(2)).await?;
        
        Ok(CommandOutput {
            command: command.to_string(),
            output,
            exit_code: None, // Can't determine without integration
        })
    }
}
```

### 2.2 PTY Creation Failures

```rust
pub struct PtyRecovery {
    shells: Vec<ShellConfig>,
    current_shell: usize,
}

impl PtyRecovery {
    pub async fn create_pty_with_fallback(
        &mut self,
        profile: TerminalProfile,
    ) -> Result<Pty> {
        let mut last_error = None;
        
        // Try each configured shell
        for shell_config in &self.shells[self.current_shell..] {
            match self.try_create_pty(shell_config, &profile).await {
                Ok(pty) => return Ok(pty),
                Err(e) => {
                    log::warn!("PTY creation failed for {:?}: {}", shell_config, e);
                    last_error = Some(e);
                    self.current_shell += 1;
                }
            }
        }
        
        // All shells failed, try system default
        self.try_system_default_shell()
            .await
            .or_else(|_| {
                Err(last_error.unwrap_or_else(|| {
                    Error::new("No available shells")
                }))
            })
    }
}
```

---

## 3. FILE SYSTEM ERROR RECOVERY

### 3.1 I/O Operation Recovery

```rust
// lapce-ai-rust/src/recovery/filesystem.rs

pub struct FileSystemRecovery {
    retry_policy: RetryPolicy,
    cache: FileCache,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

impl FileSystemRecovery {
    pub async fn read_with_recovery(
        &self,
        path: &Path,
    ) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.cache.get(path) {
            if !cached.is_stale() {
                return Ok(cached.data.clone());
            }
        }
        
        // Retry with exponential backoff
        let mut delay = self.retry_policy.initial_delay;
        let mut last_error = None;
        
        for attempt in 0..self.retry_policy.max_attempts {
            match tokio::fs::read(path).await {
                Ok(data) => {
                    self.cache.put(path, data.clone());
                    return Ok(data);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    // Immediate retry for transient errors
                    continue;
                }
                Err(e) if self.is_retryable(&e) && attempt < self.retry_policy.max_attempts - 1 => {
                    last_error = Some(e);
                    tokio::time::sleep(delay).await;
                    delay = (delay.as_secs_f64() * self.retry_policy.exponential_base)
                        .min(self.retry_policy.max_delay.as_secs_f64())
                        .into();
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        Err(last_error.unwrap().into())
    }
    
    fn is_retryable(&self, error: &io::Error) -> bool {
        matches!(
            error.kind(),
            io::ErrorKind::TimedOut 
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::UnexpectedEof
        )
    }
}
```

### 3.2 File Watcher Recovery

```rust
pub struct WatcherRecovery {
    watcher: Option<RecommendedWatcher>,
    paths: HashSet<PathBuf>,
    reconnect_attempts: u32,
}

impl WatcherRecovery {
    pub async fn maintain_watches(&mut self) -> Result<()> {
        loop {
            match &mut self.watcher {
                Some(w) => {
                    // Health check
                    if !self.is_healthy(w).await {
                        log::warn!("Watcher unhealthy, recreating...");
                        self.recreate_watcher().await?;
                    }
                }
                None => {
                    // Try to create watcher
                    self.create_watcher().await?;
                    self.rewatch_all_paths().await?;
                }
            }
            
            // Check every 30 seconds
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
    
    async fn recreate_watcher(&mut self) -> Result<()> {
        self.watcher = None;
        self.create_watcher().await?;
        self.rewatch_all_paths().await
    }
}
```

---

## 4. IPC/NETWORK ERROR RECOVERY

### 4.1 Connection Recovery

```rust
// lapce-ai-rust/src/recovery/ipc.rs

pub struct IpcRecovery {
    connection_pool: ConnectionPool,
    circuit_breaker: CircuitBreaker,
    message_queue: MessageQueue,
}

impl IpcRecovery {
    pub async fn send_with_recovery(
        &mut self,
        message: Message,
    ) -> Result<Response> {
        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return Err(Error::ServiceUnavailable);
        }
        
        // Try to send
        match self.try_send(&message).await {
            Ok(response) => {
                self.circuit_breaker.record_success();
                Ok(response)
            }
            Err(e) if e.is_transient() => {
                self.circuit_breaker.record_failure();
                
                // Queue for retry
                self.message_queue.enqueue(message.clone());
                
                // Try to reconnect
                self.reconnect().await?;
                
                // Retry queued messages
                self.flush_queue().await
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                Err(e)
            }
        }
    }
    
    async fn reconnect(&mut self) -> Result<()> {
        let backoff = ExponentialBackoff::default();
        
        retry_with_backoff(backoff, || async {
            self.connection_pool.create_connection().await
        }).await
    }
}

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicInstant,
    state: AtomicState,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn is_open(&self) -> bool {
        match self.state.load(Ordering::Acquire) {
            State::Open => {
                // Check if timeout expired
                let elapsed = self.last_failure.elapsed();
                if elapsed > self.timeout {
                    self.state.store(State::HalfOpen, Ordering::Release);
                    false
                } else {
                    true
                }
            }
            State::Closed | State::HalfOpen => false,
        }
    }
}
```

---

## 5. PARSER ERROR RECOVERY

### 5.1 Escape Sequence Recovery

```rust
// lapce-ai-rust/src/recovery/parser.rs

pub struct ParserRecovery {
    parser: OscParser,
    buffer: RingBuffer,
    recovery_mode: RecoveryMode,
}

#[derive(Debug, Clone)]
enum RecoveryMode {
    SkipInvalid,      // Skip invalid sequences
    BufferPartial,    // Buffer incomplete sequences
    ResetOnError,     // Reset parser state
    RawPassthrough,   // No parsing
}

impl ParserRecovery {
    pub fn parse_with_recovery(&mut self, input: &[u8]) -> Vec<ShellMarker> {
        match self.parser.parse(input) {
            Ok(markers) => {
                self.recovery_mode = RecoveryMode::SkipInvalid;
                markers
            }
            Err(ParseError::Incomplete) => {
                // Buffer for next chunk
                self.buffer.append(input);
                
                if self.buffer.len() > MAX_BUFFER_SIZE {
                    // Buffer overflow, reset
                    log::warn!("Parser buffer overflow, resetting");
                    self.buffer.clear();
                    self.parser.reset();
                }
                
                Vec::new()
            }
            Err(ParseError::Invalid) => {
                match self.recovery_mode {
                    RecoveryMode::SkipInvalid => {
                        // Try to find next valid sequence
                        self.skip_to_next_escape(input)
                    }
                    RecoveryMode::ResetOnError => {
                        self.parser.reset();
                        self.parser.parse(input).unwrap_or_default()
                    }
                    RecoveryMode::RawPassthrough => {
                        // Don't parse, just pass through
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            Err(e) => {
                log::error!("Parser error: {}", e);
                self.parser.reset();
                Vec::new()
            }
        }
    }
}
```

---

## 6. RESOURCE ERROR RECOVERY

### 6.1 Memory Management

```rust
// lapce-ai-rust/src/recovery/resource.rs

pub struct ResourceManager {
    memory_monitor: MemoryMonitor,
    cpu_monitor: CpuMonitor,
    pressure_level: AtomicU8,
}

impl ResourceManager {
    pub async fn manage_resources(&self) {
        loop {
            let memory_usage = self.memory_monitor.current_usage();
            let cpu_usage = self.cpu_monitor.current_usage();
            
            let pressure = self.calculate_pressure(memory_usage, cpu_usage);
            self.pressure_level.store(pressure, Ordering::Release);
            
            match pressure {
                0..=30 => {
                    // Normal operation
                }
                31..=60 => {
                    // Moderate pressure - reduce caches
                    self.reduce_caches().await;
                }
                61..=80 => {
                    // High pressure - aggressive cleanup
                    self.aggressive_cleanup().await;
                }
                81..=100 => {
                    // Critical - emergency measures
                    self.emergency_cleanup().await;
                }
                _ => {}
            }
            
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    
    async fn reduce_caches(&self) {
        // Reduce cache sizes by 50%
        GLOBAL_FILE_CACHE.resize(0.5);
        GLOBAL_BUFFER_POOL.trim(0.5);
    }
    
    async fn aggressive_cleanup(&self) {
        // Clear all caches
        GLOBAL_FILE_CACHE.clear();
        GLOBAL_BUFFER_POOL.clear();
        
        // Close idle connections
        CONNECTION_POOL.close_idle().await;
    }
}
```

---

## 7. PERFORMANCE BENCHMARKS

### 7.1 Latency Targets

| Operation | Target P50 | Target P99 | Target P99.9 | Measurement |
|-----------|------------|------------|--------------|-------------|
| Terminal command | 1ms | 10ms | 100ms | End-to-end |
| File read (cached) | 10μs | 100μs | 1ms | From cache |
| File read (disk) | 100μs | 1ms | 10ms | From disk |
| IPC round-trip | 100μs | 500μs | 2ms | Local IPC |
| Parser (per KB) | 10μs | 50μs | 100μs | Parse time |
| Event dispatch | 1μs | 10μs | 100μs | In-process |

### 7.2 Throughput Targets

| Operation | Target | Units | Notes |
|-----------|--------|-------|-------|
| Terminal output | 10MB/s | Bytes/sec | Streaming |
| File operations | 1000/s | Ops/sec | Mixed R/W |
| IPC messages | 10,000/s | Msg/sec | Based on SharedMemory |
| Event processing | 100,000/s | Events/sec | In-memory |
| Parser throughput | 100MB/s | Bytes/sec | Escape sequences |

### 7.3 Benchmark Implementation

```rust
// lapce-ai-rust/src/bench/latency.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn benchmark_terminal_command(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("terminal_echo", |b| {
        b.to_async(&runtime).iter(|| async {
            let mut terminal = create_test_terminal().await;
            let output = terminal.execute_command("echo test").await.unwrap();
            black_box(output);
        });
    });
}

pub fn benchmark_osc_parser(c: &mut Criterion) {
    let mut parser = OscParser::new();
    let input = b"\x1b]633;C\x07Hello World\x1b]633;D;0\x07";
    
    c.bench_function("parse_osc", |b| {
        b.iter(|| {
            let markers = parser.parse(black_box(input));
            black_box(markers);
        });
    });
}

pub fn benchmark_file_cache(c: &mut Criterion) {
    let cache = FileCache::new(1000);
    let path = PathBuf::from("/test/file.txt");
    let data = vec![0u8; 1024];
    
    c.bench_function("cache_hit", |b| {
        cache.put(&path, data.clone());
        b.iter(|| {
            let cached = cache.get(black_box(&path));
            black_box(cached);
        });
    });
}

criterion_group!(
    benches,
    benchmark_terminal_command,
    benchmark_osc_parser,
    benchmark_file_cache
);
criterion_main!(benches);
```

### 7.4 Memory Benchmarks

```rust
// lapce-ai-rust/src/bench/memory.rs

use dhat::{Dhat, DhatAlloc};

#[global_allocator]
static ALLOC: DhatAlloc = DhatAlloc;

#[test]
fn memory_profile_terminal() {
    let _dhat = Dhat::start_heap_profiling();
    
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    runtime.block_on(async {
        let mut terminal = create_test_terminal().await;
        
        // Run 1000 commands
        for _ in 0..1000 {
            let _ = terminal.execute_command("echo test").await;
        }
    });
    
    let stats = dhat::HeapStats::get();
    assert!(stats.total_bytes < 10_000_000); // <10MB
    assert!(stats.total_blocks < 10_000);     // <10k allocations
}
```

---

## 8. MONITORING & TELEMETRY

### 8.1 Health Monitoring

```rust
// lapce-ai-rust/src/monitor/health.rs

pub struct HealthMonitor {
    metrics: Arc<Metrics>,
    alerts: mpsc::Sender<Alert>,
}

impl HealthMonitor {
    pub fn record_operation(&self, op: &str, duration: Duration, success: bool) {
        self.metrics.record_latency(op, duration);
        
        if success {
            self.metrics.increment_success(op);
        } else {
            self.metrics.increment_failure(op);
            
            // Check failure rate
            let rate = self.metrics.failure_rate(op);
            if rate > 0.1 {  // >10% failure rate
                self.send_alert(Alert::HighFailureRate { 
                    operation: op.to_string(),
                    rate,
                });
            }
        }
    }
    
    pub fn check_latency(&self, op: &str) {
        let p99 = self.metrics.latency_p99(op);
        let target = self.get_latency_target(op);
        
        if p99 > target * 2.0 {
            self.send_alert(Alert::HighLatency {
                operation: op.to_string(),
                latency: p99,
                target,
            });
        }
    }
}
```

### 8.2 Metrics Collection

```rust
pub struct Metrics {
    histograms: DashMap<String, Histogram>,
    counters: DashMap<String, AtomicU64>,
}

impl Metrics {
    pub fn record_latency(&self, op: &str, duration: Duration) {
        self.histograms
            .entry(op.to_string())
            .or_insert_with(|| Histogram::new(3))
            .record(duration.as_micros() as u64);
    }
    
    pub fn latency_percentile(&self, op: &str, percentile: f64) -> Duration {
        self.histograms
            .get(op)
            .map(|h| {
                let micros = h.value_at_percentile(percentile);
                Duration::from_micros(micros)
            })
            .unwrap_or_default()
    }
}
```

---

## 9. RELIABILITY PATTERNS

### 9.1 Timeout Management

```rust
pub struct TimeoutManager {
    defaults: HashMap<String, Duration>,
    overrides: DashMap<String, Duration>,
}

impl TimeoutManager {
    pub fn get_timeout(&self, operation: &str) -> Duration {
        self.overrides
            .get(operation)
            .map(|d| *d)
            .or_else(|| self.defaults.get(operation).copied())
            .unwrap_or(Duration::from_secs(30))
    }
    
    pub fn with_timeout<F, T>(
        &self,
        operation: &str,
        future: F,
    ) -> impl Future<Output = Result<T>>
    where
        F: Future<Output = T>,
    {
        let timeout = self.get_timeout(operation);
        
        async move {
            tokio::time::timeout(timeout, future)
                .await
                .map_err(|_| Error::Timeout(operation.to_string()))
        }
    }
}
```

### 9.2 Graceful Degradation

```rust
pub struct DegradationManager {
    quality_level: AtomicU8,
    features: HashMap<String, FeatureFlag>,
}

impl DegradationManager {
    pub fn degrade(&self) {
        let current = self.quality_level.load(Ordering::Acquire);
        if current > 0 {
            self.quality_level.store(current - 1, Ordering::Release);
            self.apply_quality_level(current - 1);
        }
    }
    
    fn apply_quality_level(&self, level: u8) {
        match level {
            0 => {
                // Minimum functionality
                self.disable_feature("shell_integration");
                self.disable_feature("file_watching");
                self.disable_feature("decorations");
            }
            1 => {
                // Reduced functionality
                self.disable_feature("shell_integration");
                self.disable_feature("decorations");
            }
            2 => {
                // Most features enabled
                self.disable_feature("decorations");
            }
            _ => {
                // Full functionality
            }
        }
    }
}
```

---

## 10. TESTING ERROR SCENARIOS

### 10.1 Chaos Testing

```rust
#[cfg(test)]
mod chaos_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_terminal_crash_recovery() {
        let mut terminal = create_test_terminal().await;
        let recovery = TerminalRecovery::default();
        
        // Simulate crash
        terminal.kill_process();
        
        // Should recover
        let result = recovery.execute_with_recovery(
            &mut terminal,
            "echo recovered"
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_network_partition() {
        let mut ipc = create_test_ipc().await;
        let recovery = IpcRecovery::default();
        
        // Simulate network partition
        ipc.disconnect();
        
        // Should queue and retry
        let future = recovery.send_with_recovery(Message::Test);
        
        // Restore connection after delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        ipc.reconnect();
        
        let result = future.await;
        assert!(result.is_ok());
    }
}
```

### 10.2 Stress Testing

```rust
#[tokio::test]
async fn stress_test_concurrent_operations() {
    let manager = ResourceManager::default();
    let terminals: Vec<_> = (0..100)
        .map(|_| create_test_terminal())
        .collect();
    
    let futures: Vec<_> = terminals
        .iter()
        .map(|t| t.execute_command("stress --cpu 1 --timeout 10s"))
        .collect();
    
    // All should complete despite resource pressure
    let results = futures::future::join_all(futures).await;
    
    let success_rate = results.iter()
        .filter(|r| r.is_ok())
        .count() as f64 / results.len() as f64;
    
    assert!(success_rate > 0.95); // >95% success rate
}
```

---

## 11. KEY RELIABILITY METRICS

### 11.1 Target SLOs

| Metric | Target | Measurement Period |
|--------|--------|-------------------|
| Availability | 99.9% | 30 days |
| Error Rate | <0.1% | 24 hours |
| P99 Latency | <100ms | 1 hour |
| Recovery Time | <5s | Per incident |
| Data Loss | 0% | Always |

### 11.2 Recovery Time Objectives

| Failure Type | RTO | RPO | Notes |
|--------------|-----|-----|-------|
| Terminal crash | 1s | 0 | Auto-restart |
| IPC disconnect | 5s | 0 | Message queuing |
| Parser error | 0s | 0 | Immediate recovery |
| Resource exhaustion | 30s | 0 | Gradual recovery |
| File system error | 10s | 0 | Retry with backoff |

---

## 12. NEXT STEPS (Step 7 Preview)

Based on error recovery and benchmarks, **Step 7: Final Comprehensive Guide** will:

1. Consolidate all findings into implementation guide
2. Provide step-by-step migration instructions
3. Create code templates and examples
4. Document best practices
5. Define maintenance procedures

---

**Step 6 Status:** ✅ **COMPLETE**  
**Recovery Strategies:** 10+ patterns defined  
**Benchmark Targets:** Specified for all operations  
**Reliability Target:** 99.9% availability  
**Next:** Step 7 - Final Comprehensive Guide
