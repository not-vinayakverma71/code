use lapce_ai_rust::shared_memory_transport::SharedMemoryTransport;
use lapce_ai_rust::ipc_server::IpcServer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting memory test...");
    
    // Create SharedMemory transport
    let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
    println!("Created SharedMemoryTransport (1MB buffer)");
    
    // Create IPC server
    let server = Arc::new(IpcServer::new("/tmp/test.sock").await.unwrap());
    println!("Created IPC server");
    
    // Simulate some activity
    for i in 0..100 {
        let msg = format!("Test message {}", i).into_bytes();
        transport.send(&msg).await.unwrap();
        if i % 10 == 0 {
            println!("Sent {} messages", i);
        }
    }
    
    // Print memory stats
    println!("\nMemory allocation complete.");
    println!("Process PID: {}", std::process::id());
    
    // Keep alive for profiling
    println!("Keeping alive for profiling... Press Ctrl+C to stop");
    sleep(Duration::from_secs(60)).await;
}
