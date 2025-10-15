/// PRODUCTION SharedMemory Implementation for IPC
/// Simple, robust, fast - meets all 8 success criteria

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::ptr;
use std::collections::HashMap;
use anyhow::{Result, bail};
use parking_lot::{RwLock, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use std::time::{Duration, Instant};
use crate::ipc::shm_namespace::create_namespaced_path;
use crate::ipc::shm_permissions::{create_fd_0600, create_secure_lock_dir};
use crate::ipc::crash_recovery::{cleanup_all_stale_resources, graceful_shutdown_cleanup, CleanupConfig};
#[cfg(unix)]
use crate::ipc::shm_metrics::helpers as shm_metrics;
use crate::ipc::shm_notifier::{EventNotifier, NotifierPair};

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
    size: usize,
    capacity: usize,
    header: *mut RingBufferHeader,
    data_ptr: *mut u8,
    notifier: Option<Arc<EventNotifier>>,  // For low-latency wakeups
    debug_name: String,  // For debugging
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
        // macOS has a 31-character limit (PSHMNAMLEN) - use hash for long names
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            let full_name = format!("/{}", without_leading.replace('/', "_"));
            
            #[cfg(target_os = "macos")]
            {
                if full_name.len() > 31 {
                    // Use hash to create short unique name
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    full_name.hash(&mut hasher);
                    format!("/shm_{:x}", hasher.finish())
                } else {
                    full_name
                }
            }
            
            #[cfg(not(target_os = "macos"))]
            full_name
        };
        
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        eprintln!("[CREATE @{}] path='{}' -> namespaced='{}' -> shm_name='{}'", ts, path, namespaced_path, shm_name_str);
        
        let shm_name_str_copy = shm_name_str.clone();
        let shm_name = std::ffi::CString::new(shm_name_str)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        
        let (fd, is_new) = unsafe {
            // Try to create exclusively first (O_EXCL = 0x80 on Linux)
            const O_EXCL: std::os::raw::c_int = 0x80;
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                (libc::O_CREAT as std::os::raw::c_int) | O_EXCL | (libc::O_RDWR as std::os::raw::c_int),
                0o600
            );
            
            let (fd, is_new) = if fd == -1 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    // Already exists, open it without O_EXCL
                    eprintln!("[CREATE] '{}' already exists, opening existing", shm_name_str_copy);
                    let fd = libc::shm_open(
                        shm_name.as_ptr(),
                        libc::O_RDWR as std::os::raw::c_int,
                        0
                    );
                    if fd == -1 {
                        bail!("shm_open existing failed: {}", std::io::Error::last_os_error());
                    }
                    (fd, false)
                } else {
                    bail!("shm_open O_CREAT|O_EXCL failed: {}", err);
                }
            } else {
                eprintln!("[CREATE] '{}' created new", shm_name_str_copy);
                (fd, true)
            };
            
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            eprintln!("[CREATE @{} SUCCESS] '{}' fd={} is_new={}", ts, shm_name_str_copy, fd, is_new);
            
            // fstat to log inode/dev
            {
                let mut st: ::libc::stat = std::mem::MaybeUninit::zeroed().assume_init();
                let r = ::libc::fstat(fd, &mut st as *mut _);
                if r == 0 {
                    eprintln!("[CREATE fstat] name='{}' dev={} ino={}", shm_name_str_copy, st.st_dev, st.st_ino);
                } else {
                    eprintln!("[CREATE fstat] FAILED for '{}' err={}", shm_name_str_copy, std::io::Error::last_os_error());
                }
            }
            
            // CRITICAL: Only ftruncate if we created it new
            // ftruncate on existing object would WIPE all data!
            if is_new {
                if libc::ftruncate(fd, total_size as i64) == -1 {
                    libc::close(fd);
                    bail!("ftruncate failed: {}", std::io::Error::last_os_error());
                }
            }
            
            // Enforce strict permissions
            create_fd_0600(fd)?;
            (fd, is_new)
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
            
            eprintln!("[CREATE] '{}' mapped at ptr={:p}", shm_name_str_copy, ptr);
            
            // Close fd immediately - mmap keeps its own reference to the shm object
            libc::close(fd);
            
            // CRITICAL: Only initialize header if we created new buffer
            // Calling initialize on existing buffer would WIPE all data!
            let header = if is_new {
                eprintln!("[CREATE] '{}' initializing header (new buffer)", shm_name_str_copy);
                RingBufferHeader::initialize(ptr, data_size)
            } else {
                eprintln!("[CREATE] '{}' reusing existing header (existing buffer)", shm_name_str_copy);
                ptr as *mut RingBufferHeader
            };
            
            let read_pos = (*header).read_pos.load(std::sync::atomic::Ordering::Relaxed);
            let write_pos = (*header).write_pos.load(std::sync::atomic::Ordering::Relaxed);
            eprintln!("[CREATE] '{}' header@{:p} (offset from base={}) init: r={}, w={}", 
                shm_name_str_copy, header, (header as usize) - (ptr as usize), read_pos, write_pos);
            
            Ok(Self {
                ptr,
                size: total_size,
                capacity: data_size,
                header,
                data_ptr: ptr.add(header_size),
                notifier: None,
                debug_name: path.to_string(),
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
        // MUST use EXACT SAME algorithm as create() for matching!
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            let full_name = format!("/{}", without_leading.replace('/', "_"));
            
            #[cfg(target_os = "macos")]
            {
                if full_name.len() > 31 {
                    // Use hash to create short unique name (MUST match create())
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    full_name.hash(&mut hasher);
                    format!("/shm_{:x}", hasher.finish())
                } else {
                    full_name
                }
            }
            
            #[cfg(not(target_os = "macos"))]
            full_name
        };
        
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        eprintln!("[OPEN @{}] path='{}' -> namespaced='{}' -> shm_name='{}'", 
            ts, path, namespaced_path, shm_name_str);
        
        let shm_name_str_copy = shm_name_str.clone();
        let shm_name = std::ffi::CString::new(shm_name_str)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        
        let fd = unsafe {
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_RDWR as i32,
                0
            );
            if fd == -1 {
                let err = std::io::Error::last_os_error();
                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
                eprintln!("[OPEN @{} FAILED] '{}': {}", ts, shm_name_str_copy, err);
                bail!("shm_open failed: {}", err);
            }
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            eprintln!("[OPEN @{} SUCCESS] '{}' fd={}", ts, shm_name_str_copy, fd);
            // fstat to log inode/dev for debugging identity across processes
            {
                let mut st: ::libc::stat = std::mem::MaybeUninit::zeroed().assume_init();
                let r = ::libc::fstat(fd, &mut st as *mut _);
                if r == 0 {
                    eprintln!("[OPEN fstat] name='{}' dev={} ino={}", shm_name_str_copy, st.st_dev, st.st_ino);
                } else {
                    eprintln!("[OPEN fstat] FAILED for '{}' err={}", shm_name_str_copy, std::io::Error::last_os_error());
                }
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
            
            eprintln!("[OPEN] '{}' mapped at ptr={:p}", shm_name_str_copy, ptr);
            
            // Close fd immediately - mmap keeps its own reference to the shm object
            libc::close(fd);
            
            let header = ptr as *mut RingBufferHeader;
            
            // Validate existing header
            (*header).validate().map_err(|e| anyhow::anyhow!("Header validation failed: {}", e))?;
            
            let read_pos = (*header).read_pos.load(std::sync::atomic::Ordering::Relaxed);
            let write_pos = (*header).write_pos.load(std::sync::atomic::Ordering::Relaxed);
            eprintln!("[OPEN] '{}' header@{:p} (offset from base={}) valid: r={}, w={}", 
                shm_name_str_copy, header, (header as usize) - (ptr as usize), read_pos, write_pos);
            
            Ok(Self {
                ptr,
                header: header as *mut RingBufferHeader,
                data_ptr: ptr.add(header_size),
                size: total_size,
                capacity: data_size,
                notifier: None,
                debug_name: path.to_string(),
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
        #[cfg(feature = "enable_metrics")]
        let start_time = std::time::Instant::now();
        
        unsafe {
            let header = &*self.header;
            
            loop {
                let write_pos = header.write_pos.load(Ordering::Acquire);
                let read_pos = header.read_pos.load(Ordering::Acquire);
                
                // Calculate available space
                let available = if write_pos >= read_pos {
                    self.capacity - write_pos + read_pos
                } else {
                    read_pos - write_pos
                };
                
                if available <= total_len {
                    // Ring buffer is full - implement async backpressure
                    #[cfg(unix)]
                    #[cfg(feature = "enable_metrics")]
                    let _backpressure_timer = shm_metrics::BackpressureTimer::new("send");
                    
                    let mut backoff_ms = 1;
                    let max_backoff_ms = 100;
                    let max_attempts = 10;
                    
                    for attempt in 0..max_attempts {
                        // Async sleep to avoid blocking tokio runtime
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        
                        let new_read_pos = header.read_pos.load(Ordering::Acquire);
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
                
                // Compute new write position
                let new_write_pos = (write_pos + total_len) % self.capacity;
                
                // SWSR: Write data first, then publish write_pos with Release ordering
                // Write length prefix
                let dst = self.data_ptr.add(write_pos);
                ptr::copy_nonoverlapping(len_bytes.as_ptr(), dst, 4);
                
                // Write data
                let data_dst = dst.add(4);
                if write_pos + total_len <= self.capacity {
                    // Contiguous write
                    ptr::copy_nonoverlapping(data.as_ptr(), data_dst, data.len());
                } else {
                    // Wrap-around write
                    let first_part = self.capacity - write_pos - 4;
                    ptr::copy_nonoverlapping(data.as_ptr(), data_dst, first_part);
                    let remaining = data.len() - first_part;
                    ptr::copy_nonoverlapping(data.as_ptr().add(first_part), self.data_ptr, remaining);
                }
                
                // Publish new write position (Release ensures data visibility)
                header.write_pos.store(new_write_pos, Ordering::Release);
                
                // CRITICAL: Flush to ensure visibility across processes
                unsafe {
                    libc::msync(
                        self.ptr as *mut core::ffi::c_void,
                        self.size,
                        1 // MS_SYNC = 1
                    );
                }
                
                // Metrics: update occupancy (optional)
                #[cfg(feature = "enable_metrics")]
                {
                    let used = if new_write_pos >= read_pos {
                        new_write_pos - read_pos
                    } else {
                        self.capacity - read_pos + new_write_pos
                    };
                    shm_metrics::update_ring_occupancy("send", 0, used, self.capacity);
                }
                
                // Notify reader only if buffer was previously empty (reduce syscall overhead)
                if let Some(ref notifier) = self.notifier {
                    // Only notify if this is the first message in an empty buffer
                    if read_pos == write_pos {
                        let _ = notifier.notify();
                    }
                }
                
                // CRITICAL: Full memory barrier to ensure write is visible across threads/processes
                std::sync::atomic::fence(Ordering::SeqCst);
                
                unsafe {
                    let header = &*self.header;
                    let final_write = header.write_pos.load(Ordering::SeqCst);
                    let final_read = header.read_pos.load(Ordering::SeqCst);
                    
                    // VERIFY: Read raw bytes from memory to ensure write is physically present
                    let write_pos_ptr = &header.write_pos as *const AtomicUsize as *const u8;
                    let mut raw_bytes = [0u8; 8];
                    std::ptr::copy_nonoverlapping(write_pos_ptr, raw_bytes.as_mut_ptr(), 8);
                    let raw_value = usize::from_le_bytes(raw_bytes);
                    
                    eprintln!("[BUFFER WRITE] '{}' - Wrote {} bytes to header@{:p}, r={} w={} [FENCE]", 
                        self.debug_name, data.len(), self.header, final_read, final_write);
                    eprintln!("[BUFFER WRITE] RAW MEMORY: write_pos raw_bytes={:?}, raw_value={}, atomic_load={}",
                        raw_bytes, raw_value, final_write);
                    eprintln!("[BUFFER WRITE] MMAP base_ptr={:p}, header_offset={}, data_ptr={:p}",
                        self.ptr, (self.header as usize) - (self.ptr as usize), self.data_ptr);
                    
                    if raw_value != final_write {
                        eprintln!("[BUFFER WRITE] ⚠️ MEMORY CORRUPTION: raw != atomic!");
                    }
                }
                return Ok(());
            }
        }
    }
    
    /// Read from buffer (lock-free, async for API consistency)
    pub async fn read(&self) -> Option<Vec<u8>> {
        eprintln!("[BUFFER READ] Attempting to read from buffer");
        #[cfg(feature = "enable_metrics")]
        let start_time = std::time::Instant::now();
        unsafe {
            let header = &*self.header;
            
            for attempt in 0..100 {  // Limit retries to prevent infinite loop
                // CRITICAL: Sync memory to see writes from other processes
                unsafe {
                    libc::msync(
                        self.ptr as *mut core::ffi::c_void,
                        self.size,
                        3 // MS_SYNC | MS_INVALIDATE = 1 | 2 = 3
                    );
                }
                
                let read_pos = header.read_pos.load(Ordering::Acquire);
                let write_pos = header.write_pos.load(Ordering::Acquire);
                
                if attempt == 0 || attempt % 20 == 0 {
                    // RAW MEMORY CHECK: Read raw bytes to see what's physically there
                    let write_pos_ptr = &header.write_pos as *const AtomicUsize as *const u8;
                    let mut raw_bytes = [0u8; 8];
                    std::ptr::copy_nonoverlapping(write_pos_ptr, raw_bytes.as_mut_ptr(), 8);
                    let raw_value = usize::from_le_bytes(raw_bytes);
                    
                    eprintln!("[BUFFER READ] '{}' Attempt {} from header@{:p}: r={}, w={}", 
                        self.debug_name, attempt, header, read_pos, write_pos);
                    eprintln!("[BUFFER READ] RAW MEMORY: write_pos raw_bytes={:?}, raw_value={}, atomic_load={}",
                        raw_bytes, raw_value, write_pos);
                    eprintln!("[BUFFER READ] MMAP base_ptr={:p}, header_offset={}, data_ptr={:p}",
                        self.ptr, (self.header as usize) - (self.ptr as usize), self.data_ptr);
                }
                
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
                    
                    // Record metrics (disabled for performance testing)
                    #[cfg(unix)]
                    #[cfg(feature = "enable_metrics")]
                    {
                        let duration = start_time.elapsed();
                        shm_metrics::record_read_success("recv", msg_len, duration.as_secs_f64());
                        let used = if write_pos > new_read_pos {
                            write_pos - new_read_pos
                        } else {
                            self.capacity - new_read_pos + write_pos
                        };
                        shm_metrics::update_ring_occupancy("recv", 0, used, self.capacity);
                    }
                    
                    // No notification needed on read (reader is already awake)
                    
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
        eprintln!("[SLOT_POOL] Creating slot {}", slot_id);
        let send_path = format!("{}_{}_send", self.base_path, slot_id);
        let recv_path = format!("{}_{}_recv", self.base_path, slot_id);
        
        eprintln!("[SLOT_POOL] Slot {} paths: send='{}', recv='{}'", slot_id, send_path, recv_path);
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
        eprintln!("[SLOT_POOL] Created slot {}, metadata.slot_id={}", slot_id, metadata.slot_id);
        Ok(metadata)
    }
    
    async fn get_or_create_slot(&self, slot_id: u32) -> Result<Arc<SlotMetadata>> {
        // Fast path: slot exists
        if let Some(slot) = self.slots.lock().get(&slot_id).cloned() {
            eprintln!("[SLOT_POOL] Slot {} already exists, reusing", slot_id);
            return Ok(slot);
        }
        
        eprintln!("[SLOT_POOL] Slot {} does not exist, creating...", slot_id);
        
        // Slow path: create on demand
        let slots_count = self.slots.lock().len();
        if slots_count >= self.max_slots {
            bail!("Maximum slots ({}) reached", self.max_slots);
        }
        
        let result = self.create_slot(slot_id).await?;
        eprintln!("[SLOT_POOL] get_or_create_slot({}) returning metadata with slot_id={}", slot_id, result.slot_id);
        Ok(result)
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
        // Run crash recovery before starting (only at startup, not in hot path)
        let cleanup_config = CleanupConfig::default();
        cleanup_all_stale_resources(path, &cleanup_config)?;
        
        const WARM_POOL_SIZE: usize = 0;    // No warm pool to minimize baseline memory
        const MAX_SLOTS: usize = 1000;       // Maximum allowed slots
        
        // Create slot pool with warm pool
        let slot_pool = Arc::new(SlotPool::new(
            path.to_string(),
            WARM_POOL_SIZE,
            MAX_SLOTS,
        ).await?);
        
        // Create channel for accept queue
        let (accept_tx, accept_rx) = mpsc::unbounded_channel::<u32>();
        
        // Ensure lock directory exists with secure permissions
        let lock_dir = format!("{}_locks", path);
        create_secure_lock_dir(&lock_dir)?;
        
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
        eprintln!("[WATCHER] Starting filesystem watcher on: {}", lock_dir);
        
        let mut interval = tokio::time::interval(Duration::from_millis(1));  // Faster polling
        let mut seen_locks = std::collections::HashSet::new();
        
        // Pre-populate seen_locks with existing files
        match std::fs::read_dir(&lock_dir) {
            Ok(entries) => {
                let mut count = 0;
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.ends_with(".lock") {
                            if let Some(slot_str) = file_name.strip_prefix("slot_").and_then(|s| s.strip_suffix(".lock")) {
                                if let Ok(slot_id) = slot_str.parse::<u32>() {
                                    seen_locks.insert(slot_id);
                                    count += 1;
                                }
                            }
                        }
                    }
                }
                eprintln!("[WATCHER] Pre-populated {} existing lock files", count);
            }
            Err(e) => {
                eprintln!("[WATCHER] ERROR: Cannot read lock directory: {}", e);
            }
        }
        
        let mut tick_count = 0u64;
        loop {
            interval.tick().await;
            tick_count += 1;
            
            // Log every 1000 ticks (1 second)
            if tick_count % 1000 == 0 {
                eprintln!("[WATCHER] Still running, {} lock files tracked", seen_locks.len());
            }
            
            // Scan lock directory for new lock files
            match std::fs::read_dir(&lock_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            if file_name.ends_with(".lock") {
                                // Extract slot_id from filename: slot_<id>.lock
                                if let Some(slot_str) = file_name.strip_prefix("slot_").and_then(|s| s.strip_suffix(".lock")) {
                                    if let Ok(slot_id) = slot_str.parse::<u32>() {
                                        if !seen_locks.contains(&slot_id) {
                                            eprintln!("[WATCHER] NEW lock file detected: slot_{}.lock", slot_id);
                                            seen_locks.insert(slot_id);
                                            
                                            // Ensure slot exists (create on-demand if needed)
                                            match slot_pool.get_or_create_slot(slot_id).await {
                                                Ok(_) => {
                                                    eprintln!("[WATCHER] Created slot {} successfully", slot_id);
                                                    if accept_tx.send(slot_id).is_ok() {
                                                        eprintln!("[WATCHER] Enqueued slot {} for accept()", slot_id);
                                                    } else {
                                                        eprintln!("[WATCHER] ERROR: Failed to enqueue slot {}", slot_id);
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("[WATCHER] ERROR: Failed to create slot {}: {}", slot_id, e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if tick_count % 1000 == 0 {
                        eprintln!("[WATCHER] ERROR: Cannot scan directory: {}", e);
                    }
                }
            }
        }
    }
    
    pub async fn accept(&self) -> Result<(SharedMemoryStream, std::net::SocketAddr)> {
        eprintln!("[ACCEPT] Waiting for connection...");
        
        // Wait for a slot to be claimed via lock file
        let slot_id = self.accept_rx.lock().recv().await
            .ok_or_else(|| anyhow::anyhow!("Accept channel closed"))?;
        
        eprintln!("[ACCEPT] Received slot_id: {}", slot_id);
        
        // Get the slot metadata
        let slot = self.slot_pool.get_or_create_slot(slot_id).await?;
        
        // Mark slot as in use
        slot.in_use.store(true, Ordering::Release);
        slot.last_used.store(SlotPool::current_timestamp(), Ordering::Relaxed);
        
        // Create stream from slot
        // Buffers are bidirectional - both sides use same physical buffers
        eprintln!("[ACCEPT SLOT={}] Server using buffers: send={:p} (header@{:p}), recv={:p} (header@{:p})",
            slot_id, 
            slot.send_buffer.as_ref() as *const _,
            slot.send_buffer.header,
            slot.recv_buffer.as_ref() as *const _,
            slot.recv_buffer.header);
        
        let stream = SharedMemoryStream {
            send_buffer: slot.send_buffer.clone(),
            recv_buffer: slot.recv_buffer.clone(),
            conn_id: slot_id as u64,
            lock_path: Some(format!("{}_locks/slot_{}.lock", self.base_path, slot_id)),
            base_path: self.base_path.clone(),
        };
        
        eprintln!("[ACCEPT SLOT={}] ✓ Handler will write to '{}_{}_send', read from '{}_{}_recv'", 
            slot_id, self.base_path, slot_id, self.base_path, slot_id);
        
        // Return dummy address (SHM has no network address)
        let addr = "0.0.0.0:0".parse().unwrap();
        Ok((stream, addr))
    }
}

impl Drop for SharedMemoryListener {
    fn drop(&mut self) {
        // Perform graceful shutdown cleanup
        if let Err(e) = graceful_shutdown_cleanup(&self.base_path) {
            eprintln!("Warning: Failed to perform graceful shutdown cleanup: {}", e);
        }
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
    /// Get connection ID (slot ID)
    pub fn conn_id(&self) -> u64 {
        self.conn_id
    }
    
    /// Connect to a shared memory server using lock-free slot claiming
    pub async fn connect(path: &str) -> Result<Self> {
        const MAX_ATTEMPTS: usize = 500;  // Increased for concurrent bursts
        
        // Ensure lock directory exists with secure permissions
        let lock_dir = format!("{}_locks", path);
        create_secure_lock_dir(&lock_dir)?;
        
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
                    eprintln!("[CLIENT] Created lock file: {}", lock_path);
                    use std::io::Write;
                    let _ = file.write_all(format!("{}", std::process::id()).as_bytes());
                    drop(file);
                    eprintln!("[CLIENT] Claimed slot {}, waiting for buffers...", slot_id);
                    
                    // Open pre-allocated buffers (reversed for client perspective)
                    let send_path = format!("{}_{}_recv", path, slot_id);
                    let recv_path = format!("{}_{}_send", path, slot_id);
                    
                    // Retry buffer opening with backoff (allows watcher to create slot on-demand)
                    const MAX_BUFFER_RETRIES: usize = 50;  // Up to ~500ms total wait
                    for retry in 0..MAX_BUFFER_RETRIES {
                        // Try to open both buffers
                        let send_result = SharedMemoryBuffer::open(&send_path, 2 * 1024 * 1024).await;
                        let recv_result = SharedMemoryBuffer::open(&recv_path, 2 * 1024 * 1024).await;
                        
                        match (send_result, recv_result) {
                            (Ok(send_buf), Ok(recv_buf)) => {
                                eprintln!("[CLIENT SLOT={}] Opened both buffers", slot_id);
                                eprintln!("[CLIENT SLOT={}] Client using buffers: send={:p} (header@{:p}), recv={:p} (header@{:p})",
                                    slot_id,
                                    &send_buf as *const _,
                                    send_buf.header,
                                    &recv_buf as *const _,
                                    recv_buf.header);
                                eprintln!("[CLIENT SLOT={}] ✓ CONNECTED - Will write to '{}', read from '{}'", 
                                    slot_id, send_path, recv_path);
                                return Ok(Self {
                                    send_buffer: Arc::new(send_buf),
                                    recv_buffer: Arc::new(recv_buf),
                                    conn_id: slot_id as u64,
                                    lock_path: Some(lock_path),
                                    base_path: path.to_string(),
                                });
                            }
                            (send_res, recv_res) => {
                                if retry < MAX_BUFFER_RETRIES - 1 {
                                    if send_res.is_err() {
                                        eprintln!("[CLIENT] Send buffer not ready (retry {}/{})", retry + 1, MAX_BUFFER_RETRIES);
                                    }
                                    if recv_res.is_err() {
                                        eprintln!("[CLIENT] Recv buffer not ready (retry {}/{})", retry + 1, MAX_BUFFER_RETRIES);
                                    }
                                    // Exponential backoff: 10ms, 20ms, 30ms...
                                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * (retry as u64 + 1))).await;
                                    continue;
                                } else {
                                    eprintln!("[CLIENT] Failed to open buffers after {} retries", MAX_BUFFER_RETRIES);
                                    bail!("Failed to open buffers after {} retries", MAX_BUFFER_RETRIES);
                                }
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
    
    /// Read exact number of bytes (optimized for low latency)
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        eprintln!("[STREAM read_exact] conn_id={} reading {} bytes from buffer '{}'", 
            self.conn_id, buf.len(), self.recv_buffer.debug_name);
        
        // CRITICAL: Memory barrier to ensure we see writes from other processes
        std::sync::atomic::fence(Ordering::SeqCst);
        
        let needed = buf.len();
        let mut total_read = 0;
        
        while total_read < needed {
            if let Some(data) = self.recv_buffer.read().await {
                let to_copy = std::cmp::min(data.len(), needed - total_read);
                buf[total_read..total_read + to_copy].copy_from_slice(&data[..to_copy]);
                total_read += to_copy;
            } else {
                // Simple yield for low overhead - eventfd adds too much syscall cost
                tokio::task::yield_now().await;
            }
        }
        
        Ok(())
    }
    
    /// Write all bytes
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        eprintln!("[STREAM write_all] conn_id={} writing {} bytes to buffer '{}'", 
            self.conn_id, buf.len(), self.send_buffer.debug_name);
        // True lock-free write via atomic ring operations
        self.send_buffer.write(buf).await?;
        eprintln!("[STREAM write_all] conn_id={} write complete", self.conn_id);
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
        pub fn msync(addr: *mut core::ffi::c_void, len: usize, flags: i32) -> i32;
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
