/// Simple IPC Server Test - Minimal version to debug startup
use std::sync::Arc;
use anyhow::Result;

use lapce_ai_rust::{
    shared_memory_complete::{SharedMemoryListener, SharedMemoryStream},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting simple SharedMemory test server...");
    
    // Test SharedMemoryListener directly
    let mut listener = SharedMemoryListener::bind("test_ipc")?;
    println!("SharedMemoryListener bound to 'test_ipc'");
    
    // Accept connections in a loop
    tokio::spawn(async move {
        println!("Waiting for connections...");
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    println!("Got connection!");
                    
                    // Handle connection
                    tokio::spawn(async move {
                        let mut buf = [0u8; 1024];
                        loop {
                            // Read message length
                            if stream.read_exact(&mut buf[..4]).await.is_err() {
                                break;
                            }
                            let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                            
                            if len > 1024 {
                                break;
                            }
                            
                            // Read message
                            if stream.read_exact(&mut buf[..len]).await.is_err() {
                                break;
                            }
                            
                            // Echo back
                            let _ = stream.write_all(&(len as u32).to_le_bytes()).await;
                            let _ = stream.write_all(&buf[..len]).await;
                        }
                    });
                },
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Keep server running
    println!("Server running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");
    
    Ok(())
}
