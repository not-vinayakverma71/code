#![cfg(unix)]

/// Volatile IPC Test Server - for comprehensive_multiprocess_ipc test
/// Uses control socket + volatile buffers instead of directory watching

use std::sync::Arc;
use lapce_ai_rust::ipc::ipc_server_volatile::IpcServerVolatile;
use lapce_ai_rust::ipc::binary_codec::MessageType;
use lapce_ai_rust::dispatcher::MessageDispatcher;
use bytes::Bytes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let socket_path = args.get(1)
        .expect("Usage: ipc_test_server_volatile <socket_path>");
    
    eprintln!("[SERVER] Starting volatile IPC server on: {}", socket_path);
    eprintln!("[SERVER] Process ID: {}", std::process::id());
    
    // Create server and dispatcher
    let server = IpcServerVolatile::new(socket_path).await?;
    let dispatcher = Arc::new(MessageDispatcher::new());
    
    // Register handler that routes through dispatcher
    for msg_type in [
        MessageType::CompletionRequest,
        MessageType::CompletionResponse,
        MessageType::StreamChunk,
        MessageType::Error,
    ] {
        let dispatcher = dispatcher.clone();
        server.register_handler(msg_type, move |data: Bytes| {
            let dispatcher = dispatcher.clone();
            async move {
                eprintln!("[SERVER] Received {} bytes, routing to dispatcher", data.len());
                
                // Parse UI message (JSON OutboundMessage from bridge)
                let ui_msg: lapce_ai_rust::dispatcher::InboundMessage = 
                    serde_json::from_slice(&data)
                        .unwrap_or_else(|e| {
                            eprintln!("[SERVER] Failed to parse UI message: {}", e);
                            // Fallback: create a cancel task message
                            lapce_ai_rust::dispatcher::InboundMessage::CancelTask
                        });
                
                // Route through dispatcher
                let responses = dispatcher.dispatch(ui_msg).await
                    .unwrap_or_else(|e| {
                        eprintln!("[SERVER] Dispatcher error: {}", e);
                        vec![lapce_ai_rust::dispatcher::OutboundMessage::Error {
                            message: e.to_string(),
                        }]
                    });
                
                // Serialize first response back to UI as InboundMessage JSON
                // (UI expects lapce-app InboundMessage, so we need a translation layer)
                let response_json = if let Some(resp) = responses.first() {
                    // For now, echo the response as JSON bytes
                    // TODO: Add proper translation from dispatcher::OutboundMessage to UI InboundMessage
                    serde_json::to_vec(resp).unwrap_or_default()
                } else {
                    vec![]
                };
                
                eprintln!("[SERVER] Returning {} bytes to UI", response_json.len());
                Ok(Bytes::from(response_json))
            }
        });
    }
    
    eprintln!("[SERVER] Handlers registered, starting server...");
    
    // Serve forever
    server.serve().await?;
    
    Ok(())
}
