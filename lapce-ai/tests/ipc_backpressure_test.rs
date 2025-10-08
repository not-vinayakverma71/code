/// Backpressure chaos tests for IPC implementation
use lapce_ai_rust::ipc::backpressure::{BackpressureConfig, BackpressureHandler};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::time::{Duration, Instant};

#[tokio::test]
async fn test_backpressure_under_load() {
    let config = BackpressureConfig {
        initial_backoff_us: 100,
        max_backoff_us: 10_000,
        multiplier: 2.0,
        jitter_factor: 0.3,
        max_retries: 5,
        non_blocking: false,
    };
    
    let handler = Arc::new(BackpressureHandler::new(config));
    let write_success_counter = Arc::new(AtomicU64::new(0));
    let buffer_full = Arc::new(AtomicBool::new(false));
    
    // Simulate high load with buffer becoming full periodically
    let mut handles = vec![];
    
    for i in 0..100 {
        let handler = handler.clone();
        let counter = write_success_counter.clone();
        let buffer = buffer_full.clone();
        
        let handle = tokio::spawn(async move {
            let write_fn = || {
                let buf_full = buffer.load(Ordering::Relaxed);
                async move {
                    if buf_full {
                        Ok(false) // Buffer full, need retry
                    } else {
                        counter.fetch_add(1, Ordering::Relaxed);
                        Ok(true)  // Write succeeded
                    }
                }
            };
            
            handler.write_with_backpressure(write_fn).await
        });
        
        handles.push(handle);
        
        // Periodically make buffer full to trigger backpressure
        if i % 20 == 10 {
            buffer_full.store(true, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(5)).await;
            buffer_full.store(false, Ordering::Relaxed);
        }
    }
    
    // Wait for all writes
    for handle in handles {
        let _ = handle.await;
    }
    
    let stats = handler.get_stats();
    let total = stats.total_writes.load(Ordering::Relaxed);
    let blocked = stats.blocked_writes.load(Ordering::Relaxed);
    
    println!("Backpressure test results:");
    println!("Total writes: {}", total);
    println!("Blocked writes: {}", blocked);
    println!("Success rate: {:.1}%", (write_success_counter.load(Ordering::Relaxed) as f64 / 100.0) * 100.0);
    
    assert!(total >= 90, "Should complete most writes");
    assert!(blocked > 0, "Should have some blocked writes");
}

#[tokio::test]
async fn test_chaos_backpressure() {
    // Chaos test with random buffer availability
    let config = BackpressureConfig {
        initial_backoff_us: 50,
        max_backoff_us: 5_000,
        multiplier: 1.5,
        jitter_factor: 0.5,
        max_retries: 8,
        non_blocking: false,
    };
    
    let handler = Arc::new(BackpressureHandler::new(config));
    let chaos_counter = Arc::new(AtomicU64::new(0));
    
    // Launch chaos writers
    let mut handles = vec![];
    for _ in 0..50 {
        let handler = handler.clone();
        let counter = chaos_counter.clone();
        
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            
            let write_fn = || {
                let counter = counter.clone();
                async move {
                    // Random success/failure
                    let success = rand::random::<f64>() > 0.3; // 70% success rate
                    if success {
                        counter.fetch_add(1, Ordering::Relaxed);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            };
            
            let result = handler.write_with_backpressure(write_fn).await;
            (result, start.elapsed())
        });
        
        handles.push(handle);
    }
    
    // Collect results
    let mut successes = 0;
    let mut failures = 0;
    let mut total_latency_ms = 0.0;
    
    for handle in handles {
        if let Ok((result, duration)) = handle.await {
            if result.is_ok() {
                successes += 1;
            } else {
                failures += 1;
            }
            total_latency_ms += duration.as_secs_f64() * 1000.0;
        }
    }
    
    println!("\nChaos test results:");
    println!("Successes: {}/{}", successes, successes + failures);
    println!("Average latency: {:.2}ms", total_latency_ms / 50.0);
    
    let stats = handler.get_stats();
    println!("Max backoff observed: {}μs", stats.max_backoff_observed_us.load(Ordering::Relaxed));
    
    // Print retry histogram
    println!("Retry histogram:");
    for (i, count) in stats.retry_histogram.iter().enumerate() {
        let c = count.load(Ordering::Relaxed);
        if c > 0 {
            println!("  {} retries: {}", i, c);
        }
    }
    
    assert!(successes >= 30, "Should succeed for majority despite chaos");
}

#[test]
fn test_backoff_calculation() {
    let config = BackpressureConfig {
        initial_backoff_us: 1000,
        max_backoff_us: 100_000,
        multiplier: 2.0,
        jitter_factor: 0.0, // No jitter for predictable test
        max_retries: 10,
        non_blocking: false,
    };
    
    let handler = BackpressureHandler::new(config);
    
    // Test exponential growth
    assert_eq!(handler.calculate_backoff(0).as_micros(), 1000);
    assert_eq!(handler.calculate_backoff(1).as_micros(), 2000);
    assert_eq!(handler.calculate_backoff(2).as_micros(), 4000);
    assert_eq!(handler.calculate_backoff(3).as_micros(), 8000);
    
    // Test capping at max
    assert_eq!(handler.calculate_backoff(10).as_micros(), 100_000);
}
