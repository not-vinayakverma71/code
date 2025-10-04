// HOUR 2: Provider-Specific Error Patterns - Exact 1:1 Translation from TypeScript
// Based on error patterns found in codex-reference/api/providers/*.ts

use std::time::Duration;
use super::errors::{LapceError, Result};

/// Provider-specific error patterns from TypeScript
pub struct ProviderErrorPatterns;

impl ProviderErrorPatterns {
    /// OpenAI/GPT-5 specific error handling - matches openai-native.ts
    pub fn handle_openai_error(error: &str, status: Option<u16>) -> LapceError {
        // Pattern: "Previous response ID not found"
        if error.contains("Previous response") || error.contains("not found") {
            if status == Some(400) {
                return LapceError::InvalidRequest {
                    message: format!("Previous response ID not found: {}", error),
                    error_type: "previous_response_error".to_string(),
                    status_code: 400,
                };
            }
        }
        
        // Pattern: "OpenAI SDK did not return an AsyncIterable"
        if error.contains("AsyncIterable") || error.contains("streaming response") {
            return LapceError::Provider {
                provider: "openai".to_string(),
                message: error.to_string(),
                retry_after: None,
            };
        }
        
        // Pattern: "GPT-5 API retry failed"
        if error.contains("retry failed") {
            return LapceError::Provider {
                provider: "gpt5".to_string(),
                message: error.to_string(),
                retry_after: Some(Duration::from_secs(1)),
            };
        }
        
        // Pattern: "Responses API error"
        if error.contains("Responses API error") {
            return LapceError::Provider {
                provider: "openai".to_string(),
                message: error.to_string(),
                retry_after: None,
            };
        }
        
        // Default OpenAI error
        LapceError::Provider {
            provider: "openai".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
    
    /// Anthropic error patterns - from context-error-handling.ts
    pub fn handle_anthropic_error(error: &str, status: Option<u16>) -> LapceError {
        // Pattern: Context window exceeded
        let context_patterns = [
            "prompt is too long",
            "maximum.*tokens",
            "context.*too.*long",
            "exceeds.*context",
            "token.*limit",
            "context_length_exceeded",
            "max_tokens_to_sample",
        ];
        
        let error_lower = error.to_lowercase();
        for pattern in context_patterns {
            if error_lower.contains(pattern) {
                return LapceError::ContextWindowExceeded {
                    provider: "anthropic".to_string(),
                    message: error.to_string(),
                    max_tokens: None,
                    used_tokens: None,
                };
            }
        }
        
        // Pattern: Rate limit
        if status == Some(429) || error_lower.contains("rate limit") {
            return LapceError::RateLimit {
                message: error.to_string(),
                retry_after: Some(Duration::from_secs(60)),
                provider: Some("anthropic".to_string()),
            };
        }
        
        // Default Anthropic error
        LapceError::Provider {
            provider: "anthropic".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
    
    /// Ollama error patterns - from ollama.ts line 110
    pub fn handle_ollama_error(error: &str) -> LapceError {
        // Pattern: "Ollama completion error: ${error.message}"
        if error.starts_with("Ollama completion error:") {
            return LapceError::Provider {
                provider: "ollama".to_string(),
                message: error.to_string(),
                retry_after: None,
            };
        }
        
        // Default Ollama error
        LapceError::Provider {
            provider: "ollama".to_string(),
            message: format!("Ollama completion error: {}", error),
            retry_after: None,
        }
    }
    
    /// Bedrock error patterns
    pub fn handle_bedrock_error(error: &str, status: Option<u16>) -> LapceError {
        // Pattern: Throttling
        if status == Some(429) || error.contains("ThrottlingException") {
            return LapceError::RateLimit {
                message: error.to_string(),
                retry_after: Some(Duration::from_secs(2)),
                provider: Some("bedrock".to_string()),
            };
        }
        
        // Pattern: Model not found
        if status == Some(404) || error.contains("ModelNotFoundException") {
            return LapceError::ComponentNotFound(format!("bedrock_model: {}", error));
        }
        
        // Default Bedrock error
        LapceError::Provider {
            provider: "bedrock".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
    
    /// Vertex AI error patterns
    pub fn handle_vertex_error(error: &str, status: Option<u16>) -> LapceError {
        // Pattern: Quota exceeded
        if status == Some(429) || error.contains("quota") {
            return LapceError::RateLimit {
                message: error.to_string(),
                retry_after: Some(Duration::from_secs(60)),
                provider: Some("vertex".to_string()),
            };
        }
        
        // Pattern: Authentication
        if status == Some(401) || status == Some(403) {
            return LapceError::AuthenticationFailed {
                message: error.to_string(),
                provider: Some("vertex".to_string()),
            };
        }
        
        // Default Vertex error
        LapceError::Provider {
            provider: "vertex".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
    
    /// OpenRouter error patterns - matches checkIsOpenRouterContextWindowError
    pub fn handle_openrouter_error(error: &str, status: Option<u16>) -> LapceError {
        let error_lower = error.to_lowercase();
        
        // Pattern: Context window errors (status 400)
        if status == Some(400) {
            let context_patterns = [
                r"\bcontext\s*(?:length|window)\b",
                r"\bmaximum\s*context\b",
                r"\b(?:input\s*)?tokens?\s*exceed",
                r"\btoo\s*many\s*tokens?\b",
            ];
            
            for pattern in context_patterns {
                if regex::Regex::new(pattern).unwrap().is_match(&error_lower) {
                    return LapceError::ContextWindowExceeded {
                        provider: "openrouter".to_string(),
                        message: error.to_string(),
                        max_tokens: None,
                        used_tokens: None,
                    };
                }
            }
        }
        
        // Default OpenRouter error
        LapceError::Provider {
            provider: "openrouter".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
    
    /// Cerebras error patterns - matches checkIsCerebrasContextWindowError
    pub fn handle_cerebras_error(error: &str, status: Option<u16>) -> LapceError {
        // Pattern: "Please reduce the length of the messages or completion"
        if status == Some(400) && error.contains("Please reduce the length of the messages or completion") {
            return LapceError::ContextWindowExceeded {
                provider: "cerebras".to_string(),
                message: error.to_string(),
                max_tokens: None,
                used_tokens: None,
            };
        }
        
        // Default Cerebras error
        LapceError::Provider {
            provider: "cerebras".to_string(),
            message: error.to_string(),
            retry_after: None,
        }
    }
}

/// Retry strategy for provider errors - matches TypeScript retry patterns
pub struct ProviderRetryStrategy {
    /// Provider name
    provider: String,
    
    /// Base retry delay
    base_delay: Duration,
    
    /// Maximum retry attempts
    max_attempts: u32,
}

impl ProviderRetryStrategy {
    /// Create retry strategy for provider
    pub fn for_provider(provider: &str) -> Self {
        match provider {
            "openai" | "gpt5" => Self {
                provider: provider.to_string(),
                base_delay: Duration::from_millis(100),
                max_attempts: 3,
            },
            "anthropic" => Self {
                provider: provider.to_string(),
                base_delay: Duration::from_secs(1),
                max_attempts: 2,
            },
            "bedrock" => Self {
                provider: provider.to_string(),
                base_delay: Duration::from_secs(2),
                max_attempts: 3,
            },
            "vertex" => Self {
                provider: provider.to_string(),
                base_delay: Duration::from_secs(1),
                max_attempts: 2,
            },
            _ => Self {
                provider: provider.to_string(),
                base_delay: Duration::from_millis(500),
                max_attempts: 2,
            },
        }
    }
    
    /// Should retry for this error?
    pub fn should_retry(&self, error: &LapceError, attempt: u32) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        
        match error {
            // Always retry rate limits
            LapceError::RateLimit { .. } => true,
            
            // Retry provider errors for transient issues
            LapceError::Provider { provider, message, .. } => {
                if provider != &self.provider {
                    return false;
                }
                
                // Retry patterns from TypeScript
                let retry_patterns = [
                    "retry",
                    "timeout",
                    "network",
                    "connection",
                    "temporarily",
                    "AsyncIterable",
                ];
                
                let msg_lower = message.to_lowercase();
                retry_patterns.iter().any(|pattern| msg_lower.contains(pattern))
            }
            
            // Retry context window errors with smaller payload
            LapceError::ContextWindowExceeded { provider, .. } => {
                provider == &self.provider && attempt == 0
            }
            
            // Don't retry authentication or validation errors
            LapceError::AuthenticationFailed { .. } | 
            LapceError::InvalidRequest { .. } => false,
            
            // Retry timeouts
            LapceError::Timeout { .. } => true,
            
            _ => false,
        }
    }
    
    /// Calculate retry delay
    pub fn retry_delay(&self, error: &LapceError, attempt: u32) -> Duration {
        // Use error's retry_after if available
        if let Some(delay) = error.retry_delay() {
            return delay;
        }
        
        // Exponential backoff
        let multiplier = 2_u32.pow(attempt);
        self.base_delay * multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_previous_response_error() {
        let error = ProviderErrorPatterns::handle_openai_error(
            "Previous response ID not found", 
            Some(400)
        );
        
        match error {
            LapceError::InvalidRequest { error_type, status_code, .. } => {
                assert_eq!(error_type, "previous_response_error");
                assert_eq!(status_code, 400);
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_anthropic_context_window_error() {
        let error = ProviderErrorPatterns::handle_anthropic_error(
            "prompt is too long for context window",
            Some(400)
        );
        
        assert!(matches!(error, LapceError::ContextWindowExceeded { .. }));
    }

    #[test]
    fn test_provider_retry_strategy() {
        let strategy = ProviderRetryStrategy::for_provider("openai");
        
        let rate_limit = LapceError::RateLimit {
            message: "too many requests".to_string(),
            retry_after: None,
            provider: Some("openai".to_string()),
        };
        
        assert!(strategy.should_retry(&rate_limit, 0));
        assert!(strategy.should_retry(&rate_limit, 1));
        assert!(strategy.should_retry(&rate_limit, 2));
        assert!(!strategy.should_retry(&rate_limit, 3));
    }
}
