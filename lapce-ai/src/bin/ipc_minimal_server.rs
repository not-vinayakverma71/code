/// Minimal IPC server for memory baseline measurement
/// Build with: cargo build --release --bin ipc_minimal_server
/// Run with: target/release/ipc_minimal_server
/// Measure RSS via: cat /proc/<pid>/status | grep VmRSS

use anyhow::Result;
use lapce_ai_rust::ipc::ipc_server::IpcServer;
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Minimal setup - no logging, no extra allocations
    let socket_path = "/tmp/lapce-ipc-minimal.sock";
    
    // Clean up socket if it exists
    let _ = std::fs::remove_file(socket_path);
    
    // Create minimal IPC server (connection pool included)
    let server = Arc::new(IpcServer::new(socket_path).await?);
    
    // Print PID for measurement
    println!("IPC Server PID: {}", std::process::id());
    println!("Waiting for memory measurement...");
    
    // Wait forever (allows external measurement)
    tokio::signal::ctrl_c().await?;
    
    Ok(())
}
