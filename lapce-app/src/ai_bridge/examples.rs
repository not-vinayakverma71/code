/// Example usage patterns for AI Bridge
/// Shows how Lapce UI components will interact with the bridge

use super::{BridgeClient, OutboundMessage};
use super::shm_transport::ShmTransport;

/// Example 1: Initialize bridge on app startup
pub fn example_app_startup() {
    // Create transport (will connect to lapce-ai backend)
    let socket_path = "/tmp/lapce_ai.sock";
    let transport = ShmTransport::new(socket_path);
    
    // Create bridge client
    let bridge = BridgeClient::new(Box::new(transport));
    
    // Attempt connection
    if let Err(e) = bridge.connect() {
        eprintln!("[BRIDGE EXAMPLE] Connection failed: {}", e);
    } else {
        println!("[BRIDGE EXAMPLE] Connected to AI backend");
    }
}

/// Example 2: User sends a new task
pub fn example_send_task(bridge: &BridgeClient, user_input: &str) {
    let message = OutboundMessage::NewTask {
        text: user_input.to_string(),
        images: vec![],
        model: Some("claude-sonnet-4".to_string()),
        mode: Some("Code".to_string()),
    };
    
    match bridge.send(message) {
        Ok(_) => println!("[BRIDGE EXAMPLE] Task sent successfully"),
        Err(e) => eprintln!("[BRIDGE EXAMPLE] Send failed: {}", e),
    }
}

/// Example 3: Poll for responses in UI event loop
pub fn example_event_loop(bridge: &BridgeClient) {
    // This would be called in Lapce's main event loop
    while let Some(message) = bridge.try_receive() {
        println!("[BRIDGE EXAMPLE] Received: {:?}", message);
        
        // Match on message type and update UI accordingly
        // e.g., display streaming text, show ask prompts, etc.
    }
}

/// Example 4: Handle disconnection/reconnection
pub fn example_reconnect(bridge: &BridgeClient) {
    use super::messages::ConnectionStatusType;
    
    let status = bridge.status();
    
    match status {
        ConnectionStatusType::Disconnected => {
            println!("[BRIDGE EXAMPLE] Backend disconnected, attempting reconnect...");
            if let Err(e) = bridge.connect() {
                eprintln!("[BRIDGE EXAMPLE] Reconnect failed: {}", e);
            }
        }
        ConnectionStatusType::Connected => {
            println!("[BRIDGE EXAMPLE] Backend connected");
        }
        ConnectionStatusType::Connecting => {
            println!("[BRIDGE EXAMPLE] Connection in progress...");
        }
    }
}
