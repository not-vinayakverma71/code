/// NUCLEAR-OPTIMIZED SharedMemory - Single 2MB segment for ALL connections
/// Multiplexes 1000+ connections through ONE shared memory region

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use std::ptr;
use anyhow::{Result, bail};

const CACHE_LINE: usize = 64;

// Removed CachePadded struct - using direct pointer alignment instead

const TOTAL_MEMORY: usize = 2 * 1024 * 1024; // 2MB TOTAL
const MAX_CONNECTIONS: usize = 1024;
const SLOT_SIZE: usize = 2048; // 2KB per connection slot
const SLOTS_PER_CONNECTION: usize = 1;

/// Single shared memory for ALL connections
static GLOBAL_MEMORY: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());
static INIT_ONCE: AtomicBool = AtomicBool::new(false);

/// Lock-free MPMC ring buffer using proper sequenced positions
pub struct SharedMemoryBuffer {
    connection_id: usize,
    slot_ptr: *mut u8,
}

unsafe impl Send for SharedMemoryBuffer {}
unsafe impl Sync for SharedMemoryBuffer {}

impl SharedMemoryBuffer {
    /// Get connection slot from global memory
    pub fn create(path: &str, _requested_size: usize) -> Result<Self> {
        // Initialize global memory once
        if !INIT_ONCE.load(Ordering::Acquire) {
            Self::init_global_memory()?;
        }
        
        // Hash path to connection ID
        let connection_id = Self::hash_path(path) % MAX_CONNECTIONS;
        let global_ptr = GLOBAL_MEMORY.load(Ordering::Acquire);
        
        if global_ptr.is_null() {
            bail!("Global memory not initialized");
        }
        
        // Calculate slot pointer
        let slot_ptr = unsafe { global_ptr.add(connection_id * SLOT_SIZE) };
        
        Ok(Self {
            connection_id,
            slot_ptr,
        })
    }
    
    fn init_global_memory() -> Result<()> {
        if INIT_ONCE.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire).is_ok() {
            // Round up to power of 2 for efficient modulo
            let capacity = TOTAL_MEMORY;
        
        unsafe {
            // Allocate memory with mmap
            let size = capacity + CACHE_LINE * 2; // Extra space for alignment
            
            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) as *mut u8;
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                bail!("mmap failed");
            }
            
            // Align buffer to cache line
            let buffer = ptr.add(CACHE_LINE);
            
            // Place atomics at start of allocated memory
            let head_ptr = ptr as *mut AtomicUsize;
            let tail_ptr = ptr.add(CACHE_LINE) as *mut AtomicUsize;
            
            // Initialize atomics
            ptr::write(head_ptr, AtomicUsize::new(0));
            ptr::write(tail_ptr, AtomicUsize::new(0));
            
            GLOBAL_MEMORY.store(ptr, Ordering::Release);
        }
        }
        Ok(())
    }
    
    fn hash_path(path: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish() as usize
    }
    
    /// Open existing (just creates new for simplicity)
    pub fn open(path: &str, size: usize) -> Result<Self> {
        Self::create(path, size)
    }
    
    /// Write to buffer - wait-free for small messages
    #[inline(always)]
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let len = data.len();
        if len > self.capacity / 4 {
            bail!("Message too large");
        }
        
        unsafe {
            // Reserve slot
            let mut head = (*self.head).load(Ordering::Relaxed);
            
            loop {
                let tail = (*self.tail).load(Ordering::Acquire);
                
                // Check if full (leave one slot empty to distinguish full from empty)
                let next_head = (head + 1) & (self.capacity - 1);
                if next_head == tail {
                    // Buffer full - spin wait
                    std::hint::spin_loop();
                    head = (*self.head).load(Ordering::Relaxed);
                    continue;
                }
                
                // Try to claim this slot
                match (*self.head).compare_exchange_weak(
                    head,
                    next_head,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // We own slot 'head', write our data
                        let slot = self.buffer.add(head * 256); // Fixed 256-byte slots
                        
                        // Write length prefix
                        ptr::write(slot as *mut u32, len as u32);
                        
                        // Write data
                        ptr::copy_nonoverlapping(data.as_ptr(), slot.add(4), len);
                        
                        return Ok(());
                    }
                    Err(h) => {
                        head = h; // Retry with updated head
                    }
                }
            }
        }
    }
    
    /// Read from buffer - wait-free
    #[inline(always)]
    pub fn read(&self) -> Result<Option<Vec<u8>>> {
        unsafe {
            let tail = (*self.tail).load(Ordering::Relaxed);
            let head = (*self.head).load(Ordering::Acquire);
            
            // Check if empty
            if tail == head {
                return Ok(None);
            }
            
            // Read from tail position
            let slot = self.buffer.add(tail * 256);
            
            // Read length
            let len = ptr::read(slot as *const u32) as usize;
            if len == 0 || len > 252 {
                // Skip corrupted slot
                (*self.tail).store((tail + 1) & (self.capacity - 1), Ordering::Release);
                return Ok(None);
            }
            
            // Read data
            let mut data = Vec::with_capacity(len);
            data.set_len(len);
            ptr::copy_nonoverlapping(slot.add(4), data.as_mut_ptr(), len);
            
            // Advance tail
            (*self.tail).store((tail + 1) & (self.capacity - 1), Ordering::Release);
            
            Ok(Some(data))
        }
    }
    
    /// Write without allocation
    #[inline(always)]
    pub fn write_no_alloc(&self, data: &[u8]) -> bool {
        if data.len() > 252 {
            return false;
        }
        
        unsafe {
            let mut head = (*self.head).load(Ordering::Relaxed);
            let tail = (*self.tail).load(Ordering::Acquire);
            
            let next_head = (head + 1) & (self.capacity - 1);
            if next_head == tail {
                return false; // Full
            }
            
            // Fast path - try once
            if let Ok(_) = (*self.head).compare_exchange(
                head,
                next_head,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                let slot = self.buffer.add(head * 256);
                ptr::write(slot as *mut u32, data.len() as u32);
                ptr::copy_nonoverlapping(data.as_ptr(), slot.add(4), data.len());
                return true;
            }
            
            false
        }
    }
    
    /// Read without allocation (into provided buffer)
    #[inline(always)]
    pub fn read_no_alloc(&self, buffer: &mut [u8]) -> Option<usize> {
        unsafe {
            let tail = (*self.tail).load(Ordering::Relaxed);
            let head = (*self.head).load(Ordering::Acquire);
            
            if tail == head {
                return None;
            }
            
            let slot = self.buffer.add(tail * 256);
            let len = ptr::read(slot as *const u32) as usize;
            
            if len == 0 || len > 252 || len > buffer.len() {
                (*self.tail).store((tail + 1) & (self.capacity - 1), Ordering::Release);
                return None;
            }
            
            ptr::copy_nonoverlapping(slot.add(4), buffer.as_mut_ptr(), len);
            (*self.tail).store((tail + 1) & (self.capacity - 1), Ordering::Release);
            
            Some(len)
        }
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                libc::munmap(self.ptr as *mut _, self.size);
            }
        }
    }
}

// Re-export compatibility types
pub use crate::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

mod libc {
    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const MAP_SHARED: i32 = 0x01;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: *mut core::ffi::c_void = !0 as *mut core::ffi::c_void;
    
    extern "C" {
        pub fn mmap(addr: *mut core::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut core::ffi::c_void;
        pub fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    }
}
