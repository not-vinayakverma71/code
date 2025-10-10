// Tool execution router for IPC messages - P1-1c
// Bridges IPC server to ToolExecutionHandler

use std::sync::Arc;
use bytes::Bytes;
use anyhow::Result;

use crate::handlers::tools::{ToolExecutionHandler, ToolExecutionRequest};
use crate::ipc::binary_codec::{Message, MessageType, MessagePayload};

/// Register tool execution handlers with IPC server
pub fn register_tool_handlers(
    server: &mut crate::ipc::ipc_server::IpcServer,
) -> Result<()> {
    let handler = Arc::new(ToolExecutionHandler::new());
    
    // Register handler for ExecuteTool message type
    server.register_handler(
        MessageType::ExecuteTool,
        create_tool_handler(handler.clone()),
    );
    
    // Register handler for tool streaming updates
    server.register_handler(
        MessageType::ToolProgress,
        create_progress_handler(),
    );
    
    Ok(())
}

/// Create handler for ExecuteTool messages
fn create_tool_handler(
    handler: Arc<ToolExecutionHandler>,
) -> Box<dyn Fn(Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Bytes, crate::ipc::errors::IpcError>> + Send>> + Send + Sync> {
    Box::new(move |data: Bytes| {
        let handler = handler.clone();
        Box::pin(async move {
            // Decode the request
            let mut codec = crate::ipc::binary_codec::BinaryCodec::new();
            let msg = codec.decode(&data)?;
            
            // Extract tool execution request from payload
            let request = match msg.payload {
                MessagePayload::ExecuteTool { tool_name, params, workspace_path, user_id, correlation_id, require_approval } => {
                    // Parse JSON params from string
                    let params_value = serde_json::from_str(&params)
                        .map_err(|e| crate::ipc::errors::IpcError::protocol(
                            format!("Invalid JSON params: {}", e)
                        ))?;
                    
                    ToolExecutionRequest {
                        tool_name,
                        params: params_value,
                        workspace_path,
                        user_id,
                        correlation_id,
                        require_approval,
                    }
                }
                _ => {
                    return Err(crate::ipc::errors::IpcError::protocol(
                        "Invalid message payload for ExecuteTool"
                    ));
                }
            };
            
            // Execute the tool
            let response = handler.execute(request).await
                .map_err(|e| crate::ipc::errors::IpcError::handler(e.to_string()))?;
            
            // Encode response
            let response_msg = Message {
                id: msg.id,
                msg_type: MessageType::ToolResult,
                payload: MessagePayload::ToolResult {
                    correlation_id: response.correlation_id,
                    success: response.error.is_none(),
                    result: response.result.map(|v| serde_json::to_string(&v).unwrap_or_default()),
                    error: response.error,
                },
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let response_bytes = codec.encode(&response_msg)
                .map_err(|e| crate::ipc::errors::IpcError::codec("binary", e.to_string()))?;
            
            Ok(response_bytes)
        })
    })
}

/// Create handler for tool progress messages
fn create_progress_handler() -> Box<dyn Fn(Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Bytes, crate::ipc::errors::IpcError>> + Send>> + Send + Sync> {
    Box::new(move |data: Bytes| {
        Box::pin(async move {
            // For now, just acknowledge progress updates
            let mut codec = crate::ipc::binary_codec::BinaryCodec::new();
            let msg = codec.decode(&data)?;
            
            // Create acknowledgment
            let ack_msg = Message {
                id: msg.id,
                msg_type: MessageType::Ack,
                payload: MessagePayload::Ack,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            let response_bytes = codec.encode(&ack_msg)
                .map_err(|e| crate::ipc::errors::IpcError::codec("binary", e.to_string()))?;
            
            Ok(response_bytes)
        })
    })
}
