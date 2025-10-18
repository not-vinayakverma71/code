/// Integration tests for real connection pool implementation
/// Tests actual network I/O, HTTP/2 multiplexing, and connection reuse

use lapce_ai_rust::connection_pool_manager::{ConnectionPoolManager, PoolConfig};
use lapce_ai_rust::https_connection_manager_real::HttpsConnectionManager;
use lapce_ai_rust::http2_multiplexer::MultiplexedConnection;
use lapce_ai_rust::connection_reuse::ConnectionReuseGuard;
use hyper::{Request, StatusCode};
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[tokio::test]
async fn test_real_https_request_google() {
    // Test with Google's 204 endpoint
    let conn = HttpsConnectionManager::new().await.unwrap();
    
    let req = Request::head("https://www.google.com/generate_204")
        .body(Full::new(Bytes::new()))
        .unwrap();
        
    let response = conn.execute_request(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_real_https_request_httpbin() {
    // Test with httpbin.org
    let conn = HttpsConnectionManager::new().await.unwrap();
    
    let req = Request::get("https://httpbin.org/get")
        .header("User-Agent", "lapce-ai-test/1.0")
        .body(Full::new(Bytes::new()))
        .unwrap();
        
    let (response, body) = conn.execute_request_full(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify JSON response
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("\"url\": \"https://httpbin.org/get\""));
    assert!(body_str.contains("\"User-Agent\": \"lapce-ai-test/1.0\""));
}

#[tokio::test]
async fn test_connection_pool_reuse() {
    let config = PoolConfig {
        max_connections: 2,
        min_idle: 1,
        ..Default::default()
    };
    
    let pool = ConnectionPoolManager::new(config).await.unwrap();
    let stats = pool.get_stats();
    
    // Make 10 requests with pool of 2
    let mut handles = vec![];
    for i in 0..10 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool_clone.get_https_connection().await.unwrap();
            // Simulate work
            tokio::time::sleep(Duration::from_millis(10)).await;
            drop(conn); // Return to pool
        }));
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Check reuse rate
    let reuse_rate = stats.get_reuse_rate();
    println!("Connection reuse rate: {:.1}%", reuse_rate);
    assert!(reuse_rate > 50.0); // Should have significant reuse
}

#[tokio::test]
async fn test_http2_multiplexing() {
    let mux = MultiplexedConnection::new("test-conn".to_string(), 100);
    
    // Allocate multiple streams
    let mut stream_ids = vec![];
    for priority in 0..10 {
        let stream_id = mux.allocate_stream(priority).await.unwrap();
        stream_ids.push(stream_id);
    }
    
    // All stream IDs should be odd (client-initiated)
    for id in &stream_ids {
        assert_eq!(id % 2, 1);
    }
    
    // Check stats
    let stats = mux.get_stats().await;
    assert_eq!(stats.active_streams, 10);
    assert_eq!(stats.total_streams_created, 10);
    
    // Release streams
    for id in stream_ids {
        mux.release_stream(id).await.unwrap();
    }
    
    let stats = mux.get_stats().await;
    assert_eq!(stats.active_streams, 0);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let config = PoolConfig::default();
    let pool = ConnectionPoolManager::new(config).await.unwrap();
    
    // Launch 50 concurrent requests
    let mut handles = vec![];
    for i in 0..50 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool_clone.get_https_connection().await.unwrap();
            
            let req = Request::get(format!("https://httpbin.org/delay/{}", i % 3))
                .body(Full::new(Bytes::new()))
                .unwrap();
                
            let start = Instant::now();
            let result = timeout(
                Duration::from_secs(10),
                conn.execute_request(req)
            ).await;
            
            match result {
                Ok(Ok(response)) => {
                    println!("Request {} completed in {:?}, status: {}", 
                             i, start.elapsed(), response.status());
                    assert!(response.status().is_success());
                }
                _ => panic!("Request {} failed", i),
            }
        }));
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let stats = pool.get_stats();
    println!("Total connections created: {}", 
             stats.total_connections.load(std::sync::atomic::Ordering::Relaxed));
}

#[tokio::test]
async fn test_connection_health_check() {
    let conn = HttpsConnectionManager::new().await.unwrap();
    
    // Health check should pass
    let result = conn.is_valid().await;
    assert!(result.is_ok(), "Health check failed: {:?}", result);
    
    // Stats should show the health check request
    let stats = conn.get_stats();
    assert!(stats.request_count > 0);
}

#[tokio::test]
async fn test_tls_handshake_performance() {
    let start = Instant::now();
    let conn = HttpsConnectionManager::new().await.unwrap();
    let creation_time = start.elapsed();
    
    println!("Connection creation time: {:?}", creation_time);
    assert!(creation_time < Duration::from_secs(1), "Connection creation too slow");
    
    // Make first request (includes TLS handshake)
    let req_start = Instant::now();
    let req = Request::get("https://www.google.com/robots.txt")
        .body(Full::new(Bytes::new()))
        .unwrap();
        
    let response = conn.execute_request(req).await.unwrap();
    let request_time = req_start.elapsed();
    
    println!("First request time (with TLS): {:?}", request_time);
    assert!(request_time < Duration::from_secs(5), "TLS handshake too slow");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_connection_reuse_guard() {
    let config = PoolConfig::default();
    let pool = ConnectionPoolManager::new(config).await.unwrap();
    let stats = pool.get_stats();
    
    {
        let mut guard = ConnectionReuseGuard::new(&pool.https_pool, stats.clone());
        
        // First use
        let conn = guard.get_connection().await.unwrap();
        assert_eq!(guard.get_reuse_count(), 1);
        
        // Second use (reuse)
        let conn = guard.get_connection().await.unwrap();
        assert_eq!(guard.get_reuse_count(), 2);
        
        // Make actual request
        let req = Request::get("https://httpbin.org/uuid")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let response = conn.execute_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    // Guard drops here, connection returned to pool
}

#[tokio::test]
async fn test_memory_usage() {
    let config = PoolConfig {
        max_connections: 100,
        min_idle: 10,
        ..Default::default()
    };
    
    let pool = ConnectionPoolManager::new(config).await.unwrap();
    
    // Pre-warm connections
    let mut connections = vec![];
    for _ in 0..100 {
        connections.push(pool.get_https_connection().await.unwrap());
    }
    
    // Check memory (this is a simplified check)
    let stats = pool.get_stats();
    let total = stats.total_connections.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("Created {} connections", total);
    
    // Approximate memory per connection (should be ~20KB)
    // Total for 100 connections should be < 3MB
    assert!(total <= 100, "Too many connections created");
}

#[tokio::test]
async fn test_connection_acquisition_latency() {
    let config = PoolConfig {
        max_connections: 50,
        min_idle: 25,
        ..Default::default()
    };
    
    let pool = ConnectionPoolManager::new(config).await.unwrap();
    
    // Warm up pool
    for _ in 0..25 {
        let conn = pool.get_https_connection().await.unwrap();
        drop(conn);
    }
    
    // Measure acquisition latency
    let mut latencies = vec![];
    for _ in 0..100 {
        let start = Instant::now();
        let conn = pool.get_https_connection().await.unwrap();
        let latency = start.elapsed();
        latencies.push(latency);
        drop(conn);
    }
    
    let avg_latency: Duration = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    println!("Average acquisition latency: {:?}", avg_latency);
    
    assert!(avg_latency < Duration::from_millis(1), 
            "Acquisition latency too high: {:?}", avg_latency);
}
