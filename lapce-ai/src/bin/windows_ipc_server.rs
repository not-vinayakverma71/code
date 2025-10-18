/// Windows IPC server binary for cross-process testing
/// Separate process is required for shared memory atomics to work correctly

#![cfg(windows)]

use anyhow::Result;
use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryListener;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }
    
    let socket_path = &args[1];
    eprintln!("[SERVER] Binding to {}", socket_path);
    
    let listener = SharedMemoryListener::bind(socket_path)?;
    eprintln!("[SERVER] Listening on {}", socket_path);
    
    // Accept connections and echo data back
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                eprintln!("[SERVER] Accepted connection");
                tokio::spawn(async move {
                    loop {
                        match stream.recv().await {
                            Ok(Some(data)) => {
                                if stream.send(&data).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => tokio::time::sleep(tokio::time::Duration::from_millis(1)).await,
                            Err(_) => break,
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("[SERVER] Accept error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
