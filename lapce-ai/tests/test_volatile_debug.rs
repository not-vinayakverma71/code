/// Debug test - single client with detailed logging
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[cfg(unix)]
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

const TEST_SOCKET: &str = "/tmp/test_volatile_debug.sock";

#[tokio::test]
async fn test_single_message_debug() {
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  DEBUG TEST: Single Message with Detailed Logging       ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    // Spawn server
    println!("[TEST] Starting server...");
    let mut server = Command::new("./target/release/ipc_test_server_volatile")
        .arg(TEST_SOCKET)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn server");
    
    println!("[TEST] Server PID: {}", server.id());
    
    // Wait for server
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Connect client
    println!("\n[TEST] Connecting client...");
    let client = timeout(Duration::from_secs(10), IpcClientVolatile::connect(TEST_SOCKET))
        .await
        .expect("Connection timeout")
        .expect("Connection failed");
    
    println!("[TEST] ✓ Client connected\n");
    
    // Send one message
    let data = b"Test message with logging";
    println!("[TEST] Sending message: {:?}\n", std::str::from_utf8(data).unwrap());
    
    let response = timeout(Duration::from_secs(10), client.send_bytes(data))
        .await
        .expect("Send timeout")
        .expect("Send failed");
    
    println!("\n[TEST] ✓ Received response: {} bytes", response.len());
    println!("[TEST] Response: {:?}", std::str::from_utf8(&response).unwrap());
    
    assert_eq!(response, data, "Response mismatch");
    
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  ✅ TEST PASSED                                          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    let _ = server.kill();
    let _ = std::fs::remove_file(TEST_SOCKET);
}
