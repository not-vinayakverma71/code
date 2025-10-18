// Optimized IPC implementation targeting < 10μs latency and > 1M msg/sec throughput
use std::sync::Arc;
use parking_lot::RwLock;
use ahash::AHashMap;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use bytes::{BytesMut, Bytes};
use crossbeam::queue::ArrayQueue;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

pub mod benchmark;

// Zero-copy message header
#[repr(C)]
#[derive(Debug, Copy, Clone, AsBytes, FromBytes, FromZeroes)]
pub struct MessageHeader {
    pub msg_type: u8,
    pub origin: u8,
    pub payload_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcMessageType {
    Connect = 0,
    Disconnect = 1,
    Ack = 2,
    TaskCommand = 3,
    TaskEvent = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcOrigin {
    Client = 0,
    Server = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ack {
    pub client_id: String,
    pub pid: u32,
    pub ppid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    Ack {
        origin: IpcOrigin,
        data: Ack,
    },
    TaskCommand {
        origin: IpcOrigin,
        client_id: String,
        data: serde_json::Value,
    },
    TaskEvent {
        origin: IpcOrigin,
        relay_client_id: Option<String>,
        data: serde_json::Value,
    },
}

// Lock-free buffer pool
pub struct BufferPool {
    small: Arc<ArrayQueue<BytesMut>>,  // 256 bytes
    medium: Arc<ArrayQueue<BytesMut>>, // 4KB
    large: Arc<ArrayQueue<BytesMut>>,  // 64KB
}

impl BufferPool {
    pub fn new() -> Self {
        let small = Arc::new(ArrayQueue::new(1000));
        let medium = Arc::new(ArrayQueue::new(100));
        let large = Arc::new(ArrayQueue::new(10));
        
        // Pre-allocate buffers
        for _ in 0..500 {
            small.push(BytesMut::with_capacity(256)).ok();
        }
        for _ in 0..50 {
            medium.push(BytesMut::with_capacity(4096)).ok();
        }
        for _ in 0..5 {
            large.push(BytesMut::with_capacity(65536)).ok();
        }
        
        Self { small, medium, large }
    }
    
    pub fn get(&self, size: usize) -> BytesMut {
        if size <= 256 {
            self.small.pop().unwrap_or_else(|| BytesMut::with_capacity(256))
        } else if size <= 4096 {
            self.medium.pop().unwrap_or_else(|| BytesMut::with_capacity(4096))
        } else {
            self.large.pop().unwrap_or_else(|| BytesMut::with_capacity(65536))
        }
    }
    
    pub fn put(&self, mut buf: BytesMut) {
        buf.clear();
        let cap = buf.capacity();
        if cap <= 256 {
            let _ = self.small.push(buf);
        } else if cap <= 4096 {
            let _ = self.medium.push(buf);
        } else {
            let _ = self.large.push(buf);
        }
    }
}

// Optimized server with lock-free operations where possible
pub struct OptimizedIpcServer {
    socket_path: String,
    clients: Arc<RwLock<AHashMap<String, Arc<UnixStream>>>>,
    buffer_pool: Arc<BufferPool>,
    is_listening: Arc<ArcSwap<bool>>,
}

impl OptimizedIpcServer {
    pub fn new(socket_path: String) -> Self {
        Self {
            socket_path,
            clients: Arc::new(RwLock::new(AHashMap::new())),
            buffer_pool: Arc::new(BufferPool::new()),
            is_listening: Arc::new(ArcSwap::from_pointee(false)),
        }
    }
    
    pub async fn listen(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.is_listening.store(Arc::new(true));
        let _ = std::fs::remove_file(&self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)?;
        
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);
            let client_id = generate_client_id();
            
            // Send ACK with zero-copy header
            let ack = Ack {
                client_id: client_id.clone(),
                pid: std::process::id(),
                ppid: get_ppid(),
            };
            
            let ack_msg = IpcMessage::Ack {
                origin: IpcOrigin::Server,
                data: ack,
            };
            
            // Fast path: direct write without intermediate buffer
            let payload = serde_json::to_vec(&ack_msg)?;
            let header = MessageHeader {
                msg_type: IpcMessageType::Ack as u8,
                origin: IpcOrigin::Server as u8,
                payload_len: payload.len() as u32,
            };
            
            // Write directly to stream - Arc doesn't have try_clone
            let mut stream = stream;
            (&*stream).write_all(header.as_bytes()).await?;
            (&*stream).write_all(&payload).await?;
            
            // Store client with read-write lock for fast reads
            self.clients.write().insert(client_id.clone(), stream.clone());
            
            // Handle client in background
            let clients = self.clients.clone();
            let buffer_pool = self.buffer_pool.clone();
            
            tokio::spawn(async move {
                handle_client_optimized(client_id, stream, clients, buffer_pool).await;
            });
        }
    }
    
    pub async fn broadcast(&self, message: IpcMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::to_vec(&message)?;
        let header = MessageHeader {
            msg_type: IpcMessageType::TaskEvent as u8,
            origin: IpcOrigin::Server as u8,
            payload_len: payload.len() as u32,
        };
        
        // Read lock for iterating clients
        let clients = self.clients.read();
        let mut futures = Vec::with_capacity(clients.len());
        
        for (_, stream) in clients.iter() {
            let stream = stream.clone();
            let header_bytes = Bytes::copy_from_slice(header.as_bytes());
            let payload_bytes = Bytes::from(payload.clone());
            
            futures.push(tokio::spawn(async move {
                let mut writer = BufWriter::new(stream.as_ref());
                let _ = writer.write_all(&header_bytes).await;
                let _ = writer.write_all(&payload_bytes).await;
                let _ = writer.flush().await;
            }));
        }
        
        // Wait for all broadcasts to complete
        for future in futures {
            let _ = future.await;
        }
        
        Ok(())
    }
    
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    pub fn is_listening(&self) -> bool {
        **self.is_listening.load()
    }
}

async fn handle_client_optimized(
    client_id: String,
    stream: Arc<UnixStream>,
    clients: Arc<RwLock<AHashMap<String, Arc<UnixStream>>>>,
    buffer_pool: Arc<BufferPool>,
) {
    let mut reader = BufReader::with_capacity(8192, stream.as_ref());
    let mut header_buf = [0u8; std::mem::size_of::<MessageHeader>()];
    
    loop {
        // Read header with zero-copy
        match reader.read_exact(&mut header_buf).await {
            Ok(_) => {
                if let Some(header) = MessageHeader::read_from(&header_buf) {
                    // Get buffer from pool
                    let mut payload_buf = buffer_pool.get(header.payload_len as usize);
                    payload_buf.resize(header.payload_len as usize, 0);
                    
                    // Read payload
                    if reader.read_exact(&mut payload_buf).await.is_ok() {
                        // Process message in background to avoid blocking
                        tokio::spawn(async move {
                            if let Ok(msg) = serde_json::from_slice::<IpcMessage>(&payload_buf) {
                                // Process message
                                match msg {
                                    IpcMessage::TaskCommand { .. } => {
                                        // Handle command
                                    }
                                    _ => {}
                                }
                            }
                        });
                        
                        // Return buffer to pool
                        buffer_pool.put(payload_buf);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    
    // Remove client on disconnect
    clients.write().remove(&client_id);
}

// Optimized client
pub struct OptimizedIpcClient {
    socket_path: String,
    stream: Option<Arc<UnixStream>>,
    client_id: Option<String>,
    is_connected: Arc<ArcSwap<bool>>,
    buffer_pool: Arc<BufferPool>,
}

impl OptimizedIpcClient {
    pub fn new(socket_path: String) -> Self {
        Self {
            socket_path,
            stream: None,
            client_id: None,
            is_connected: Arc::new(ArcSwap::from_pointee(false)),
            buffer_pool: Arc::new(BufferPool::new()),
        }
    }
    
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let stream = Arc::new(stream);
        self.is_connected.store(Arc::new(true));
        
        // Read ACK header
        let mut reader = BufReader::new(stream.as_ref());
        let mut header_buf = [0u8; std::mem::size_of::<MessageHeader>()];
        reader.read_exact(&mut header_buf).await?;
        
        if let Some(header) = MessageHeader::read_from(&header_buf) {
            let mut payload_buf = vec![0u8; header.payload_len as usize];
            reader.read_exact(&mut payload_buf).await?;
            
            if let Ok(msg) = serde_json::from_slice::<IpcMessage>(&payload_buf) {
                if let IpcMessage::Ack { data, .. } = msg {
                    self.client_id = Some(data.client_id);
                }
            }
        }
        
        self.stream = Some(stream);
        Ok(())
    }
    
    pub async fn send_message(&mut self, message: IpcMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(stream) = &self.stream {
            let payload = serde_json::to_vec(&message)?;
            let header = MessageHeader {
                msg_type: IpcMessageType::TaskCommand as u8,
                origin: IpcOrigin::Client as u8,
                payload_len: payload.len() as u32,
            };
            
            let mut writer = BufWriter::new(stream.as_ref());
            writer.write_all(header.as_bytes()).await?;
            writer.write_all(&payload).await?;
            writer.flush().await?;
        }
        Ok(())
    }
    
    pub fn disconnect(&mut self) {
        self.stream = None;
        self.is_connected.store(Arc::new(false));
    }
    
    pub fn is_connected(&self) -> bool {
        **self.is_connected.load()
    }
    
    pub fn client_id(&self) -> Option<&String> {
        self.client_id.as_ref()
    }
}

fn generate_client_id() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..6).map(|_| rand::thread_rng().gen()).collect();
    hex::encode(bytes)
}

fn get_ppid() -> u32 {
    #[cfg(unix)]
    unsafe { libc::getppid() as u32 }
    
    #[cfg(not(unix))]
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_optimized_connection() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock").to_str().unwrap().to_string();
        
        let server = Arc::new(OptimizedIpcServer::new(socket_path.clone()));
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            let _ = server_clone.listen().await;
        });
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut client = OptimizedIpcClient::new(socket_path);
        client.connect().await.unwrap();
        
        assert!(client.is_connected());
        assert!(client.client_id().is_some());
        
        handle.abort();
    }
    
    #[tokio::test]
    async fn test_optimized_throughput() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock").to_str().unwrap().to_string();
        
        let server = Arc::new(OptimizedIpcServer::new(socket_path.clone()));
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            let _ = server_clone.listen().await;
        });
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut client = OptimizedIpcClient::new(socket_path);
        client.connect().await.unwrap();
        
        // Send messages
        for i in 0..1000 {
            let message = IpcMessage::TaskCommand {
                origin: IpcOrigin::Client,
                client_id: client.client_id().unwrap().clone(),
                data: serde_json::json!({"index": i}),
            };
            client.send_message(message).await.unwrap();
        }
        
        handle.abort();
    }
}
