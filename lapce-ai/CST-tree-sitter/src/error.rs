//! Comprehensive error handling for CST-tree-sitter
//! Production-grade error types with context and recovery strategies

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreeSitterError {
    #[error("Failed to parse file: {path}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Unsupported language: {language}")]
    UnsupportedLanguage { language: String },

    #[error("Failed to load language parser: {language}")]
    LanguageLoadFailed {
        language: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Query compilation failed for {language}")]
    QueryCompilationFailed {
        language: String,
        query_type: QueryType,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("File I/O error: {path}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Operation timed out after {timeout_ms}ms: {operation}")]
    Timeout {
        operation: String,
        timeout_ms: u64,
    },

    #[error("Memory limit exceeded: {current_mb}MB (limit: {limit_mb}MB)")]
    MemoryLimitExceeded {
        current_mb: usize,
        limit_mb: usize,
    },

    #[error("File too large: {size_mb}MB (limit: {limit_mb}MB)")]
    FileTooLarge {
        path: PathBuf,
        size_mb: usize,
        limit_mb: usize,
    },

    #[error("Invalid UTF-8 in file: {path}")]
    InvalidUtf8 {
        path: PathBuf,
        #[source]
        source: std::str::Utf8Error,
    },

    #[error("Cache error: {message}")]
    CacheError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Query execution failed: {message}")]
    QueryExecutionFailed { message: String },

    #[error("Node not found at position: line {line}, column {column}")]
    NodeNotFound { line: usize, column: usize },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Tags,
    Highlights,
    Locals,
    Injections,
    Folds,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Tags => write!(f, "tags"),
            QueryType::Highlights => write!(f, "highlights"),
            QueryType::Locals => write!(f, "locals"),
            QueryType::Injections => write!(f, "injections"),
            QueryType::Folds => write!(f, "folds"),
        }
    }
}

pub type Result<T> = std::result::Result<T, TreeSitterError>;

/// Error recovery context
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub file_path: Option<PathBuf>,
    pub language: Option<String>,
    pub retry_count: usize,
    pub recoverable: bool,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            file_path: None,
            language: None,
            retry_count: 0,
            recoverable: false,
        }
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_language(mut self, lang: String) -> Self {
        self.language = Some(lang);
        self
    }

    pub fn mark_recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Error recovery strategy - PRODUCTION GRADE: NEVER SKIP FILES
pub enum RecoveryStrategy {
    /// Retry aggressively (up to 10 attempts with backoff)
    Retry { max_attempts: usize, backoff_ms: u64 },
    /// Use fallback parsing (simplified, best-effort)
    Fallback { alternative: String },
    /// Abort ONLY for catastrophic failures (corrupted library)
    /// NEVER abort for individual file failures
    Abort,
}

impl TreeSitterError {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            TreeSitterError::Timeout { .. }
                | TreeSitterError::CacheError { .. }
                | TreeSitterError::ResourceExhausted { .. }
        )
    }

    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            // Timeouts: retry aggressively with longer backoff
            TreeSitterError::Timeout { .. } => RecoveryStrategy::Retry {
                max_attempts: 10,
                backoff_ms: 500,
            },
            // Unsupported language: fallback to basic tokenization
            TreeSitterError::UnsupportedLanguage { .. } => RecoveryStrategy::Fallback {
                alternative: "basic_tokenizer".to_string(),
            },
            // Large files: retry with streaming parser
            TreeSitterError::FileTooLarge { .. } => RecoveryStrategy::Retry {
                max_attempts: 5,
                backoff_ms: 1000,
            },
            // Memory issues: retry with garbage collection
            TreeSitterError::MemoryLimitExceeded { .. } => RecoveryStrategy::Retry {
                max_attempts: 3,
                backoff_ms: 2000,
            },
            // Resource exhaustion: retry with backoff
            TreeSitterError::ResourceExhausted { .. } => RecoveryStrategy::Retry {
                max_attempts: 5,
                backoff_ms: 1000,
            },
            // Parse failures: fallback to simpler parsing
            TreeSitterError::ParseFailed { .. } => RecoveryStrategy::Fallback {
                alternative: "best_effort_parse".to_string(),
            },
            // Query errors: fallback to basic extraction
            TreeSitterError::QueryCompilationFailed { .. } | TreeSitterError::QueryExecutionFailed { .. } => {
                RecoveryStrategy::Fallback {
                    alternative: "regex_extraction".to_string(),
                }
            },
            // Only abort for catastrophic library corruption
            _ => RecoveryStrategy::Retry {
                max_attempts: 3,
                backoff_ms: 500,
            },
        }
    }

    pub fn log_context(&self, context: &ErrorContext) {
        tracing::error!(
            error = %self,
            operation = %context.operation,
            file = ?context.file_path,
            language = ?context.language,
            retry_count = context.retry_count,
            recoverable = context.recoverable,
            "Tree-sitter operation failed"
        );
    }
}

/// Helper trait for adding context to errors
pub trait ErrorExt<T> {
    fn with_context(self, context: ErrorContext) -> Result<T>;
}

impl<T> ErrorExt<T> for Result<T> {
    fn with_context(self, context: ErrorContext) -> Result<T> {
        self.map_err(|e| {
            e.log_context(&context);
            e
        })
    }
}
