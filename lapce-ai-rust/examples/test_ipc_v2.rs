/// Test IPC Server V2 - Testing 1:1 TypeScript Translation
use lapce_ai_rust::ipc_server_v2::{IpcServer, IpcMessage, IpcMessageType, IpcOrigin, TaskCommand, TaskCommandName};
use lapce_ai_rust::ipc_client_v2::IpcClient;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing IPC Server V2 (1:1 TypeScript Port)");
    println!("===========================================\n");
    
    // Test 1: Server creation
    println!("1. Creating server...");
    let server = IpcServer::new(
        "/tmp/test_ipc_v2.sock".to_string(),
        Some(Box::new(|msg| println!("[SERVER] {}", msg)))
    );
    println!("   ✅ Server created at: {}", server.socket_path());
    
    // Test 2: Start listening
    println!("\n2. Starting server...");
    server.listen().await?;
    println!("   ✅ Server listening: {}", server.is_listening().await);
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Test 3: Client connection
    println!("\n3. Creating client...");
    let client = IpcClient::new(
        "/tmp/test_ipc_v2.sock".to_string(),
        Some(Box::new(|msg| println!("[CLIENT] {}", msg)))
    );
    
    println!("   Connecting client...");
    client.connect().await?;
    sleep(Duration::from_millis(100)).await;
    
    println!("   Client connected: {}", client.is_connected().await);
    
    // Test 4: Send a command
    println!("\n4. Sending TaskCommand...");
    let command = TaskCommand::CancelTask {
        data: "test-task-123".to_string()
    };
    
    client.send_command(command).await;
    sleep(Duration::from_millis(100)).await;
    
    // Test 5: Disconnect
    println!("\n5. Disconnecting client...");
    client.disconnect().await;
    sleep(Duration::from_millis(100)).await;
    
    println!("\n✅ Basic IPC test completed!");
    println!("   This is a 1:1 port of TypeScript IPC");
    
    // Cleanup
    let _ = std::fs::remove_file("/tmp/test_ipc_v2.sock");
    
    Ok(())
}
