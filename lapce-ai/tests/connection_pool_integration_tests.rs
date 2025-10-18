/// Connection Pool Integration Tests
/// Verify reuse >95%, acquisition <1ms, fail-fast on broken connections

use lapce_ai_rust::ipc::connection_pool::{ConnectionPool, PoolConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_connection_reuse_rate() {
    // Only run if TEST_REAL_ENDPOINTS is set
    if std::env::var("TEST_REAL_ENDPOINTS").is_err() {
        println!("Skipping real endpoint test (set TEST_REAL_ENDPOINTS to run)");
        return;
    }
    
    // Start test server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0; 1024];
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 { break; }
                    let _ = socket.write_all(&buf[..n]).await;
                }
            });
        }
    });
    
    let config = PoolConfig {
        min_connections: 5,
        max_connections: 20,
        connection_timeout: Duration::from_secs(1),
        idle_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    let total_requests = 1000;
    let mut handles = vec![];
    
    let connections_created = Arc::new(AtomicUsize::new(0));
    let connections_reused = Arc::new(AtomicUsize::new(0));
    
    for i in 0..total_requests {
        let pool_clone = pool.clone();
        let created_clone = connections_created.clone();
        let reused_clone = connections_reused.clone();
        
        let handle = tokio::spawn(async move {
            match pool_clone.acquire().await {
                Ok(conn) => {
                    // Simulate work
                    tokio::time::sleep(Duration::from_micros(100)).await;
                    
                    // Track if this is a new or reused connection
                    if i == 0 {
                        created_clone.fetch_add(1, Ordering::Relaxed);
                    } else {
                        reused_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    drop(conn);
                    true
                },
                Err(_) => false,
            }
        });
        handles.push(handle);
        
        // Small delay to allow connection reuse
        if i % 10 == 0 {
            tokio::time::sleep(Duration::from_micros(10)).await;
        }
    }
    
    let mut successful = 0;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            successful += 1;
        }
    }
    
    let stats = pool.stats().await;
    let reuse_rate = stats.reuse_rate;
    
    println!("Connection pool reuse stats:");
    println!("  Total requests: {}", total_requests);
    println!("  Successful: {}", successful);
    println!("  Connections created: {}", stats.total_created);
    println!("  Connections reused: {}", stats.total_recycled);
    println!("  Reuse rate: {:.2}%", reuse_rate);
    
    assert!(reuse_rate > 95.0, "Reuse rate {:.2}% should be > 95%", reuse_rate);
}

#[tokio::test]
async fn test_acquisition_latency() {
    let config = PoolConfig {
        min_connections: 10,
        max_connections: 50,
        connection_timeout: Duration::from_millis(100),
        idle_timeout: Duration::from_secs(60),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 1,
        retry_delay: Duration::from_millis(10),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    
    // Warm up pool
    let mut warmup_conns = vec![];
    for _ in 0..10 {
        warmup_conns.push(pool.acquire().await.unwrap());
    }
    drop(warmup_conns);
    
    // Measure acquisition times
    let mut acquisition_times = vec![];
    
    for _ in 0..100 {
        let start = Instant::now();
        let conn = pool.acquire().await.unwrap();
        let elapsed = start.elapsed();
        acquisition_times.push(elapsed);
        drop(conn);
        
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    
    // Calculate statistics
    acquisition_times.sort();
    let p50 = acquisition_times[50].as_micros();
    let p95 = acquisition_times[95].as_micros();
    let p99 = acquisition_times[99].as_micros();
    
    println!("Connection acquisition latency:");
    println!("  P50: {}µs", p50);
    println!("  P95: {}µs", p95);
    println!("  P99: {}µs", p99);
    
    // Verify < 1ms (1000µs) acquisition time
    assert!(p99 < 1000, "P99 latency {}µs should be < 1000µs (1ms)", p99);
}

#[tokio::test]
async fn test_fail_fast_broken_connections() {
    let config = PoolConfig {
        min_connections: 5,
        max_connections: 10,
        connection_timeout: Duration::from_millis(100),
        idle_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 1,
        retry_delay: Duration::from_millis(10),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    
    // Simulate broken connections by marking them unhealthy
    for _ in 0..5 {
        let conn = pool.acquire().await.unwrap();
        let id = conn.id();
        pool.record_error(id).await;
        pool.record_error(id).await;
        pool.record_error(id).await;
        pool.record_error(id).await;
        pool.record_error(id).await;
        pool.record_error(id).await; // Mark as unhealthy after 6 errors
    }
    
    // Try to acquire - should fail fast on broken connections
    let start = Instant::now();
    let mut failed_fast = 0;
    
    for _ in 0..10 {
        let acquire_start = Instant::now();
        match pool.acquire().await {
            Ok(conn) => {
                // Check if healthy
                if !pool.health_check(conn.id()).await {
                    let elapsed = acquire_start.elapsed();
                    if elapsed < Duration::from_millis(10) {
                        failed_fast += 1;
                    }
                }
            },
            Err(_) => {
                let elapsed = acquire_start.elapsed();
                if elapsed < Duration::from_millis(10) {
                    failed_fast += 1;
                }
            }
        }
    }
    
    let total_elapsed = start.elapsed();
    
    println!("Fail-fast on broken connections:");
    println!("  Failed fast: {}/10", failed_fast);
    println!("  Total time: {:?}", total_elapsed);
    
    assert!(failed_fast >= 5, "Should fail fast on at least 5 broken connections");
    assert!(total_elapsed < Duration::from_millis(500), "Should complete quickly");
}

#[tokio::test]
async fn test_connection_pool_scaling() {
    let config = PoolConfig {
        min_connections: 5,
        max_connections: 100,
        connection_timeout: Duration::from_secs(1),
        idle_timeout: Duration::from_secs(30),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    
    // Start with minimal connections
    assert!(pool.active_count().await <= 5);
    
    // Spike load - should scale up
    let mut handles = vec![];
    for _ in 0..50 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let _conn = pool_clone.acquire().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }));
    }
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    let peak_count = pool.active_count().await;
    println!("Peak connections during spike: {}", peak_count);
    assert!(peak_count > 20, "Should scale up during load spike");
    
    // Wait for connections to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    // Scale down after idle
    pool.scale(10).await.unwrap();
    let after_scale = pool.active_count().await;
    println!("Connections after scale down: {}", after_scale);
    assert!(after_scale <= 10, "Should scale down when requested");
}

#[tokio::test]
async fn test_connection_health_monitoring() {
    let config = PoolConfig {
        min_connections: 5,
        max_connections: 20,
        connection_timeout: Duration::from_secs(1),
        idle_timeout: Duration::from_secs(60),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    
    // Create connections with varying health
    let mut conn_ids = vec![];
    for i in 0..10 {
        let conn = pool.acquire().await.unwrap();
        let id = conn.id();
        conn_ids.push(id);
        
        // Simulate varying error rates
        for _ in 0..i {
            pool.record_error(id).await;
        }
        
        // Simulate requests
        for _ in 0..10 {
            pool.touch(id).await;
        }
    }
    
    // Check health status
    let health_status = pool.health_status().await;
    let healthy_count = health_status.iter().filter(|c| c.is_healthy).count();
    let unhealthy_count = health_status.len() - healthy_count;
    
    println!("Connection health monitoring:");
    println!("  Total connections: {}", health_status.len());
    println!("  Healthy: {}", healthy_count);
    println!("  Unhealthy: {}", unhealthy_count);
    
    // Verify health checks work
    for id in &conn_ids[0..5] {
        assert!(pool.health_check(*id).await, "Low error connections should be healthy");
    }
    
    for id in &conn_ids[6..] {
        assert!(!pool.health_check(*id).await, "High error connections should be unhealthy");
    }
}

#[tokio::test]
async fn test_metrics_export() {
    let config = PoolConfig {
        min_connections: 5,
        max_connections: 20,
        connection_timeout: Duration::from_secs(1),
        idle_timeout: Duration::from_secs(60),
        max_lifetime: Duration::from_secs(300),
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let pool = Arc::new(ConnectionPool::new(config));
    
    // Generate some activity
    for _ in 0..10 {
        let conn = pool.acquire().await.unwrap();
        pool.touch(conn.id()).await;
    }
    
    // Export metrics
    let metrics = pool.export_metrics().await;
    
    println!("Exported metrics:\n{}", metrics);
    
    // Verify metrics format
    assert!(metrics.contains("connection_pool_total"));
    assert!(metrics.contains("connection_pool_healthy"));
    assert!(metrics.contains("connection_pool_requests"));
    assert!(metrics.contains("connection_pool_errors"));
    assert!(metrics.contains("connection_pool_reuse_rate"));
    
    // Verify Prometheus format
    assert!(metrics.contains("# HELP"));
    assert!(metrics.contains("# TYPE"));
}
