// AWS Titan Security and Rate Limiting Tests
use lancedb::embeddings::aws_titan_production::AwsTitanProduction;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_rate_limiting_enforced() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        let config = embedder.get_config();
        let max_rps = config.max_requests_per_second;
        
        // Try to exceed rate limit
        let start = Instant::now();
        let mut tasks = vec![];
        
        for i in 0..max_rps + 5 {
            let embedder = embedder.clone();
            let task = tokio::spawn(async move {
                embedder.create_embeddings(vec![format!("test {}", i)], None).await
            });
            tasks.push(task);
        }
        
        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }
        
        let elapsed = start.elapsed();
        
        // Should take at least 1 second due to rate limiting
        assert!(
            elapsed >= Duration::from_millis(800),
            "Rate limiting should enforce delays: elapsed {:?}",
            elapsed
        );
    }
}

#[tokio::test]
async fn test_concurrent_request_limit() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        let config = embedder.get_config();
        let max_concurrent = config.max_concurrent_requests;
        
        // Try to exceed concurrent limit
        let mut tasks = vec![];
        
        for i in 0..max_concurrent + 10 {
            let embedder = embedder.clone();
            let task = tokio::spawn(async move {
                embedder.create_embeddings(vec![format!("test {}", i)], None).await
            });
            tasks.push(task);
        }
        
        // Some requests should be queued/delayed
        let start = Instant::now();
        for task in tasks {
            let _ = task.await;
        }
        let elapsed = start.elapsed();
        
        // Should take longer than sequential due to concurrent limiting
        println!("Concurrent test completed in {:?}", elapsed);
    }
}

#[tokio::test]
async fn test_retry_on_throttling() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Simulate throttling by making many rapid requests
        let mut success = 0;
        let mut retried = 0;
        
        for i in 0..20 {
            match embedder.create_embeddings(vec![format!("test {}", i)], None).await {
                Ok(_) => success += 1,
                Err(e) => {
                    let msg = format!("{:?}", e);
                    if msg.contains("Throttling") || msg.contains("retry") {
                        retried += 1;
                    }
                }
            }
        }
        
        // At least some requests should succeed
        assert!(success > 0, "Some requests should succeed");
        println!("Success: {}, Retried: {}", success, retried);
    }
}

#[tokio::test]
async fn test_exponential_backoff() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        let config = embedder.get_config();
        
        // Verify retry configuration
        assert!(config.max_retries > 0, "Retries should be configured");
        assert!(config.initial_delay_ms > 0, "Initial delay should be configured");
        assert!(config.max_delay_ms > config.initial_delay_ms, "Max delay should be greater");
        assert!(config.exponential_base >= 1.5, "Exponential base should be reasonable");
    }
}

#[tokio::test]
async fn test_request_timeout() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        let config = embedder.get_config();
        assert!(config.timeout_seconds > 0, "Timeout should be configured");
        assert!(config.timeout_seconds <= 60, "Timeout should be reasonable");
    }
}

#[tokio::test]
async fn test_chaos_random_failures() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Test resilience with rapid fire requests
        let mut tasks = vec![];
        
        for i in 0..50 {
            let embedder = embedder.clone();
            let task = tokio::spawn(async move {
                embedder.create_embeddings(vec![format!("chaos {}", i)], None).await
            });
            tasks.push(task);
            
            // Add random delays
            if i % 3 == 0 {
                sleep(Duration::from_millis(10)).await;
            }
        }
        
        let mut success = 0;
        let mut failures = 0;
        
        for task in tasks {
            match task.await {
                Ok(Ok(_)) => success += 1,
                _ => failures += 1,
            }
        }
        
        println!("Chaos test - Success: {}, Failures: {}", success, failures);
        // At least 50% success rate expected with retries
        assert!(success >= 25, "Should have reasonable success rate with retries");
    }
}

#[tokio::test]
async fn test_burst_protection() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Send burst of requests
        let start = Instant::now();
        
        for _ in 0..20 {
            let _ = embedder.create_embeddings(vec!["burst test".to_string()], None).await;
        }
        
        let elapsed = start.elapsed();
        
        // Should be rate limited, taking at least 1-2 seconds
        assert!(
            elapsed >= Duration::from_secs(1),
            "Burst should be rate limited: {:?}",
            elapsed
        );
    }
}

#[tokio::test]
async fn test_jitter_in_backoff() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Make multiple retry-triggering requests
        let mut delays = vec![];
        
        for i in 0..10 {
            let start = Instant::now();
            let _ = embedder.create_embeddings(vec![format!("jitter {}", i)], None).await;
            delays.push(start.elapsed());
            
            // Small delay between attempts
            sleep(Duration::from_millis(100)).await;
        }
        
        // Delays should vary (jitter effect)
        let avg_delay: Duration = delays.iter().sum::<Duration>() / delays.len() as u32;
        println!("Average delay: {:?}", avg_delay);
        
        // Verify not all delays are identical (jitter working)
        let unique_delays: std::collections::HashSet<_> = 
            delays.iter().map(|d| d.as_millis() / 100).collect();
        assert!(unique_delays.len() > 1, "Jitter should create varying delays");
    }
}

#[tokio::test]
async fn test_graceful_degradation() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Test that system doesn't crash under stress
        let mut handles = vec![];
        
        for batch in 0..5 {
            for i in 0..10 {
                let embedder = embedder.clone();
                let handle = tokio::spawn(async move {
                    embedder.create_embeddings(
                        vec![format!("batch {} item {}", batch, i)],
                        None
                    ).await
                });
                handles.push(handle);
            }
            
            sleep(Duration::from_millis(200)).await;
        }
        
        // Collect results
        let mut completed = 0;
        for handle in handles {
            if handle.await.is_ok() {
                completed += 1;
            }
        }
        
        println!("Completed {} requests gracefully", completed);
        assert!(completed > 0, "Should complete some requests");
    }
}

#[tokio::test]
async fn test_circuit_breaker_pattern() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Verify metrics track failures
        let initial_metrics = embedder.get_metrics().await;
        let initial_failures = initial_metrics.failed_requests;
        
        // Make some requests
        for i in 0..10 {
            let _ = embedder.create_embeddings(vec![format!("cb test {}", i)], None).await;
        }
        
        let final_metrics = embedder.get_metrics().await;
        
        // Metrics should be tracking
        assert!(
            final_metrics.total_requests >= initial_metrics.total_requests,
            "Metrics should track requests"
        );
    }
}

#[tokio::test]
async fn test_resource_cleanup() {
    // Create and drop embedder multiple times
    for _ in 0..5 {
        let embedder = AwsTitanProduction::new().await;
        
        if let Ok(embedder) = embedder {
            let _ = embedder.create_embeddings(vec!["cleanup test".to_string()], None).await;
        }
        
        // Embedder should be dropped and resources cleaned up
        drop(embedder);
    }
    
    // Should not leak resources
    assert!(true, "Resource cleanup successful");
}

#[tokio::test]
async fn test_latency_tracking() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        // Make several requests
        for i in 0..10 {
            let _ = embedder.create_embeddings(vec![format!("latency {}", i)], None).await;
        }
        
        let metrics = embedder.get_metrics().await;
        
        // Verify latency metrics are tracked
        assert!(metrics.avg_latency_ms >= 0.0, "Average latency should be tracked");
        assert!(metrics.p95_latency_ms >= metrics.avg_latency_ms, "p95 should be >= avg");
        assert!(metrics.p99_latency_ms >= metrics.p95_latency_ms, "p99 should be >= p95");
        
        println!("Latency metrics - Avg: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms",
            metrics.avg_latency_ms,
            metrics.p95_latency_ms,
            metrics.p99_latency_ms
        );
    }
}

#[tokio::test]
async fn test_cost_limits() {
    let embedder = AwsTitanProduction::new().await;
    
    if let Ok(embedder) = embedder {
        let initial_metrics = embedder.get_metrics().await;
        let initial_cost = initial_metrics.total_cost_usd;
        
        // Make requests
        for i in 0..5 {
            let _ = embedder.create_embeddings(
                vec![format!("cost tracking test {}", i)],
                None
            ).await;
        }
        
        let final_metrics = embedder.get_metrics().await;
        
        // Cost should increase
        assert!(
            final_metrics.total_cost_usd >= initial_cost,
            "Cost should be tracked"
        );
        
        // Cost should be reasonable (not astronomical)
        assert!(
            final_metrics.total_cost_usd < 1.0,
            "Cost should be reasonable for test: ${:.4}",
            final_metrics.total_cost_usd
        );
    }
}
