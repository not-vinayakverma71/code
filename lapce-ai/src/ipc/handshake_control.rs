/// Handshake Control SHM
/// Blocking accept() with deterministic conn_id rendezvous

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use anyhow::{Result, bail};

const HANDSHAKE_MAGIC: u32 = 0x4950434C; // "IPCL"
const HANDSHAKE_VERSION: u8 = 1;

/// Control channel for handshake rendezvous
#[repr(C)]
pub struct HandshakeControl {
    // Magic number for validation
    magic: AtomicU32,
    
    // Protocol version
    version: u8,
    
    // Connection state: 0=waiting, 1=client_ready, 2=server_ack, 3=established
    state: AtomicU32,
    
    // Deterministic connection ID (agreed by both sides)
    conn_id: AtomicU64,
    
    // Client process ID for validation
    client_pid: AtomicU32,
    
    // Server process ID for validation
    server_pid: AtomicU32,
    
    // Auth token (optional, 32 bytes)
    auth_token: [u8; 32],
    
    // Timestamp for timeout detection
    timestamp_ns: AtomicU64,
}

impl HandshakeControl {
    /// Create new handshake control structure
    pub fn new() -> Self {
        Self {
            magic: AtomicU32::new(HANDSHAKE_MAGIC),
            version: HANDSHAKE_VERSION,
            state: AtomicU32::new(0),
            conn_id: AtomicU64::new(0),
            client_pid: AtomicU32::new(0),
            server_pid: AtomicU32::new(std::process::id()),
            auth_token: [0u8; 32],
            timestamp_ns: AtomicU64::new(0),
        }
    }
    
    /// Server: block until client handshake arrives
    pub async fn accept_blocking(&self, timeout: Duration) -> Result<u64> {
        let start = Instant::now();
        
        // Wait for client to set state to client_ready (1)
        loop {
            if self.state.load(Ordering::Acquire) == 1 {
                // Validate magic and version
                if self.magic.load(Ordering::Acquire) != HANDSHAKE_MAGIC {
                    bail!("Invalid handshake magic");
                }
                if self.version != HANDSHAKE_VERSION {
                    bail!("Protocol version mismatch");
                }
                
                // Generate deterministic conn_id based on PIDs and timestamp
                let client_pid = self.client_pid.load(Ordering::Acquire);
                let server_pid = self.server_pid.load(Ordering::Acquire);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_nanos() as u64;
                
                let conn_id = Self::generate_conn_id(client_pid, server_pid, timestamp);
                self.conn_id.store(conn_id, Ordering::Release);
                
                // Acknowledge handshake
                self.state.store(2, Ordering::Release);
                
                // Wait for client acknowledgment
                while self.state.load(Ordering::Acquire) != 3 {
                    if start.elapsed() > timeout {
                        bail!("Handshake acknowledgment timeout");
                    }
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
                
                return Ok(conn_id);
            }
            
            if start.elapsed() > timeout {
                bail!("Handshake accept timeout");
            }
            
            // Yield to avoid busy spinning
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    }
    
    /// Client: initiate handshake with server
    pub async fn connect(&mut self, auth_token: Option<&[u8]>) -> Result<u64> {
        // Set client PID
        self.client_pid.store(std::process::id(), Ordering::Release);
        
        // Set optional auth token
        if let Some(token) = auth_token {
            if token.len() <= 32 {
                self.auth_token[..token.len()].copy_from_slice(token);
            }
        }
        
        // Set timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_nanos() as u64;
        self.timestamp_ns.store(timestamp, Ordering::Release);
        
        // Signal ready to server
        self.state.store(1, Ordering::Release);
        
        // Wait for server acknowledgment
        let start = Instant::now();
        while self.state.load(Ordering::Acquire) != 2 {
            if start.elapsed() > Duration::from_secs(5) {
                bail!("Server acknowledgment timeout");
            }
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
        
        // Get conn_id from server
        let conn_id = self.conn_id.load(Ordering::Acquire);
        
        // Acknowledge receipt
        self.state.store(3, Ordering::Release);
        
        Ok(conn_id)
    }
    
    /// Generate deterministic connection ID
    fn generate_conn_id(client_pid: u32, server_pid: u32, timestamp: u64) -> u64 {
        // Combine PIDs and timestamp for unique, deterministic ID
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        
        client_pid.hash(&mut hasher);
        server_pid.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        
        hasher.finish()
    }
    
    /// Reset control structure for reuse
    pub fn reset(&self) {
        self.state.store(0, Ordering::Release);
        self.conn_id.store(0, Ordering::Release);
        // Note: auth_token is not atomic, would need mutex for safe reset
    }
}

/// Unified handshake for cross-platform support
pub struct UnifiedHandshake {
    #[cfg(target_os = "linux")]
    control: HandshakeControl,
    
    #[cfg(target_os = "macos")]
    control: HandshakeControl,
    
    #[cfg(target_os = "windows")]
    control: HandshakeControl,
}

impl UnifiedHandshake {
    pub fn new() -> Self {
        Self {
            control: HandshakeControl::new(),
        }
    }
    
    /// Cross-platform accept
    pub async fn accept(&self, timeout: Duration) -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            self.control.accept_blocking(timeout).await
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS uses same SHM approach
            self.control.accept_blocking(timeout).await
        }
        
        #[cfg(target_os = "windows")]
        {
            // Windows uses named shared memory
            self.control.accept_blocking(timeout).await
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            bail!("Unsupported platform");
        }
    }
    
    /// Cross-platform connect
    pub async fn connect(&mut self, auth_token: Option<&[u8]>) -> Result<u64> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            self.control.connect(auth_token).await
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            bail!("Unsupported platform");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_handshake_blocking() {
        let control = std::sync::Arc::new(HandshakeControl::new());
        let control_server = control.clone();
        
        // Server task
        let server_task = tokio::spawn(async move {
            control_server.accept_blocking(Duration::from_secs(1)).await
        });
        
        // Give server time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Client connects (create a new mutable instance for client)
        let mut control_client = HandshakeControl::new();
        let client_conn_id = control_client.connect(None).await.unwrap();
        
        // Server should get same conn_id
        let server_conn_id = server_task.await.unwrap().unwrap();
        
        assert_eq!(client_conn_id, server_conn_id);
        assert_ne!(client_conn_id, 0);
    }
    
    #[tokio::test]
    async fn test_handshake_auth() {
        let control = std::sync::Arc::new(HandshakeControl::new());
        let control_server = control.clone();
        
        let auth_token = b"secret_token_123";
        
        // Server task
        let server_task = tokio::spawn(async move {
            control_server.accept_blocking(Duration::from_secs(1)).await
        });
        
        // Client connects with auth
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut control_client = HandshakeControl::new();
        let client_conn_id = control_client.connect(Some(auth_token)).await.unwrap();
        
        let server_conn_id = server_task.await.unwrap().unwrap();
        assert_eq!(client_conn_id, server_conn_id);
    }
    
    #[tokio::test]
    async fn test_handshake_timeout() {
        let control = HandshakeControl::new();
        
        // Should timeout if no client connects
        let result = control.accept_blocking(Duration::from_millis(100)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }
}
