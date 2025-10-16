/// Simple volatile IPC test - single message round-trip
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[cfg(unix)]
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

const TEST_SOCKET: &str = "/tmp/test_volatile_simple.sock";

#[tokio::test]
async fn test_volatile_single_message() {
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    
    println!("\n[TEST] Starting volatile IPC server...");
    
    // Spawn server
    let mut server = Command::new("cargo")
        .args(&["run", "--release", "--bin", "ipc_test_server_volatile", "--", TEST_SOCKET])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");
    
    println!("[TEST] Server PID: {}", server.id());
    
    // Wait for server to start
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Connect client
    println!("[TEST] Connecting client...");
    let client = match timeout(Duration::from_secs(5), IpcClientVolatile::connect(TEST_SOCKET)).await {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            let _ = server.kill();
            panic!("Client connection failed: {}", e);
        }
        Err(_) => {
            let _ = server.kill();
            panic!("Client connection timeout");
        }
    };
    
    println!("[TEST] ✓ Client connected");
    
    // Send single message
    let data = b"Hello volatile IPC!";
    println!("[TEST] Sending: {:?}", std::str::from_utf8(data).unwrap());
    
    let response = match timeout(Duration::from_secs(10), client.send_bytes(data)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            let _ = server.kill();
            panic!("Send failed: {}", e);
        }
        Err(_) => {
            let _ = server.kill();
            panic!("Send timeout");
        }
    };
    
    println!("[TEST] Received: {} bytes", response.len());
    println!("[TEST] Data: {:?}", std::str::from_utf8(&response).unwrap());
    
    assert_eq!(response, data, "Response mismatch");
    
    println!("\n✅ TEST PASSED: Volatile IPC works!");
    
    let _ = server.kill();
    let _ = std::fs::remove_file(TEST_SOCKET);
}
