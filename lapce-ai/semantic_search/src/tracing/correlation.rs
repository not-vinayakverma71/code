// Correlation ID Support - SEM-018-A
use uuid::Uuid;
use tracing::{instrument, Span};
use std::sync::Arc;

/// Correlation ID for distributed tracing
#[derive(Debug, Clone)]
pub struct CorrelationId(Arc<String>);

impl CorrelationId {
    /// Generate new correlation ID
    pub fn new() -> Self {
        Self(Arc::new(Uuid::new_v4().to_string()))
    }
    
    /// Create from existing ID
    pub fn from_string(id: String) -> Self {
        Self(Arc::new(id))
    }
    
    /// Get the ID as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Attach to current span
    pub fn attach_to_span(&self) {
        Span::current().record("correlation_id", &self.as_str());
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

/// Context with correlation ID
pub struct TracingContext {
    pub correlation_id: CorrelationId,
    pub user_id: Option<String>,
    pub request_path: Option<String>,
}

impl TracingContext {
    pub fn new() -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            user_id: None,
            request_path: None,
        }
    }
    
    pub fn with_correlation_id(mut self, id: CorrelationId) -> Self {
        self.correlation_id = id;
        self
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_correlation_id_generation() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1.as_str(), id2.as_str());
    }
    
    #[test]
    fn test_correlation_id_from_string() {
        let id_str = "test-correlation-id";
        let id = CorrelationId::from_string(id_str.to_string());
        assert_eq!(id.as_str(), id_str);
    }
    
    #[test]
    fn test_tracing_context() {
        let ctx = TracingContext::new()
            .with_user_id("user123".to_string());
        
        assert!(ctx.user_id.is_some());
        assert_eq!(ctx.user_id.unwrap(), "user123");
    }
}
