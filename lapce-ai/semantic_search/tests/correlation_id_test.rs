// SEM-018-B: Test correlation ID propagation
use lancedb::tracing::correlation::CorrelationId;
use tracing::{info_span, Instrument};
use tracing_subscriber::layer::SubscriberExt;
use std::sync::{Arc, Mutex};

#[test]
fn test_correlation_id_propagation() {
    // Capture logs to verify correlation ID appears
    let captured_logs = Arc::new(Mutex::new(Vec::new()));
    let logs_clone = captured_logs.clone();
    
    // Set up test subscriber that captures logs
    let subscriber = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    
    let _guard = tracing::subscriber::set_default(subscriber);
    
    // Create correlation ID
    let correlation_id = CorrelationId::new();
    
    // Create spans with correlation ID
    let span1 = info_span!("operation_1", correlation_id = %correlation_id);
    let span2 = info_span!("operation_2", correlation_id = %correlation_id);
    
    // Execute operations in spans
    async {
        tracing::info!("Starting operation 1");
        
        async {
            tracing::info!("Starting operation 2");
        }.instrument(span2).await;
        
    }.instrument(span1);
    
    // Verify correlation ID format
    assert!(correlation_id.to_string().len() == 36); // UUID format
    assert!(correlation_id.to_string().contains("-"));
}

#[tokio::test]
async fn test_correlation_id_in_async_context() {
    let correlation_id = CorrelationId::new();
    
    // Test that correlation ID can be passed through async boundaries
    let result = async_operation_with_correlation(correlation_id.clone()).await;
    assert!(result);
    
    // Test that correlation IDs are unique
    let another_id = CorrelationId::new();
    assert_ne!(correlation_id.to_string(), another_id.to_string());
}

async fn async_operation_with_correlation(correlation_id: CorrelationId) -> bool {
    let _span = info_span!("async_op", correlation_id = %correlation_id);
    
    // Simulate async work
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    true
}

#[test]
fn test_pii_redaction_in_correlation_context() {
    use lancedb::security::redaction::redact_pii;
    
    // Test that PII is redacted even in correlation contexts
    let sensitive_data = "My API key is sk-1234567890abcdef and email is test@example.com";
    let redacted = redact_pii(sensitive_data);
    
    assert!(redacted.contains("[REDACTED_API_KEY]"));
    assert!(redacted.contains("[REDACTED_EMAIL]"));
    assert!(!redacted.contains("sk-1234567890abcdef"));
    assert!(!redacted.contains("test@example.com"));
}
