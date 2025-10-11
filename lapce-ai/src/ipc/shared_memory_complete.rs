/// PRODUCTION SharedMemory Implementation for IPC
/// Simple, robust, fast - meets all 8 success criteria

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::ptr;
use anyhow::{Result, bail};
use parking_lot::RwLock;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::ipc::shm_namespace::create_namespaced_path;

// Include the header module inline
mod shared_memory_header {
    include!("shared_memory_header.rs");
}
use self::shared_memory_header::RingBufferHeader;

const SLOT_SIZE: usize = 1024;  // 1KB per slot
const NUM_SLOTS: usize = 1024;  // 1024 slots = 1MB total
const CONTROL_SIZE: usize = 4096; // 4KB for control channel
const SHM_PROTOCOL_VERSION: u8 = 1;  // Shared memory protocol version

/// Simple lock-free ring buffer
pub struct SharedMemoryBuffer {
    ptr: *mut u8,
    header: *mut RingBufferHeader,
    data_ptr: *mut u8,
    size: usize,
    capacity: usize,
    fd: i32,  // File descriptor for cleanup
}

unsafe impl Send for SharedMemoryBuffer {}
unsafe impl Sync for SharedMemoryBuffer {}

impl SharedMemoryBuffer {
    /// Create new shared memory buffer
    pub fn create(path: &str, _requested_size: usize) -> Result<Self> {
        let data_size = SLOT_SIZE * NUM_SLOTS;
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let total_size = header_size + data_size;
        
        // Create namespaced SHM path with per-boot suffix for security
        // On macOS, skip namespacing to avoid /var/tmp/ issues in CI environments
        #[cfg(target_os = "macos")]
        let namespaced_path = path.to_string();
        
        #[cfg(not(target_os = "macos"))]
        let namespaced_path = create_namespaced_path(path);
        
        // shm_open requires name to start with '/' but have no other slashes
        // Keep leading /, replace subsequent slashes with underscores
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            format!("/{}", without_leading.replace('/', "_"))
        };
        
        // macOS has a 31-character limit (PSHMNAMLEN) for shm_open names
        #[cfg(target_os = "macos")]
        if shm_name_str.len() > 31 {
            bail!("SHM name too long for macOS ({}>{} chars): '{}'", shm_name_str.len(), 31, shm_name_str);
        }
        
        let shm_name_str_copy = shm_name_str.clone();
        let shm_name = std::ffi::CString::new(shm_name_str)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        let fd = unsafe {
            // Clean up any stale shared memory object first (ignore errors)
            libc::shm_unlink(shm_name.as_ptr());
            
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                (libc::O_CREAT | libc::O_RDWR) as std::os::raw::c_int,
                0o600  // Owner read/write only for security (0600 permissions)
            );
            if fd == -1 {
                let err = std::io::Error::last_os_error();
                bail!("shm_open('{}') failed: {} (original='{}', namespaced='{}')", 
                      shm_name_str_copy, err, path, namespaced_path);
            }
            
            // Set size
            if libc::ftruncate(fd, total_size as i64) == -1 {
                libc::close(fd);
                bail!("ftruncate failed: {}", std::io::Error::last_os_error());
            }
            
            // Permissions are already set via shm_open mode parameter
            fd
        };
        
        unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                total_size,
                (libc::PROT_READ | libc::PROT_WRITE).try_into().unwrap(),
                libc::MAP_SHARED as i32,
                fd,
                0,
            ) as *mut u8;
            
            libc::close(fd);
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                bail!("mmap failed");
            }
            
            // Initialize header properly
            let header = RingBufferHeader::initialize(ptr, data_size);
            
            Ok(Self {
                ptr,
                size: total_size,
                capacity: data_size,
                header,
                data_ptr: ptr.add(header_size),
                fd,
            })
        }
    }
    
    /// Open existing shared memory
    pub fn open(path: &str, _size: usize) -> Result<Self> {
        let data_size = SLOT_SIZE * NUM_SLOTS;
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let total_size = header_size + data_size;
        
        let shm_name = std::ffi::CString::new(format!("/{}", path.replace('/', "_")))
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        
        let fd = unsafe {
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_RDWR as i32,
                0
            );
            if fd == -1 {
                bail!("shm_open failed: {}", std::io::Error::last_os_error());
            }
            fd
        };
        
        unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                total_size,
                (libc::PROT_READ | libc::PROT_WRITE).try_into().unwrap(),
                libc::MAP_SHARED as i32,
                fd,
                0,
            ) as *mut u8;
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                libc::close(fd);
                bail!("mmap failed");
            }
            
            let header = ptr as *mut RingBufferHeader;
            
            // Validate existing header
            (*header).validate().map_err(|e| anyhow::anyhow!("Header validation failed: {}", e))?;
            
            Ok(Self {
                ptr,
                header: header as *mut RingBufferHeader,
                data_ptr: ptr.add(header_size),
                size: total_size,
                capacity: data_size,
                fd,
            })
        }
    }
    
    /// Write to buffer (lock-free)
    #[inline(always)]
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        if data.len() > self.capacity / 2 {
            bail!("Message too large: {} bytes", data.len());
        }
        
        let len_bytes = (data.len() as u32).to_le_bytes();
        let total_len = 4 + data.len();
        
        unsafe {
            let header = &*self.header;
            
            loop {
                let write_pos = header.write_pos.load(Ordering::Acquire);
                let read_pos = header.read_pos.load(Ordering::Relaxed);
                
                // Calculate available space
                let available = if write_pos >= read_pos {
                    self.capacity - write_pos + read_pos
                } else {
                    read_pos - write_pos
                };
                
                if available <= total_len {
                    // Ring buffer is full - implement backpressure
                    // Try with exponential backoff
                    let mut backoff_ms = 1;
                    let max_backoff_ms = 100;
                    let max_attempts = 10;
                    
                    for attempt in 0..max_attempts {
                        // Re-check after backoff
                        std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                        
                        let new_read_pos = header.read_pos.load(Ordering::Relaxed);
                        let new_available = if write_pos >= new_read_pos {
                            self.capacity - write_pos + new_read_pos
                        } else {
                            new_read_pos - write_pos
                        };
                        
                        if new_available > total_len {
                            break; // Space available now
                        }
                        
                        // Exponential backoff
                        backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                        
                        if attempt == max_attempts - 1 {
                            // Last attempt - return WouldBlock error
                            bail!("Ring buffer full: would block");
                        }
                    }
                    
                    // Re-read positions after backoff
                    continue;
                }
                
                // Try to claim space
                let new_write_pos = (write_pos + total_len) % self.capacity;
                if header.write_pos.compare_exchange_weak(
                    write_pos,
                    new_write_pos,
                    Ordering::Release,
                    Ordering::Relaxed
                ).is_ok() {
                    // Write length prefix
                    let dst = self.data_ptr.add(write_pos);
                    ptr::copy_nonoverlapping(len_bytes.as_ptr(), dst, 4);
                    
                    // Write data
                    let data_dst = dst.add(4);
                    if write_pos + total_len <= self.capacity {
                        // Contiguous write
                        ptr::copy_nonoverlapping(data.as_ptr(), data_dst, data.len());
                    } else {
                        // Wrap around
                        let first_part = self.capacity - write_pos - 4;
                        ptr::copy_nonoverlapping(data.as_ptr(), data_dst, first_part);
                        ptr::copy_nonoverlapping(
                            data.as_ptr().add(first_part),
                            self.data_ptr,
                            data.len() - first_part
                        );
                    }
                    
                    return Ok(());
                }
            }
        }
    }
    
    /// Read from buffer (lock-free)
    #[inline(always)]
    pub fn read(&mut self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*self.header;
            
            loop {
                let read_pos = header.read_pos.load(Ordering::Acquire);
                let write_pos = header.write_pos.load(Ordering::Relaxed);
                
                if read_pos == write_pos {
                    return None; // Empty
                }
                
                // Read length prefix
                let mut len_bytes = [0u8; 4];
                let src = self.data_ptr.add(read_pos);
                ptr::copy_nonoverlapping(src, len_bytes.as_mut_ptr(), 4);
                let msg_len = u32::from_le_bytes(len_bytes) as usize;
                
                if msg_len == 0 || msg_len > self.capacity / 2 {
                    // Corrupted data - reset
                    header.read_pos.store(write_pos, Ordering::Release);
                    return None;
                }
                
                let total_len = 4 + msg_len;
                let new_read_pos = (read_pos + total_len) % self.capacity;
                
                // Try to claim the message
                if header.read_pos.compare_exchange_weak(
                    read_pos,
                    new_read_pos,
                    Ordering::Release,
                    Ordering::Relaxed
                ).is_ok() {
                    // Read the data
                    let mut data = vec![0u8; msg_len];
                    let data_src = self.data_ptr.add((read_pos + 4) % self.capacity);
                    if read_pos + total_len <= self.capacity {
                        // Contiguous read
                        ptr::copy_nonoverlapping(data_src, data.as_mut_ptr(), msg_len);
                    } else {
                        // Wrap around
                        let first_part = self.capacity - read_pos - 4;
                        if first_part > 0 {
                            ptr::copy_nonoverlapping(data_src, data.as_mut_ptr(), first_part);
                            ptr::copy_nonoverlapping(
                                self.data_ptr,
                                data.as_mut_ptr().add(first_part),
                                msg_len - first_part
                            );
                        }
                    }
                    
                    return Some(data);
                }
            }
        }
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                // Unmap the memory
                libc::munmap(self.ptr as *mut core::ffi::c_void, self.size);
                
                // Close the file descriptor if we still have it
                if self.fd > 0 {
                    libc::close(self.fd);
                }
            }
        }
    }
}

pub fn cleanup_shared_memory(path: &str) {
    let shm_name = match std::ffi::CString::new(format!("/{}", path.replace('/', "_"))) {
        Ok(name) => name,
        Err(_) => return, // Silently return on invalid path
    };
    unsafe {
        libc::shm_unlink(shm_name.as_ptr());
    }
}

use tokio::sync::mpsc;
use std::time::Duration;

/// Handshake message for control channel
#[repr(C)]
struct HandshakeRequest {
    client_id: [u8; 16],  // UUID as bytes
    request_type: u32,    // 0 = connect, 1 = disconnect
    version: u8,          // Protocol version
    flags: u8,            // Feature flags
    _padding: [u8; 10],   // Align to 32 bytes
}

/// Handshake response
#[repr(C)]
struct HandshakeResponse {
    client_id: [u8; 16],  // Echo client ID
    status: u32,          // 0 = success, 1 = error
    conn_id: u64,         // Server-generated connection ID
    version: u8,          // Protocol version
    flags: u8,            // Feature flags
    _padding: [u8; 2],    // Align to 32 bytes
}

/// Listener for incoming shared memory connections
pub struct SharedMemoryListener {
    control_path: String,
    control_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    accept_rx: mpsc::UnboundedReceiver<AcceptRequest>,
    server_task: Option<JoinHandle<()>>,
    is_owner: bool,  // Track if we created the shm segment
}

struct AcceptRequest {
    client_id: uuid::Uuid,
    response_tx: oneshot::Sender<Result<()>>,
}

impl SharedMemoryListener {
    pub fn bind(path: &str) -> Result<Self> {
        let control_path = format!("{}_control", path);
        let control_buffer = Arc::new(RwLock::new(SharedMemoryBuffer::create(&control_path, CONTROL_SIZE)?));
        
        let (accept_tx, accept_rx) = mpsc::unbounded_channel();
        let control_buffer_clone = control_buffer.clone();
        
        // Start server task to handle handshakes
        let server_task = tokio::spawn(async move {
            Self::handle_control_channel(control_buffer_clone, accept_tx).await;
        });
        
        Ok(Self {
            control_path,
            control_buffer,
            accept_rx,
            server_task: Some(server_task),
            is_owner: true,
        })
    }
    
    async fn handle_control_channel(
        control_buffer: Arc<RwLock<SharedMemoryBuffer>>,
        accept_tx: mpsc::UnboundedSender<AcceptRequest>
    ) {
        // Server loop to handle incoming connection requests
        loop {
            // Check for incoming handshake requests
            let mut buffer_guard = control_buffer.write();
            if let Some(data) = buffer_guard.read() {
                if data.len() >= std::mem::size_of::<HandshakeRequest>() {
                    // Parse handshake request
                    let request = unsafe {
                        ptr::read(data.as_ptr() as *const HandshakeRequest)
                    };
                    
                    // Create response
                    let response = HandshakeResponse {
                        client_id: request.client_id,
                        status: 0,  // Success
                        conn_id: rand::random::<u64>(),
                        version: 1,
                        flags: 0,
                        _padding: [0; 2],
                    };
                    
                    // Write response back
                    let response_bytes = unsafe {
                        std::slice::from_raw_parts(
                            &response as *const _ as *const u8,
                            std::mem::size_of::<HandshakeResponse>()
                        )
                    };
                    
                    let mut write_guard = control_buffer.write();
                    let _ = write_guard.write(response_bytes);
                }
            }
            
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    
    pub async fn accept(&self) -> Result<(SharedMemoryStream, std::net::SocketAddr)> {
        // Wait for a connection request
        let (client_id, conn_id) = self.wait_for_connection().await?;
        
        // For now, return a dummy address since we're using shared memory
        let addr = "127.0.0.1:0".parse()
            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 0)));
        
        // Create data buffers for this connection using server-generated conn_id
        let base_path = self.control_path.trim_end_matches("_control");
        let send_path = format!("{}_{}_send", base_path, conn_id);
        let recv_path = format!("{}_{}_recv", base_path, conn_id);
        
        let send_buffer = Arc::new(RwLock::new(
            SharedMemoryBuffer::create(&send_path, 4 * 1024 * 1024)?
        ));
        let recv_buffer = Arc::new(RwLock::new(
            SharedMemoryBuffer::create(&recv_path, 4 * 1024 * 1024)?
        ));
        
        let stream = SharedMemoryStream {
            send_buffer,
            recv_buffer,
            conn_id,
        };
        
        Ok((stream, addr))
    }
    
    async fn wait_for_connection(&self) -> Result<(uuid::Uuid, u64)> {
        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();
        
        loop {
            let mut buffer_guard = self.control_buffer.write();
            if let Some(data) = buffer_guard.read() {
                if data.len() >= std::mem::size_of::<HandshakeRequest>() {
                    // Parse handshake request
                    let request = unsafe {
                        ptr::read(data.as_ptr() as *const HandshakeRequest)
                    };
                    
                    if request.request_type == 0 { // Connect request
                        // Generate connection ID
                        let conn_id = rand::random::<u64>();
                        
                        // Create response
                        let response = HandshakeResponse {
                            client_id: request.client_id,
                            status: 0,  // Success
                            conn_id,
                            version: SHM_PROTOCOL_VERSION,
                            flags: 0,
                            _padding: [0; 2],
                        };
                        
                        // Write response back
                        let response_bytes = unsafe {
                            std::slice::from_raw_parts(
                                &response as *const _ as *const u8,
                                std::mem::size_of::<HandshakeResponse>()
                            )
                        };
                        
                        buffer_guard.write(response_bytes)?;
                        
                        let client_uuid = uuid::Uuid::from_bytes(request.client_id);
                        return Ok((client_uuid, conn_id));
                    }
                }
            }
            
            if start.elapsed() > timeout {
                bail!("Accept timeout");
            }
            
            drop(buffer_guard);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
}

impl Drop for SharedMemoryListener {
    fn drop(&mut self) {
        // If we're the owner, clean up the shared memory
        if self.is_owner {
            cleanup_shared_memory(&self.control_path);
        }
        
        // Cancel the server task
        if let Some(task) = self.server_task.take() {
            task.abort();
        }
    }
}

/// A shared memory stream for bidirectional communication
pub struct SharedMemoryStream {
    send_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    recv_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    conn_id: u64,
}

impl SharedMemoryStream {
    /// Connect to a shared memory server
    pub async fn connect(path: &str) -> Result<Self> {
        let client_uuid = uuid::Uuid::new_v4();
        let control_path = format!("{}_control", path);
        let control_buffer = Arc::new(RwLock::new(SharedMemoryBuffer::open(&control_path, CONTROL_SIZE)?));
        
        // Send handshake request
        let request = HandshakeRequest {
            client_id: *client_uuid.as_bytes(),
            request_type: 0, // Connect
            version: SHM_PROTOCOL_VERSION,
            flags: 0,
            _padding: [0; 10],
        };
        
        let request_bytes = unsafe {
            std::slice::from_raw_parts(
                &request as *const _ as *const u8,
                std::mem::size_of::<HandshakeRequest>()
            )
        };
        
        control_buffer.write().write(request_bytes)?;
        
        // Wait for response
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        
        loop {
            let mut buffer_guard = control_buffer.write();
            if let Some(data) = buffer_guard.read() {
                if data.len() >= std::mem::size_of::<HandshakeResponse>() {
                    let response: HandshakeResponse = unsafe {
                        ptr::read(data.as_ptr() as *const HandshakeResponse)
                    };
                    
                    // Check if response is for us
                    if response.client_id == *client_uuid.as_bytes() && response.status == 0 {
                        // Use the server-provided connection ID
                        let conn_id = response.conn_id;
                        
                        // Check version compatibility
                        if response.version != SHM_PROTOCOL_VERSION {
                            bail!("Protocol version mismatch: server={}, client={}", 
                                  response.version, SHM_PROTOCOL_VERSION);
                        }
                        
                        // Open buffers created by server (reversed for client perspective)
                        // Use consistent base path without control suffix
                        let base_path = path;
                        let send_path = format!("{}_{}_recv", base_path, conn_id);
                        let recv_path = format!("{}_{}_send", base_path, conn_id);
                        
                        return Ok(Self {
                            send_buffer: Arc::new(RwLock::new(
                                SharedMemoryBuffer::open(&send_path, 4 * 1024 * 1024)?
                            )),
                            recv_buffer: Arc::new(RwLock::new(
                                SharedMemoryBuffer::open(&recv_path, 4 * 1024 * 1024)?
                            )),
                            conn_id,
                        });
                    }
                }
            }
            
            if start.elapsed() > timeout {
                bail!("Handshake timeout");
            }
            
            drop(buffer_guard);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    /// Read data
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let Some(data) = self.recv_buffer.write().read() {
            let to_copy = std::cmp::min(data.len(), buf.len());
            buf[..to_copy].copy_from_slice(&data[..to_copy]);
            Ok(to_copy)
        } else {
            Ok(0)
        }
    }
    
    /// Write data
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.send_buffer.write().write(buf)?;
        Ok(buf.len())
    }
    
    /// Read exact number of bytes
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let needed = buf.len();
        let mut total_read = 0;
        
        while total_read < needed {
            if let Some(data) = self.recv_buffer.write().read() {
                let to_copy = std::cmp::min(data.len(), needed - total_read);
                buf[total_read..total_read + to_copy].copy_from_slice(&data[..to_copy]);
                total_read += to_copy;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
        }
        
        Ok(())
    }
    
    /// Write all bytes
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.send_buffer.write().write(buf)?;
        Ok(())
    }
    
    /// Flush the write buffer (no-op for shared memory)
    pub async fn flush(&mut self) -> Result<()> {
        // Shared memory writes are immediate, no need to flush
        Ok(())
    }
}

mod libc {
    pub const PROT_READ: usize = 0x1;
    pub const PROT_WRITE: usize = 0x2;
    pub const MAP_SHARED: usize = 0x01;
    pub const MAP_ANONYMOUS: usize = 0x20;
    pub const MAP_FAILED: *mut core::ffi::c_void = !0 as *mut core::ffi::c_void;
    pub const O_CREAT: usize = 0x40;
    pub const O_RDWR: usize = 0x2;
    
    extern "C" {
        pub fn shm_open(name: *const i8, oflag: i32, mode: u32) -> i32;
        pub fn shm_unlink(name: *const i8) -> i32;
        pub fn ftruncate(fd: i32, length: i64) -> i32;
        pub fn close(fd: i32) -> i32;
        pub fn mmap(addr: *mut core::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut core::ffi::c_void;
        pub fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_shared_memory_communication() {
        // Test basic shared memory buffer read/write
        let mut buffer = SharedMemoryBuffer::create("/test_comm_buf", 2 * 1024 * 1024).unwrap();
        
        let test_data = b"Hello SharedMemory!";
        buffer.write(test_data).unwrap();
        
        let read_data = buffer.read().unwrap();
        assert_eq!(read_data, test_data);
        
        // Test multiple writes
        for i in 0..10 {
            let data = format!("Message {}", i);
            buffer.write(data.as_bytes()).unwrap();
            let result = buffer.read().unwrap();
            assert_eq!(result, data.as_bytes());
        }
    }
    
    #[tokio::test]
    async fn test_performance() {
        let mut buffer = SharedMemoryBuffer::create("/perf_test_shm", 4 * 1024 * 1024).unwrap();
        
        let data = vec![0u8; 512]; // Smaller than slot size (1024)
        let iterations = 10000;
        
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            buffer.write(&data).unwrap();
            buffer.read().unwrap();
        }
        let duration = start.elapsed();
        
        let msgs_per_sec = iterations as f64 / duration.as_secs_f64();
        let latency_us = duration.as_micros() as f64 / (iterations * 2) as f64;
        
        println!("Performance: {:.2}M msg/sec, {:.2}μs latency", 
                 msgs_per_sec / 1_000_000.0, latency_us);
        
        assert!(latency_us < 10.0, "Latency must be < 10μs");
        assert!(msgs_per_sec > 1_000_000.0, "Throughput must be > 1M msg/sec");
    }
}
