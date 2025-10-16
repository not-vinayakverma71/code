/// Comprehensive Multi-Process IPC Integration Test - Windows Version
/// Tests IPC with separate processes using Windows CreateFileMapping API

use std::process::{Command, Child, Stdio};
use std::time::Duration;

const TEST_SOCKET: &str = "\\\\.\\pipe\\test_comprehensive_multiprocess_ipc_windows";

#[cfg(windows)]
fn spawn_server_process() -> Result<Child, std::io::Error> {
    eprintln!("[TEST] Spawning Windows IPC server...");
    
    let binary_path = std::env::current_exe()
        .ok()
        .and_then(|test_exe| {
            test_exe.parent()
                .and_then(|deps| deps.parent())
                .map(|debug| debug.join("ipc_test_server.exe"))
        })
        .filter(|p| p.exists());
    
    let child = if let Some(bin_path) = binary_path {
        Command::new(bin_path)
            .arg(TEST_SOCKET)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new("cargo")
            .args(&["run", "--bin", "ipc_test_server", "--", TEST_SOCKET])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    
    std::thread::sleep(Duration::from_secs(3));
    Ok(child)
}

#[cfg(windows)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_windows_comprehensive_multiprocess_ipc() {
    use lapce_ai_rust::ipc::ipc_client::IpcClient;
    use lapce_ai_rust::ipc::binary_codec::MessageType;
    use std::time::Instant;
    
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ COMPREHENSIVE MULTI-PROCESS IPC TEST - Windows           ║");
    println!("║ Testing REAL server/client in SEPARATE OS processes      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
    
    // Spawn server
    let mut server = spawn_server_process().expect("Failed to spawn server");
    std::thread::sleep(Duration::from_secs(2));
    
    // Connect client
    println!("[TEST] Connecting client...");
    let client = IpcClient::connect(TEST_SOCKET).await.expect("Failed to connect");
    
    // Performance test
    const NUM_MESSAGES: usize = 1000;
    let test_data = b"Hello from Windows IPC test!";
    
    println!("[TEST] Starting performance test: {} messages", NUM_MESSAGES);
    let start = Instant::now();
    
    for i in 0..NUM_MESSAGES {
        let response = client.send_bytes(test_data).await.expect("Send failed");
        assert!(!response.is_empty(), "Empty response at iteration {}", i);
    }
    
    let duration = start.elapsed();
    let msg_per_sec = (NUM_MESSAGES as f64) / duration.as_secs_f64();
    
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ WINDOWS IPC PERFORMANCE RESULTS                           ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║ Messages:     {:>10}                                   ║", NUM_MESSAGES);
    println!("║ Duration:     {:>10.3} sec                            ║", duration.as_secs_f64());
    println!("║ Throughput:   {:>10.0} msg/sec                        ║", msg_per_sec);
    println!("║ Latency (avg): {:>9.3} ms                             ║", (duration.as_secs_f64() * 1000.0) / NUM_MESSAGES as f64);
    println!("╚═══════════════════════════════════════════════════════════╝\n");
    
    // Cleanup
    let _ = server.kill();
    
    println!("✅ Windows multi-process IPC test PASSED");
}
