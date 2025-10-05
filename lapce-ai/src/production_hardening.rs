/// Production Hardening Implementation (Day 29)
/// Comprehensive error handling, monitoring, and graceful degradation

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, Context};
use thiserror::Error;
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram, register_gauge};
use tracing::{error, warn, info, debug, instrument};
use tokio::sync::{RwLock, Semaphore};
use dashmap::DashMap;

/// Custom error types for better error handling
#[derive(Error, Debug)]
pub enum LapceAiError {
    #[error("IPC error: {0}")]
    Ipc(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Connection pool exhausted")]
    PoolExhausted,
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    half_open_requests: Arc<Semaphore>,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_threshold,
            success_threshold,
            timeout,
            half_open_requests: Arc::new(Semaphore::new(1)),
        }
    }

    #[instrument(skip(self, f))]
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let state = self.state.read().await.clone();
        match state {
            CircuitState::Open(last_failure) => {
                if last_failure.elapsed() > self.timeout {
                    let _permit = self.half_open_requests.try_acquire();
                    match f.await {
                        Ok(result) => {
                            *self.state.write().await = CircuitState::Closed;
                            Ok(result)
                        }
                        Err(e) => {
                            *self.state.write().await = CircuitState::Open(Instant::now());
                            Err(e)
                        }
                    }
                } else {
                    Err(anyhow::anyhow!("Circuit breaker is open"))
                }
            }
            CircuitState::HalfOpen => {
                // Transition state - same as Open for now
                match f.await {
                    Ok(result) => {
                        *self.state.write().await = CircuitState::Closed;
                        Ok(result)
                    }
                    Err(e) => {
                        *self.state.write().await = CircuitState::Open(Instant::now());
                        Err(e)
                    }
                }
            }
            CircuitState::Closed => {
                match f.await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        // In production, track failures properly
                        *self.state.write().await = CircuitState::Open(Instant::now());
                        Err(e)
                    }
                }
            }
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: Arc<RwLock<f64>>,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn acquire(&self, tokens_needed: f64) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        let mut last_refill = self.last_refill.write().await;
        
        // Refill tokens based on time elapsed
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let tokens_to_add = (elapsed * self.refill_rate).min(self.max_tokens - *tokens);
        *tokens += tokens_to_add;
        *last_refill = now;
        
        // Check if enough tokens available
        if *tokens >= tokens_needed {
            *tokens -= tokens_needed;
            Ok(())
        } else {
            Err(LapceAiError::RateLimited.into())
        }
    }
}

/// Metrics collector for monitoring
pub struct MetricsCollector {
    request_counter: Counter,
    error_counter: Counter,
    latency_histogram: Histogram,
    active_connections: Gauge,
    cache_hit_rate: Gauge,
    memory_usage: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            request_counter: register_counter!("lapce_ai_requests_total", "Total requests")?,
            error_counter: register_counter!("lapce_ai_errors_total", "Total errors")?,
            latency_histogram: register_histogram!(
                "lapce_ai_latency_seconds",
                "Request latency",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
            )?,
            active_connections: register_gauge!("lapce_ai_active_connections", "Active connections")?,
            cache_hit_rate: register_gauge!("lapce_ai_cache_hit_rate", "Cache hit rate")?,
            memory_usage: register_gauge!("lapce_ai_memory_bytes", "Memory usage in bytes")?,
        })
    }

    pub fn record_request(&self) {
        self.request_counter.inc();
    }

    pub fn record_error(&self) {
        self.error_counter.inc();
    }

    pub fn record_latency(&self, duration: Duration) {
        self.latency_histogram.observe(duration.as_secs_f64());
    }

    pub fn set_active_connections(&self, count: f64) {
        self.active_connections.set(count);
    }

    pub fn set_cache_hit_rate(&self, rate: f64) {
        self.cache_hit_rate.set(rate);
    }

    pub fn update_memory_usage(&self) {
        if let Ok(mem_info) = sys_info::mem_info() {
            let used = (mem_info.total - mem_info.avail) * 1024;
            self.memory_usage.set(used as f64);
        }
    }
}

/// Health check endpoint
pub struct HealthCheck {
    checks: Arc<DashMap<String, HealthStatus>>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub last_check: Instant,
}

impl HealthCheck {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(DashMap::new()),
        }
    }

    pub fn register_check(&self, name: String, status: HealthStatus) {
        self.checks.insert(name, status);
    }

    pub async fn check_all(&self) -> HealthReport {
        let mut overall_healthy = true;
        let mut details = Vec::new();
        
        for entry in self.checks.iter() {
            let (name, status) = entry.pair();
            if !status.healthy {
                overall_healthy = false;
            }
            details.push((name.clone(), status.clone()));
        }
        
        HealthReport {
            healthy: overall_healthy,
            timestamp: Instant::now(),
            details,
        }
    }
}

#[derive(Debug)]
pub struct HealthReport {
    pub healthy: bool,
    pub timestamp: Instant,
    pub details: Vec<(String, HealthStatus)>,
}

/// Graceful degradation manager
pub struct GracefulDegradation {
    feature_flags: Arc<DashMap<String, bool>>,
    fallback_strategies: Arc<DashMap<String, FallbackStrategy>>,
}

#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    ReturnDefault,
    UseCache,
    ReduceQuality,
    DisableFeature,
}

impl GracefulDegradation {
    pub fn new() -> Self {
        Self {
            feature_flags: Arc::new(DashMap::new()),
            fallback_strategies: Arc::new(DashMap::new()),
        }
    }

    pub fn set_feature(&self, name: String, enabled: bool) {
        self.feature_flags.insert(name, enabled);
    }

    pub fn is_feature_enabled(&self, name: &str) -> bool {
        self.feature_flags.get(name).map_or(false, |v| *v)
    }

    pub fn register_fallback(&self, service: String, strategy: FallbackStrategy) {
        self.fallback_strategies.insert(service, strategy);
    }

    #[instrument(skip(self, primary, fallback))]
    pub async fn execute_with_fallback<F, G, T>(
        &self,
        service: &str,
        primary: F,
        fallback: G,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
        G: std::future::Future<Output = T>,
    {
        match primary.await {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!("Primary service failed: {}, using fallback", e);
                
                if let Some(strategy) = self.fallback_strategies.get(service) {
                    match strategy.value() {
                        FallbackStrategy::ReturnDefault | 
                        FallbackStrategy::UseCache | 
                        FallbackStrategy::ReduceQuality => {
                            Ok(fallback.await)
                        }
                        FallbackStrategy::DisableFeature => {
                            self.set_feature(service.to_string(), false);
                            Err(e)
                        }
                    }
                } else {
                    Ok(fallback.await)
                }
            }
        }
    }
}

/// Input validator for security
pub struct InputValidator {
    max_size: usize,
    allowed_patterns: Vec<regex::Regex>,
    blocked_patterns: Vec<regex::Regex>,
}

impl InputValidator {
    pub fn new() -> Self {
        Self {
            max_size: 10_000_000, // 10MB
            allowed_patterns: vec![],
            blocked_patterns: vec![
                regex::Regex::new(r"(?i)<script").unwrap(),
                regex::Regex::new(r"(?i)javascript:").unwrap(),
                regex::Regex::new(r"(?i)on\w+=").unwrap(),
            ],
        }
    }

    pub fn validate(&self, input: &str) -> Result<()> {
        // Check size
        if input.len() > self.max_size {
            return Err(LapceAiError::InvalidInput("Input too large".into()).into());
        }
        
        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(input) {
                return Err(LapceAiError::InvalidInput("Blocked pattern detected".into()).into());
            }
        }
        
        Ok(())
    }
}

/// Production configuration
#[derive(Debug, Clone)]
pub struct ProductionConfig {
    pub enable_monitoring: bool,
    pub enable_tracing: bool,
    pub enable_health_checks: bool,
    pub enable_rate_limiting: bool,
    pub enable_circuit_breaker: bool,
    pub enable_graceful_degradation: bool,
    pub max_request_size: usize,
    pub request_timeout: Duration,
    pub shutdown_timeout: Duration,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            enable_tracing: true,
            enable_health_checks: true,
            enable_rate_limiting: true,
            enable_circuit_breaker: true,
            enable_graceful_degradation: true,
            max_request_size: 10_000_000,
            request_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
        }
    }
}

/// Initialize production environment
pub fn init_production(config: ProductionConfig) -> Result<()> {
    // Initialize tracing
    if config.enable_tracing {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .init();
    }
    
    info!("Production environment initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));
        
        // Should work initially
        let result = cb.call(async { Ok::<_, anyhow::Error>(42) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0, 1.0);
        
        // Should allow initial requests
        assert!(limiter.acquire(5.0).await.is_ok());
        assert!(limiter.acquire(5.0).await.is_ok());
        
        // Should reject when exhausted
        assert!(limiter.acquire(1.0).await.is_err());
    }

    #[test]
    fn test_input_validator() {
        let validator = InputValidator::new();
        
        assert!(validator.validate("normal input").is_ok());
        assert!(validator.validate("<script>alert('xss')</script>").is_err());
    }
}
