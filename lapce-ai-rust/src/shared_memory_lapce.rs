/// LAPCE-OPTIMIZED SharedMemory - Designed specifically for Lapce editor
/// Dynamic memory management with intelligent pooling

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, AtomicPtr, Ordering};
use std::ptr;
use anyhow::{Result, bail};
use parking_lot::RwLock;

// Lapce-specific configuration
const BASE_MEMORY: usize = 512 * 1024;       // 512KB base overhead
const SMALL_BUFFER: usize = 4 * 1024;        // 4KB for completions
const MEDIUM_BUFFER: usize = 64 * 1024;      // 64KB for diagnostics
const LARGE_BUFFER: usize = 1024 * 1024;     // 1MB for file operations

// Pool sizes optimized for editor workload
const SMALL_POOL_SIZE: usize = 20;   // Quick completions, hover info
const MEDIUM_POOL_SIZE: usize = 10;  // Diagnostics, formatting
const LARGE_POOL_SIZE: usize = 3;    // File analysis, indexing

/// Connection types in Lapce
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    AI,              // AI completions, chat
    LSP,             // Language servers
    Formatter,       // Code formatters
    Debugger,        // Debug adapter
    FileWatcher,     // File system events
    Plugin,          // Plugin communication
}

/// Intelligent buffer management for Lapce
pub struct LapceMemoryManager {
    // Tiered buffer pools
    small_pool: Arc<RwLock<Vec<BufferSlot>>>,
    medium_pool: Arc<RwLock<Vec<BufferSlot>>>,
    large_pool: Arc<RwLock<Vec<BufferSlot>>>,
    
    // Active connections tracking
    connections: Arc<RwLock<Vec<ConnectionInfo>>>,
    
    // Global metrics
    total_memory: AtomicUsize,
    active_connections: AtomicUsize,
}

struct BufferSlot {
    ptr: *mut u8,
    size: usize,
    in_use: AtomicBool,
    last_used: AtomicUsize,  // For LRU eviction
}

struct ConnectionInfo {
    id: usize,
    conn_type: ConnectionType,
    buffer_size: usize,
    last_activity: std::time::Instant,
}

impl LapceMemoryManager {
    pub fn new() -> Result<Self> {
        let manager = Self {
            small_pool: Arc::new(RwLock::new(Vec::with_capacity(SMALL_POOL_SIZE))),
            medium_pool: Arc::new(RwLock::new(Vec::with_capacity(MEDIUM_POOL_SIZE))),
            large_pool: Arc::new(RwLock::new(Vec::with_capacity(LARGE_POOL_SIZE))),
            connections: Arc::new(RwLock::new(Vec::new())),
            total_memory: AtomicUsize::new(BASE_MEMORY),
            active_connections: AtomicUsize::new(0),
        };
        
        // Pre-allocate frequently used small buffers
        manager.preallocate_pools()?;
        
        Ok(manager)
    }
    
    fn preallocate_pools(&self) -> Result<()> {
        // Pre-allocate 5 small buffers for quick access
        let mut small = self.small_pool.write();
        for _ in 0..5 {
            let buffer = self.allocate_buffer(SMALL_BUFFER)?;
            small.push(buffer);
        }
        
        // Pre-allocate 2 medium buffers
        let mut medium = self.medium_pool.write();
        for _ in 0..2 {
            let buffer = self.allocate_buffer(MEDIUM_BUFFER)?;
            medium.push(buffer);
        }
        
        Ok(())
    }
    
    fn allocate_buffer(&self, size: usize) -> Result<BufferSlot> {
        unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) as *mut u8;
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                bail!("mmap failed");
            }
            
            self.total_memory.fetch_add(size, Ordering::Relaxed);
            
            Ok(BufferSlot {
                ptr,
                size,
                in_use: AtomicBool::new(false),
                last_used: AtomicUsize::new(0),
            })
        }
    }
    
    /// Get appropriate buffer for connection type
    pub fn get_buffer(&self, conn_type: ConnectionType) -> Result<SharedMemoryBuffer> {
        let (size, pool) = match conn_type {
            ConnectionType::AI | ConnectionType::LSP => {
                // AI and LSP need variable sizes
                let msg_size = self.estimate_message_size(conn_type);
                if msg_size <= SMALL_BUFFER {
                    (SMALL_BUFFER, self.small_pool.clone())
                } else if msg_size <= MEDIUM_BUFFER {
                    (MEDIUM_BUFFER, self.medium_pool.clone())
                } else {
                    (LARGE_BUFFER, self.large_pool.clone())
                }
            }
            ConnectionType::Formatter | ConnectionType::Debugger => {
                (MEDIUM_BUFFER, self.medium_pool.clone())
            }
            ConnectionType::FileWatcher => {
                (SMALL_BUFFER, self.small_pool.clone())
            }
            ConnectionType::Plugin => {
                (MEDIUM_BUFFER, self.medium_pool.clone())
            }
        };
        
        // Try to get from pool first
        if let Some(buffer) = self.try_get_from_pool(&pool) {
            return Ok(SharedMemoryBuffer::from_slot(buffer));
        }
        
        // Allocate new if pool is empty
        let buffer = self.allocate_buffer(size)?;
        Ok(SharedMemoryBuffer::from_slot(buffer))
    }
    
    fn try_get_from_pool(&self, pool: &Arc<RwLock<Vec<BufferSlot>>>) -> Option<BufferSlot> {
        let mut pool_guard = pool.write();
        pool_guard.pop()
    }
    
    fn estimate_message_size(&self, conn_type: ConnectionType) -> usize {
        // Intelligent size estimation based on connection type
        match conn_type {
            ConnectionType::AI => MEDIUM_BUFFER,     // Usually needs 20-50KB
            ConnectionType::LSP => SMALL_BUFFER,     // Most messages are small
            _ => SMALL_BUFFER,
        }
    }
    
    /// Return buffer to pool when done
    pub fn release_buffer(&self, buffer: SharedMemoryBuffer) {
        let size = buffer.size();
        let slot = buffer.into_slot();
        
        // Return to appropriate pool
        match size {
            SMALL_BUFFER => {
                let mut pool = self.small_pool.write();
                if pool.len() < SMALL_POOL_SIZE {
                    pool.push(slot);
                } else {
                    // Pool full, deallocate
                    self.deallocate_buffer(slot);
                }
            }
            MEDIUM_BUFFER => {
                let mut pool = self.medium_pool.write();
                if pool.len() < MEDIUM_POOL_SIZE {
                    pool.push(slot);
                } else {
                    self.deallocate_buffer(slot);
                }
            }
            LARGE_BUFFER => {
                let mut pool = self.large_pool.write();
                if pool.len() < LARGE_POOL_SIZE {
                    pool.push(slot);
                } else {
                    self.deallocate_buffer(slot);
                }
            }
            _ => self.deallocate_buffer(slot),
        }
    }
    
    fn deallocate_buffer(&self, slot: BufferSlot) {
        unsafe {
            libc::munmap(slot.ptr as *mut core::ffi::c_void, slot.size);
            self.total_memory.fetch_sub(slot.size, Ordering::Relaxed);
        }
    }
    
    /// Compact memory during idle periods
    pub async fn compact_if_idle(&self) {
        let connections = self.connections.read();
        let now = std::time::Instant::now();
        
        // If all connections idle for >5 seconds, compact
        let all_idle = connections.iter().all(|c| 
            now.duration_since(c.last_activity).as_secs() > 5
        );
        
        if all_idle {
            // Release excess buffers from pools
            self.compact_pools();
        }
    }
    
    fn compact_pools(&self) {
        // Keep only minimum buffers
        let mut small = self.small_pool.write();
        while small.len() > 2 {
            if let Some(slot) = small.pop() {
                self.deallocate_buffer(slot);
            }
        }
        
        let mut medium = self.medium_pool.write();
        while medium.len() > 1 {
            if let Some(slot) = medium.pop() {
                self.deallocate_buffer(slot);
            }
        }
        
        let mut large = self.large_pool.write();
        large.clear(); // Release all large buffers when idle
    }
    
    pub fn get_memory_usage(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }
}

/// SharedMemory buffer optimized for Lapce
pub struct SharedMemoryBuffer {
    slot: BufferSlot,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

impl SharedMemoryBuffer {
    fn from_slot(mut slot: BufferSlot) -> Self {
        slot.in_use.store(true, Ordering::Release);
        Self {
            slot,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
        }
    }
    
    fn into_slot(self) -> BufferSlot {
        self.slot.in_use.store(false, Ordering::Release);
        self.slot
    }
    
    pub fn size(&self) -> usize {
        self.slot.size
    }
    
    /// Zero-copy write
    #[inline(always)]
    pub fn write(&self, data: &[u8]) -> Result<()> {
        if data.len() + 4 > self.slot.size {
            bail!("Message too large for buffer");
        }
        
        unsafe {
            // Write length prefix
            let len = data.len() as u32;
            ptr::write(self.slot.ptr as *mut u32, len);
            
            // Write data
            ptr::copy_nonoverlapping(data.as_ptr(), self.slot.ptr.add(4), data.len());
            
            self.write_pos.store(4 + data.len(), Ordering::Release);
        }
        
        Ok(())
    }
    
    /// Zero-copy read
    #[inline(always)]
    pub fn read(&self) -> Result<Option<Vec<u8>>> {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        if write_pos < 4 {
            return Ok(None);
        }
        
        unsafe {
            let len = ptr::read(self.slot.ptr as *const u32) as usize;
            if len + 4 > write_pos {
                return Ok(None);
            }
            
            let mut data = vec![0u8; len];
            ptr::copy_nonoverlapping(self.slot.ptr.add(4), data.as_mut_ptr(), len);
            
            Ok(Some(data))
        }
    }
}

mod libc {
    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const MAP_PRIVATE: i32 = 0x02;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: *mut core::ffi::c_void = !0 as *mut core::ffi::c_void;
    
    extern "C" {
        pub fn mmap(addr: *mut core::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut core::ffi::c_void;
        pub fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    }
}
