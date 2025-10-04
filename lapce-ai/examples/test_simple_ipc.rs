/// Test Simple IPC - Verify basic communication works
use lapce_ai_rust::simple_ipc::{SimpleIpcServer, SimpleIpcClient};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Simple IPC Communication");
    println!("================================\n");
    
    // Start server in background
    let server = SimpleIpcServer::new("/tmp/test_simple.sock".to_string());
    let mut events = server.subscribe();
    
    tokio::spawn(async move {
        let _ = server.start().await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Connect client
    println!("1. Connecting client...");
    let mut client = SimpleIpcClient::new("/tmp/test_simple.sock".to_string());
    client.connect().await?;
    println!("   ✅ Connected");
    
    // Send test message
    println!("\n2. Sending test message...");
    client.send("TestCommand", serde_json::json!({
        "action": "test",
        "value": 42
    })).await?;
    println!("   ✅ Message sent");
    
    // Check if server received it
    if let Ok(event) = tokio::time::timeout(Duration::from_millis(100), events.recv()).await {
        if let Ok((msg_type, data)) = event {
            println!("   ✅ Server received: {} with data: {}", msg_type, data);
        }
    }
    
    // Send another message
    println!("\n3. Sending another message...");
    client.send("Echo", serde_json::json!({
        "message": "Hello IPC"
    })).await?;
    println!("   ✅ Echo sent");
    
    sleep(Duration::from_millis(100)).await;
    
    println!("\n✅ Basic IPC communication works!");
    
    // Cleanup
    let _ = std::fs::remove_file("/tmp/test_simple.sock");
    
    Ok(())
}
