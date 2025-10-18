/// Shared Memory IPC Transport for AI Bridge
/// Connects Lapce UI to lapce-ai-rust backend via IPC

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use super::bridge::BridgeError;
use super::messages::{ConnectionStatusType, InboundMessage, OutboundMessage};
use super::transport::Transport;

// Platform-specific IPC clients - internal to ShmTransport only
#[cfg(unix)]
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

#[cfg(windows)]
use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryStream;

/// ShmTransport: Real IPC connection to lapce-ai backend
pub struct ShmTransport {
    client: Arc<Mutex<Option<IpcClientHandle>>>,
    inbound_queue: Arc<Mutex<VecDeque<InboundMessage>>>,
    status: Arc<Mutex<ConnectionStatusType>>,
    socket_path: String,
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Handle to IPC client with runtime (platform-agnostic wrapper)
struct IpcClientHandle {
    #[cfg(unix)]
    client: IpcClientVolatile,
    
    #[cfg(windows)]
    client: SharedMemoryStream,
}

impl ShmTransport {
    /// Create new transport (disconnected initially)
    pub fn new(socket_path: impl Into<String>) -> Self {
        // Create dedicated runtime for IPC operations
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ai-bridge-ipc")
            .enable_all()
            .build()
            .expect("Failed to create IPC runtime");
        
        Self {
            client: Arc::new(Mutex::new(None)),
            inbound_queue: Arc::new(Mutex::new(VecDeque::new())),
            status: Arc::new(Mutex::new(ConnectionStatusType::Disconnected)),
            socket_path: socket_path.into(),
            runtime: Arc::new(runtime),
        }
    }
}

impl Transport for ShmTransport {
    fn send(&self, message: OutboundMessage) -> Result<(), BridgeError> {
        // Route messages through binary protocol
        let serialized = match &message {
            OutboundMessage::LspRequest { id, method, uri, language_id, params } => {
                // Encode as binary LspRequest message
                Self::encode_lsp_request(id, method, uri, language_id, params)?
            }
            OutboundMessage::LspCancel { request_id } => {
                // Encode as binary Cancel message
                Self::encode_lsp_cancel(request_id)?
            }
            OutboundMessage::ProviderChatStream { model, messages, max_tokens, temperature } => {
                // Encode as binary ChatMessage for streaming
                Self::encode_provider_chat_stream(model, messages, max_tokens, temperature)?
            }
            _ => {
                // All other messages use JSON protocol
                serde_json::to_vec(&message)
                    .map_err(|e| BridgeError::SerializationError(e.to_string()))?
            }
        };

        let runtime = self.runtime.clone();
        let client_guard = self.client.lock().unwrap();
        let handle = client_guard.as_ref().ok_or(BridgeError::Disconnected)?;

        #[cfg(unix)]
        {
            // Clone the IPC client (cheap; it holds Arcs internally)
            let ipc_client = handle.client.clone();
            // Send bytes to backend; backend echoes or responds per BinaryCodec handlers
            let response = runtime
                .block_on(async move { ipc_client.send_bytes(&serialized).await })
                .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

            // If backend returned a UI message, enqueue it
            if !response.is_empty() {
                // Try binary LSP decode first, then JSON fallback
                match Self::decode_lsp_response(&response) {
                    Ok(Some(msg)) => {
                        self.inbound_queue.lock().unwrap().push_back(msg);
                    }
                    Ok(None) | Err(_) => {
                        // Fallback to JSON decode for non-LSP messages
                        if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
                            self.inbound_queue.lock().unwrap().push_back(msg);
                        }
                    }
                }
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            let ipc_client = handle.client.clone();
            let response = runtime
                .block_on(async move { 
                    ipc_client.send(&serialized).await
                        .and_then(|_| ipc_client.recv().await)
                        .map(|opt| opt.unwrap_or_default())
                })
                .map_err(|e| BridgeError::SendFailed(format!("Windows IPC error: {}", e)))?;

            if !response.is_empty() {
                // Try binary LSP decode first, then JSON fallback
                match Self::decode_lsp_response(&response) {
                    Ok(Some(msg)) => {
                        self.inbound_queue.lock().unwrap().push_back(msg);
                    }
                    Ok(None) | Err(_) => {
                        // Fallback to JSON decode for non-LSP messages
                        if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
                            self.inbound_queue.lock().unwrap().push_back(msg);
                        }
                    }
                }
            }
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(BridgeError::SendFailed(
                "IPC transport not available on this platform".into(),
            ))
        }
    }
    
    fn try_receive(&self) -> Option<InboundMessage> {
        let mut queue = self.inbound_queue.lock().unwrap();
        queue.pop_front()
    }
    
    fn status(&self) -> ConnectionStatusType {
        self.status.lock().unwrap().clone()
    }
    
    fn connect(&mut self) -> Result<(), BridgeError> {
        let socket_path = self.socket_path.clone();
        let runtime = self.runtime.clone();
        
        eprintln!("[SHM_TRANSPORT] Connecting to: {}", socket_path);

        #[cfg(unix)]
        {
            // Real IPC connection to lapce-ai backend
            let ipc_client = runtime
                .block_on(async { IpcClientVolatile::connect(&socket_path).await })
                .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

            let handle = IpcClientHandle { client: ipc_client };
            *self.client.lock().unwrap() = Some(handle);
            *self.status.lock().unwrap() = ConnectionStatusType::Connected;
            eprintln!("[SHM_TRANSPORT] Connected via real IPC");
            
            // Start background receiver task for streaming messages
            self.start_receiver_task();
            
            Ok(())
        }

        #[cfg(windows)]
        {
            let ipc_client = runtime
                .block_on(async { SharedMemoryStream::connect(&socket_path).await })
                .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

            let handle = IpcClientHandle { client: ipc_client };
            *self.client.lock().unwrap() = Some(handle);
            *self.status.lock().unwrap() = ConnectionStatusType::Connected;
            eprintln!("[SHM_TRANSPORT] Connected via Windows IPC");
            
            // Start background receiver task for streaming messages
            self.start_receiver_task();
            
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(BridgeError::ConnectionFailed(
                "IPC transport not available on this platform".into(),
            ))
        }
    }
    
    fn disconnect(&mut self) -> Result<(), BridgeError> {
        *self.client.lock().unwrap() = None;
        *self.status.lock().unwrap() = ConnectionStatusType::Disconnected;
        
        eprintln!("[SHM_TRANSPORT] Disconnected");
        Ok(())
    }
}

// ============================================================================
// LSP Message Encoding (Binary Protocol)
// ============================================================================

impl ShmTransport {
    /// Encode LSP request as binary message
    fn encode_lsp_request(
        id: &str,
        method: &str,
        uri: &str,
        language_id: &str,
        params: &serde_json::Value,
    ) -> Result<Vec<u8>, BridgeError> {
        use lapce_ai_rust::ipc::binary_codec::{
            BinaryCodec, Message, MessageType, MessagePayload, LspRequestPayload,
        };

        let params_json = serde_json::to_string(params)
            .map_err(|e| BridgeError::SerializationError(format!("LSP params: {}", e)))?;

        let payload = LspRequestPayload {
            id: id.to_string(),
            method: method.to_string(),
            uri: uri.to_string(),
            language_id: language_id.to_string(),
            params_json,
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = Message {
            id: timestamp, // Use timestamp as message ID
            msg_type: MessageType::LspRequest,
            payload: MessagePayload::LspRequest(payload),
            timestamp,
        };

        let mut codec = BinaryCodec::new();
        codec.encode(&msg)
            .map(|bytes| bytes.to_vec())
            .map_err(|e| BridgeError::SerializationError(format!("Binary codec: {}", e)))
    }

    /// Encode LSP cancel as binary message
    fn encode_lsp_cancel(request_id: &str) -> Result<Vec<u8>, BridgeError> {
        use lapce_ai_rust::ipc::binary_codec::{
            BinaryCodec, Message, MessageType, MessagePayload,
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = Message {
            id: timestamp, // Use timestamp as message ID
            msg_type: MessageType::Cancel,
            payload: MessagePayload::Cancel {
                request_id: request_id.to_string(),
            },
            timestamp,
        };

        let mut codec = BinaryCodec::new();
        codec.encode(&msg)
            .map(|bytes| bytes.to_vec())
            .map_err(|e| BridgeError::SerializationError(format!("Binary codec: {}", e)))
    }

    /// Decode binary LSP response
    fn decode_lsp_response(data: &[u8]) -> Result<Option<InboundMessage>, BridgeError> {
        use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, MessageType, MessagePayload};

        let mut codec = BinaryCodec::new();
        let msg = codec.decode(data)
            .map_err(|e| BridgeError::SerializationError(format!("Decode error: {}", e)))?;

        match msg.payload {
            MessagePayload::LspResponse(payload) => {
                let result = if payload.ok {
                    serde_json::from_str(&payload.result_json).ok()
                } else {
                    None
                };

                Ok(Some(InboundMessage::LspResponse {
                    id: payload.id,
                    ok: payload.ok,
                    result,
                    error: payload.error,
                    error_code: payload.error_code,
                }))
            }
            MessagePayload::LspDiagnostics(payload) => {
                let diagnostics: Vec<super::messages::LspDiagnostic> =
                    serde_json::from_str(&payload.diagnostics_json)
                        .unwrap_or_default();

                Ok(Some(InboundMessage::LspDiagnostics {
                    uri: payload.uri,
                    version: payload.version,
                    diagnostics,
                }))
            }
            MessagePayload::LspProgress(payload) => {
                let value: serde_json::Value = serde_json::from_str(&payload.value_json)
                    .unwrap_or(serde_json::json!({}));

                let kind = value.get("kind")
                    .and_then(|k| k.as_str())
                    .map(|s| match s {
                        "begin" => super::messages::LspProgressKind::Begin,
                        "report" => super::messages::LspProgressKind::Report,
                        _ => super::messages::LspProgressKind::End,
                    })
                    .unwrap_or(super::messages::LspProgressKind::Begin);

                Ok(Some(InboundMessage::LspProgress {
                    token: payload.token,
                    kind,
                    title: value.get("title").and_then(|t| t.as_str()).map(String::from),
                    message: value.get("message").and_then(|m| m.as_str()).map(String::from),
                    percentage: value.get("percentage").and_then(|p| p.as_u64()).map(|p| p as u32),
                }))
            }
            _ => {
                // Not an LSP message, return None to try JSON decode
                Ok(None)
            }
        }
    }
    
    /// Encode ProviderChatStream as binary message (for AI chat)
    fn encode_provider_chat_stream(
        model: &str,
        messages: &[super::messages::ProviderChatMessage],
        max_tokens: &Option<u32>,
        temperature: &Option<f32>,
    ) -> Result<Vec<u8>, BridgeError> {
        use lapce_ai_rust::ipc::ipc_messages::ProviderChatStreamRequest;
        
        let request = ProviderChatStreamRequest {
            model: model.to_string(),
            messages: messages.iter().map(|m| {
                lapce_ai_rust::ipc::ipc_messages::ProviderMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                }
            }).collect(),
            max_tokens: *max_tokens,
            temperature: *temperature,
        };
        
        // Serialize as JSON (backend handler expects JSON-deserialized ProviderChatStreamRequest)
        serde_json::to_vec(&request)
            .map_err(|e| BridgeError::SerializationError(format!("ProviderChatStream: {}", e)))
    }
}

// ============================================================================
// Background receiver task (for async message handling)
// ============================================================================

impl ShmTransport {
    /// Start background task to receive messages from IPC
    /// This enables streaming messages (provider chunks, tool progress, LSP events)
    /// to be received without blocking on send() calls
    pub fn start_receiver_task(&self) {
        let client = self.client.clone();
        let queue = self.inbound_queue.clone();
        let status = self.status.clone();
        let runtime = self.runtime.clone();
        
        eprintln!("[SHM_TRANSPORT] Starting background receiver task");
        
        // Spawn background task on the IPC runtime
        runtime.spawn(async move {
            let mut poll_interval = tokio::time::interval(std::time::Duration::from_millis(16)); // 60fps polling
            
            loop {
                poll_interval.tick().await;
                
                // Check if still connected
                {
                    let status_guard = status.lock().unwrap();
                    if *status_guard != ConnectionStatusType::Connected {
                        eprintln!("[SHM_TRANSPORT] Receiver task stopping (disconnected)");
                        break;
                    }
                }
                
                // For request-response IPC, streaming messages are handled via the send() path
                // This task is mainly for future async/push-based messaging support
                // Currently, the backend sends streaming chunks as responses to ProviderChatStream requests
                // The polling loop in ai_chat_view.rs handles those via try_receive()
                
                // If we add unsolicited push messages in the future, handle them here:
                // e.g., tool lifecycle events, LSP diagnostics, etc.
            }
            
            eprintln!("[SHM_TRANSPORT] Receiver task terminated");
        });
    }
}
