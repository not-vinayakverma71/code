/// Test Working IPC - Verify basic communication
use lapce_ai_rust::working_ipc::{MinimalServer, MinimalClient};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Minimal Working IPC");
    println!("===========================\n");
    
    let socket_path = "/tmp/test_working.sock";
    
    // Start server
    let server = MinimalServer::new(socket_path.to_string());
    tokio::spawn(async move {
        let _ = server.start().await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Connect client
    println!("1. Connecting client...");
    let mut client = MinimalClient::new();
    client.connect(socket_path).await?;
    println!("   ✅ Connected");
    
    // Read ACK
    let ack = client.recv().await?;
    println!("   ✅ Received ACK: {}", ack);
    
    // Send test message
    println!("\n2. Sending test message...");
    client.send(r#"{"type":"TestMessage","data":"Hello IPC"}"#).await?;
    println!("   ✅ Sent");
    
    // Read echo
    let echo = client.recv().await?;
    println!("   ✅ Received echo: {}", echo);
    
    println!("\n✅ Basic IPC working!");
    
    // Cleanup
    let _ = std::fs::remove_file(socket_path);
    
    Ok(())
}
