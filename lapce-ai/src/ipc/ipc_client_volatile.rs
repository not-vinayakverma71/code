/// IPC Client using volatile buffers and control handshake
/// Replaces lock-file based connection

use std::sync::{Arc, Mutex};
use anyhow::Result;
use bytes::Bytes;
use crate::ipc::control_socket::ControlClient;
#[cfg(target_os = "linux")]
use crate::ipc::shm_buffer_futex::FutexSharedMemoryBuffer as SharedMemoryBuffer;

#[cfg(not(target_os = "linux"))]
use crate::ipc::shm_buffer_volatile::VolatileSharedMemoryBuffer as SharedMemoryBuffer;
use crate::ipc::binary_codec::{BinaryCodec, MessageType};

pub struct IpcClientVolatile {
    send_buffer: Arc<SharedMemoryBuffer>,
    recv_buffer: Arc<SharedMemoryBuffer>,
    codec: Arc<Mutex<BinaryCodec>>,
    slot_id: u32,
}

impl IpcClientVolatile {
    pub async fn connect(base_path: &str) -> Result<Self> {
        eprintln!("[CLIENT VOLATILE] Connecting to {}", base_path);
        
        // Perform handshake and receive eventfd doorbells
        let handshake = ControlClient::handshake(base_path).await?;
        let response = handshake.response;
        
        eprintln!("[CLIENT VOLATILE] Got slot {}", response.slot_id);
        
        // Open buffers (note: reversed perspective - client writes to recv, reads from send)
        let mut send_buffer = SharedMemoryBuffer::open(&response.recv_shm_name)?;
        let mut recv_buffer = SharedMemoryBuffer::open(&response.send_shm_name)?;
        
        // Attach eventfd doorbells
        eprintln!("[CLIENT VOLATILE] Received doorbell fds: send={}, recv={}", handshake.send_doorbell_fd, handshake.recv_doorbell_fd);
        
        // CLIENT perspective: we wait on SEND doorbell when reading, ring RECV doorbell when writing
        recv_buffer.attach_doorbell_fd(handshake.send_doorbell_fd); // Wait on this when reading
        send_buffer.attach_doorbell_fd(handshake.recv_doorbell_fd); // Ring this when writing
        
        eprintln!("[CLIENT VOLATILE] Opened buffers with doorbells");
        
        Ok(Self {
            send_buffer: Arc::new(send_buffer),
            recv_buffer: Arc::new(recv_buffer),
            codec: Arc::new(Mutex::new(BinaryCodec::new())),
            slot_id: response.slot_id,
        })
    }
    
    pub async fn send_bytes(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Verbose logging disabled for performance
        // eprintln!("[CLIENT {}] ======== SEND START ========", self.slot_id);
        // eprintln!("[CLIENT {}] Input data: {} bytes: {:?}", self.slot_id, data.len(), std::str::from_utf8(data).unwrap_or("<binary>"));
        // eprintln!("[CLIENT {}] Creating message...", self.slot_id);
        let msg = crate::ipc::binary_codec::Message {
            id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64,
            msg_type: MessageType::CompletionRequest,
            payload: crate::ipc::binary_codec::MessagePayload::CompletionRequest(
                crate::ipc::binary_codec::CompletionRequest {
                    prompt: String::from_utf8_lossy(data).to_string(),
                    model: "test".to_string(),
                    max_tokens: 100,
                    temperature: 0.7,
                    stream: false,
                }
            ),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
        };
        // eprintln!("[CLIENT {}] Encoding message...", self.slot_id);
        let msg_bytes = self.codec.lock().unwrap().encode(&msg)?;
        // eprintln!("[CLIENT {}] ✓ ENCODED: {} bytes", self.slot_id, msg_bytes.len());
        
        // Write to send buffer
        // eprintln!("[CLIENT {}] Writing to send_buffer...", self.slot_id);
        self.send_buffer.write(&msg_bytes)?;
        // eprintln!("[CLIENT {}] ✓ WROTE to send_buffer", self.slot_id);
        
        // Wait for response using eventfd doorbell (efficient blocking)
        // eprintln!("[CLIENT {}] Waiting for response...", self.slot_id);
        let start = std::time::Instant::now();
        let timeout_ms = 5000;
        
        let mut buffer = Vec::new();
        
        // Wait on doorbell with timeout
        loop {
            let elapsed_ms = start.elapsed().as_millis() as i32;
            let remaining_ms = timeout_ms - elapsed_ms;
            
            if remaining_ms <= 0 {
                eprintln!("[CLIENT {}] ✗ TIMEOUT", self.slot_id);
                anyhow::bail!("Response timeout");
            }
            
            // Wait for doorbell notification
            let _ = self.recv_buffer.wait_doorbell(remaining_ms)?;
            // if self.recv_buffer.wait_doorbell(remaining_ms)? {
            //     eprintln!("[CLIENT {}] Doorbell rang!", self.slot_id);
            // }
            
            // Try to read data
            match self.recv_buffer.read(&mut buffer, 64 * 1024) {
                Ok(n) if n > 0 => {
                    // eprintln!("[CLIENT {}] ✓ READ {} bytes from recv_buffer", self.slot_id, n);
                    // eprintln!("[CLIENT {}] Buffer first 32 bytes: {:?}", self.slot_id, &buffer[..n.min(32)]);
                    
                    // Decode response
                    // eprintln!("[CLIENT {}] Decoding response...", self.slot_id);
                    let msg = self.codec.lock().unwrap().decode(&buffer)?;
                    // eprintln!("[CLIENT {}] ✓ DECODED msg_type={:?} id={}", self.slot_id, msg.msg_type, msg.id);
                    
                    // Extract payload - just echo back the prompt as bytes
                    let payload = match msg.payload {
                        crate::ipc::binary_codec::MessagePayload::CompletionRequest(req) => req.prompt.into_bytes(),
                        crate::ipc::binary_codec::MessagePayload::CompletionResponse(resp) => resp.text.into_bytes(),
                        _ => vec![],
                    };
                    
                    // eprintln!("[CLIENT {}] ✓ SUCCESS: {} bytes payload, took {:?}", self.slot_id, payload.len(), start.elapsed());
                    // eprintln!("[CLIENT {}] ======== SEND END ========", self.slot_id);
                    return Ok(payload);
                }
                Ok(_) => {
                    // Doorbell rang but no data yet (spurious wakeup), loop again
                    continue;
                }
                Err(e) => {
                    anyhow::bail!("Read error: {}", e);
                }
            }
        }
    }
}

impl Clone for IpcClientVolatile {
    fn clone(&self) -> Self {
        Self {
            send_buffer: self.send_buffer.clone(),
            recv_buffer: self.recv_buffer.clone(),
            codec: self.codec.clone(),
            slot_id: self.slot_id,
        }
    }
}
