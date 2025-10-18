/// Example: Testing AI Bridge connection to lapce-ai backend
/// 
/// Run with: cargo run --example ai_bridge_test
/// 
/// Prerequisites:
/// 1. Start lapce-ai server: cd ../lapce-ai && cargo run --release --bin ipc_test_server_volatile /tmp/lapce_ai.sock
/// 2. Run this example: cargo run --example ai_bridge_test

use lapce_app::ai_bridge::{BridgeClient, ShmTransport, Transport, OutboundMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AI Bridge Connection Test ===\n");
    
    // Create transport
    let socket_path = "/tmp/lapce_ai.sock";
    println!("1. Creating ShmTransport for: {}", socket_path);
    let mut transport = ShmTransport::new(socket_path);
    
    // Create bridge client
    println!("2. Creating BridgeClient...");
    let client = BridgeClient::new(Box::new(transport));
    
    // Check initial status
    println!("3. Initial status: {:?}", client.status());
    
    // Attempt connection
    println!("4. Connecting to lapce-ai backend...");
    match client.connect() {
        Ok(_) => {
            println!("   ✅ Connected successfully!");
            println!("   Status: {:?}", client.status());
        }
        Err(e) => {
            println!("   ❌ Connection failed: {}", e);
            println!("\n💡 Make sure lapce-ai server is running:");
            println!("   cd lapce-ai");
            println!("   cargo run --release --bin ipc_test_server_volatile /tmp/lapce_ai.sock");
            return Ok(());
        }
    }
    
    // Send a test message
    println!("\n5. Sending test message...");
    let test_msg = OutboundMessage::ChatMessage {
        content: "Hello from Lapce UI!".to_string(),
        context: None,
    };
    
    match client.send(test_msg) {
        Ok(_) => println!("   ✅ Message sent successfully!"),
        Err(e) => println!("   ❌ Send failed: {}", e),
    }
    
    // Try to receive response
    println!("\n6. Checking for responses...");
    for i in 0..5 {
        if let Some(msg) = client.try_receive() {
            println!("   ✅ Received: {:?}", msg);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        if i == 4 {
            println!("   ℹ️  No response (implementation pending)");
        }
    }
    
    // Disconnect
    println!("\n7. Disconnecting...");
    match client.disconnect() {
        Ok(_) => println!("   ✅ Disconnected cleanly"),
        Err(e) => println!("   ❌ Disconnect error: {}", e),
    }
    
    println!("\n=== Test Complete ===");
    println!("\n📋 Next Steps:");
    println!("   1. Implement actual IPC send/receive in shm_transport.rs");
    println!("   2. Add message serialization (bincode)");
    println!("   3. Add async tokio runtime");
    println!("   4. Test with real AI responses");
    
    Ok(())
}
