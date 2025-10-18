//! Context Window Error Detection
//!
//! Direct 1:1 port from Codex/src/core/context/context-management/context-error-handling.ts
//! Lines 1-115 complete verbatim logic
//!
//! Detects context window exceeded errors from:
//! - Anthropic (invalid_request_error with prompt too long patterns)
//! - OpenAI (LengthFinishReasonError, 400 with token/context length)
//! - OpenRouter (400 with context length patterns)
//! - Cerebras (400 with specific message pattern)

use serde_json::Value;
use std::collections::HashMap;

/// Main entry point: checks if error is a context window exceeded error
/// Port of checkContextWindowExceededError() from context-error-handling.ts lines 3-10
pub fn check_context_window_exceeded_error(error: &Value) -> bool {
    check_is_openai_context_window_error(error)
        || check_is_openrouter_context_window_error(error)
        || check_is_anthropic_context_window_error(error)
        || check_is_cerebras_context_window_error(error)
}

/// Checks for OpenRouter-style context window errors
/// Port of checkIsOpenRouterContextWindowError() from context-error-handling.ts lines 12-35
fn check_is_openrouter_context_window_error(error: &Value) -> bool {
    if !error.is_object() {
        return false;
    }
    
    let obj = error.as_object().unwrap();
    
    // Extract status from multiple possible locations
    let status = extract_status(obj);
    let message = extract_message(obj);
    
    // Known OpenAI/OpenRouter-style signal (code 400 and message includes "context length")
    const CONTEXT_ERROR_PATTERNS: &[&str] = &[
        r"(?i)\bcontext\s*(?:length|window)\b",
        r"(?i)\bmaximum\s*context\b",
        r"(?i)\b(?:input\s*)?tokens?\s*exceed",
        r"(?i)\btoo\s*many\s*tokens?\b",
    ];
    
    if status != "400" {
        return false;
    }
    
    for pattern in CONTEXT_ERROR_PATTERNS {
        if regex::Regex::new(pattern).unwrap().is_match(&message) {
            return true;
        }
    }
    
    false
}

/// Checks for OpenAI-style context window errors
/// Port of checkIsOpenAIContextWindowError() from context-error-handling.ts lines 38-56
///
/// Docs: https://platform.openai.com/docs/guides/error-codes/api-errors
fn check_is_openai_context_window_error(error: &Value) -> bool {
    // Check for LengthFinishReasonError by name field
    if let Some(name) = error.get("name").and_then(|v| v.as_str()) {
        if name == "LengthFinishReasonError" {
            return true;
        }
    }
    
    // Check for APIError with code 400 and token/context length message
    const KNOWN_CONTEXT_ERROR_SUBSTRINGS: &[&str] = &["token", "context length"];
    
    if let Some(code) = error.get("code") {
        let code_str = code.to_string();
        if code_str == "400" || code_str == "\"400\"" {
            if let Some(message) = error.get("message").and_then(|v| v.as_str()) {
                for substring in KNOWN_CONTEXT_ERROR_SUBSTRINGS {
                    if message.contains(substring) {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Checks for Anthropic-style context window errors
/// Port of checkIsAnthropicContextWindowError() from context-error-handling.ts lines 58-96
fn check_is_anthropic_context_window_error(error: &Value) -> bool {
    if !error.is_object() {
        return false;
    }
    
    // Check for Anthropic-specific error structure
    let error_type = error
        .get("error")
        .and_then(|e| e.get("error"))
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str());
    
    if error_type != Some("invalid_request_error") {
        return false;
    }
    
    let message = error
        .get("error")
        .and_then(|e| e.get("error"))
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    
    // More specific patterns for context window errors
    const CONTEXT_WINDOW_PATTERNS: &[&str] = &[
        r"(?i)prompt is too long",
        r"(?i)maximum.*tokens",
        r"(?i)context.*too.*long",
        r"(?i)exceeds.*context",
        r"(?i)token.*limit",
        r"(?i)context_length_exceeded",
        r"(?i)max_tokens_to_sample",
    ];
    
    // Additional check for Anthropic-specific error codes
    let error_code = error
        .get("error")
        .and_then(|e| e.get("error"))
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str());
    
    if error_code == Some("context_length_exceeded") 
        || error_code == Some("invalid_request_error") 
    {
        for pattern in CONTEXT_WINDOW_PATTERNS {
            if regex::Regex::new(pattern).unwrap().is_match(message) {
                return true;
            }
        }
    }
    
    // Check patterns even without specific error code
    for pattern in CONTEXT_WINDOW_PATTERNS {
        if regex::Regex::new(pattern).unwrap().is_match(message) {
            return true;
        }
    }
    
    false
}

/// Checks for Cerebras-style context window errors
/// Port of checkIsCerebrasContextWindowError() from context-error-handling.ts lines 98-114
fn check_is_cerebras_context_window_error(error: &Value) -> bool {
    if !error.is_object() {
        return false;
    }
    
    let obj = error.as_object().unwrap();
    let status = extract_status(obj);
    let message = extract_message(obj);
    
    status == "400" 
        && message.contains("Please reduce the length of the messages or completion")
}

// Helper functions

fn extract_status(obj: &serde_json::Map<String, Value>) -> String {
    let v = obj
        .get("status")
        .or_else(|| obj.get("code"))
        .or_else(|| obj.get("error").and_then(|e| e.get("status")))
        .or_else(|| obj.get("response").and_then(|r| r.get("status")));

    if let Some(v) = v {
        if let Some(s) = v.as_str() {
            return s.to_string();
        }
        if let Some(i) = v.as_i64() {
            return i.to_string();
        }
    }

    String::new()
}

fn extract_message(obj: &serde_json::Map<String, Value>) -> String {
    obj.get("message")
        .or_else(|| obj.get("error").and_then(|e| e.get("message")))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_openai_length_finish_reason_error() {
        let error = json!({
            "name": "LengthFinishReasonError",
            "message": "Token limit exceeded"
        });
        
        assert!(check_context_window_exceeded_error(&error));
    }
    
    #[test]
    fn test_openai_api_error_400() {
        let error = json!({
            "code": "400",
            "message": "This model's maximum context length is 8192 tokens"
        });
        
        assert!(check_context_window_exceeded_error(&error));
    }
    
    #[test]
    fn test_openrouter_context_length() {
        let error = json!({
            "status": "400",
            "message": "Request exceeds context length limit"
        });
        
        assert!(check_context_window_exceeded_error(&error));
    }
    
    #[test]
    fn test_anthropic_prompt_too_long() {
        let error = json!({
            "error": {
                "error": {
                    "type": "invalid_request_error",
                    "message": "prompt is too long: 150000 tokens > 100000 maximum"
                }
            }
        });
        
        assert!(check_context_window_exceeded_error(&error));
    }
    
    #[test]
    fn test_cerebras_specific_message() {
        let error = json!({
            "status": "400",
            "message": "Please reduce the length of the messages or completion"
        });
        
        assert!(check_context_window_exceeded_error(&error));
    }
    
    #[test]
    fn test_non_context_error() {
        let error = json!({
            "status": "500",
            "message": "Internal server error"
        });
        
        assert!(!check_context_window_exceeded_error(&error));
    }
}
