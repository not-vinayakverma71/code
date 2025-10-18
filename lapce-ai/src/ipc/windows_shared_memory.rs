/// Windows Shared Memory Implementation
/// Provides API parity with Unix shared_memory_complete module
/// Uses Win32 CreateFileMapping/MapViewOfFile for shared memory

use anyhow::{Result, bail};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

// Windows-specific imports
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::System::SystemInformation::*;
use windows_sys::Win32::Security::*;

// Include the shared header module inline (same as Unix)
mod shared_memory_header {
    include!("shared_memory_header.rs");
}
use shared_memory_header::*;

// Constants matching Unix implementation
const NUM_SLOTS: usize = 32;
const SLOT_SIZE: usize = 128 * 1024;  // 128KB per slot
const CONTROL_SIZE: usize = 4096;     // 4KB for control channel
const SHM_PROTOCOL_VERSION: u8 = 1;

/// Simple lock-free ring buffer (Windows version)
pub struct SharedMemoryBuffer {
    ptr: *mut u8,
    header: *mut RingBufferHeader,
    data_ptr: *mut u8,
    size: usize,
    capacity: usize,
    mapping_handle: HANDLE,  // Windows mapping handle (closed in Drop)
    // Note: view is unmapped via ptr in Drop
}

unsafe impl Send for SharedMemoryBuffer {}
unsafe impl Sync for SharedMemoryBuffer {}

impl SharedMemoryBuffer {
    /// Sanitize name for Windows object namespace (deterministic across processes)
    fn sanitize_name(path: &str) -> String {
        // Sanitize path: keep only alphanumeric, underscore, dash
        let sanitized = path
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect::<String>();
        
        // Deterministic object name under the Session-local namespace.
        // Important: No PID or time-based randomness so both processes open the SAME mapping.
        // Use a single component after Local\\ to avoid nested object directories.
        let mut name = format!("Local\\LapceAI_{}", sanitized);
        
        // Cap length to be conservative for kernel object names
        if name.len() > 240 {
            name.truncate(240);
        }
        name
    }
    
    /// Convert string to wide string for Windows API
    fn to_wide_string(s: &str) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
    
    /// Create new shared memory buffer
    pub fn create(path: &str, _requested_size: usize) -> Result<Self> {
        let data_size = SLOT_SIZE * NUM_SLOTS;
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let total_size = header_size + data_size;
        
        // Sanitize and create Windows object name
        let object_name = Self::sanitize_name(path);
        let wide_name = Self::to_wide_string(&object_name);
        
        unsafe {
            // Create file mapping object
            let mapping_handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null_mut(),
                PAGE_READWRITE,
                (total_size >> 32) as u32,
                total_size as u32,
                wide_name.as_ptr(),
            );
            
            if mapping_handle.is_null() {
                bail!("Failed to create file mapping: {}", object_name);
            }
            
            // Map view of file into process address space
            let ptr = MapViewOfFile(
                mapping_handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                total_size,
            ).Value as *mut u8;
            
            if ptr.is_null() {
                CloseHandle(mapping_handle);
                let err = std::io::Error::last_os_error();
                bail!("MapViewOfFile failed: {}", err);
            }
            
            // Initialize header
            let header = RingBufferHeader::initialize(ptr, data_size);
            
            Ok(Self {
                ptr,
                size: total_size,
                capacity: data_size,
                header,
                data_ptr: ptr.add(header_size),
                mapping_handle,
            })
        }
    }
    
    /// Open existing shared memory
    pub fn open(path: &str, _size: usize) -> Result<Self> {
        let data_size = SLOT_SIZE * NUM_SLOTS;
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let total_size = header_size + data_size;
        
        // Sanitize and create Windows object name
        let object_name = Self::sanitize_name(path);
        let wide_name = Self::to_wide_string(&object_name);
        
        unsafe {
            // Open existing file mapping
            let mapping_handle = OpenFileMappingW(
                FILE_MAP_ALL_ACCESS,
                0,  // FALSE - don't inherit handle
                wide_name.as_ptr(),
            );
            
            if mapping_handle.is_null() {
                let err = std::io::Error::last_os_error();
                bail!("OpenFileMappingW failed for '{}': {}", object_name, err);
            }
            
            // Map view of file
            let ptr = MapViewOfFile(
                mapping_handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                total_size,
            ).Value as *mut u8;
            
            if ptr.is_null() {
                CloseHandle(mapping_handle);
                let err = std::io::Error::last_os_error();
                bail!("MapViewOfFile failed: {}", err);
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
                mapping_handle,
            })
        }
    }
    
    /// Write to buffer (lock-free) - identical to Unix
    #[inline(always)]
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        if data.len() > SLOT_SIZE {
            bail!("Data too large: {} > {}", data.len(), SLOT_SIZE);
        }
        
        unsafe {
            let header = &*self.header;
            
            // Find free slot
            for _ in 0..NUM_SLOTS * 2 {
                let write_pos = header.write_pos.load(Ordering::Acquire);
                let read_pos = header.read_pos.load(Ordering::Acquire);
                
                if write_pos - read_pos >= NUM_SLOTS {
                    std::thread::yield_now();
                    continue;
                }
                
                let slot_idx = write_pos % NUM_SLOTS;
                let slot_ptr = self.data_ptr.add(slot_idx * SLOT_SIZE);
                
                // Write length prefix
                let len_bytes = (data.len() as u32).to_le_bytes();
                ptr::copy_nonoverlapping(len_bytes.as_ptr(), slot_ptr, 4);
                
                // Write data
                ptr::copy_nonoverlapping(data.as_ptr(), slot_ptr.add(4), data.len());
                
                // Publish
                header.write_pos.store(write_pos + 1, Ordering::Release);
                return Ok(());
            }
            
            bail!("Buffer full after retries");
        }
    }
    
    /// Read from buffer (lock-free) - identical to Unix
    pub fn read(&mut self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*self.header;
            
            for _ in 0..10 {
                let read_pos = header.read_pos.load(Ordering::Acquire);
                let write_pos = header.write_pos.load(Ordering::Acquire);
                
                if read_pos >= write_pos {
                    return None;
                }
                
                let slot_idx = read_pos % NUM_SLOTS;
                let slot_ptr = self.data_ptr.add(slot_idx * SLOT_SIZE);
                
                // Read length
                let mut len_bytes = [0u8; 4];
                ptr::copy_nonoverlapping(slot_ptr, len_bytes.as_mut_ptr(), 4);
                let len = u32::from_le_bytes(len_bytes) as usize;
                
                if len == 0 || len > SLOT_SIZE {
                    header.read_pos.store(read_pos + 1, Ordering::Release);
                    continue;
                }
                
                // Read data
                let mut data = vec![0u8; len];
                ptr::copy_nonoverlapping(slot_ptr.add(4), data.as_mut_ptr(), len);
                
                // Advance read position
                if header.read_pos.compare_exchange(
                    read_pos,
                    read_pos + 1,
                    Ordering::Release,
                    Ordering::Relaxed
                ).is_ok() {
                    return Some(data);
                }
            }
            
            None
        }
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                // Unmap the view
                let view_addr = MEMORY_MAPPED_VIEW_ADDRESS { Value: self.ptr as *mut _ };
                UnmapViewOfFile(view_addr);
            }
            if !self.mapping_handle.is_null() {
                // Close the mapping handle
                CloseHandle(self.mapping_handle);
            }
        }
    }
}

/// Listener for incoming shared memory connections (Windows)
pub struct SharedMemoryListener {
    control_path: String,
    control_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    accept_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<AcceptRequest>>>,
    _control_task: JoinHandle<()>,
    is_owner: bool,
}

#[derive(Debug)]
struct AcceptRequest {
    conn_id: u64,
    response_tx: oneshot::Sender<Result<()>>,
}

impl SharedMemoryListener {
    pub fn bind(path: &str) -> Result<Self> {
        let control_path = format!("{}_control", path);
        let control_buffer = Arc::new(RwLock::new(SharedMemoryBuffer::create(&control_path, CONTROL_SIZE)?));
        
        let (accept_tx, accept_rx) = mpsc::unbounded_channel();
        
        let control_buffer_clone = control_buffer.clone();
        let _control_task = tokio::spawn(async move {
            Self::handle_control_channel(control_buffer_clone, accept_tx).await;
        });
        
        Ok(Self {
            control_path,
            control_buffer,
            accept_rx: Arc::new(tokio::sync::Mutex::new(accept_rx)),
            _control_task,
            is_owner: true,
        })
    }
    
    async fn handle_control_channel(
        control_buffer: Arc<RwLock<SharedMemoryBuffer>>,
        accept_tx: mpsc::UnboundedSender<AcceptRequest>,
    ) {
        loop {
            // Poll for incoming connection requests
            if let Some(data) = control_buffer.write().read() {
                if data.len() >= 9 && data[0] == b'C' {
                    // Connection request format: 'C' + 8 bytes conn_id
                    let mut conn_id_bytes = [0u8; 8];
                    conn_id_bytes.copy_from_slice(&data[1..9]);
                    let conn_id = u64::from_le_bytes(conn_id_bytes);
                    
                    let (response_tx, response_rx) = oneshot::channel();
                    let req = AcceptRequest { conn_id, response_tx };
                    
                    if accept_tx.send(req).is_err() {
                        break;
                    }
                    
                    // Wait for accept to complete
                    if response_rx.await.is_ok() {
                        // Send ACK
                        let ack = [b'A'; 1];
                        let _ = control_buffer.write().write(&ack);
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }
    
    pub async fn accept(&self) -> Result<(SharedMemoryStream, std::net::SocketAddr)> {
        let mut rx = self.accept_rx.lock().await;
        let req = rx.recv().await
            .ok_or_else(|| anyhow::anyhow!("Control channel closed"))?;
        
        let base_path = self.control_path.trim_end_matches("_control");
        let send_path = format!("{}_{}_send", base_path, req.conn_id);
        let recv_path = format!("{}_{}_recv", base_path, req.conn_id);
        
        // Create buffers for this connection
        let send_buffer = SharedMemoryBuffer::create(&send_path, 4 * 1024 * 1024)?;
        let recv_buffer = SharedMemoryBuffer::create(&recv_path, 4 * 1024 * 1024)?;
        
        // Signal accept complete
        let _ = req.response_tx.send(Ok(()));
        
        // Return dummy SocketAddr for API compatibility (shared memory doesn't use network addresses)
        let dummy_addr = "127.0.0.1:0".parse().unwrap();
        
        Ok((SharedMemoryStream {
            send_buffer: Arc::new(RwLock::new(send_buffer)),
            recv_buffer: Arc::new(RwLock::new(recv_buffer)),
            conn_id: req.conn_id,
        }, dummy_addr))
    }
}

impl Drop for SharedMemoryListener {
    fn drop(&mut self) {
        // Windows doesn't need explicit cleanup like shm_unlink
        // Objects are destroyed when last handle closes
    }
}

/// Shared memory stream (Windows)
pub struct SharedMemoryStream {
    send_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    recv_buffer: Arc<RwLock<SharedMemoryBuffer>>,
    conn_id: u64,
}

impl SharedMemoryStream {
    pub async fn connect(path: &str) -> Result<Self> {
        let control_path = format!("{}_control", path);
        
        // Open control channel
        let mut control = SharedMemoryBuffer::open(&control_path, CONTROL_SIZE)?;
        
        // Generate connection ID
        let conn_id = std::process::id() as u64 + 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
        
        // Send connection request
        let mut request = vec![b'C'];
        request.extend_from_slice(&conn_id.to_le_bytes());
        control.write(&request)?;
        
        // Wait for ACK
        let start = std::time::Instant::now();
        loop {
            if let Some(data) = control.read() {
                if data.len() == 1 && data[0] == b'A' {
                    break;
                }
            }
            
            if start.elapsed() > std::time::Duration::from_secs(5) {
                bail!("Connection timeout");
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        // Open data channels (note: send/recv swapped for client)
        let base_path = control_path.trim_end_matches("_control");
        let send_path = format!("{}_{}_recv", base_path, conn_id);
        let recv_path = format!("{}_{}_send", base_path, conn_id);
        
        Ok(Self {
            send_buffer: Arc::new(RwLock::new(
                SharedMemoryBuffer::open(&send_path, 4 * 1024 * 1024)?
            )),
            recv_buffer: Arc::new(RwLock::new(
                SharedMemoryBuffer::open(&recv_path, 4 * 1024 * 1024)?
            )),
            conn_id,
        })
    }
    
    pub fn conn_id(&self) -> u64 {
        self.conn_id
    }
    
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        self.send_buffer.write().write(data)
    }
    
    pub async fn recv(&self) -> Result<Option<Vec<u8>>> {
        Ok(self.recv_buffer.write().read())
    }
}

// Implement AsyncRead for SharedMemoryStream
impl tokio::io::AsyncRead for SharedMemoryStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(data) = self.recv_buffer.write().read() {
            let to_copy = std::cmp::min(data.len(), buf.remaining());
            buf.put_slice(&data[..to_copy]);
            std::task::Poll::Ready(Ok(()))
        } else {
            std::task::Poll::Pending
        }
    }
}

// Implement AsyncWrite for SharedMemoryStream
impl tokio::io::AsyncWrite for SharedMemoryStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.send_buffer.write().write(buf) {
            Ok(_) => std::task::Poll::Ready(Ok(buf.len())),
            Err(e) => std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}
