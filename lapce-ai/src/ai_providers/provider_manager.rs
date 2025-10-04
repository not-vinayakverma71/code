/// Provider Manager - EXACT from 03-AI-PROVIDERS-CONSOLIDATED.md
/// Dispatch, metrics, health monitoring, circuit breaking

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, bail};
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::RwLock;
use futures::stream::BoxStream;
use governor::{Quota, RateLimiter};
use governor::state::{NotKeyed, InMemoryState};
use governor::clock::DefaultClock;

use super::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities
};

/// Provider configuration
#[derive(Debug, Clone)]
pub struct ProvidersConfig {
    pub providers: HashMap<String, ProviderConfig>,
    pub default_provider: String,
    pub health_check_interval: Duration,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_retries: u32,
    pub timeout: Duration,
    pub rate_limit_override: Option<u32>,
}

/// Provider metrics
#[derive(Debug, Default)]
pub struct ProviderMetrics {
    pub total_requests: std::sync::atomic::AtomicU64,
    pub failed_requests: std::sync::atomic::AtomicU64,
    pub total_tokens: std::sync::atomic::AtomicU64,
    pub total_latency_ms: std::sync::atomic::AtomicU64,
    pub cache_hits: std::sync::atomic::AtomicU64,
    pub cache_misses: std::sync::atomic::AtomicU64,
}

impl ProviderMetrics {
    pub fn record_request(&self, latency_ms: u64, tokens: u64, success: bool) {
        use std::sync::atomic::Ordering;
        
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
    }
}

/// Health monitor for providers
pub struct HealthMonitor {
    health_status: DashMap<String, HealthStatus>,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(interval: Duration) -> Self {
        Self {
            health_status: DashMap::new(),
            check_interval: interval,
        }
    }
    
    pub fn start_monitoring(self: Arc<Self>, providers: Arc<DashMap<String, Arc<dyn AiProvider + Send + Sync + 'static>>>) {
        let status = self.health_status.clone();
        let interval = self.check_interval;
        
        // Move the providers into a variable with explicit lifetime
        let providers = providers.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                // Collect provider names and refs first
                let provider_list: Vec<(String, Arc<dyn AiProvider + Send + Sync>)> = 
                    providers.iter().map(|entry| {
                        (entry.key().clone(), entry.value().clone())
                    }).collect();
                
                // Now check health for each provider
                for (name, provider) in provider_list {
                    let health = match provider.health_check().await {
                        Ok(h) => h,
                        Err(e) => HealthStatus {
                            healthy: false,
                            latency_ms: 0,
                            error: Some(e.to_string()),
                            rate_limit_remaining: None,
                        }
                    };
                    
                    status.insert(name, health);
                }
            }
        });
    }
    
    pub fn is_healthy(&self, provider: &str) -> bool {
        self.health_status
            .get(provider)
            .map(|h| h.healthy)
            .unwrap_or(false)
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<std::sync::atomic::AtomicU32>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            threshold,
            timeout,
        }
    }
    
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        use std::sync::atomic::Ordering;
        
        // Check circuit state
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() > self.timeout {
                        *state = CircuitState::HalfOpen;
                    } else {
                        bail!("Circuit breaker is open");
                    }
                } else {
                    bail!("Circuit breaker is open");
                }
            }
            CircuitState::HalfOpen | CircuitState::Closed => {}
        }
        
        drop(state);
        
        // Execute the function
        match f.await {
            Ok(result) => {
                // Reset on success
                self.failure_count.store(0, Ordering::Relaxed);
                *self.state.write().await = CircuitState::Closed;
                Ok(result)
            }
            Err(e) => {
                // Increment failure count
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if failures >= self.threshold {
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure_time.write().await = Some(Instant::now());
                }
                
                Err(e)
            }
        }
    }
}

/// Adaptive rate limiter
pub struct AdaptiveRateLimiter {
    tokens: Arc<std::sync::atomic::AtomicU32>,
    max_tokens: u32,
    refill_rate: u32,
    last_refill: Arc<RwLock<Instant>>,
}

impl AdaptiveRateLimiter {
    pub fn new(max_tokens: u32, refill_rate: u32) -> Self {
        use std::sync::atomic::AtomicU32;
        
        Self {
            tokens: Arc::new(AtomicU32::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    pub async fn acquire(&self, tokens: u32) -> Result<()> {
        use std::sync::atomic::Ordering;
        
        // Refill tokens based on time elapsed
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;
        let elapsed = now.duration_since(*last_refill);
        let refill_amount = (elapsed.as_secs() as u32) * self.refill_rate;
        
        if refill_amount > 0 {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + refill_amount).min(self.max_tokens);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
        
        drop(last_refill);
        
        // Try to acquire tokens
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current < tokens {
                bail!("Rate limit exceeded");
            }
            
            match self.tokens.compare_exchange(
                current,
                current - tokens,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }
}

/// The main Provider Manager
pub struct ProviderManager {
    providers: Arc<DashMap<String, Arc<dyn AiProvider + Send + Sync + 'static>>>,
    default_provider: RwLock<String>,
    health_monitor: Arc<HealthMonitor>,
    metrics: Arc<ProviderMetrics>,
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
    rate_limiters: DashMap<String, Arc<AdaptiveRateLimiter>>,
}

impl ProviderManager {
    /// Initialize provider manager with configuration
    pub async fn new(config: ProvidersConfig) -> Result<Self> {
        let providers = Arc::new(DashMap::new());
        let circuit_breakers = DashMap::new();
        let rate_limiters = DashMap::new();
        
        // Initialize providers concurrently
        let mut handles = Vec::new();
        
        for (name, provider_config) in config.providers {
            let providers_clone = providers.clone();
            let handle = tokio::spawn(async move {
                // Create provider based on name
                let provider = Self::create_provider(&provider_config).await?;
                providers_clone.insert(name.clone(), Arc::from(provider));
                Ok::<String, anyhow::Error>(name)
            });
            handles.push(handle);
        }
        
        // Wait for all providers to initialize
        for handle in handles {
            let name = handle.await??;
            
            // Create circuit breaker for each provider
            circuit_breakers.insert(
                name.clone(),
                Arc::new(CircuitBreaker::new(
                    config.circuit_breaker_threshold,
                    config.circuit_breaker_timeout,
                ))
            );
            
            // Create rate limiter for each provider  
            rate_limiters.insert(
                name,
                Arc::new(AdaptiveRateLimiter::new(60, 1)) // Default 60 RPM
            );
        }
        
        let health_monitor = Arc::new(HealthMonitor::new(config.health_check_interval));
        health_monitor.clone().start_monitoring(providers.clone());
        
        Ok(Self {
            providers,
            default_provider: RwLock::new(config.default_provider),
            health_monitor,
            metrics: Arc::new(ProviderMetrics::default()),
            circuit_breakers,
            rate_limiters,
        })
    }
    
    /// Create a provider instance based on configuration
    async fn create_provider(config: &ProviderConfig) -> Result<Arc<dyn AiProvider + Send + Sync + 'static>> {
        use crate::ai_providers::provider_registry::{ProviderRegistry, ProviderInitConfig};
        
        let init_config = ProviderInitConfig {
            provider_type: config.name.clone(),
            api_key: Some(config.api_key.clone()),
            base_url: config.base_url.clone(),
            region: None, // ProviderConfig doesn't have region field
            project_id: None,
            location: None,
            deployment_name: None,
            api_version: None,
        };
        
        let provider_arc = ProviderRegistry::create_provider(init_config).await?;
        Ok(provider_arc)
    }
    
    /// Route completion request to provider
    pub async fn complete(&self, mut request: CompletionRequest) -> Result<CompletionResponse> {
        let provider_name = self.get_provider_for_request(&request).await?;
        // Check rate limit
        if let Some(limiter) = self.rate_limiters.get(&provider_name) {
            limiter.value().acquire(1).await?;
        }
        
        // Get provider
        let provider = self.providers
            .get(&provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?
            .clone();
        
        // Execute with circuit breaker
        let start = Instant::now();
        let result = if let Some(breaker) = self.circuit_breakers.get(&provider_name) {
            breaker.call(provider.complete(request)).await
        } else {
            provider.complete(request).await
        };
        
        // Record metrics
        let latency_ms = start.elapsed().as_millis() as u64;
        let tokens = result.as_ref()
            .ok()
            .and_then(|r| r.usage.as_ref())
            .map(|u| u.total_tokens as u64)
            .unwrap_or(0);
        
        self.metrics.record_request(latency_ms, tokens, result.is_ok());
        
        result
    }
    
    /// Route chat request to provider
    pub async fn chat(&self, mut request: ChatRequest) -> Result<ChatResponse> {
        let provider_name = self.get_provider_for_chat(&request).await?;
        
        // Check rate limit
        if let Some(limiter) = self.rate_limiters.get(&provider_name) {
            limiter.value().acquire(1).await?;
        }
        
        // Get provider
        let provider = self.providers
            .get(&provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?
            .clone();
        
        // Execute with circuit breaker
        let start = Instant::now();
        let result = if let Some(breaker) = self.circuit_breakers.get(&provider_name) {
            breaker.call(provider.chat(request)).await
        } else {
            provider.chat(request).await
        };
        
        // Record metrics
        let latency_ms = start.elapsed().as_millis() as u64;
        let tokens = result.as_ref()
            .ok()
            .and_then(|r| r.usage.as_ref())
            .map(|u| u.total_tokens as u64)
            .unwrap_or(0);
        
        self.metrics.record_request(latency_ms, tokens, result.is_ok());
        
        result
    }
    
    /// Get provider for request (with fallback logic)
    async fn get_provider_for_request(&self, request: &CompletionRequest) -> Result<String> {
        // Check if model specifies a provider
        if request.model.contains('/') {
            let parts: Vec<&str> = request.model.split('/').collect();
            if parts.len() >= 2 {
                let provider = parts[0].to_string();
                if self.providers.contains_key(&provider) && self.health_monitor.is_healthy(&provider) {
                    return Ok(provider);
                }
            }
        }
        
        // Use default provider if healthy
        let default = self.default_provider.read().await.clone();
        if self.health_monitor.is_healthy(&default) {
            return Ok(default);
        }
        
        // Find any healthy provider as fallback
        for entry in self.providers.iter() {
            if self.health_monitor.is_healthy(entry.key()) {
                return Ok(entry.key().clone());
            }
        }
        
        bail!("No healthy providers available")
    }
    
    /// Get provider for chat request
    async fn get_provider_for_chat(&self, request: &ChatRequest) -> Result<String> {
        // Similar logic to get_provider_for_request
        if request.model.contains('/') {
            let parts: Vec<&str> = request.model.split('/').collect();
            if parts.len() >= 2 {
                let provider = parts[0].to_string();
                if self.providers.contains_key(&provider) && self.health_monitor.is_healthy(&provider) {
                    return Ok(provider);
                }
            }
        }
        
        let default = self.default_provider.read().await.clone();
        if self.health_monitor.is_healthy(&default) {
            return Ok(default);
        }
        
        for entry in self.providers.iter() {
            if self.health_monitor.is_healthy(entry.key()) {
                return Ok(entry.key().clone());
            }
        }
        
        bail!("No healthy providers available")
    }
}
