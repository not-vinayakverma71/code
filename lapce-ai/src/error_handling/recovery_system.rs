// HOUR 1: Recovery System Foundation - 1:1 Translation from TypeScript
// Based on recovery patterns from codex-reference

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::RwLock;

use super::errors::{LapceError, ErrorType, Result};
use super::classifier::ErrorClassifier;
use super::circuit_breaker::CircuitBreaker;

/// Recovery system for handling errors with automatic recovery
pub struct RecoverySystem {
    /// Registered recovery strategies
    strategies: HashMap<ErrorType, Box<dyn RecoveryStrategy>>,
    
    /// Circuit breakers for components
    circuit_breakers: DashMap<String, CircuitBreaker>,
    
    /// Fallback handlers
    fallbacks: HashMap<String, Box<dyn Fallback>>,
    
    /// Health checker
    health_checker: Arc<HealthChecker>,
    
    /// Error classifier
    classifier: ErrorClassifier,
}

/// Recovery strategy trait
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    /// Check if this strategy can recover from the error
    async fn can_recover(&self, error: &LapceError) -> bool;
    
    /// Attempt to recover from the error
    async fn recover(&self, error: &LapceError) -> Result<RecoveryAction>;
}

/// Recovery actions that can be taken
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry with delay and max attempts
    Retry { 
        delay: Duration, 
        max_attempts: u32 
    },
    
    /// Use fallback handler
    Fallback { 
        handler: String 
    },
    
    /// Open circuit breaker
    CircuitBreak { 
        duration: Duration 
    },
    
    /// Degrade feature
    Degrade { 
        feature: String 
    },
    
    /// Restart component
    Restart { 
        component: String 
    },
    
    /// No recovery possible
    NoRecovery,
}

/// Fallback handler trait
#[async_trait]
pub trait Fallback: Send + Sync {
    /// Execute the fallback
    async fn execute(&self) -> Result<()>;
}

/// Health checker for components
pub struct HealthChecker {
    /// Component health scores
    health_scores: DashMap<String, f64>,
    
    /// Health thresholds
    thresholds: HashMap<String, f64>,
}

impl RecoverySystem {
    /// Create new recovery system
    pub fn new() -> Self {
        Self {
            strategies: Self::default_strategies(),
            circuit_breakers: DashMap::new(),
            fallbacks: HashMap::new(),
            health_checker: Arc::new(HealthChecker::new()),
            classifier: ErrorClassifier::new(),
        }
    }
    
    /// Handle an error with recovery attempts
    pub async fn handle_error(&self, error: LapceError) -> Result<()> {
        // Classify error
        let error_type = self.classifier.classify(&error);
        
        // Check circuit breaker
        if let Some(breaker) = self.circuit_breakers.get(&error_type.to_string()) {
            if breaker.is_open().await {
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
                        self.open_circuit_breaker(&error_type.to_string(), duration).await;
                        return Err(error);
                    }
                    RecoveryAction::Degrade { feature } => {
                        self.degrade_feature(&feature).await;
                        return Ok(());
                    }
                    RecoveryAction::Restart { component } => {
                        self.restart_component(&component).await?;
                        return Ok(());
                    }
                    RecoveryAction::NoRecovery => {
                        return Err(error);
                    }
                }
            }
        }
        
        // No recovery possible
        self.log_unrecoverable_error(&error);
        Err(error)
    }
    
    /// Retry operation with exponential backoff
    async fn retry_with_backoff(&self, initial_delay: Duration, max_attempts: u32) -> Result<()> {
        // This will be implemented with retry policy
        tracing::info!("Retrying with backoff: {:?}, max attempts: {}", initial_delay, max_attempts);
        Ok(())
    }
    
    /// Execute fallback handler
    async fn execute_fallback(&self, handler_name: &str) -> Result<()> {
        if let Some(fallback) = self.fallbacks.get(handler_name) {
            fallback.execute().await
        } else {
            tracing::warn!("Fallback handler not found: {}", handler_name);
            Err(LapceError::Generic {
                context: "recovery".to_string(),
                message: format!("Fallback handler not found: {}", handler_name),
                source: None,
            })
        }
    }
    
    /// Open circuit breaker for component
    async fn open_circuit_breaker(&self, component: &str, duration: Duration) {
        if let Some(breaker) = self.circuit_breakers.get(component) {
            breaker.open(duration).await;
        } else {
            // Create new circuit breaker if not exists
            let breaker = CircuitBreaker::new(5, 3, duration);
            breaker.open(duration).await;
            self.circuit_breakers.insert(component.to_string(), breaker);
        }
        tracing::warn!("Circuit breaker opened for component: {} for {:?}", component, duration);
    }
    
    /// Degrade feature functionality
    async fn degrade_feature(&self, feature: &str) {
        // This will be implemented with degradation manager
        tracing::info!("Degrading feature: {}", feature);
        self.health_checker.update_health_score(feature, 0.5).await;
    }
    
    /// Restart component
    async fn restart_component(&self, component: &str) -> Result<()> {
        // This will be implemented with component manager
        tracing::info!("Restarting component: {}", component);
        Ok(())
    }
    
    /// Log unrecoverable error
    fn log_unrecoverable_error(&self, error: &LapceError) {
        tracing::error!("Unrecoverable error: {}", error);
        // Additional telemetry would go here
    }
    
    /// Get default recovery strategies
    fn default_strategies() -> HashMap<ErrorType, Box<dyn RecoveryStrategy>> {
        let mut strategies: HashMap<ErrorType, Box<dyn RecoveryStrategy>> = HashMap::new();
        
        // Transient errors - retry with backoff
        strategies.insert(ErrorType::Transient, Box::new(TransientRecoveryStrategy));
        
        // Rate limit errors - wait and retry
        strategies.insert(ErrorType::RateLimit, Box::new(RateLimitRecoveryStrategy));
        
        // Resource exhaustion - degrade
        strategies.insert(ErrorType::ResourceExhaustion, Box::new(ResourceRecoveryStrategy));
        
        // Timeout - retry with increased timeout
        strategies.insert(ErrorType::Timeout, Box::new(TimeoutRecoveryStrategy));
        
        strategies
    }
    
    /// Register custom recovery strategy
    pub fn register_strategy(&mut self, error_type: ErrorType, strategy: Box<dyn RecoveryStrategy>) {
        self.strategies.insert(error_type, strategy);
    }
    
    /// Register fallback handler
    pub fn register_fallback(&mut self, name: String, fallback: Box<dyn Fallback>) {
        self.fallbacks.insert(name, fallback);
    }
    
    /// Get circuit breaker for component
    pub fn get_circuit_breaker(&self, component: &str) -> Option<dashmap::mapref::one::Ref<String, CircuitBreaker>> {
        self.circuit_breakers.get(component)
    }
}

impl HealthChecker {
    /// Create new health checker
    pub fn new() -> Self {
        Self {
            health_scores: DashMap::new(),
            thresholds: Self::default_thresholds(),
        }
    }
    
    /// Get default health thresholds
    fn default_thresholds() -> HashMap<String, f64> {
        let mut thresholds = HashMap::new();
        thresholds.insert("default".to_string(), 0.7);
        thresholds
    }
    
    /// Update health score for component
    pub async fn update_health_score(&self, component: &str, score: f64) {
        self.health_scores.insert(component.to_string(), score);
    }
    
    /// Get health score for component
    pub fn get_health_score(&self, component: &str) -> Option<f64> {
        self.health_scores.get(component).map(|entry| *entry.value())
    }
    
    /// Check if component is healthy
    pub fn is_healthy(&self, component: &str) -> bool {
        let threshold = self.thresholds.get(component)
            .or_else(|| self.thresholds.get("default"))
            .copied()
            .unwrap_or(0.7);
        
        self.get_health_score(component)
            .map(|score| score >= threshold)
            .unwrap_or(true)
    }
}

// Default recovery strategy implementations

/// Recovery strategy for transient errors
struct TransientRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for TransientRecoveryStrategy {
    async fn can_recover(&self, error: &LapceError) -> bool {
        error.is_retryable()
    }
    
    async fn recover(&self, _error: &LapceError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Retry {
            delay: Duration::from_millis(100),
            max_attempts: 3,
        })
    }
}

/// Recovery strategy for rate limit errors
struct RateLimitRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for RateLimitRecoveryStrategy {
    async fn can_recover(&self, error: &LapceError) -> bool {
        matches!(error, LapceError::RateLimit { .. } | LapceError::Provider { retry_after: Some(_), .. })
    }
    
    async fn recover(&self, error: &LapceError) -> Result<RecoveryAction> {
        let delay = error.retry_delay().unwrap_or(Duration::from_secs(60));
        Ok(RecoveryAction::Retry {
            delay,
            max_attempts: 1,
        })
    }
}

/// Recovery strategy for resource exhaustion
struct ResourceRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for ResourceRecoveryStrategy {
    async fn can_recover(&self, error: &LapceError) -> bool {
        matches!(error, LapceError::ResourceExhausted { .. })
    }
    
    async fn recover(&self, error: &LapceError) -> Result<RecoveryAction> {
        if let LapceError::ResourceExhausted { resource, .. } = error {
            Ok(RecoveryAction::Degrade {
                feature: format!("{}_intensive", resource),
            })
        } else {
            Ok(RecoveryAction::NoRecovery)
        }
    }
}

/// Recovery strategy for timeout errors
struct TimeoutRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for TimeoutRecoveryStrategy {
    async fn can_recover(&self, error: &LapceError) -> bool {
        matches!(error, LapceError::Timeout { .. })
    }
    
    async fn recover(&self, _error: &LapceError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Retry {
            delay: Duration::from_millis(500),
            max_attempts: 2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recovery_system_basic() {
        let recovery_system = RecoverySystem::new();
        
        let timeout_err = LapceError::Timeout {
            operation: "test".to_string(),
            duration: Duration::from_secs(5),
        };
        
        // Should attempt recovery for timeout error
        let result = recovery_system.handle_error(timeout_err).await;
        // Initially will succeed as retry_with_backoff is stubbed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_checker() {
        let health_checker = HealthChecker::new();
        
        // Initially healthy (no score = default to healthy)
        assert!(health_checker.is_healthy("test_component"));
        
        // Update health score
        health_checker.update_health_score("test_component", 0.5).await;
        assert!(!health_checker.is_healthy("test_component"));
        
        // Update to healthy score
        health_checker.update_health_score("test_component", 0.8).await;
        assert!(health_checker.is_healthy("test_component"));
    }
}
