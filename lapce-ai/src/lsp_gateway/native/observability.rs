/// Observability (LSP-019)
/// Structured tracing with spans, correlation IDs, error taxonomy

use std::sync::Arc;
use uuid::Uuid;

/// Correlation ID for request tracing
#[derive(Debug, Clone)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Error code taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Client errors (4xx equivalent)
    InvalidRequest = 4000,
    InvalidParams = 4001,
    MethodNotFound = 4002,
    PayloadTooLarge = 4003,
    RateLimitExceeded = 4004,
    Unauthorized = 4005,
    
    // Server errors (5xx equivalent)
    InternalError = 5000,
    ParseError = 5001,
    SymbolNotFound = 5002,
    DocumentNotOpen = 5003,
    TimeoutError = 5004,
    ConcurrencyError = 5005,
    
    // Service unavailable
    ServiceOverloaded = 5030,
    ServiceShuttingDown = 5031,
}

impl ErrorCode {
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
    
    pub fn category(&self) -> &'static str {
        match self {
            ErrorCode::InvalidRequest
            | ErrorCode::InvalidParams
            | ErrorCode::MethodNotFound
            | ErrorCode::PayloadTooLarge
            | ErrorCode::RateLimitExceeded
            | ErrorCode::Unauthorized => "client_error",
            
            ErrorCode::InternalError
            | ErrorCode::ParseError
            | ErrorCode::SymbolNotFound
            | ErrorCode::DocumentNotOpen
            | ErrorCode::TimeoutError
            | ErrorCode::ConcurrencyError => "server_error",
            
            ErrorCode::ServiceOverloaded
            | ErrorCode::ServiceShuttingDown => "service_unavailable",
        }
    }
    
    pub fn message(&self) -> &'static str {
        match self {
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::InvalidParams => "Invalid parameters",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::PayloadTooLarge => "Payload too large",
            ErrorCode::RateLimitExceeded => "Rate limit exceeded",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::ParseError => "Parse error",
            ErrorCode::SymbolNotFound => "Symbol not found",
            ErrorCode::DocumentNotOpen => "Document not open",
            ErrorCode::TimeoutError => "Timeout",
            ErrorCode::ConcurrencyError => "Concurrency error",
            ErrorCode::ServiceOverloaded => "Service overloaded",
            ErrorCode::ServiceShuttingDown => "Service shutting down",
        }
    }
}

/// Structured error with taxonomy
#[derive(Debug)]
pub struct LspError {
    pub code: ErrorCode,
    pub message: String,
    pub correlation_id: Option<CorrelationId>,
    pub method: Option<String>,
    pub uri: Option<String>,
}

impl LspError {
    pub fn new(code: ErrorCode, message: String) -> Self {
        Self {
            code,
            message,
            correlation_id: None,
            method: None,
            uri: None,
        }
    }
    
    pub fn with_correlation_id(mut self, id: CorrelationId) -> Self {
        self.correlation_id = Some(id);
        self
    }
    
    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
    
    pub fn with_uri(mut self, uri: String) -> Self {
        self.uri = Some(uri);
        self
    }
    
    /// Log structured error
    pub fn log(&self) {
        tracing::error!(
            code = self.code.as_u16(),
            category = self.code.category(),
            message = %self.message,
            correlation_id = self.correlation_id.as_ref().map(|id| id.as_str()),
            method = self.method.as_deref(),
            uri = self.uri.as_deref(),
            "LSP error"
        );
    }
}

impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code.as_u16(), self.message)
    }
}

impl std::error::Error for LspError {}

/// Request context with tracing span
pub struct RequestContext {
    pub correlation_id: CorrelationId,
    pub method: String,
    pub uri: Option<String>,
    pub language_id: Option<String>,
    pub start_time: std::time::Instant,
}

impl RequestContext {
    pub fn new(method: String) -> Self {
        let correlation_id = CorrelationId::new();
        
        tracing::info!(
            correlation_id = %correlation_id,
            method = %method,
            "LSP request started"
        );
        
        Self {
            correlation_id,
            method,
            uri: None,
            language_id: None,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn with_uri(mut self, uri: String) -> Self {
        self.uri = Some(uri);
        self
    }
    
    pub fn with_language_id(mut self, language_id: String) -> Self {
        self.language_id = Some(language_id);
        self
    }
    
    /// Log successful completion
    pub fn log_success(&self, result_size: usize) {
        let duration_ms = self.start_time.elapsed().as_millis();
        
        tracing::info!(
            correlation_id = %self.correlation_id,
            method = %self.method,
            uri = self.uri.as_deref(),
            language_id = self.language_id.as_deref(),
            duration_ms = duration_ms,
            result_size = result_size,
            status = "success",
            "LSP request completed"
        );
    }
    
    /// Log error
    pub fn log_error(&self, error: &LspError) {
        let duration_ms = self.start_time.elapsed().as_millis();
        
        tracing::error!(
            correlation_id = %self.correlation_id,
            method = %self.method,
            uri = self.uri.as_deref(),
            language_id = self.language_id.as_deref(),
            duration_ms = duration_ms,
            error_code = error.code.as_u16(),
            error_category = error.code.category(),
            error_message = %error.message,
            status = "error",
            "LSP request failed"
        );
    }
    
    /// Create tracing span for this request
    pub fn span(&self) -> tracing::Span {
        tracing::info_span!(
            "lsp_request",
            correlation_id = %self.correlation_id,
            method = %self.method,
            uri = self.uri.as_deref(),
            language_id = self.language_id.as_deref(),
        )
    }
}

/// Observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Enable structured logging
    pub enable_structured_logging: bool,
    /// Enable correlation IDs
    pub enable_correlation_ids: bool,
    /// Log request payloads (PII-redacted)
    pub log_payloads: bool,
    /// Log response sizes
    pub log_response_sizes: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enable_structured_logging: true,
            enable_correlation_ids: true,
            log_payloads: false, // Off by default for performance
            log_response_sizes: true,
        }
    }
}

/// Initialize tracing subscriber
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .compact() // Compact format for structured logging
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_correlation_id() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        
        assert_ne!(id1.as_str(), id2.as_str());
        assert!(!id1.as_str().is_empty());
    }
    
    #[test]
    fn test_error_code_taxonomy() {
        assert_eq!(ErrorCode::InvalidRequest.category(), "client_error");
        assert_eq!(ErrorCode::InternalError.category(), "server_error");
        assert_eq!(ErrorCode::ServiceOverloaded.category(), "service_unavailable");
        
        assert_eq!(ErrorCode::InvalidRequest.as_u16(), 4000);
        assert_eq!(ErrorCode::InternalError.as_u16(), 5000);
    }
    
    #[test]
    fn test_lsp_error() {
        let error = LspError::new(
            ErrorCode::ParseError,
            "Failed to parse document".to_string(),
        )
        .with_correlation_id(CorrelationId::from_string("test-123".to_string()))
        .with_method("textDocument/definition".to_string())
        .with_uri("file:///test.rs".to_string());
        
        assert_eq!(error.code, ErrorCode::ParseError);
        assert_eq!(error.message, "Failed to parse document");
        assert_eq!(error.correlation_id.as_ref().unwrap().as_str(), "test-123");
        assert_eq!(error.method.as_deref(), Some("textDocument/definition"));
        assert_eq!(error.uri.as_deref(), Some("file:///test.rs"));
    }
    
    #[test]
    fn test_request_context() {
        let ctx = RequestContext::new("textDocument/hover".to_string())
            .with_uri("file:///test.rs".to_string())
            .with_language_id("rust".to_string());
        
        assert_eq!(ctx.method, "textDocument/hover");
        assert_eq!(ctx.uri.as_deref(), Some("file:///test.rs"));
        assert_eq!(ctx.language_id.as_deref(), Some("rust"));
        assert!(!ctx.correlation_id.as_str().is_empty());
    }
}
