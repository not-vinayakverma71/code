/// Minimal test to debug SharedMemory handshake
use std::sync::Arc;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

#[tokio::test]
async fn test_basic_handshake() {
    // Enable logging
    let _ = env_logger::builder().is_test(true).try_init();
    
    println!("\n🔍 Testing SharedMemory handshake...");
    
    let socket_path = format!("/tmp/test_handshake_{}.sock", std::process::id());
    let _ = std::fs::remove_file(&socket_path);
    
    // Create listener
    println!("1. Creating listener...");
    let listener = Arc::new(SharedMemoryListener::bind(&socket_path).unwrap());
    println!("   ✓ Listener created");
    
    // Spawn server accept task
    let listener_clone = listener.clone();
    let server = tokio::spawn(async move {
        println!("2. Server waiting for connection...");
        match listener_clone.accept().await {
            Ok((stream, _addr)) => {
                println!("   ✓ Server accepted connection");
                Ok(stream)
            }
            Err(e) => {
                println!("   ✗ Server accept failed: {}", e);
                Err(e)
            }
        }
    });
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Connect client
    println!("3. Client connecting...");
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        SharedMemoryStream::connect(&socket_path)
    ).await {
        Ok(Ok(_stream)) => {
            println!("   ✓ Client connected");
            
            // Wait for server to finish
            match tokio::time::timeout(tokio::time::Duration::from_secs(1), server).await {
                Ok(Ok(Ok(_))) => {
                    println!("\n✅ Handshake successful!");
                }
                Ok(Ok(Err(e))) => {
                    panic!("Server failed: {}", e);
                }
                Ok(Err(e)) => {
                    panic!("Server task panicked: {:?}", e);
                }
                Err(_) => {
                    panic!("Server accept timed out");
                }
            }
        }
        Ok(Err(e)) => {
            panic!("Client connect failed: {}", e);
        }
        Err(_) => {
            panic!("Client connect timed out");
        }
    }
    
    let _ = std::fs::remove_file(&socket_path);
}
