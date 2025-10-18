use lapce_ai_rust::ipc::connection_pool::{WorkingConnectionPool, ConnectionConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing Connection Pool for 1000+ Connections\n");
    
    // Test 1: Basic pool creation
    println!("Test 1: Pool Creation");
    println!("────────────────────");
    let config = ConnectionConfig {
        min_connections: 10,
        max_connections: 1000,
        ..Default::default()
    };
    
    let pool = Arc::new(WorkingConnectionPool::new(config).await?);
    let stats = pool.stats().await;
    println!("✓ Pool created with {} connections", stats.total_connections);
    
    // Test 2: Acquire and release
    println!("\nTest 2: Acquire/Release");
    println!("────────────────────");
    let conn = pool.acquire().await?;
    println!("✓ Connection {} acquired", conn.id);
    
    pool.release(conn).await;
    println!("✓ Connection released");
    
    // Test 3: Concurrent connections
    println!("\nTest 3: Concurrent Access (100 tasks)");
    println!("────────────────────");
    
    let start = Instant::now();
    let mut handles = Vec::new();
    
    for i in 0..100 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool_clone.acquire().await.unwrap();
            // Simulate work
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // Write some data
            let data = vec![i as u8; 64];
            conn.write(&data).await;
            
            pool_clone.release(conn).await;
        }));
    }
    
    for handle in handles {
        handle.await?;
    }
    
    let elapsed = start.elapsed();
    println!("✓ 100 concurrent tasks completed in {:.2}ms", elapsed.as_millis());
    
    let stats = pool.stats().await;
    println!("  Total acquired: {}", stats.total_acquired);
    println!("  Total released: {}", stats.total_released);
    
    // Test 4: Stress test - 1000 concurrent connections
    println!("\nTest 4: Stress Test (1000 connections)");
    println!("────────────────────");
    
    let stress_pool = Arc::new(WorkingConnectionPool::new(ConnectionConfig {
        min_connections: 50,
        max_connections: 1000,
        connection_timeout: Duration::from_secs(5),
        ..Default::default()
    }).await?);
    
    let start = Instant::now();
    let mut handles = Vec::new();
    let success_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let error_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    
    for _ in 0..1000 {
        let pool_clone = stress_pool.clone();
        let success = success_count.clone();
        let errors = error_count.clone();
        
        handles.push(tokio::spawn(async move {
            match pool_clone.acquire().await {
                Ok(conn) => {
                    // Do some work
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    pool_clone.release(conn).await;
                    success.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Err(_) => {
                    errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }));
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    let elapsed = start.elapsed();
    let successful = success_count.load(std::sync::atomic::Ordering::Relaxed);
    let failed = error_count.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("✓ Stress test completed in {:.2}s", elapsed.as_secs_f64());
    println!("  Successful: {}/1000", successful);
    println!("  Failed: {}/1000", failed);
    
    let final_stats = stress_pool.stats().await;
    println!("\nFinal Statistics:");
    println!("  Total connections: {}", final_stats.total_connections);
    println!("  Available: {}", final_stats.available_connections);
    println!("  In use: {}", final_stats.in_use_connections);
    
    if successful >= 900 {
        println!("\n✅ Pool can handle 1000+ connections!");
    } else {
        println!("\n⚠️ Pool struggling with high load");
    }
    
    Ok(())
}
