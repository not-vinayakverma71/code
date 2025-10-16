#![cfg(unix)]

/// Volatile IPC Test Server - for comprehensive_multiprocess_ipc test
/// Uses control socket + volatile buffers instead of directory watching

use std::sync::Arc;
use lapce_ai_rust::ipc::ipc_server_volatile::IpcServerVolatile;
use lapce_ai_rust::ipc::binary_codec::MessageType;
use bytes::Bytes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let socket_path = args.get(1)
        .expect("Usage: ipc_test_server_volatile <socket_path>");
    
    eprintln!("[SERVER] Starting volatile IPC server on: {}", socket_path);
    eprintln!("[SERVER] Process ID: {}", std::process::id());
    
    // Create server
    let server = IpcServerVolatile::new(socket_path).await?;
    
    // Register echo handler for all message types
    for msg_type in [
        MessageType::CompletionRequest,
        MessageType::CompletionResponse,
        MessageType::StreamChunk,
        MessageType::Error,
    ] {
        server.register_handler(msg_type, |data: Bytes| async move {
            eprintln!("[SERVER] Handler echoing {} bytes", data.len());
            Ok(data)
        });
    }
    
    eprintln!("[SERVER] Handlers registered, starting server...");
    
    // Serve forever
    server.serve().await?;
    
    Ok(())
}
