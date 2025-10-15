/// Multi-Process IPC Test - Verifies true inter-process communication
/// Uses fork() or separate processes to test real shared memory IPC

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const TEST_SOCKET: &str = "/tmp/lapce-multiprocess-test.sock";

#[tokio::test]
async fn test_multiprocess_ipc() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ Multi-Process IPC Test: True Inter-Process Communication    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    let lock_dir = format!("{}_locks", TEST_SOCKET);
    let _ = std::fs::remove_dir_all(&lock_dir);
    
    // Start server in separate process
    println!("🚀 Starting server process...");
    let mut server_process = Command::new(std::env::current_exe().unwrap())
        .arg("--test")
        .arg("multiprocess_server")
        .arg("--exact")
        .arg("--nocapture")
        .spawn()
        .expect("Failed to start server process");
    
    // Wait for server to be ready
    sleep(Duration::from_secs(2)).await;
    println!("✅ Server process started (PID: {})", server_process.id());
    
    // Start client in separate process
    println!("🚀 Starting client process...");
    let client_output = Command::new(std::env::current_exe().unwrap())
        .arg("--test")
        .arg("multiprocess_client")
        .arg("--exact")
        .arg("--nocapture")
        .output()
        .expect("Failed to run client process");
    
    let client_stdout = String::from_utf8_lossy(&client_output.stdout);
    let client_stderr = String::from_utf8_lossy(&client_output.stderr);
    
    println!("📊 Client output:\n{}", client_stdout);
    if !client_stderr.is_empty() {
        println!("⚠️  Client stderr:\n{}", client_stderr);
    }
    
    // Check if client succeeded
    if client_output.status.success() {
        println!("✅ Client process completed successfully");
    } else {
        println!("❌ Client process failed with status: {:?}", client_output.status);
    }
    
    // Kill server
    server_process.kill().expect("Failed to kill server");
    let _ = server_process.wait();
    println!("🛑 Server process terminated");
    
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    let _ = std::fs::remove_dir_all(&lock_dir);
    
    // Assert client succeeded
    assert!(client_output.status.success(), "Multi-process IPC test failed");
}

#[tokio::test]
async fn multiprocess_server() {
    use lapce_ai_rust::ipc::ipc_server::IpcServer;
    use lapce_ai_rust::ipc::binary_codec::MessageType;
    use bytes::Bytes;
    
    println!("[SERVER PROCESS] PID={} Starting...", std::process::id());
    
    // Cleanup
    let _ = std::fs::remove_file(TEST_SOCKET);
    
    // Create server
    let server = Arc::new(IpcServer::new(TEST_SOCKET).await.expect("Server creation failed"));
    
    // Register echo handler
    server.register_handler(MessageType::CompletionRequest, |data: Bytes| async move {
        println!("[SERVER PROCESS] Received {} bytes, echoing back", data.len());
        Ok(data)
    });
    
    println!("[SERVER PROCESS] Ready, serving...");
    
    // Serve forever (will be killed by test)
    server.serve().await.expect("Server failed");
}

#[tokio::test]
async fn multiprocess_client() {
    use lapce_ai_rust::ipc::ipc_client::IpcClient;
    
    println!("[CLIENT PROCESS] PID={} Starting...", std::process::id());
    
    // Wait a bit for server
    sleep(Duration::from_millis(500)).await;
    
    // Connect
    let client = IpcClient::connect(TEST_SOCKET).await.expect("Client connection failed");
    println!("[CLIENT PROCESS] Connected to server");
    
    // Wait for server to accept
    sleep(Duration::from_millis(500)).await;
    
    // Create test message
    let payload = b"Hello from separate process!";
    let msg_type = 1u16; // CompletionRequest
    let msg_id = 42u64;
    
    // Build 24-byte header
    let mut header = vec![0u8; 24];
    header[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());  // Magic
    header[4] = 1;  // Version
    header[5] = 0;  // Flags
    header[6..8].copy_from_slice(&msg_type.to_le_bytes());  // Type
    header[8..12].copy_from_slice(&(payload.len() as u32).to_le_bytes());  // Length
    header[12..20].copy_from_slice(&msg_id.to_le_bytes());  // Message ID
    header[20..24].fill(0);  // CRC placeholder
    
    // Calculate CRC32
    let mut full_msg = Vec::new();
    full_msg.extend_from_slice(&header);
    full_msg.extend_from_slice(payload);
    let crc = crc32fast::hash(&full_msg);
    header[20..24].copy_from_slice(&crc.to_le_bytes());
    
    // Rebuild with correct CRC
    full_msg.clear();
    full_msg.extend_from_slice(&header);
    full_msg.extend_from_slice(payload);
    
    // Send messages
    let iterations = 10;
    let start = Instant::now();
    let mut successful = 0;
    
    for i in 0..iterations {
        match client.send_bytes(&full_msg).await {
            Ok(response) => {
                successful += 1;
                println!("[CLIENT PROCESS] Message {} round-trip: {} bytes sent, {} bytes received", 
                    i, full_msg.len(), response.len());
            },
            Err(e) => {
                println!("[CLIENT PROCESS] Message {} failed: {}", i, e);
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!("\n[CLIENT PROCESS] Results:");
    println!("  Messages: {}/{} successful", successful, iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Avg latency: {:.2} µs", elapsed.as_micros() as f64 / iterations as f64);
    
    // Exit with success if all messages succeeded
    if successful == iterations {
        println!("[CLIENT PROCESS] ✅ All messages succeeded!");
        std::process::exit(0);
    } else {
        println!("[CLIENT PROCESS] ❌ Some messages failed");
        std::process::exit(1);
    }
}
