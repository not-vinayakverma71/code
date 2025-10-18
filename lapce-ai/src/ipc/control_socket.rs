/// Control plane for IPC handshake using Unix domain sockets
/// Replaces filesystem watching for robust cross-process rendezvous

use serde::{Serialize, Deserialize};
use anyhow::{Result, bail};
use std::path::Path;
use std::os::unix::io::AsRawFd;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::ipc::platform_buffer::PlatformDoorbell;
use crate::ipc::fd_pass;

/// Handshake request from client
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeRequest {
    pub client_pid: u32,
    pub protocol_version: u16,
}

/// Handshake response from server
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeResponse {
    pub slot_id: u32,
    pub send_shm_name: String,  // Client writes here
    pub recv_shm_name: String,  // Client reads from here
    pub ring_capacity: u32,
    pub protocol_version: u32,
    // Note: eventfd file descriptors passed via SCM_RIGHTS, not serialized
    // Client receives: send_doorbell_fd, recv_doorbell_fd
}

const PROTOCOL_VERSION: u16 = 1;
const MAX_MESSAGE_SIZE: usize = 4096;

/// Control plane server
pub struct ControlServer {
    pub listener: UnixListener,
    control_path: String,
}

impl ControlServer {
    /// Bind control socket
    pub async fn bind(base_path: &str) -> Result<Self> {
        let control_path = format!("{}.ctl", base_path);
        
        // Remove stale socket
        let _ = std::fs::remove_file(&control_path);
        
        // Use std::os::unix::net::UnixListener with high backlog, then convert to tokio
        use std::os::unix::net::UnixListener as StdUnixListener;
        let std_listener = StdUnixListener::bind(&control_path)?;
        std_listener.set_nonblocking(true)?;
        
        // Convert to tokio UnixListener
        let listener = UnixListener::from_std(std_listener)?;
        
        #[cfg(unix)]
        {
            // Set permissions to 0600
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&control_path, perms)?;
        }
        
        eprintln!("[CONTROL] Bound control socket: {}", control_path);
        
        Ok(Self {
            listener,
            control_path,
        })
    }
    
    /// Accept a handshake from client
    pub async fn accept_handshake(
        &mut self, 
        slot_id: u32, 
        send_shm: &str, 
        recv_shm: &str, 
        capacity: u32,
        send_doorbell_fd: i32,
        recv_doorbell_fd: i32,
    ) -> Result<()> {
        let mut stream = self.listener.accept().await?.0;
        
        // Read request using ASYNC I/O (don't block runtime)
        let mut buf = vec![0u8; MAX_MESSAGE_SIZE];
        
        use tokio::io::AsyncReadExt;
        let n = stream.read(&mut buf).await?;
        
        let _request: HandshakeRequest = bincode::deserialize(&buf[..n])?;
        eprintln!("[CONTROL] Received handshake request");
        
        // Send response
        let response = HandshakeResponse {
            slot_id,
            send_shm_name: send_shm.to_string(),
            recv_shm_name: recv_shm.to_string(),
            ring_capacity: capacity,
            protocol_version: PROTOCOL_VERSION as u32,
        };
        
        let response_bytes = bincode::serialize(&response)?;
        
        // Convert to std::net::TcpStream for FD passing (SCM_RIGHTS needs blocking socket)
        let stream_std = stream.into_std()?;
        stream_std.set_nonblocking(false)?;
        
        // Send response WITH eventfd file descriptors via SCM_RIGHTS
        eprintln!("[CONTROL] Sending response with doorbell fds: send={}, recv={}", send_doorbell_fd, recv_doorbell_fd);
        fd_pass::send_fds(&stream_std, &[send_doorbell_fd, recv_doorbell_fd], &response_bytes)?;
        
        eprintln!("[CONTROL] Sent handshake response with doorbells");
        Ok(())
    }
    
    /// Get control socket path
    pub fn path(&self) -> &str {
        &self.control_path
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.control_path);
    }
}

/// Control plane client
pub struct ControlClient;

/// Client handshake result with doorbells
pub struct HandshakeResult {
    pub response: HandshakeResponse,
    pub send_doorbell_fd: i32,
    pub recv_doorbell_fd: i32,
}

impl ControlClient {
    /// Connect and perform handshake, receiving eventfd doorbells
    pub async fn handshake(base_path: &str) -> Result<HandshakeResult> {
        let control_path = format!("{}.ctl", base_path);
        
        eprintln!("[CONTROL CLIENT] Connecting to: {}", control_path);
        
        let stream = UnixStream::connect(&control_path).await?;
        let stream_std = stream.into_std()?;
        
        // Set to blocking mode for FD passing
        stream_std.set_nonblocking(false)?;
        
        eprintln!("[CONTROL CLIENT] Connected");
        
        // Send request
        let request = HandshakeRequest {
            client_pid: std::process::id(),
            protocol_version: PROTOCOL_VERSION,
        };
        
        let buf = bincode::serialize(&request)?;
        
        use std::io::Write;
        (&stream_std).write_all(&buf)?;
        (&stream_std).flush()?;
        
        eprintln!("[CONTROL CLIENT] Sent handshake request");
        
        // Receive response WITH eventfd file descriptors
        let (fds, response_bytes) = fd_pass::recv_fds(&stream_std, 2)?;
        
        if fds.len() != 2 {
            bail!("Expected 2 file descriptors, got {}", fds.len());
        }
        
        let response: HandshakeResponse = bincode::deserialize(&response_bytes)?;
        
        eprintln!("[CONTROL CLIENT] Received handshake response: {:?}", response);
        eprintln!("[CONTROL CLIENT] Received doorbell fds: send={}, recv={}", fds[0], fds[1]);
        
        Ok(HandshakeResult {
            response,
            send_doorbell_fd: fds[0],
            recv_doorbell_fd: fds[1],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_control_handshake() {
        let test_path = "/tmp/test_control_socket";
        let _ = std::fs::remove_file(format!("{}.ctl", test_path));
        
        // Spawn server
        let server = ControlServer::bind(test_path).await.unwrap();
        
        let server_handle = tokio::spawn(async move {
            let (req, stream) = server.accept_handshake().await.unwrap();
            assert_eq!(req.protocol_version, PROTOCOL_VERSION);
            
            let response = HandshakeResponse {
                slot_id: 42,
                send_shm_name: "/test_send".to_string(),
                recv_shm_name: "/test_recv".to_string(),
                ring_capacity: 1024,
                protocol_version: PROTOCOL_VERSION,
            };
            
            ControlServer::send_response(stream, response).await.unwrap();
        });
        
        // Give server time to bind
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Client handshake
        let response = ControlClient::handshake(test_path).await.unwrap();
        
        assert_eq!(response.slot_id, 42);
        assert_eq!(response.send_shm_name, "/test_send");
        assert_eq!(response.recv_shm_name, "/test_recv");
        assert_eq!(response.ring_capacity, 1024);
        
        server_handle.await.unwrap();
        
        let _ = std::fs::remove_file(format!("{}.ctl", test_path));
    }
}
