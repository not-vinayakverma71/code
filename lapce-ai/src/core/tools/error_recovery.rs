// Error tracking and recovery system - P1-10
// Provides resilient error handling and recovery mechanisms

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Error severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Tool execution error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedError {
    pub id: String,
    pub tool_name: String,
    pub error_type: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub timestamp: u64,
    pub context: ErrorContext,
    pub recovery_attempted: bool,
    pub recovery_successful: bool,
}

/// Error context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub correlation_id: String,
    pub user_id: String,
    pub workspace_path: String,
    pub input_args: Option<serde_json::Value>,
    pub stack_trace: Option<String>,
    pub related_errors: Vec<String>,
}

/// Recovery strategy for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay_ms: u64 },
    Fallback { alternative_tool: String },
    Skip,
    Abort,
    Manual,
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    errors: Arc<RwLock<VecDeque<TrackedError>>>,
    max_errors: usize,
    recovery_strategies: Arc<RwLock<std::collections::HashMap<String, RecoveryStrategy>>>,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_errors: 1000,
            recovery_strategies: Arc::new(RwLock::new(Self::default_strategies())),
        }
    }
    
    /// Default recovery strategies
    fn default_strategies() -> std::collections::HashMap<String, RecoveryStrategy> {
        let mut strategies = std::collections::HashMap::new();
        
        // Network errors - retry with backoff
        strategies.insert(
            "NetworkError".to_string(),
            RecoveryStrategy::Retry { max_attempts: 3, delay_ms: 1000 }
        );
        
        // File not found - skip
        strategies.insert(
            "FileNotFound".to_string(),
            RecoveryStrategy::Skip
        );
        
        // Permission denied - manual intervention
        strategies.insert(
            "PermissionDenied".to_string(),
            RecoveryStrategy::Manual
        );
        
        // Tool not found - fallback
        strategies.insert(
            "ToolNotFound".to_string(),
            RecoveryStrategy::Fallback { alternative_tool: "execute_command".to_string() }
        );
        
        strategies
    }
    
    /// Track an error
    pub async fn track_error(&self, error: TrackedError) {
        let mut errors = self.errors.write().await;
        
        // Maintain max size
        if errors.len() >= self.max_errors {
            errors.pop_front();
        }
        
        errors.push_back(error);
    }
    
    /// Get recovery strategy for error type
    pub async fn get_recovery_strategy(&self, error_type: &str) -> RecoveryStrategy {
        let strategies = self.recovery_strategies.read().await;
        strategies.get(error_type)
            .cloned()
            .unwrap_or(RecoveryStrategy::Skip)
    }
    
    /// Attempt recovery
    pub async fn attempt_recovery(
        &self,
        error: &TrackedError,
    ) -> Result<RecoveryResult> {
        let strategy = self.get_recovery_strategy(&error.error_type).await;
        
        match strategy {
            RecoveryStrategy::Retry { max_attempts, delay_ms } => {
                Ok(RecoveryResult::Retry { 
                    delay_ms,
                    attempt: 1,
                    max_attempts 
                })
            }
            RecoveryStrategy::Fallback { alternative_tool } => {
                Ok(RecoveryResult::Fallback { alternative_tool })
            }
            RecoveryStrategy::Skip => {
                Ok(RecoveryResult::Skip)
            }
            RecoveryStrategy::Abort => {
                Ok(RecoveryResult::Abort)
            }
            RecoveryStrategy::Manual => {
                Ok(RecoveryResult::RequiresManualIntervention)
            }
        }
    }
    
    /// Get error statistics
    pub async fn get_statistics(&self) -> ErrorStatistics {
        let errors = self.errors.read().await;
        
        let mut by_severity = std::collections::HashMap::new();
        let mut by_tool = std::collections::HashMap::new();
        let mut recovery_success = 0;
        let mut recovery_failed = 0;
        
        for error in errors.iter() {
            *by_severity.entry(error.severity).or_insert(0) += 1;
            *by_tool.entry(error.tool_name.clone()).or_insert(0) += 1;
            
            if error.recovery_attempted {
                if error.recovery_successful {
                    recovery_success += 1;
                } else {
                    recovery_failed += 1;
                }
            }
        }
        
        ErrorStatistics {
            total_errors: errors.len(),
            by_severity,
            by_tool,
            recovery_success_rate: if recovery_success + recovery_failed > 0 {
                (recovery_success as f64) / ((recovery_success + recovery_failed) as f64)
            } else {
                0.0
            },
        }
    }
    
    /// Clear old errors
    pub async fn clear_old_errors(&self, older_than_secs: u64) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - older_than_secs;
        
        let mut errors = self.errors.write().await;
        errors.retain(|e| e.timestamp > cutoff);
    }
}

/// Recovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryResult {
    Retry { delay_ms: u64, attempt: u32, max_attempts: u32 },
    Fallback { alternative_tool: String },
    Skip,
    Abort,
    RequiresManualIntervention,
}

/// Error statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStatistics {
    pub total_errors: usize,
    pub by_severity: std::collections::HashMap<ErrorSeverity, usize>,
    pub by_tool: std::collections::HashMap<String, usize>,
    pub recovery_success_rate: f64,
}

/// Global error recovery manager
lazy_static::lazy_static! {
    pub static ref ERROR_RECOVERY: Arc<ErrorRecoveryManager> = 
        Arc::new(ErrorRecoveryManager::new());
}

/// Helper to create tracked error
pub fn create_tracked_error(
    tool_name: &str,
    error: &anyhow::Error,
    context: ErrorContext,
) -> TrackedError {
    TrackedError {
        id: uuid::Uuid::new_v4().to_string(),
        tool_name: tool_name.to_string(),
        error_type: error.to_string().split(':').next().unwrap_or("Unknown").to_string(),
        message: error.to_string(),
        severity: ErrorSeverity::Error,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        context,
        recovery_attempted: false,
        recovery_successful: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_error_tracking() {
        let manager = ErrorRecoveryManager::new();
        
        let error = TrackedError {
            id: "test-1".to_string(),
            tool_name: "test_tool".to_string(),
            error_type: "NetworkError".to_string(),
            message: "Connection timeout".to_string(),
            severity: ErrorSeverity::Error,
            timestamp: 0,
            context: ErrorContext {
                correlation_id: "corr-1".to_string(),
                user_id: "user-1".to_string(),
                workspace_path: "/test".to_string(),
                input_args: None,
                stack_trace: None,
                related_errors: vec![],
            },
            recovery_attempted: false,
            recovery_successful: false,
        };
        
        manager.track_error(error.clone()).await;
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_errors, 1);
        
        let strategy = manager.get_recovery_strategy("NetworkError").await;
        match strategy {
            RecoveryStrategy::Retry { max_attempts, .. } => {
                assert_eq!(max_attempts, 3);
            }
            _ => panic!("Expected Retry strategy"),
        }
    }
}
