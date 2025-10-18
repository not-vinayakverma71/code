//! Error types for prompt system
//!
//! Translation from CHUNK-01 Part 5: Error Recovery

use thiserror::Error;

/// Prompt generation errors
///
/// Reference: CHUNK-01-PROMPTS-SYSTEM.md Part 5 (lines 629-658)
#[derive(Error, Debug)]
pub enum PromptError {
    /// Mode not found by slug
    #[error("Mode not found: {0}")]
    ModeNotFound(String),
    
    /// Failed to load custom rules or instructions
    #[error("Rule load error: {0}")]
    RuleLoadError(#[from] std::io::Error),
    
    /// Generated prompt exceeds maximum size
    #[error("Prompt too large: {actual} > {max} characters")]
    PromptTooLarge {
        actual: usize,
        max: usize,
    },
    
    /// File is outside workspace boundary
    #[error("File outside workspace: {0}")]
    OutsideWorkspace(String),
    
    /// Invalid regex pattern
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),
    
    /// Symlink cycle detected
    #[error("Symlink cycle detected at: {0}")]
    SymlinkCycle(String),
    
    /// Binary file encountered where text expected
    #[error("Binary file not supported: {0}")]
    BinaryFile(String),
    
    /// Tokenizer error
    #[error("Tokenizer error: {0}")]
    TokenizerError(String),
    
    /// Generic error with context
    #[error("Prompt generation failed: {0}")]
    GenerationFailed(String),
}

/// Result type for prompt operations
pub type PromptResult<T> = Result<T, PromptError>;

/// Error codes for integration with error_recovery_v2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptErrorCode {
    /// E_PROMPT_001 - Mode not found
    ModeNotFound,
    
    /// E_PROMPT_002 - Rule load failure
    RuleLoadFailed,
    
    /// E_PROMPT_003 - Prompt too large
    PromptTooLarge,
    
    /// E_PROMPT_004 - Workspace boundary violation
    WorkspaceBoundary,
    
    /// E_PROMPT_005 - Invalid regex
    InvalidRegex,
    
    /// E_PROMPT_006 - Symlink cycle
    SymlinkCycle,
    
    /// E_PROMPT_007 - Binary file
    BinaryFile,
    
    /// E_PROMPT_008 - Tokenizer error
    TokenizerError,
    
    /// E_PROMPT_999 - Generic error
    GenerationFailed,
}

impl PromptErrorCode {
    /// Get error code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::ModeNotFound => "E_PROMPT_001",
            Self::RuleLoadFailed => "E_PROMPT_002",
            Self::PromptTooLarge => "E_PROMPT_003",
            Self::WorkspaceBoundary => "E_PROMPT_004",
            Self::InvalidRegex => "E_PROMPT_005",
            Self::SymlinkCycle => "E_PROMPT_006",
            Self::BinaryFile => "E_PROMPT_007",
            Self::TokenizerError => "E_PROMPT_008",
            Self::GenerationFailed => "E_PROMPT_999",
        }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::RuleLoadFailed | Self::PromptTooLarge
        )
    }
}

impl From<&PromptError> for PromptErrorCode {
    fn from(error: &PromptError) -> Self {
        match error {
            PromptError::ModeNotFound(_) => PromptErrorCode::ModeNotFound,
            PromptError::RuleLoadError(_) => PromptErrorCode::RuleLoadFailed,
            PromptError::PromptTooLarge { .. } => PromptErrorCode::PromptTooLarge,
            PromptError::OutsideWorkspace(_) => PromptErrorCode::WorkspaceBoundary,
            PromptError::InvalidRegex(_) => PromptErrorCode::InvalidRegex,
            PromptError::SymlinkCycle(_) => PromptErrorCode::SymlinkCycle,
            PromptError::BinaryFile(_) => PromptErrorCode::BinaryFile,
            PromptError::TokenizerError(_) => PromptErrorCode::TokenizerError,
            PromptError::GenerationFailed(_) => PromptErrorCode::GenerationFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_codes() {
        let err = PromptError::ModeNotFound("test".to_string());
        let code = PromptErrorCode::from(&err);
        assert_eq!(code.code(), "E_PROMPT_001");
        assert!(!code.is_recoverable());
    }
    
    #[test]
    fn test_recoverable_errors() {
        let err = PromptError::RuleLoadError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test",
        ));
        let code = PromptErrorCode::from(&err);
        assert!(code.is_recoverable());
        
        let err2 = PromptError::PromptTooLarge { actual: 100, max: 50 };
        let code2 = PromptErrorCode::from(&err2);
        assert!(code2.is_recoverable());
    }
}
