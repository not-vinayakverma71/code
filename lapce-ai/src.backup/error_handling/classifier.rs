// HOUR 1: Error Classifier - 1:1 Translation from TypeScript
// Based on error classification patterns from codex-reference

use std::collections::HashMap;
use super::errors::{LapceError, ErrorType, ErrorSeverity};

/// Error classifier for determining recovery strategies
pub struct ErrorClassifier {
    /// Custom classification rules
    custom_rules: HashMap<String, ErrorType>,
    
    /// Pattern-based classification
    pattern_rules: Vec<PatternRule>,
}

/// Pattern-based classification rule
struct PatternRule {
    /// Pattern to match against error message
    pattern: regex::Regex,
    
    /// Error type to assign if pattern matches
    error_type: ErrorType,
    
    /// Priority (higher = more specific)
    priority: u32,
}

impl ErrorClassifier {
    /// Create new error classifier
    pub fn new() -> Self {
        Self {
            custom_rules: Self::default_custom_rules(),
            pattern_rules: Self::default_pattern_rules(),
        }
    }
    
    /// Classify an error - matches TypeScript error classification logic
    pub fn classify(&self, error: &LapceError) -> ErrorType {
        // First check built-in classification
        let base_type = error.classify();
        
        // Check custom rules by error string
        let error_str = error.to_string();
        if let Some(custom_type) = self.custom_rules.get(&error_str) {
            return custom_type.clone();
        }
        
        // Check pattern rules
        let mut matched_rules: Vec<(&PatternRule, ErrorType)> = Vec::new();
        for rule in &self.pattern_rules {
            if rule.pattern.is_match(&error_str) {
                matched_rules.push((rule, rule.error_type.clone()));
            }
        }
        
        // Sort by priority and return highest priority match
        if !matched_rules.is_empty() {
            matched_rules.sort_by_key(|(rule, _)| rule.priority);
            if let Some((_, error_type)) = matched_rules.last() {
                return error_type.clone();
            }
        }
        
        // Return base classification
        base_type
    }
    
    /// Check if error is retryable - matches TypeScript retry logic
    pub fn is_retryable(&self, error: &LapceError) -> bool {
        let error_type = self.classify(error);
        matches!(
            error_type,
            ErrorType::Transient | ErrorType::RateLimit | ErrorType::ResourceExhaustion | ErrorType::Timeout
        )
    }
    
    /// Get recommended recovery action
    pub fn recommend_recovery(&self, error: &LapceError) -> RecoveryRecommendation {
        let error_type = self.classify(error);
        
        match error_type {
            ErrorType::Transient => RecoveryRecommendation::RetryWithBackoff {
                initial_delay_ms: 100,
                max_attempts: 3,
                exponential_base: 2.0,
            },
            ErrorType::RateLimit => RecoveryRecommendation::RetryAfterDelay {
                delay_ms: error.retry_delay()
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(60000),
            },
            ErrorType::ResourceExhaustion => RecoveryRecommendation::Degrade {
                feature: "resource_intensive_operations".to_string(),
                fallback: Some("lightweight_mode".to_string()),
            },
            ErrorType::Timeout => RecoveryRecommendation::RetryWithBackoff {
                initial_delay_ms: 500,
                max_attempts: 2,
                exponential_base: 3.0,
            },
            ErrorType::CircuitBreaker => RecoveryRecommendation::CircuitBreak {
                duration_ms: 30000,
            },
            ErrorType::Authentication => RecoveryRecommendation::RequiresIntervention {
                message: "Authentication credentials need to be updated".to_string(),
            },
            ErrorType::Validation => RecoveryRecommendation::NoRetry {
                reason: "Validation errors require fixing the input".to_string(),
            },
            _ => RecoveryRecommendation::NoRetry {
                reason: "Unknown error type".to_string(),
            },
        }
    }
    
    /// Default custom rules - matches TypeScript error mappings
    fn default_custom_rules() -> HashMap<String, ErrorType> {
        let mut rules = HashMap::new();
        
        // Specific error messages that override default classification
        rules.insert("Connection reset by peer".to_string(), ErrorType::Transient);
        rules.insert("Connection refused".to_string(), ErrorType::Transient);
        rules.insert("Network unreachable".to_string(), ErrorType::Transient);
        rules.insert("Too many open files".to_string(), ErrorType::ResourceExhaustion);
        rules.insert("Out of memory".to_string(), ErrorType::ResourceExhaustion);
        
        rules
    }
    
    /// Default pattern rules - matches TypeScript error patterns
    fn default_pattern_rules() -> Vec<PatternRule> {
        vec![
            PatternRule {
                pattern: regex::Regex::new(r"(?i)connection.*reset").unwrap(),
                error_type: ErrorType::Transient,
                priority: 10,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)too\s+many\s+requests").unwrap(),
                error_type: ErrorType::RateLimit,
                priority: 20,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)quota.*exceeded").unwrap(),
                error_type: ErrorType::RateLimit,
                priority: 20,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)memory.*exhausted").unwrap(),
                error_type: ErrorType::ResourceExhaustion,
                priority: 15,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)timeout|timed\s+out").unwrap(),
                error_type: ErrorType::Timeout,
                priority: 15,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)unauthorized|forbidden|401|403").unwrap(),
                error_type: ErrorType::Authentication,
                priority: 25,
            },
            PatternRule {
                pattern: regex::Regex::new(r"(?i)invalid.*request|bad.*request|400").unwrap(),
                error_type: ErrorType::Validation,
                priority: 20,
            },
        ]
    }
    
    /// Add custom classification rule
    pub fn add_custom_rule(&mut self, error_message: String, error_type: ErrorType) {
        self.custom_rules.insert(error_message, error_type);
    }
    
    /// Add pattern-based classification rule
    pub fn add_pattern_rule(&mut self, pattern: &str, error_type: ErrorType, priority: u32) -> Result<(), regex::Error> {
        let regex = regex::Regex::new(pattern)?;
        self.pattern_rules.push(PatternRule {
            pattern: regex,
            error_type,
            priority,
        });
        Ok(())
    }
}

/// Recovery recommendation from classifier
#[derive(Debug, Clone)]
pub enum RecoveryRecommendation {
    /// Retry with exponential backoff
    RetryWithBackoff {
        initial_delay_ms: u64,
        max_attempts: u32,
        exponential_base: f64,
    },
    
    /// Retry after specific delay
    RetryAfterDelay {
        delay_ms: u64,
    },
    
    /// Degrade functionality
    Degrade {
        feature: String,
        fallback: Option<String>,
    },
    
    /// Open circuit breaker
    CircuitBreak {
        duration_ms: u64,
    },
    
    /// No retry - permanent failure
    NoRetry {
        reason: String,
    },
    
    /// Requires manual intervention
    RequiresIntervention {
        message: String,
    },
}

/// Error statistics for classification analysis
#[derive(Debug, Clone, Default)]
pub struct ErrorStatistics {
    /// Count by error type
    pub type_counts: HashMap<ErrorType, usize>,
    
    /// Count by severity
    pub severity_counts: HashMap<ErrorSeverity, usize>,
    
    /// Total errors classified
    pub total_errors: usize,
    
    /// Retryable errors
    pub retryable_count: usize,
    
    /// Permanent errors
    pub permanent_count: usize,
}

impl ErrorStatistics {
    /// Update statistics with new error
    pub fn record_error(&mut self, error: &LapceError, classifier: &ErrorClassifier) {
        let error_type = classifier.classify(error);
        let severity = error.severity();
        let is_retryable = classifier.is_retryable(error);
        
        *self.type_counts.entry(error_type).or_insert(0) += 1;
        *self.severity_counts.entry(severity).or_insert(0) += 1;
        self.total_errors += 1;
        
        if is_retryable {
            self.retryable_count += 1;
        } else {
            self.permanent_count += 1;
        }
    }
    
    /// Get retry success rate
    pub fn retry_success_rate(&self) -> f64 {
        if self.retryable_count == 0 {
            0.0
        } else {
            self.retryable_count as f64 / self.total_errors as f64
        }
    }
    
    /// Get most common error type
    pub fn most_common_type(&self) -> Option<ErrorType> {
        self.type_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(error_type, _)| error_type.clone())
    }
    
    /// Get most common severity
    pub fn most_common_severity(&self) -> Option<ErrorSeverity> {
        self.severity_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(severity, _)| *severity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_classifier_basic() {
        let classifier = ErrorClassifier::new();
        
        let timeout_err = LapceError::Timeout {
            operation: "test".to_string(),
            duration: Duration::from_secs(5),
        };
        
        assert_eq!(classifier.classify(&timeout_err), ErrorType::Timeout);
        assert!(classifier.is_retryable(&timeout_err));
    }

    #[test]
    fn test_recovery_recommendation() {
        let classifier = ErrorClassifier::new();
        
        let rate_limit_err = LapceError::RateLimit {
            message: "too many requests".to_string(),
            retry_after: Some(Duration::from_secs(60)),
            provider: Some("openai".to_string()),
        };
        
        match classifier.recommend_recovery(&rate_limit_err) {
            RecoveryRecommendation::RetryAfterDelay { delay_ms } => {
                assert_eq!(delay_ms, 60000);
            }
            _ => panic!("Expected RetryAfterDelay recommendation"),
        }
    }

    #[test]
    fn test_pattern_matching() {
        let mut classifier = ErrorClassifier::new();
        
        // Add custom pattern
        classifier.add_pattern_rule(
            r"(?i)custom.*error",
            ErrorType::Transient,
            100
        ).unwrap();
        
        let generic_err = LapceError::Generic {
            context: "test".to_string(),
            message: "This is a custom error".to_string(),
            source: None,
        };
        
        assert_eq!(classifier.classify(&generic_err), ErrorType::Transient);
    }

    #[test]
    fn test_error_statistics() {
        let classifier = ErrorClassifier::new();
        let mut stats = ErrorStatistics::default();
        
        let errors = vec![
            LapceError::Timeout {
                operation: "op1".to_string(),
                duration: Duration::from_secs(5),
            },
            LapceError::Timeout {
                operation: "op2".to_string(),
                duration: Duration::from_secs(10),
            },
            LapceError::InvalidRequest {
                message: "bad request".to_string(),
                error_type: "validation".to_string(),
                status_code: 400,
            },
        ];
        
        for error in &errors {
            stats.record_error(error, &classifier);
        }
        
        assert_eq!(stats.total_errors, 3);
        assert_eq!(stats.retryable_count, 2);
        assert_eq!(stats.permanent_count, 1);
        assert_eq!(stats.most_common_type(), Some(ErrorType::Timeout));
    }
}
