// Metrics Endpoint Test - IPC-029
// Verifies /metrics endpoint returns expected Prometheus counters and buckets

use lapce_ai_rust::ipc::IpcServer;
use std::sync::Arc;
use anyhow::Result;

#[tokio::test]
async fn test_metrics_endpoint_prometheus_format() -> Result<()> {
    // Create IPC server
    let server = Arc::new(IpcServer::new("/tmp/test_metrics.sock").await?);
    
    // Trigger some operations to generate metrics
    for _ in 0..10 {
        let _ = server.active_connection_count().await;
        let _ = server.connection_pool_stats().await;
    }
    
    // Get metrics in Prometheus format
    let metrics = server.metrics();
    let prometheus_text = metrics.export_prometheus();
    
    // Verify required metric lines are present
    let required_metrics = [
        "# HELP ipc_messages_total",
        "# TYPE ipc_messages_total counter",
        "ipc_messages_total",
        
        "# HELP ipc_message_duration_seconds",
        "# TYPE ipc_message_duration_seconds histogram",
        "ipc_message_duration_seconds_bucket",
        "ipc_message_duration_seconds_sum",
        "ipc_message_duration_seconds_count",
        
        "# HELP ipc_errors_total",
        "# TYPE ipc_errors_total counter",
        "ipc_errors_total",
        
        "# HELP ipc_active_connections",
        "# TYPE ipc_active_connections gauge",
        "ipc_active_connections",
        
        "# HELP ipc_buffer_utilization",
        "# TYPE ipc_buffer_utilization gauge",
        "ipc_buffer_utilization",
    ];
    
    for metric in required_metrics {
        assert!(
            prometheus_text.contains(metric),
            "Missing metric: {}",
            metric
        );
    }
    
    // Verify pool metrics
    let pool_metrics = server.export_pool_metrics().await;
    
    let pool_required = [
        "# HELP ipc_pool_total_connections",
        "# TYPE ipc_pool_total_connections gauge",
        "# HELP ipc_pool_active_connections",  
        "# TYPE ipc_pool_active_connections gauge",
        "# HELP ipc_pool_failed_connections_total",
        "# TYPE ipc_pool_failed_connections_total counter",
        "# HELP ipc_pool_healthy_connections",
        "# TYPE ipc_pool_healthy_connections gauge",
    ];
    
    for metric in pool_required {
        assert!(
            pool_metrics.contains(metric),
            "Missing pool metric: {}",
            metric
        );
    }
    
    // Verify format compliance
    verify_prometheus_format(&prometheus_text)?;
    verify_prometheus_format(&pool_metrics)?;
    
    println!("✓ Metrics endpoint test passed");
    println!("  - All required counters present");
    println!("  - All required histograms present");
    println!("  - Prometheus format valid");
    
    Ok(())
}

fn verify_prometheus_format(text: &str) -> Result<()> {
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue; // Comments and empty lines are valid
        }
        
        // Metric lines should have format: metric_name{labels} value timestamp
        // or: metric_name value timestamp
        // or: metric_name value
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        // First part should be metric name (optionally with labels)
        let metric_part = parts[0];
        assert!(
            metric_part.chars().any(|c| c.is_alphanumeric() || c == '_'),
            "Invalid metric name format: {}",
            metric_part
        );
        
        // Second part should be a numeric value
        if parts.len() >= 2 {
            let value_str = parts[1];
            // Allow +Inf, -Inf, NaN for special float values
            if !["Inf", "+Inf", "-Inf", "NaN"].contains(&value_str) {
                value_str.parse::<f64>()
                    .expect(&format!("Invalid metric value: {}", value_str));
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_metrics_buckets_and_quantiles() -> Result<()> {
    let server = Arc::new(IpcServer::new("/tmp/test_metrics_buckets.sock").await?);
    let metrics = server.metrics();
    
    // Simulate various latencies to populate histogram buckets
    use std::time::Duration;
    let latencies = vec![
        Duration::from_micros(5),    // <10µs
        Duration::from_micros(8),
        Duration::from_micros(15),   // 10-20µs  
        Duration::from_micros(25),   // 20-50µs
        Duration::from_micros(75),   // 50-100µs
        Duration::from_micros(150),  // 100-200µs
        Duration::from_micros(500),  // 200µs-1ms
        Duration::from_millis(2),    // 1-5ms
        Duration::from_millis(10),   // >5ms
    ];
    
    for latency in latencies {
        metrics.record_message_processing(latency);
    }
    
    let prometheus_text = metrics.export_prometheus();
    
    // Verify histogram buckets are present
    let expected_buckets = [
        "le=\"0.00001\"",   // 10µs
        "le=\"0.00002\"",   // 20µs
        "le=\"0.00005\"",   // 50µs
        "le=\"0.0001\"",    // 100µs
        "le=\"0.0002\"",    // 200µs
        "le=\"0.001\"",     // 1ms
        "le=\"0.005\"",     // 5ms
        "le=\"+Inf\"",      // Infinity
    ];
    
    for bucket in expected_buckets {
        assert!(
            prometheus_text.contains(bucket),
            "Missing histogram bucket: {}",
            bucket
        );
    }
    
    println!("✓ Histogram buckets test passed");
    
    Ok(())
}
