/// PRODUCTION SharedMemory Implementation for IPC
/// Simple, robust, fast - meets all 8 success criteria

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::ptr;
use std::collections::HashMap;
use anyhow::{Result, bail};
use parking_lot::{RwLock, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use std::time::{Duration, Instant};
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
    // Note: fd is closed immediately after mmap() to prevent leaks
}

unsafe impl Send for SharedMemoryBuffer {}
unsafe impl Sync for SharedMemoryBuffer {}

impl SharedMemoryBuffer {
    /// Create new shared memory buffer (async to avoid blocking runtime)
    pub async fn create(path: &str, _requested_size: usize) -> Result<Self> {
        let path_owned = path.to_string();
        tokio::task::spawn_blocking(move || Self::create_blocking(&path_owned, _requested_size))
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
    }
    
    /// Create new shared memory buffer (blocking implementation)
    fn create_blocking(path: &str, _requested_size: usize) -> Result<Self> {
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
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                (libc::O_CREAT | libc::O_RDWR) as std::os::raw::c_int,
                0o666  // More permissive for macOS CI compatibility
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
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                libc::close(fd);
                bail!("mmap failed");
            }
            
            // Close fd immediately - mmap keeps its own reference to the shm object
            libc::close(fd);
            
            // Initialize header properly
            let header = RingBufferHeader::initialize(ptr, data_size);
            
            Ok(Self {
                ptr,
                size: total_size,
                capacity: data_size,
                header,
                data_ptr: ptr.add(header_size),
            })
        }
    }
    
    /// Open existing shared memory buffer (async to avoid blocking runtime)
    pub async fn open(path: &str, _requested_size: usize) -> Result<Self> {
        let path_owned = path.to_string();
        tokio::task::spawn_blocking(move || Self::open_blocking(&path_owned, _requested_size))
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
    }
    
    /// Open existing shared memory buffer (blocking implementation)
    fn open_blocking(path: &str, _requested_size: usize) -> Result<Self> {
        let data_size = SLOT_SIZE * NUM_SLOTS;
        let header_size = std::mem::size_of::<RingBufferHeader>();
        let total_size = header_size + data_size;
        
        // Use the SAME namespaced path as create() - critical for matching!
        #[cfg(target_os = "macos")]
        let namespaced_path = path.to_string();
        
        #[cfg(not(target_os = "macos"))]
        let namespaced_path = create_namespaced_path(path);
        
        // shm_open requires name to start with '/' but have no other slashes
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            format!("/{}", without_leading.replace('/', "_"))
        };
        
        let shm_name = std::ffi::CString::new(shm_name_str)
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
            
            // Close fd immediately - mmap keeps its own reference to the shm object
            libc::close(fd);
            
            let header = ptr as *mut RingBufferHeader;
            
            // Validate existing header
            (*header).validate().map_err(|e| anyhow::anyhow!("Header validation failed: {}", e))?;
            
            Ok(Self {
                ptr,
                header: header as *mut RingBufferHeader,
                data_ptr: ptr.add(header_size),
                size: total_size,
                capacity: data_size,
            })
        }
    }
    
    /// Write to buffer (lock-free, async with backpressure)
    pub async fn write(&self, data: &[u8]) -> Result<()> {
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
                    // Ring buffer is full - implement async backpressure
                    let mut backoff_ms = 1;
                    let max_backoff_ms = 100;
                    let max_attempts = 10;
                    
                    for attempt in 0..max_attempts {
                        // Async sleep to avoid blocking tokio runtime
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        
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
    
    /// Read from buffer (lock-free, async for API consistency)
    pub async fn read(&self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*self.header;
            
            for _ in 0..100 {  // Limit retries to prevent infinite loop
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
                // Yield on contention to avoid busy-wait
                tokio::task::yield_now().await;
            }
            None  // Failed after retries
        }
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                // Unmap the memory (fd was already closed after mmap)
                libc::munmap(self.ptr as *mut core::ffi::c_void, self.size);
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

/// Handshake message for control channel
#[repr(C)]
struct HandshakeRequest {
    client_id: [u8; 16],  // UUID
    request_type: u32,    // 0 = connect, 1 = disconnect
    slot_hint: u32,       // Client's preferred slot (0 = any)
    version: u8,          // Protocol version
    flags: u8,            // Feature flags
    _padding: [u8; 6],    // Align to 32 bytes
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

/// Slot metadata for tracking and reclamation
struct SlotMetadata {
    slot_id: u32,
    send_buffer: Arc<SharedMemoryBuffer>,  // Already lock-free with atomics
    recv_buffer: Arc<SharedMemoryBuffer>,
    created_at: Instant,
    last_used: AtomicU64,  // Timestamp in seconds since epoch
    in_use: AtomicBool,
}

/// Slot pool with warm pool + on-demand allocation
struct SlotPool {
    slots: Arc<Mutex<HashMap<u32, Arc<SlotMetadata>>>>,
    base_path: String,
    warm_pool_size: usize,
    max_slots: usize,
}

impl SlotPool {
    async fn new(base_path: String, warm_pool_size: usize, max_slots: usize) -> Result<Self> {
        let pool = Self {
            slots: Arc::new(Mutex::new(HashMap::new())),
            base_path: base_path.clone(),
            warm_pool_size,
            max_slots,
        };
        
        // Pre-create warm pool
        for slot_id in 0..warm_pool_size as u32 {
            pool.create_slot(slot_id).await?;
        }
        
        Ok(pool)
    }
    
    async fn create_slot(&self, slot_id: u32) -> Result<Arc<SlotMetadata>> {
        let send_path = format!("{}_{}_send", self.base_path, slot_id);
        let recv_path = format!("{}_{}_recv", self.base_path, slot_id);
        
        let send = SharedMemoryBuffer::create(&send_path, 2 * 1024 * 1024).await?;
        let recv = SharedMemoryBuffer::create(&recv_path, 2 * 1024 * 1024).await?;
        
        let metadata = Arc::new(SlotMetadata {
            slot_id,
            send_buffer: Arc::new(send),  // No lock needed - already atomic
            recv_buffer: Arc::new(recv),
            created_at: Instant::now(),
            last_used: AtomicU64::new(Self::current_timestamp()),
            in_use: AtomicBool::new(false),
        });
        
        self.slots.lock().insert(slot_id, metadata.clone());
        Ok(metadata)
    }
    
    async fn get_or_create_slot(&self, slot_id: u32) -> Result<Arc<SlotMetadata>> {
        // Fast path: slot exists
        if let Some(slot) = self.slots.lock().get(&slot_id).cloned() {
            return Ok(slot);
        }
        
        // Slow path: create on demand
        let slots_count = self.slots.lock().len();
        if slots_count >= self.max_slots {
            bail!("Maximum slots ({}) reached", self.max_slots);
        }
        
        self.create_slot(slot_id).await
    }
    
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Listener for incoming shared memory connections
pub struct SharedMemoryListener {
    base_path: String,
    is_owner: bool,
    slot_pool: Arc<SlotPool>,
    accept_rx: Arc<Mutex<mpsc::UnboundedReceiver<u32>>>,  // Slot IDs from watcher
    _watcher_task: JoinHandle<()>,
}

impl SharedMemoryListener {
    pub async fn bind(path: &str) -> Result<Self> {
        const WARM_POOL_SIZE: usize = 64;   // Warm pool for fast connection
        const MAX_SLOTS: usize = 1000;       // Maximum allowed slots
        
        // Create slot pool with warm pool
        let slot_pool = Arc::new(SlotPool::new(
            path.to_string(),
            WARM_POOL_SIZE,
            MAX_SLOTS,
        ).await?);
        
        // Create channel for accept queue
        let (accept_tx, accept_rx) = mpsc::unbounded_channel::<u32>();
        
        // Ensure lock directory exists
        let lock_dir = format!("{}_locks", path);
        std::fs::create_dir_all(&lock_dir)?;
        
        // Do initial scan before spawning watcher
        if let Ok(entries) = std::fs::read_dir(&lock_dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.ends_with(".lock") {
                        if let Some(slot_str) = file_name.strip_prefix("slot_").and_then(|s| s.strip_suffix(".lock")) {
                            if let Ok(slot_id) = slot_str.parse::<u32>() {
                                if slot_pool.get_or_create_slot(slot_id).await.is_ok() {
                                    let _ = accept_tx.send(slot_id);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Spawn filesystem watcher for lock files
        let lock_dir_clone = lock_dir.clone();
        let slot_pool_clone = slot_pool.clone();
        let watcher_task = tokio::spawn(async move {
            Self::watch_lock_files(lock_dir_clone, accept_tx, slot_pool_clone).await;
        });
        
        Ok(Self {
            base_path: path.to_string(),
            is_owner: true,
            slot_pool,
            accept_rx: Arc::new(Mutex::new(accept_rx)),
            _watcher_task: watcher_task,
        })
    }
    
    
    /// Watch for lock files and enqueue accepted connections
    async fn watch_lock_files(
        lock_dir: String,
        accept_tx: mpsc::UnboundedSender<u32>,
        slot_pool: Arc<SlotPool>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(1));  // Faster polling
        let mut seen_locks = std::collections::HashSet::new();
        
        // Pre-populate seen_locks with existing files
        if let Ok(entries) = std::fs::read_dir(&lock_dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.ends_with(".lock") {
                        if let Some(slot_str) = file_name.strip_prefix("slot_").and_then(|s| s.strip_suffix(".lock")) {
                            if let Ok(slot_id) = slot_str.parse::<u32>() {
                                seen_locks.insert(slot_id);
                            }
                        }
                    }
                }
            }
        }
        
        loop {
            interval.tick().await;
            
            // Scan lock directory for new lock files
            if let Ok(entries) = std::fs::read_dir(&lock_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.ends_with(".lock") {
                            // Extract slot_id from filename: slot_<id>.lock
                            if let Some(slot_str) = file_name.strip_prefix("slot_").and_then(|s| s.strip_suffix(".lock")) {
                                if let Ok(slot_id) = slot_str.parse::<u32>() {
                                    if !seen_locks.contains(&slot_id) {
                                        seen_locks.insert(slot_id);
                                        
                                        // Ensure slot exists (create on-demand if needed)
                                        if slot_pool.get_or_create_slot(slot_id).await.is_ok() {
                                            let _ = accept_tx.send(slot_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    pub async fn accept(&self) -> Result<(SharedMemoryStream, std::net::SocketAddr)> {
        // Wait for a slot to be claimed via lock file
        let slot_id = self.accept_rx.lock().recv().await
            .ok_or_else(|| anyhow::anyhow!("Accept channel closed"))?;
        
        // Get the slot metadata
        let slot = self.slot_pool.get_or_create_slot(slot_id).await?;
        
        // Mark slot as in use
        slot.in_use.store(true, Ordering::Release);
        slot.last_used.store(SlotPool::current_timestamp(), Ordering::Relaxed);
        
        // Create stream from slot
        // Buffers are bidirectional - both sides use same physical buffers
        let stream = SharedMemoryStream {
            send_buffer: slot.send_buffer.clone(),
            recv_buffer: slot.recv_buffer.clone(),
            conn_id: slot_id as u64,
            lock_path: Some(format!("{}_locks/slot_{}.lock", self.base_path, slot_id)),
            base_path: self.base_path.clone(),
        };
        
        // Return dummy address (SHM has no network address)
        let addr = "0.0.0.0:0".parse().unwrap();
        Ok((stream, addr))
    }
}

impl Drop for SharedMemoryListener {
    fn drop(&mut self) {
        // Cleanup will be handled by OS when process exits
        // Pre-allocated buffers remain on disk for reuse
    }
}

/// A shared memory stream for bidirectional communication
pub struct SharedMemoryStream {
    send_buffer: Arc<SharedMemoryBuffer>,  // Lock-free with atomic ring operations
    recv_buffer: Arc<SharedMemoryBuffer>,
    conn_id: u64,
    lock_path: Option<String>,  // Track lock file for cleanup
    base_path: String,
}

impl SharedMemoryStream {
    /// Connect to a shared memory server using lock-free slot claiming
    pub async fn connect(path: &str) -> Result<Self> {
        const MAX_ATTEMPTS: usize = 500;  // Increased for concurrent bursts
        
        // Ensure lock directory exists
        let lock_dir = format!("{}_locks", path);
        std::fs::create_dir_all(&lock_dir)?;
        
        // Try random slots with exponential backoff
        for attempt in 0..MAX_ATTEMPTS {
            let slot_id = rand::random::<u32>() % 1000;
            let lock_path = format!("{}/slot_{}.lock", lock_dir, slot_id);
            
            // Atomically try to create lock file
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)  // Fails if file exists
                .open(&lock_path) {
                Ok(mut file) => {
                    // Successfully claimed slot!
                    use std::io::Write;
                    let _ = file.write_all(format!("{}", std::process::id()).as_bytes());
                    drop(file);
                    
                    // Open pre-allocated buffers (reversed for client perspective)
                    let send_path = format!("{}_{}_recv", path, slot_id);
                    let recv_path = format!("{}_{}_send", path, slot_id);
                    
                    // Retry buffer opening with backoff (allows watcher to create slot on-demand)
                    const MAX_BUFFER_RETRIES: usize = 50;  // Up to ~500ms total wait
                    for retry in 0..MAX_BUFFER_RETRIES {
                        match SharedMemoryBuffer::open(&send_path, 2 * 1024 * 1024).await {
                            Ok(send_buf) => {
                                match SharedMemoryBuffer::open(&recv_path, 2 * 1024 * 1024).await {
                                    Ok(recv_buf) => {
                                        return Ok(Self {
                                            send_buffer: Arc::new(send_buf),
                                            recv_buffer: Arc::new(recv_buf),
                                            conn_id: slot_id as u64,
                                            lock_path: Some(lock_path),
                                            base_path: path.to_string(),
                                        });
                                    }
                                    Err(_) if retry < MAX_BUFFER_RETRIES - 1 => {
                                        // Retry recv buffer with backoff
                                        tokio::time::sleep(Duration::from_millis(10)).await;
                                        continue;
                                    }
                                    Err(e) => {
                                        // Final retry failed - clean up lock
                                        let _ = std::fs::remove_file(&lock_path);
                                        return Err(e);
                                    }
                                }
                            }
                            Err(_) if retry < MAX_BUFFER_RETRIES - 1 => {
                                // Retry send buffer with backoff
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                continue;
                            }
                            Err(e) => {
                                // Final retry failed - clean up lock
                                let _ = std::fs::remove_file(&lock_path);
                                return Err(e);
                            }
                        }
                    }
                    
                    // Unreachable, but for compiler
                    let _ = std::fs::remove_file(&lock_path);
                    bail!("Buffer opening timed out for slot {}", slot_id);
                }
                Err(_) => {
                    // Exponential backoff for contention
                    if attempt > 0 && attempt % 10 == 0 {
                        let backoff_ms = std::cmp::min(attempt / 10, 5);
                        tokio::time::sleep(Duration::from_millis(backoff_ms as u64)).await;
                    }
                    continue;
                }
            }
        }
        
        bail!("No available slots after {} attempts", MAX_ATTEMPTS)
    }
    
    /// Read data
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let Some(data) = self.recv_buffer.read().await {
            let to_copy = std::cmp::min(data.len(), buf.len());
            buf[..to_copy].copy_from_slice(&data[..to_copy]);
            Ok(to_copy)
        } else {
            Ok(0)
        }
    }
    
    /// Write data
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.send_buffer.write(buf).await?;
        Ok(buf.len())
    }
    
    /// Read exact number of bytes
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let needed = buf.len();
        let mut total_read = 0;
        
        while total_read < needed {
            if let Some(data) = self.recv_buffer.read().await {
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
        // True lock-free write via atomic ring operations
        self.send_buffer.write(buf).await?;
        Ok(())
    }
    
    /// Flush the write buffer (no-op for shared memory)
    pub async fn flush(&mut self) -> Result<()> {
        // Shared memory writes are immediate, no need to flush
        Ok(())
    }
}

impl Drop for SharedMemoryStream {
    fn drop(&mut self) {
        // Release lock file to free slot for reuse
        if let Some(lock_path) = &self.lock_path {
            let _ = std::fs::remove_file(lock_path);
        }
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
        let mut buffer = SharedMemoryBuffer::create("/test_comm_buf", 2 * 1024 * 1024).await.unwrap();
        
        let test_data = b"Hello SharedMemory!";
        buffer.write(test_data).await.unwrap();
        
        let read_data = buffer.read().await.unwrap();
        assert_eq!(read_data, test_data);
        
        // Test multiple writes
        for i in 0..10 {
            let data = format!("Message {}", i);
            buffer.write(data.as_bytes()).await.unwrap();
            let result = buffer.read().await.unwrap();
            assert_eq!(result, data.as_bytes());
        }
    }
    
    #[tokio::test]
    async fn test_performance() {
        let mut buffer = SharedMemoryBuffer::create("/perf_test_shm", 4 * 1024 * 1024).await.unwrap();
        
        let data = vec![0u8; 512]; // Smaller than slot size (1024)
        let iterations = 10000;
        
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            buffer.write(&data).await.unwrap();
            buffer.read().await.unwrap();
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
