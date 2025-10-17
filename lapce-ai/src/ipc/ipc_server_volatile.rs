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
use crate::ipc::eventfd_doorbell::EventFdDoorbell;
use crate::ipc::errors::IpcResult;

const RING_CAPACITY: u32 = 1024 * 1024; // 1MB per ring

type Handler = Box<dyn Fn(Bytes) -> std::pin::Pin<Box<dyn std::future::Future<Output = IpcResult<Bytes>> + Send>> + Send + Sync>;

pub struct IpcServerVolatile {
    control_server: Arc<Mutex<ControlServer>>,
    handlers: Arc<DashMap<MessageType, Handler>>,
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
    
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        let mut shutdown_rx = self.shutdown.subscribe();
        
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    eprintln!("[SERVER VOLATILE] Shutting down");
                    return Ok(());
                }
                
                // Accept connection (fast, just TCP accept)
                result = async {
                    let mut guard = self.control_server.lock().await;
                    guard.listener.accept().await
                } => {
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
    async fn handle_new_connection(&self, stream: tokio::net::UnixStream) -> Result<()> {
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
        
        // Create eventfd doorbells
        let send_doorbell = EventFdDoorbell::new()?;
        let recv_doorbell = EventFdDoorbell::new()?;
        let send_doorbell_fd = send_doorbell.duplicate()?;
        let recv_doorbell_fd = recv_doorbell.duplicate()?;
        
        // Create buffers
        let send_buffer = SharedMemoryBuffer::create(&send_shm_name, RING_CAPACITY)?;
        let recv_buffer = SharedMemoryBuffer::create(&recv_shm_name, RING_CAPACITY)?;
        send_buffer.attach_doorbell(Arc::new(send_doorbell));
        recv_buffer.attach_doorbell(Arc::new(recv_doorbell));
        
        eprintln!("[SERVER] Slot {}: created resources", slot_id);
        
        // Read handshake request
        let mut buf = vec![0u8; 4096];
        let stream_std = stream.into_std()?;
        stream_std.set_nonblocking(false)?;
        
        use std::io::Read;
        let n = (&stream_std).read(&mut buf)?;
        let _request: crate::ipc::control_socket::HandshakeRequest = bincode::deserialize(&buf[..n])?;
        
        // Send response
        let response = crate::ipc::control_socket::HandshakeResponse {
            slot_id,
            send_shm_name,
            recv_shm_name,
            ring_capacity: RING_CAPACITY,
            protocol_version: 1,
        };
        let response_bytes = bincode::serialize(&response)?;
        crate::ipc::fd_pass::send_fds(&stream_std, &[send_doorbell_fd, recv_doorbell_fd], &response_bytes)?;
        
        eprintln!("[SERVER] Slot {}: handshake complete", slot_id);
        
        // Spawn handler
        let handlers = self.handlers.clone();
        let shutdown = self.shutdown.subscribe();
        tokio::spawn(async move {
            if let Err(e) = Self::handle_connection(slot_id, send_buffer, recv_buffer, handlers, shutdown).await {
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
                                    
                                    // Decode message
                                    match codec.decode(&buffer) {
                                        Ok(msg) => {
                                            // Extract payload bytes
                                            let payload_bytes = match &msg.payload {
                                                crate::ipc::binary_codec::MessagePayload::CompletionRequest(req) => req.prompt.as_bytes().to_vec(),
                                                crate::ipc::binary_codec::MessagePayload::CompletionResponse(resp) => resp.text.as_bytes().to_vec(),
                                                _ => vec![],
                                            };
                                            
                                            // Find and call handler
                                            if let Some(handler) = handlers.get(&msg.msg_type) {
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
}
