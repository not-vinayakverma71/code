// Error shape mapping for UI integration
// Maps ToolError variants to standardized error codes and UI-friendly messages

use super::traits::ToolError;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Standardized error code for UI consumption
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolErrorCode {
    // 1xxx: Not found / discovery errors
    NotFound = 1000,
    
    // 2xxx: Input validation errors
    InvalidArguments = 2000,
    InvalidInput = 2001,
    
    // 3xxx: Security and permission errors
    PermissionDenied = 3000,
    SecurityViolation = 3001,
    RooIgnoreBlocked = 3002,
    
    // 4xxx: Approval required
    ApprovalRequired = 4000,
    
    // 5xxx: Execution errors
    ExecutionFailed = 5000,
    Timeout = 5001,
    IoError = 5002,
    
    // 9xxx: Unknown/other
    Unknown = 9000,
}

/// UI-friendly error envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolErrorEnvelope {
    /// Error code for programmatic handling
    pub code: ToolErrorCode,
    
    /// Human-readable error message
    pub message: String,
    
    /// Detailed technical description (optional)
    pub details: Option<String>,
    
    /// Whether the error is recoverable
    pub recoverable: bool,
    
    /// Suggested actions for recovery
    pub suggestions: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolErrorEnvelope {
    /// Convert ToolError to UI-friendly envelope
    pub fn from_tool_error(error: &ToolError) -> Self {
        match error {
            ToolError::NotFound(msg) => Self {
                code: ToolErrorCode::NotFound,
                message: format!("Tool not found: {}", msg),
                details: Some(msg.clone()),
                recoverable: false,
                suggestions: vec![
                    "Check that the tool name is correct".to_string(),
                    "Verify the tool is registered in the registry".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::InvalidArguments(msg) | ToolError::InvalidArgs(msg) => Self {
                code: ToolErrorCode::InvalidArguments,
                message: format!("Invalid arguments: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "Check the argument format and types".to_string(),
                    "Refer to the tool documentation for correct usage".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::InvalidInput(msg) => Self {
                code: ToolErrorCode::InvalidInput,
                message: format!("Invalid input: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "Verify the input data is valid".to_string(),
                    "Check for encoding or format issues".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::PermissionDenied(msg) => Self {
                code: ToolErrorCode::PermissionDenied,
                message: format!("Permission denied: {}", msg),
                details: Some(msg.clone()),
                recoverable: false,
                suggestions: vec![
                    "Check file/directory permissions".to_string(),
                    "Ensure the workspace has appropriate access rights".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::SecurityViolation(msg) => Self {
                code: ToolErrorCode::SecurityViolation,
                message: format!("Security violation: {}", msg),
                details: Some(msg.clone()),
                recoverable: false,
                suggestions: vec![
                    "The operation was blocked for security reasons".to_string(),
                    "Review the security policies and try again".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::RooIgnoreBlocked(msg) => Self {
                code: ToolErrorCode::RooIgnoreBlocked,
                message: format!("Path blocked by .rooignore: {}", msg),
                details: Some(msg.clone()),
                recoverable: false,
                suggestions: vec![
                    "The path is protected by .rooignore rules".to_string(),
                    "Update .rooignore to allow access if needed".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::ApprovalRequired(msg) => Self {
                code: ToolErrorCode::ApprovalRequired,
                message: format!("Approval required: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "This operation requires explicit approval".to_string(),
                    "Review the operation details and approve if safe".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::ExecutionFailed(msg) => Self {
                code: ToolErrorCode::ExecutionFailed,
                message: format!("Execution failed: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "Check the error details for the cause".to_string(),
                    "Retry the operation if the issue is transient".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::Timeout(msg) => Self {
                code: ToolErrorCode::Timeout,
                message: format!("Operation timeout: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "The operation took too long to complete".to_string(),
                    "Try again or increase timeout settings".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::Io(err) => Self {
                code: ToolErrorCode::IoError,
                message: format!("I/O error: {}", err),
                details: Some(format!("{:?}", err)),
                recoverable: true,
                suggestions: vec![
                    "Check file system access and permissions".to_string(),
                    "Ensure sufficient disk space is available".to_string(),
                ],
                metadata: HashMap::new(),
            },
            
            ToolError::Other(msg) => Self {
                code: ToolErrorCode::Unknown,
                message: format!("Error: {}", msg),
                details: Some(msg.clone()),
                recoverable: true,
                suggestions: vec![
                    "An unexpected error occurred".to_string(),
                    "Check logs for more details".to_string(),
                ],
                metadata: HashMap::new(),
            },
        }
    }
    
    /// Convert to JSON for IPC transmission
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({
            "code": "UNKNOWN",
            "message": "Failed to serialize error",
            "recoverable": false,
            "suggestions": []
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_not_found_mapping() {
        let error = ToolError::NotFound("test_tool".to_string());
        let envelope = ToolErrorEnvelope::from_tool_error(&error);
        
        assert_eq!(envelope.code, ToolErrorCode::NotFound);
        assert!(envelope.message.contains("test_tool"));
        assert!(!envelope.recoverable);
    }
    
    #[test]
    fn test_approval_required_mapping() {
        let error = ToolError::ApprovalRequired("Delete file".to_string());
        let envelope = ToolErrorEnvelope::from_tool_error(&error);
        
        assert_eq!(envelope.code, ToolErrorCode::ApprovalRequired);
        assert!(envelope.recoverable);
        assert!(!envelope.suggestions.is_empty());
    }
    
    #[test]
    fn test_security_violation_mapping() {
        let error = ToolError::SecurityViolation("Path traversal attempt".to_string());
        let envelope = ToolErrorEnvelope::from_tool_error(&error);
        
        assert_eq!(envelope.code, ToolErrorCode::SecurityViolation);
        assert!(!envelope.recoverable);
    }
    
    #[test]
    fn test_json_serialization() {
        let error = ToolError::InvalidArguments("missing field".to_string());
        let envelope = ToolErrorEnvelope::from_tool_error(&error);
        let json = envelope.to_json();
        
        assert!(json["code"].is_string());
        assert!(json["message"].is_string());
        assert!(json["recoverable"].is_boolean());
    }
}
