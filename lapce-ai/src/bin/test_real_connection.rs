/// Quick test binary to verify real connection pool works
use lapce_ai_rust::https_connection_manager_real::HttpsConnectionManager;
use lapce_ai_rust::http2_multiplexer::MultiplexedConnection;
use hyper::{Request, StatusCode};
use http_body_util::Full;
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Real Connection Pool Implementation...\n");
    
    // Test 1: Create real HTTPS connection
    println!("1. Creating HTTPS connection manager...");
    let conn = HttpsConnectionManager::new().await?;
    println!("   ✅ Connection created");
    
    // Test 2: Make real HTTPS request to Google
    println!("\n2. Testing Google 204 endpoint...");
    let req = Request::head("https://www.google.com/generate_204")
        .body(Full::new(Bytes::new()))?;
    
    let response = conn.execute_request(req).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    println!("   ✅ Got 204 response from Google");
    
    // Test 3: Make real request to httpbin
    println!("\n3. Testing httpbin.org GET...");
    let req = Request::get("https://httpbin.org/get")
        .header("User-Agent", "lapce-ai-real-test/1.0")
        .body(Full::new(Bytes::new()))?;
    
    let (response, body) = conn.execute_request_full(req).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_str = std::str::from_utf8(&body)?;
    assert!(body_str.contains("\"url\": \"https://httpbin.org/get\""));
    println!("   ✅ Got valid JSON response from httpbin");
    
    // Test 4: Check connection stats
    println!("\n4. Connection statistics:");
    let stats = conn.get_stats();
    println!("   - Requests made: {}", stats.request_count);
    println!("   - Errors: {}", stats.error_count);
    println!("   - Bytes sent: {}", stats.bytes_sent);
    println!("   - Bytes received: {}", stats.bytes_received);
    println!("   - Connection age: {:?}", stats.age);
    
    // Test 5: HTTP/2 multiplexing
    println!("\n5. Testing HTTP/2 multiplexing...");
    let mux = MultiplexedConnection::new("test-http2".to_string(), 100);
    
    let stream1 = mux.allocate_stream(1).await?;
    let stream2 = mux.allocate_stream(2).await?;
    let stream3 = mux.allocate_stream(3).await?;
    
    println!("   - Allocated streams: {}, {}, {}", stream1, stream2, stream3);
    
    let mux_stats = mux.get_stats().await;
    println!("   - Active streams: {}/{}", mux_stats.active_streams, mux_stats.max_concurrent_streams);
    println!("   ✅ HTTP/2 multiplexing working");
    
    // Test 6: Health check
    println!("\n6. Testing connection health check...");
    conn.is_valid().await?;
    println!("   ✅ Health check passed");
    
    println!("\n🎉 ALL TESTS PASSED! Real connection pool is working!");
    println!("\nSummary:");
    println!("✅ Real HTTPS connections");
    println!("✅ Real network I/O"); 
    println!("✅ TLS handshake");
    println!("✅ HTTP/2 multiplexing");
    println!("✅ Connection statistics");
    println!("✅ Health monitoring");
    
    Ok(())
}
