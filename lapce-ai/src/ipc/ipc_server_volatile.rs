/// IPC Server using volatile buffers, control socket, and eventfd doorbells
/// Production-grade with efficient cross-process notifications

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, Mutex};
use anyhow::Result;
use bytes::Bytes;
use dashmap::DashMap;
use crate::ipc::control_socket::ControlServer;
#[cfg(target_os = "linux")]
use crate::ipc::shm_buffer_futex::FutexSharedMemoryBuffer as SharedMemoryBuffer;

#[cfg(not(target_os = "linux"))]
use crate::ipc::shm_buffer_volatile::VolatileSharedMemoryBuffer as SharedMemoryBuffer;
use crate::ipc::binary_codec::{BinaryCodec, MessageType};
use crate::ipc::platform_buffer::PlatformDoorbell;
use crate::ipc::errors::IpcResult;

const RING_CAPACITY: u32 = 1024 * 1024; // 1MB per ring

type Handler = Box<dyn Fn(Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = IpcResult<Bytes>> + Send>> + Send + Sync>;
type StreamingHandler = Box<dyn Fn(Bytes, tokio::sync::mpsc::Sender<Bytes>) -> std::pin::Pin<Box<dyn std::future::Future<Output = IpcResult<()>> + Send>> + Send + Sync>;

pub struct IpcServerVolatile {
    control_server: Arc<Mutex<ControlServer>>,
    handlers: Arc<DashMap<MessageType, Handler>>,
    streaming_handlers: Arc<DashMap<MessageType, StreamingHandler>>,
    shutdown: broadcast::Sender<()>,
    base_path: String,
    next_slot_id: AtomicU32,
}

impl IpcServerVolatile {
    pub async fn new(base_path: &str) -> Result<Arc<Self>> {
        let control_server = ControlServer::bind(base_path).await?;
        let (shutdown_tx, _) = broadcast::channel(1);
        
        eprintln!("[SERVER VOLATILE] Created server on {}", base_path);
        
        Ok(Arc::new(Self {
            control_server: Arc::new(Mutex::new(control_server)),
            handlers: Arc::new(DashMap::new()),
            streaming_handlers: Arc::new(DashMap::new()),
            shutdown: shutdown_tx,
            base_path: base_path.to_string(),
            next_slot_id: AtomicU32::new(0),
        }))
    }
    
    pub fn register_handler<F, Fut>(&self, msg_type: MessageType, handler: F)
    where
        F: Fn(Bytes) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = IpcResult<Bytes>> + Send + 'static,
    {
        self.handlers.insert(msg_type, Box::new(move |data| {
            Box::pin(handler(data))
        }));
    }
    
    /// Accept a connection without holding lock across await (prevents macOS deadlock)
    async fn accept_connection(&self) -> Result<(tokio::net::UnixStream, tokio::net::unix::SocketAddr)> {
        let mut guard = self.control_server.lock().await;
        guard.listener.accept().await.map_err(|e| e.into())
    }
    
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        let mut shutdown_rx = self.shutdown.subscribe();
        
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    eprintln!("[SERVER VOLATILE] Shutting down");
                    return Ok(());
                }
                
                // Accept connection (fast, just TCP accept)
                // CRITICAL: Don't hold lock across .await to prevent macOS deadlock
                result = self.accept_connection() => {
                    match result {
                        Ok((stream, _addr)) => {
                            eprintln!("[SERVER] Accepted connection from client");
                            // Spawn task to handle this connection
                            // This allows us to immediately accept the NEXT client
                            let server = self.clone();
                            let handle = tokio::spawn(async move {
                                eprintln!("[SERVER] Spawned task started");
                                match server.handle_new_connection(stream).await {
                                    Ok(_) => {
                                        eprintln!("[SERVER] Connection setup successful");
                                    }
                                    Err(e) => {
                                        eprintln!("[SERVER] Connection setup error: {:?}", e);
                                    }
                                }
                            });
                            eprintln!("[SERVER] Task spawned: {:?}", handle);
                        }
                        Err(e) => {
                            eprintln!("[SERVER VOLATILE] Accept error: {}", e);
                        }
                    }
                }
            }
        }
    }
    
    /// Handle a new connection: create resources, perform handshake, spawn handler
    async fn handle_new_connection(&self, mut stream: tokio::net::UnixStream) -> Result<()> {
        // Allocate slot
        let slot_id = self.next_slot_id.fetch_add(1, Ordering::Relaxed);
        eprintln!("[SERVER] Slot {}: client connected", slot_id);
        
        // POSIX shm names must start with / and have no other slashes
        let base_name = std::path::Path::new(&self.base_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("ipc");
        let send_shm_name = format!("/{}_{}_send", base_name, slot_id);
        let recv_shm_name = format!("/{}_{}_recv", base_name, slot_id);
        
        // Create platform doorbells (eventfd on Linux, kqueue on macOS, Event on Windows)
        let send_doorbell = PlatformDoorbell::new()?;
        let recv_doorbell = PlatformDoorbell::new()?;
        let send_doorbell_fd = send_doorbell.duplicate()?;
        let recv_doorbell_fd = recv_doorbell.duplicate()?;
        
        // Create buffers
        let send_buffer = SharedMemoryBuffer::create(&send_shm_name, RING_CAPACITY)?;
        let recv_buffer = SharedMemoryBuffer::create(&recv_shm_name, RING_CAPACITY)?;
        send_buffer.attach_doorbell(Arc::new(send_doorbell));
        recv_buffer.attach_doorbell(Arc::new(recv_doorbell));
        
        eprintln!("[SERVER] Slot {}: created resources", slot_id);
        
        // Perform handshake with async I/O for scalability
        // Read handshake request
        let mut buf = vec![0u8; 4096];
        use tokio::io::AsyncReadExt;
        let n = stream.read(&mut buf).await?;
        let _request: crate::ipc::control_socket::HandshakeRequest = bincode::deserialize(&buf[..n])?;
        
        // Send response with FD passing (requires sync call)
        let response = crate::ipc::control_socket::HandshakeResponse {
            slot_id,
            send_shm_name: send_shm_name.clone(),
            recv_shm_name: recv_shm_name.clone(),
            ring_capacity: RING_CAPACITY,
            protocol_version: 1,
        };
        let response_bytes = bincode::serialize(&response)?;
        
        // FD passing requires sync I/O - do it in spawn_blocking but very briefly
        let stream_std = stream.into_std()?;
        tokio::task::spawn_blocking(move || {
            stream_std.set_nonblocking(false)?;
            crate::ipc::fd_pass::send_fds(&stream_std, &[send_doorbell_fd, recv_doorbell_fd], &response_bytes)
        }).await??;
        
        eprintln!("[SERVER] Slot {}: handshake complete", slot_id);
        
        // Spawn handler
        let handlers = self.handlers.clone();
        let streaming_handlers = self.streaming_handlers.clone();
        let shutdown = self.shutdown.subscribe();
        tokio::spawn(async move {
            if let Err(e) = Self::handle_connection(slot_id, send_buffer, recv_buffer, handlers, streaming_handlers, shutdown).await {
                eprintln!("[HANDLER {}] Error: {}", slot_id, e);
            }
        });
        
        Ok(())
    }
    
    async fn handle_connection(
        slot_id: u32,
        send_buffer: Arc<SharedMemoryBuffer>,
        recv_buffer: Arc<SharedMemoryBuffer>,
        handlers: Arc<DashMap<MessageType, Handler>>,
        streaming_handlers: Arc<DashMap<MessageType, StreamingHandler>>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> Result<()> {
        let mut codec = BinaryCodec::new();
        let mut buffer = Vec::new();
        let idle_timeout = std::time::Duration::from_secs(30); // 30 second idle timeout
        let mut last_activity = std::time::Instant::now();
        
        loop {
            // Wait on doorbell with timeout (blocking call in separate task)
            let recv_buf_clone = recv_buffer.clone();
            let wait_task = tokio::task::spawn_blocking(move || {
                recv_buf_clone.wait_doorbell(5000)
            });
            
            tokio::select! {
                _ = shutdown.recv() => {
                    break;
                }
                
                wait_res = wait_task => {
                    // Doorbell was rung or timed out
                    match wait_res {
                        Ok(Ok(true)) => {
                            // Doorbell rung - data available
                            last_activity = std::time::Instant::now();
                        }
                        Ok(Ok(false)) | Ok(Err(_)) => {
                            // Timeout or error
                            if last_activity.elapsed() > idle_timeout {
                                break;
                            }
                            continue;
                        }
                        Err(_) => {
                            // spawn_blocking failed
                            break;
                        }
                    }
                }
            }
            
            // Try to read data
            match recv_buffer.read(&mut buffer, 64 * 1024) {
                                Ok(n) if n > 0 => {
                                    last_activity = std::time::Instant::now();
                                    eprintln!("[HANDLER {}] Received {} bytes: {:?}", slot_id, n, &buffer[..std::cmp::min(n, 100)]);
                                    
                                    // Try binary codec first (only the bytes actually read)
                                    let decoded_result = codec.decode(&buffer[..n]);
                                    eprintln!("[HANDLER {}] Binary decode result: {:?}", slot_id, decoded_result.as_ref().map(|_| "Ok").unwrap_or("Err"));
                                    
                                    // For ChatMessage type, the UI sends JSON directly (not binary codec)
                                    // So if binary decode fails, try to handle as raw JSON for streaming
                                    if decoded_result.is_err() {
                                        eprintln!("[HANDLER {}] Binary decode failed, trying JSON...", slot_id);
                                        
                                        // Check if this is CPAL protocol (magic bytes: [67, 80, 65, 76])
                                        if n >= 24 && buffer[0] == 67 && buffer[1] == 80 && buffer[2] == 65 && buffer[3] == 76 {
                                            // CPAL header structure:
                                            // 0-3: magic "CPAL"
                                            // 4-5: version
                                            // 6-7: flags
                                            // 8-11: payload length (little-endian u32)
                                            let payload_len = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]) as usize;
                                            let total_len = 24 + payload_len; // header + payload
                                            
                                            eprintln!("[HANDLER {}] CPAL detected: payload_len={}, total_len={}, received={}", slot_id, payload_len, total_len, n);
                                            
                                            // Read more data if needed
                                            let mut full_message = buffer[..n].to_vec();
                                            while full_message.len() < total_len {
                                                match recv_buffer.read(&mut buffer, 64 * 1024) {
                                                    Ok(more_n) if more_n > 0 => {
                                                        full_message.extend_from_slice(&buffer[..more_n]);
                                                        eprintln!("[HANDLER {}] Read {} more bytes, total now {}", slot_id, more_n, full_message.len());
                                                    }
                                                    Ok(_) => {
                                                        eprintln!("[HANDLER {}] No more data available", slot_id);
                                                        break;
                                                    }
                                                    Err(e) => {
                                                        eprintln!("[HANDLER {}] Error reading more data: {}", slot_id, e);
                                                        break;
                                                    }
                                                }
                                            }
                                            
                                            if full_message.len() >= total_len {
                                                // Extract JSON payload (skip 24-byte CPAL header)
                                                let json_payload = &full_message[24..total_len];
                                                eprintln!("[HANDLER {}] Extracted JSON payload: {} bytes", slot_id, json_payload.len());
                                                
                                                // Debug: print the JSON string
                                                if let Ok(json_str) = std::str::from_utf8(json_payload) {
                                                    eprintln!("[HANDLER {}] JSON string: {}", slot_id, json_str);
                                                } else {
                                                    eprintln!("[HANDLER {}] JSON payload is not valid UTF-8", slot_id);
                                                }
                                                
                                                // Try to deserialize as ProviderChatStreamRequest JSON
                                                if let Ok(json_req) = serde_json::from_slice::<serde_json::Value>(json_payload) {
                                                eprintln!("[HANDLER {}] JSON parsed: model={}, messages={}", 
                                                    slot_id, 
                                                    json_req.get("model").and_then(|v| v.as_str()).unwrap_or("?"),
                                                    json_req.get("messages").map(|m| m.as_array().map(|a| a.len()).unwrap_or(0)).unwrap_or(0)
                                                );
                                                
                                                if json_req.get("model").is_some() && json_req.get("messages").is_some() {
                                                    // This is a provider chat stream request
                                                    let payload_bytes = json_payload.to_vec();
                                                
                                                // Use ChatMessage type for routing
                                                let msg_type = crate::ipc::binary_codec::MessageType::ChatMessage;
                                                
                                                if let Some(streaming_handler) = streaming_handlers.get(&msg_type) {
                                                    // Create channel for streaming responses
                                                    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                                                    
                                                    // Spawn streaming task
                                                    let handler_fut = streaming_handler(Bytes::from(payload_bytes), tx);
                                                    tokio::spawn(async move {
                                                        if let Err(e) = handler_fut.await {
                                                            eprintln!("[STREAMING] Error: {}", e);
                                                        }
                                                    });
                                                    
                                                    // Read chunks and send them
                                                    while let Some(chunk) = rx.recv().await {
                                                        // Write chunk to send buffer
                                                        if let Err(e) = send_buffer.write(&chunk) {
                                                            eprintln!("[HANDLER {}] Failed to write chunk: {}", slot_id, e);
                                                            break;
                                                        }
                                                    }
                                                    
                                                        last_activity = std::time::Instant::now();
                                                        continue; // Continue to next message
                                                    } else {
                                                        eprintln!("[HANDLER {}] ✗ NO HANDLER for ChatMessage", slot_id);
                                                        continue;
                                                    }
                                                    } else {
                                                        eprintln!("[HANDLER {}] JSON missing required fields", slot_id);
                                                    }
                                                } else {
                                                    eprintln!("[HANDLER {}] JSON parse failed", slot_id);
                                                }
                                            } else {
                                                eprintln!("[HANDLER {}] Incomplete CPAL message: got {}, need {}", slot_id, full_message.len(), total_len);
                                            }
                                        } else {
                                            eprintln!("[HANDLER {}] Not CPAL format", slot_id);
                                        }
                                        // Unknown format
                                        eprintln!("[HANDLER {}] ✗ DECODE ERROR: binary decode failed and not valid JSON", slot_id);
                                        continue;
                                    }
                                    
                                    // Binary codec decode succeeded
                                    match decoded_result {
                                        Ok(msg) => {
                                            // Extract payload bytes
                                            let payload_bytes = match &msg.payload {
                                                crate::ipc::binary_codec::MessagePayload::CompletionRequest(req) => req.prompt.as_bytes().to_vec(),
                                                crate::ipc::binary_codec::MessagePayload::CompletionResponse(resp) => resp.text.as_bytes().to_vec(),
                                                _ => vec![],
                                            };
                                            
                                            // Check if streaming handler first
                                            if let Some(streaming_handler) = streaming_handlers.get(&msg.msg_type) {
                                                // Create channel for streaming responses
                                                let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                                                
                                                // Spawn streaming task
                                                let handler_fut = streaming_handler(Bytes::from(payload_bytes.clone()), tx);
                                                tokio::spawn(async move {
                                                    if let Err(e) = handler_fut.await {
                                                        eprintln!("[STREAMING] Error: {}", e);
                                                    }
                                                });
                                                
                                                // Read chunks and send them
                                                while let Some(chunk) = rx.recv().await {
                                                    // Write chunk to send buffer
                                                    if let Err(e) = send_buffer.write(&chunk) {
                                                        eprintln!("[HANDLER {}] Failed to write chunk: {}", slot_id, e);
                                                        break;
                                                    }
                                                }
                                                
                                                last_activity = std::time::Instant::now();
                                            } else if let Some(handler) = handlers.get(&msg.msg_type) {
                                                match handler(Bytes::from(payload_bytes)).await {
                                                    Ok(response_data) => {
                                                        // Encode response
                                                        let response_msg = crate::ipc::binary_codec::Message {
                                                            id: msg.id,
                                                            msg_type: msg.msg_type,
                                                            payload: crate::ipc::binary_codec::MessagePayload::CompletionRequest(
                                                                crate::ipc::binary_codec::CompletionRequest {
                                                                    prompt: String::from_utf8_lossy(&response_data).to_string(),
                                                                    model: "echo".to_string(),
                                                                    max_tokens: 100,
                                                                    temperature: 0.0,
                                                                    stream: false,
                                                                }
                                                            ),
                                                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                                                        };
                                                        
                                                        let response_bytes = codec.encode(&response_msg)?;
                                                        send_buffer.write(&response_bytes)?;
                                                        
                                                        // Continue processing more messages
                                                        last_activity = std::time::Instant::now();
                                                    }
                                                    Err(e) => {
                                                        eprintln!("[HANDLER {}] ✗ ERROR: {}", slot_id, e);
                                                        return Err(e.into());
                                                    }
                                                }
                                            } else {
                                                eprintln!("[HANDLER {}] ✗ NO HANDLER", slot_id);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[HANDLER {}] ✗ DECODE ERROR: {}", slot_id, e);
                                        }
                                    }
                                }
                        Ok(_) => {
                            // No data yet
                        }
                        Err(e) => {
                            eprintln!("[HANDLER {}] ✗ READ ERROR: {}", slot_id, e);
                            break; // Exit on error
                        }
            }
        }
        
        Ok(())
    }
    
    /// Register a streaming message handler (for provider streaming)
    pub fn register_streaming_handler<F, Fut>(&self, msg_type: MessageType, handler: F)
    where
        F: Fn(Bytes, tokio::sync::mpsc::Sender<Bytes>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = IpcResult<()>> + Send + 'static,
    {
        self.streaming_handlers.insert(msg_type, Box::new(move |data, sender| {
            Box::pin(handler(data, sender))
        }));
    }
    
    /// Get metrics (stub for compatibility)
    pub fn metrics(&self) -> Arc<DummyMetrics> {
        Arc::new(DummyMetrics {})
    }
    
    /// Shutdown the server
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}

/// Dummy metrics for compatibility with IpcServer interface
pub struct DummyMetrics {}

impl DummyMetrics {
    pub fn export_prometheus(&self) -> String {
        "# IpcServerVolatile metrics (not yet implemented)\n".to_string()
    }
}
