# Step 12: Error Handling & Recovery - Production-Grade Resilience
## Zero-Panic Architecture with Automatic Recovery

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : 1:1 TYPESCRIPT → RUST TRANSLATION OF ERROR HANDLING
**YEARS OF ERROR HANDLING PERFECTED - TRANSLATE EXACTLY**

**CRITICAL**: Error handling is BATTLE-TESTED:
- Error messages - CHARACTER-FOR-CHARACTER same
- Recovery logic - copy line by line
- Retry patterns - same timeouts, same backoff
- Study ALL error handling in `/home/verma/lapce/Codex`
- Just change syntax, preserve ALL logic

## ✅ Success Criteria
- [ ] **Memory Usage**: < 1MB error handling overhead
- [ ] **Recovery Time**: < 100ms automatic recovery
- [ ] **Circuit Breakers**: Trip in < 50ms on failures
- [ ] **Retry Logic**: Exponential backoff with jitter
- [ ] **Graceful Degradation**: Feature fallback without crash
- [ ] **State Persistence**: Checkpoint every 60s
- [ ] **Panic Handling**: Zero unhandled panics
- [ ] **Test Coverage**: Inject 1000+ error scenarios

## Overview
Our error handling system provides comprehensive error management with automatic recovery, circuit breakers, and graceful degradation, ensuring the system never crashes.

## Core Error Architecture

### Error Type System
```rust
use thiserror::Error;
use std::backtrace::Backtrace;

#[derive(Error, Debug)]
pub enum LapceError {
    #[error("IPC error: {message}")]
    Ipc {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        #[backtrace]
        backtrace: Backtrace,
    },
    
    #[error("Provider error: {provider} - {message}")]
    Provider {
        provider: String,
        message: String,
        retry_after: Option<Duration>,
    },
    
    #[error("Parse error at {file}:{line}:{column}")]
    Parse {
        file: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted {
        resource: ResourceType,
        limit: usize,
        current: usize,
    },
    
    #[error("Operation timeout after {duration:?}")]
    Timeout {
        operation: String,
        duration: Duration,
    },
}

pub type Result<T> = std::result::Result<T, LapceError>;
```

## Error Recovery Strategies

### 1. Automatic Recovery System
```rust
pub struct RecoverySystem {
    strategies: HashMap<ErrorType, Box<dyn RecoveryStrategy>>,
    circuit_breakers: DashMap<String, CircuitBreaker>,
    fallbacks: HashMap<String, Box<dyn Fallback>>,
    health_checker: Arc<HealthChecker>,
}

#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    async fn can_recover(&self, error: &LapceError) -> bool;
    async fn recover(&self, error: &LapceError) -> Result<RecoveryAction>;
}

pub enum RecoveryAction {
    Retry { delay: Duration, max_attempts: u32 },
    Fallback { handler: String },
    CircuitBreak { duration: Duration },
    Degrade { feature: String },
    Restart { component: String },
}

impl RecoverySystem {
    pub async fn handle_error(&self, error: LapceError) -> Result<()> {
        // Classify error
        let error_type = self.classify_error(&error);
        
        // Check circuit breaker
        if let Some(breaker) = self.circuit_breakers.get(&error_type.to_string()) {
            if breaker.is_open() {
                return Err(LapceError::CircuitOpen {
                    component: error_type.to_string(),
                });
            }
        }
        
        // Find recovery strategy
        if let Some(strategy) = self.strategies.get(&error_type) {
            if strategy.can_recover(&error).await {
                match strategy.recover(&error).await? {
                    RecoveryAction::Retry { delay, max_attempts } => {
                        return self.retry_with_backoff(delay, max_attempts).await;
                    }
                    RecoveryAction::Fallback { handler } => {
                        return self.execute_fallback(&handler).await;
                    }
                    RecoveryAction::CircuitBreak { duration } => {
                        self.open_circuit_breaker(&error_type.to_string(), duration);
                    }
                    RecoveryAction::Degrade { feature } => {
                        self.degrade_feature(&feature).await;
                    }
                    RecoveryAction::Restart { component } => {
                        self.restart_component(&component).await?;
                    }
                }
            }
        }
        
        // No recovery possible
        self.log_unrecoverable_error(&error);
        Err(error)
    }
}
```

### 2. Circuit Breaker Implementation
```rust
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    failure_count: AtomicU32,
    success_count: AtomicU32,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed,
    Open { opened_at: Instant },
    HalfOpen,
}

impl CircuitBreaker {
    pub async fn call<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Check state
        let state = self.state.read().await.clone();
        
        match state {
            CircuitState::Open { opened_at } => {
                if opened_at.elapsed() > self.timeout {
                    // Try half-open
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    return Err(LapceError::CircuitOpen {
                        component: "unknown".to_string(),
                    });
                }
            }
            _ => {}
        }
        
        // Execute function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }
    
    async fn on_success(&self) {
        let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        let mut state = self.state.write().await;
        if matches!(*state, CircuitState::HalfOpen) {
            if count >= self.success_threshold {
                *state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
            }
        }
    }
    
    async fn on_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        let mut state = self.state.write().await;
        if count >= self.failure_threshold {
            *state = CircuitState::Open {
                opened_at: Instant::now(),
            };
            self.success_count.store(0, Ordering::Relaxed);
        }
    }
}
```

## Retry Mechanisms

### 1. Smart Retry System
```rust
pub struct RetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    exponential_base: f64,
    jitter: bool,
}

impl RetryPolicy {
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.initial_delay;
        
        loop {
            attempt += 1;
            
            match f().await {
                Ok(result) => return Ok(result),
                Err(error) if attempt >= self.max_attempts => {
                    return Err(error);
                }
                Err(error) if !self.is_retryable(&error) => {
                    return Err(error);
                }
                Err(_) => {
                    // Calculate next delay
                    tokio::time::sleep(delay).await;
                    
                    delay = self.calculate_next_delay(delay, attempt);
                }
            }
        }
    }
    
    fn calculate_next_delay(&self, current: Duration, attempt: u32) -> Duration {
        let mut next = current.mul_f64(self.exponential_base);
        
        if next > self.max_delay {
            next = self.max_delay;
        }
        
        if self.jitter {
            // Add jitter to prevent thundering herd
            let jitter = rand::random::<f64>() * 0.1 * next.as_secs_f64();
            next = next + Duration::from_secs_f64(jitter);
        }
        
        next
    }
    
    fn is_retryable(&self, error: &LapceError) -> bool {
        matches!(error,
            LapceError::Timeout { .. } |
            LapceError::Provider { retry_after: Some(_), .. } |
            LapceError::ResourceExhausted { .. }
        )
    }
}
```

## Graceful Degradation

### 1. Feature Degradation
```rust
pub struct DegradationManager {
    features: DashMap<String, FeatureState>,
    dependencies: HashMap<String, Vec<String>>,
    health_scores: DashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct FeatureState {
    enabled: bool,
    degraded: bool,
    fallback_mode: Option<String>,
    health_threshold: f64,
}

impl DegradationManager {
    pub async fn check_and_degrade(&self) {
        for entry in self.features.iter() {
            let (name, state) = entry.pair();
            
            if let Some(score) = self.health_scores.get(name) {
                if *score < state.health_threshold {
                    self.degrade_feature(name).await;
                } else if state.degraded && *score > state.health_threshold * 1.2 {
                    self.restore_feature(name).await;
                }
            }
        }
    }
    
    pub async fn degrade_feature(&self, feature: &str) {
        if let Some(mut state) = self.features.get_mut(feature) {
            state.degraded = true;
            
            // Apply fallback mode
            if let Some(fallback) = &state.fallback_mode {
                self.activate_fallback(feature, fallback).await;
            }
            
            // Degrade dependent features
            if let Some(deps) = self.dependencies.get(feature) {
                for dep in deps {
                    Box::pin(self.degrade_feature(dep)).await;
                }
            }
            
            tracing::warn!("Feature {} degraded", feature);
        }
    }
}
```

## Crash Recovery

### 1. State Persistence
```rust
pub struct StateRecovery {
    checkpoint_interval: Duration,
    state_file: PathBuf,
    wal: WriteAheadLog,
}

impl StateRecovery {
    pub async fn checkpoint(&self, state: &AppState) -> Result<()> {
        // Create checkpoint
        let checkpoint = Checkpoint {
            timestamp: SystemTime::now(),
            state: state.clone(),
            version: env!("CARGO_PKG_VERSION"),
        };
        
        // Write to WAL first
        self.wal.append(&checkpoint).await?;
        
        // Persist to disk
        let serialized = bincode::serialize(&checkpoint)?;
        tokio::fs::write(&self.state_file, serialized).await?;
        
        // Rotate WAL
        self.wal.checkpoint().await?;
        
        Ok(())
    }
    
    pub async fn recover(&self) -> Result<Option<AppState>> {
        // Try to load checkpoint
        if self.state_file.exists() {
            let data = tokio::fs::read(&self.state_file).await?;
            let checkpoint: Checkpoint = bincode::deserialize(&data)?;
            
            // Replay WAL entries after checkpoint
            let entries = self.wal.entries_after(checkpoint.timestamp).await?;
            let mut state = checkpoint.state;
            
            for entry in entries {
                state.apply_wal_entry(entry)?;
            }
            
            return Ok(Some(state));
        }
        
        Ok(None)
    }
}
```

### 2. Component Restart
```rust
pub struct ComponentManager {
    components: DashMap<String, Arc<dyn Component>>,
    restart_policies: HashMap<String, RestartPolicy>,
    health_monitor: Arc<HealthMonitor>,
}

#[async_trait]
pub trait Component: Send + Sync {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> Result<HealthStatus>;
    fn dependencies(&self) -> Vec<String>;
}

impl ComponentManager {
    pub async fn restart_component(&self, name: &str) -> Result<()> {
        // Get component and policy
        let component = self.components.get(name)
            .ok_or(LapceError::ComponentNotFound)?;
        let policy = self.restart_policies.get(name)
            .cloned()
            .unwrap_or_default();
            
        // Stop dependent components
        let deps = component.dependencies();
        for dep in &deps {
            self.stop_component(dep).await?;
        }
        
        // Stop target component
        component.stop().await?;
        
        // Wait before restart
        tokio::time::sleep(policy.restart_delay).await;
        
        // Start component with retries
        let retry_policy = RetryPolicy {
            max_attempts: policy.max_restart_attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            exponential_base: 2.0,
            jitter: true,
        };
        
        retry_policy.execute(|| component.start()).await?;
        
        // Restart dependent components
        for dep in deps {
            self.start_component(&dep).await?;
        }
        
        Ok(())
    }
}
```

## Error Reporting

### 1. Error Telemetry
```rust
pub struct ErrorReporter {
    collectors: Vec<Box<dyn ErrorCollector>>,
    aggregator: ErrorAggregator,
    rate_limiter: RateLimiter,
}

impl ErrorReporter {
    pub async fn report(&self, error: &LapceError) {
        // Rate limit error reporting
        if !self.rate_limiter.check_key(&error.to_string()).await {
            return;
        }
        
        // Create error report
        let report = ErrorReport {
            error: error.to_string(),
            backtrace: error.backtrace().map(|b| b.to_string()),
            timestamp: SystemTime::now(),
            context: self.collect_context(),
            severity: self.classify_severity(error),
        };
        
        // Send to collectors
        for collector in &self.collectors {
            collector.collect(&report).await;
        }
        
        // Aggregate for analysis
        self.aggregator.add(report).await;
    }
    
    fn classify_severity(&self, error: &LapceError) -> ErrorSeverity {
        match error {
            LapceError::ResourceExhausted { .. } => ErrorSeverity::Critical,
            LapceError::Provider { .. } => ErrorSeverity::Warning,
            LapceError::Parse { .. } => ErrorSeverity::Info,
            _ => ErrorSeverity::Error,
        }
    }
}
```

## Panic Handler

### 1. Custom Panic Hook
```rust
pub fn install_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        // Get panic location
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
            
        // Get panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        // Log panic
        tracing::error!(
            "PANIC at {}: {}",
            location,
            message
        );
        
        // Save crash dump
        save_crash_dump(CrashDump {
            timestamp: SystemTime::now(),
            location,
            message,
            backtrace: std::backtrace::Backtrace::capture().to_string(),
        });
        
        // Attempt graceful shutdown
        if let Ok(runtime) = tokio::runtime::Runtime::new() {
            runtime.block_on(async {
                graceful_shutdown().await;
            });
        }
    }));
}
```

## Memory Profile
- **Error type definitions**: 50KB
- **Recovery strategies**: 200KB
- **Circuit breakers**: 150KB
- **State recovery**: 300KB
- **Error reporting**: 200KB
- **Component manager**: 100KB
- **Total**: ~1MB
