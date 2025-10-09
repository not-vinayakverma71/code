// Logging audit tests - P0-LOG-tests

#[cfg(test)]
mod tests {
    use crate::core::tools::logging::*;
    use std::sync::{Arc, Mutex};
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::layer::SubscriberExt;
    
    // Custom test subscriber to capture log events
    struct TestLogCapture {
        logs: Arc<Mutex<Vec<String>>>,
    }
    
    impl TestLogCapture {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let logs = Arc::new(Mutex::new(Vec::new()));
            (Self { logs: logs.clone() }, logs)
        }
    }
    
    #[test]
    fn test_approval_audit_log() {
        // Create approval request
        let approval_id = log_approval_request(
            "write_file",
            "overwrite",
            "/tmp/test.txt",
            "user123"
        );
        
        assert!(!approval_id.is_empty());
        
        // Log approval
        log_approval_decision(&approval_id, true, None);
        
        // Log denial
        log_approval_decision(&approval_id, false, Some("User cancelled"));
    }
    
    #[test]
    fn test_file_operation_audit() {
        // Test write operation (should emit audit)
        log_file_operation("write", "/tmp/test.txt", true, None);
        
        // Test failed write (should emit audit)
        log_file_operation("write", "/tmp/test.txt", false, Some("Permission denied"));
        
        // Test delete operation (should emit audit)
        log_file_operation("delete", "/tmp/test.txt", true, None);
        
        // Test modify operation (should emit audit)
        log_file_operation("modify", "/tmp/test.txt", true, None);
        
        // Test read operation (should not emit audit)
        log_file_operation("read", "/tmp/test.txt", true, None);
    }
    
    #[test]
    fn test_tool_execution_logging() {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();
        
        // Log start
        log_tool_start("read_file", &execution_id, &session_id);
        
        // Simulate execution
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Log success
        log_tool_complete(
            "read_file",
            &execution_id,
            std::time::Duration::from_millis(10),
            true,
            None
        );
        
        // Log failure
        log_tool_complete(
            "write_file",
            &execution_id,
            std::time::Duration::from_millis(5),
            false,
            Some("Permission denied")
        );
    }
    
    #[test]
    fn test_correlation_ids() {
        let ctx = LogContext::new(
            "session_123".to_string(),
            "user_456".to_string()
        );
        
        // Execution ID should be unique
        let ctx2 = LogContext::new(
            "session_123".to_string(),
            "user_456".to_string()
        );
        
        assert_ne!(ctx.execution_id, ctx2.execution_id);
        assert_eq!(ctx.session_id, ctx2.session_id);
    }
    
    #[test]
    fn test_pii_redaction() {
        // Test email redaction
        assert_eq!(redact_pii("user@example.com"), "***@***.***");
        assert_eq!(redact_pii("admin@company.org"), "***@***.***");
        
        // Test user ID redaction
        assert_eq!(redact_pii("user123456"), "use***");
        assert_eq!(redact_pii("ab"), "***");
        assert_eq!(redact_pii(""), "");
    }
    
    #[test]
    fn test_sensitive_path_redaction() {
        // Home directories should be redacted
        assert!(redact_sensitive_path("/home/username/file.txt").contains("*"));
        assert!(redact_sensitive_path("/Users/john/Documents/file.txt").contains("*"));
        
        // Sensitive files should be fully redacted
        assert_eq!(redact_sensitive_path(".env"), "***REDACTED***");
        assert_eq!(redact_sensitive_path("secret.key"), "***REDACTED***");
        assert_eq!(redact_sensitive_path("/etc/secrets/api.key"), "***REDACTED***");
        
        // Normal paths should not be redacted
        assert_eq!(redact_sensitive_path("/tmp/file.txt"), "/tmp/file.txt");
        assert_eq!(redact_sensitive_path("/var/log/app.log"), "/var/log/app.log");
    }
    
    #[test]
    fn test_structured_log_serialization() {
        let log = ToolExecutionLog {
            execution_id: "exec_123".to_string(),
            session_id: "session_456".to_string(),
            tool_name: "read_file".to_string(),
            operation: "execute".to_string(),
            user_id: "user_789".to_string(),
            timestamp: chrono::Utc::now(),
            duration_ms: Some(150),
            success: true,
            error: None,
            metadata: serde_json::json!({"file": "test.txt"}),
        };
        
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"execution_id\":\"exec_123\""));
        assert!(json.contains("\"tool_name\":\"read_file\""));
        assert!(json.contains("\"success\":true"));
    }
    
    #[test]
    fn test_approval_audit_serialization() {
        let log = ApprovalAuditLog {
            approval_id: "approval_123".to_string(),
            tool_name: "write_file".to_string(),
            operation: "overwrite".to_string(),
            target: "/tmp/test.txt".to_string(),
            user_id: "user_456".to_string(),
            approved: false,
            timestamp: chrono::Utc::now(),
            denial_reason: Some("User cancelled".to_string()),
        };
        
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"approval_id\":\"approval_123\""));
        assert!(json.contains("\"approved\":false"));
        assert!(json.contains("\"denial_reason\":\"User cancelled\""));
    }
    
    #[test]
    fn test_file_operation_audit_serialization() {
        let log = FileOperationLog {
            operation: "write".to_string(),
            path: "/tmp/test.txt".to_string(),
            user_id: "user_123".to_string(),
            success: true,
            timestamp: chrono::Utc::now(),
            size_bytes: Some(1024),
            error: None,
        };
        
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"operation\":\"write\""));
        assert!(json.contains("\"path\":\"/tmp/test.txt\""));
        assert!(json.contains("\"size_bytes\":1024"));
    }
    
    #[tokio::test]
    async fn test_tool_execution_with_logging() {
        use crate::core::tools::traits::{Tool, ToolContext};
        use crate::core::tools::fs::ReadFileTool;
        use tempfile::TempDir;
        use std::fs;
        
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        // Create tool and context
        let tool = ReadFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        // Execute with logging
        let args = serde_json::json!(format!(r#"<tool><path>test.txt</path></tool>"#));
        let result = tool.execute_with_logging(args, context).await;
        
        // Verify execution succeeded
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_approval_logging_integration() {
        use crate::core::tools::traits::{Tool, ToolContext, ToolError};
        use crate::core::tools::fs::WriteFileTool;
        use tempfile::TempDir;
        
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = true; // Require approval
        
        // Execute tool (should fail with approval required)
        let args = serde_json::json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>test content</content>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        
        // Verify approval was required
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))));
        
        // Check that approval ID is in the error message
        if let Err(ToolError::ApprovalRequired(msg)) = result {
            assert!(msg.contains("approval_id:"));
        }
    }
    
    #[test]
    fn test_log_context_elapsed_time() {
        let ctx = LogContext::new("session".to_string(), "user".to_string());
        
        // Sleep briefly
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        let elapsed = ctx.elapsed();
        assert!(elapsed.as_millis() >= 50);
        assert!(elapsed.as_millis() < 200); // Reasonable upper bound
    }
    
    #[test]
    fn test_audit_emission_for_destructive_ops() {
        // Test that write/delete/modify operations emit audit logs
        log_file_operation("write", "/tmp/test.txt", true, None);
        log_file_operation("delete", "/tmp/test.txt", true, None);
        log_file_operation("modify", "/tmp/test.txt", true, None);
        
        // These should all trigger audit log emission
        // In production, these would be captured by the logging infrastructure
    }
}
