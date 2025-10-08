// Health server metrics tests - P0-OPS-tests

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_tool_metrics_recording() {
        let metrics = ToolMetrics::new();
        
        // Record successful execution
        metrics.record_execution("read_file", Duration::from_millis(50), true).await;
        
        assert_eq!(metrics.tool_runs.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.tool_failures.load(Ordering::Relaxed), 0);
        
        // Record failed execution
        metrics.record_execution("write_file", Duration::from_millis(100), false).await;
        
        assert_eq!(metrics.tool_runs.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.tool_failures.load(Ordering::Relaxed), 1);
        
        // Check per-tool metrics
        let tool_specific = metrics.tool_specific.read().await;
        assert_eq!(tool_specific.get("read_file").unwrap().invocations, 1);
        assert_eq!(tool_specific.get("read_file").unwrap().failures, 0);
        assert_eq!(tool_specific.get("write_file").unwrap().invocations, 1);
        assert_eq!(tool_specific.get("write_file").unwrap().failures, 1);
    }
    
    #[tokio::test]
    async fn test_approval_metrics() {
        let metrics = ToolMetrics::new();
        
        // Record approval request
        metrics.record_approval_request();
        assert_eq!(metrics.approvals_requested.load(Ordering::Relaxed), 1);
        
        // Record approval
        metrics.record_approval_result(true);
        assert_eq!(metrics.approvals_approved.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.approvals_denied.load(Ordering::Relaxed), 0);
        
        // Record denial
        metrics.record_approval_request();
        metrics.record_approval_result(false);
        assert_eq!(metrics.approvals_requested.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.approvals_approved.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.approvals_denied.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_duration_tracking() {
        let metrics = ToolMetrics::new();
        
        // Record multiple executions
        for i in 1..=5 {
            metrics.record_execution(
                "test_tool",
                Duration::from_millis(i * 10),
                true
            ).await;
        }
        
        // Check durations are recorded
        let durations = metrics.execution_durations.read().await;
        assert_eq!(durations.len(), 5);
        assert_eq!(durations[0], 10);
        assert_eq!(durations[4], 50);
        
        // Check average calculation
        let tool_specific = metrics.tool_specific.read().await;
        let test_tool_metrics = tool_specific.get("test_tool").unwrap();
        assert_eq!(test_tool_metrics.invocations, 5);
        assert_eq!(test_tool_metrics.total_duration_ms, 150); // 10+20+30+40+50
        assert_eq!(test_tool_metrics.avg_duration_ms, 30); // 150/5
    }
    
    #[tokio::test]
    async fn test_metrics_endpoint_format() {
        let config = HealthServerConfig::default();
        let ipc_stats = Arc::new(IpcServerStats::default());
        let circuit_breaker = Arc::new(CircuitBreaker::new(Default::default()));
        let health_server = Arc::new(HealthServer::new(config, ipc_stats, circuit_breaker));
        
        // Record some metrics
        health_server.tool_metrics.record_execution("test_tool", Duration::from_millis(100), true).await;
        health_server.tool_metrics.record_approval_request();
        health_server.tool_metrics.record_approval_result(true);
        
        // Get metrics response
        let response = health_server.handle_metrics().await.unwrap();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let metrics_text = String::from_utf8(body.to_vec()).unwrap();
        
        // Check Prometheus format
        assert!(metrics_text.contains("# HELP tool_runs"));
        assert!(metrics_text.contains("# TYPE tool_runs counter"));
        assert!(metrics_text.contains("tool_runs 1"));
        assert!(metrics_text.contains("approvals_requested 1"));
        assert!(metrics_text.contains("approvals_approved 1"));
    }
    
    #[tokio::test]
    async fn test_health_endpoint_with_metrics() {
        let config = HealthServerConfig::default();
        let ipc_stats = Arc::new(IpcServerStats::default());
        let circuit_breaker = Arc::new(CircuitBreaker::new(Default::default()));
        let health_server = Arc::new(HealthServer::new(config, ipc_stats, circuit_breaker));
        
        // Get health response
        let response = health_server.handle_health().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let health_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(health_json["status"], "healthy");
        assert!(health_json["checks"]["ipc_server"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_metrics_overflow_protection() {
        let metrics = ToolMetrics::new();
        
        // Record more than 1000 durations
        for i in 1..=1100 {
            metrics.record_execution(
                "overflow_test",
                Duration::from_millis(i),
                true
            ).await;
        }
        
        // Check that only last 1000 are kept
        let durations = metrics.execution_durations.read().await;
        assert_eq!(durations.len(), 1000);
        assert_eq!(durations[0], 101); // First 100 were removed
        assert_eq!(durations[999], 1100);
    }
}
