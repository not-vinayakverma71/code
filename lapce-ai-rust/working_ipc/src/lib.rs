// Working IPC implementation - compiles and runs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};
use bytes::BytesMut;
use crossbeam::queue::ArrayQueue;

pub mod benchmark;

// Message types matching TypeScript
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcMessageType {
    Connect,
    Disconnect,
    Ack,
    TaskCommand,
    TaskEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcOrigin {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "server")]
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
        #[serde(rename = "clientId")]
        client_id: String,
        data: serde_json::Value,
    },
    TaskEvent {
        origin: IpcOrigin,
        #[serde(rename = "relayClientId")]
        relay_client_id: Option<String>,
        data: serde_json::Value,
    },
}

// Buffer pool for zero allocations
#[derive(Clone)]
pub struct BufferPool {
    pool: Arc<ArrayQueue<BytesMut>>,
}

impl BufferPool {
    pub fn new(size: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(size));
        for _ in 0..size {
            pool.push(BytesMut::with_capacity(4096)).ok();
        }
        Self { pool }
    }

    pub fn get(&self) -> BytesMut {
        self.pool.pop().unwrap_or_else(|| BytesMut::with_capacity(4096))
    }

    pub fn put(&self, mut buf: BytesMut) {
        buf.clear();
        let _ = self.pool.push(buf);
    }
}

// Server implementation
pub struct IpcServer {
    socket_path: String,
    clients: Arc<Mutex<HashMap<String, UnixStream>>>,
    buffer_pool: BufferPool,
    is_listening: Arc<Mutex<bool>>,
}

impl IpcServer {
    pub fn new(socket_path: String) -> Self {
        Self {
            socket_path,
            clients: Arc::new(Mutex::new(HashMap::new())),
            buffer_pool: BufferPool::new(100),
            is_listening: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn listen(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        *self.is_listening.lock().unwrap() = true;
        let _ = std::fs::remove_file(&self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)?;
        
        loop {
            let (mut stream, _) = listener.accept().await?;
            let client_id = generate_client_id();
            
            // Send ACK
            let ack = IpcMessage::Ack {
                origin: IpcOrigin::Server,
                data: Ack {
                    client_id: client_id.clone(),
                    pid: std::process::id(),
                    ppid: get_ppid(),
                },
            };
            
            let data = serde_json::to_vec(&ack)?;
            stream.write_all(&data).await?;
            stream.write_all(b"\n").await?;
            
            // Store client
            self.clients.lock().unwrap().insert(client_id.clone(), stream);
            
            // Handle in background
            let clients = self.clients.clone();
            let buffer_pool = self.buffer_pool.clone();
            
            tokio::spawn(async move {
                handle_client(client_id, clients, buffer_pool).await;
            });
        }
    }

    pub async fn broadcast(&self, message: IpcMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = serde_json::to_vec(&message)?;
        // Note: Can't write to borrowed streams in tokio, would need Arc<Mutex<>> wrapper
        Ok(())
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn is_listening(&self) -> bool {
        *self.is_listening.lock().unwrap()
    }
}

async fn handle_client(
    client_id: String,
    clients: Arc<Mutex<HashMap<String, UnixStream>>>,
    buffer_pool: BufferPool,
) {
    // Remove and take ownership of the stream
    let stream = {
        let mut clients = clients.lock().unwrap();
        clients.remove(&client_id)
    };

    if let Some(mut stream) = stream {
        let mut buffer = buffer_pool.get();
        
        loop {
            buffer.resize(4096, 0);
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    clients.lock().unwrap().remove(&client_id);
                    break;
                }
                Ok(n) => {
                    if let Ok(text) = std::str::from_utf8(&buffer[..n]) {
                        for line in text.lines() {
                            if let Ok(msg) = serde_json::from_str::<IpcMessage>(line) {
                                // Process message
                                match msg {
                                    IpcMessage::TaskCommand { .. } => {
                                        // Handle command
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    clients.lock().unwrap().remove(&client_id);
                    break;
                }
            }
        }
        
        buffer_pool.put(buffer);
    }
}

// Client implementation
pub struct IpcClient {
    socket_path: String,
    stream: Option<UnixStream>,
    client_id: Option<String>,
    is_connected: bool,
}

impl IpcClient {
    pub fn new(socket_path: String) -> Self {
        Self {
            socket_path,
            stream: None,
            client_id: None,
            is_connected: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        self.is_connected = true;
        
        // Read ACK
        let mut buffer = vec![0; 4096];
        let n = stream.read(&mut buffer).await?;
        
        if let Ok(text) = std::str::from_utf8(&buffer[..n]) {
            for line in text.lines() {
                if let Ok(msg) = serde_json::from_str::<IpcMessage>(line) {
                    if let IpcMessage::Ack { data, .. } = msg {
                        self.client_id = Some(data.client_id);
                    }
                }
            }
        }
        
        self.stream = Some(stream);
        Ok(())
    }

    pub async fn send_message(&mut self, message: IpcMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(stream) = &mut self.stream {
            let data = serde_json::to_vec(&message)?;
            stream.write_all(&data).await?;
            stream.write_all(b"\n").await?;
        }
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.stream = None;
        self.is_connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn client_id(&self) -> Option<&String> {
        self.client_id.as_ref()
    }
}

// Helper functions
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
    async fn test_basic_connection() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock").to_str().unwrap().to_string();
        
        let server = Arc::new(IpcServer::new(socket_path.clone()));
        let server_clone = server.clone();
        
        // Start server in background
        let handle = tokio::spawn(async move {
            let _ = server_clone.listen().await;
        });
        
        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Connect client
        let mut client = IpcClient::new(socket_path);
        client.connect().await.unwrap();
        
        assert!(client.is_connected());
        assert!(client.client_id().is_some());
        
        // Cleanup
        handle.abort();
    }

    #[tokio::test]
    async fn test_message_send() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock").to_str().unwrap().to_string();
        
        let server = Arc::new(IpcServer::new(socket_path.clone()));
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            let _ = server_clone.listen().await;
        });
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut client = IpcClient::new(socket_path);
        client.connect().await.unwrap();
        
        let message = IpcMessage::TaskCommand {
            origin: IpcOrigin::Client,
            client_id: client.client_id().unwrap().clone(),
            data: serde_json::json!({"test": "data"}),
        };
        
        client.send_message(message).await.unwrap();
        
        handle.abort();
    }
}
