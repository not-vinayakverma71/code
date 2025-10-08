/// Shared Memory Ring Buffer Header Layout Documentation
/// 
/// This module defines the exact memory layout for the shared memory ring buffer
/// used in high-performance IPC communication.
/// 
/// ## Memory Layout
/// ```
/// ┌───────────────────────────────────────────────────────────────┐
/// │                     Shared Memory Layout                      │
/// ├───────────────────────────────────────────────────────────────┤
/// │ Offset │ Size  │ Field        │ Type         │ Description   │
/// ├────────┼───────┼──────────────┼──────────────┼───────────────┤
/// │ 0x0000 │ 8     │ magic        │ u64          │ Magic number  │
/// │ 0x0008 │ 8     │ write_pos    │ AtomicUsize  │ Write cursor  │
/// │ 0x0010 │ 8     │ read_pos     │ AtomicUsize  │ Read cursor   │
/// │ 0x0018 │ 8     │ capacity     │ usize        │ Buffer size   │
/// │ 0x0020 │ 4     │ version      │ u32          │ Protocol ver  │
/// │ 0x0024 │ 4     │ flags        │ AtomicU32    │ Status flags  │
/// │ 0x0028 │ 8     │ sequence     │ AtomicU64    │ Message seq   │
/// │ 0x0030 │ 8     │ last_error   │ AtomicU64    │ Error code    │
/// │ 0x0038 │ 8     │ _reserved    │ [u8; 8]      │ Future use    │
/// │ 0x0040 │ ...   │ data[]       │ [u8; N]      │ Ring buffer   │
/// └────────┴───────┴──────────────┴──────────────┴───────────────┘
/// Total header size: 64 bytes (cache line aligned)
/// ```
/// 
/// ## Memory Ordering Rules
/// 
/// ### Write Operations
/// - write_pos: Acquire for load, Release for store
/// - Updates must be visible before data writes complete
/// 
/// ### Read Operations  
/// - read_pos: Acquire for load, Release for store
/// - Data reads must complete before updating position
/// 
/// ### Synchronization
/// - Full memory fence between header and data access
/// - Cache line alignment prevents false sharing

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Magic number for validation: "LAPCIPC\0" in ASCII
pub const MAGIC_NUMBER: u64 = 0x004350494350414C; 

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Header flags
pub mod flags {
    /// Buffer is ready for use
    pub const INITIALIZED: u32 = 0x0001;
    /// Writer has overrun (data loss)
    pub const OVERRUN: u32 = 0x0002;
    /// Reader is lagging
    pub const READER_LAGGING: u32 = 0x0004;
    /// Shutdown requested
    pub const SHUTDOWN: u32 = 0x0008;
}

/// Shared memory ring buffer header
/// 
/// CRITICAL: This struct MUST be #[repr(C)] for cross-process compatibility
/// Size: Exactly 64 bytes for cache line alignment
#[repr(C, align(64))]
pub struct RingBufferHeader {
    /// Magic number for validation (0x004350494350414C = "LAPCIPC\0")
    pub magic: u64,
    
    /// Write position (bytes offset from data start)
    /// Memory ordering: Acquire for load, Release for store
    pub write_pos: AtomicUsize,
    
    /// Read position (bytes offset from data start)  
    /// Memory ordering: Acquire for load, Release for store
    pub read_pos: AtomicUsize,
    
    /// Total capacity of the ring buffer in bytes
    pub capacity: usize,
    
    /// Protocol version for compatibility checking
    pub version: u32,
    
    /// Status flags (see flags module)
    pub flags: AtomicU32,
    
    /// Monotonic message sequence number
    pub sequence: AtomicU64,
    
    /// Last error code (0 = no error)
    pub last_error: AtomicU64,
    
    /// Reserved for future use (maintains 64-byte alignment)
    pub _reserved: [u8; 8],
}

impl RingBufferHeader {
    /// Initialize a new header at the given memory location
    /// 
    /// # Safety
    /// The provided pointer must point to at least 64 bytes of valid memory
    pub unsafe fn initialize(ptr: *mut u8, capacity: usize) -> *mut Self {
        let header = ptr as *mut Self;
        
        // Zero the entire header first
        std::ptr::write_bytes(ptr, 0, std::mem::size_of::<Self>());
        
        // Initialize fields
        (*header).magic = MAGIC_NUMBER;
        (*header).write_pos = AtomicUsize::new(0);
        (*header).read_pos = AtomicUsize::new(0);
        (*header).capacity = capacity;
        (*header).version = PROTOCOL_VERSION;
        (*header).flags = AtomicU32::new(flags::INITIALIZED);
        (*header).sequence = AtomicU64::new(0);
        (*header).last_error = AtomicU64::new(0);
        
        // Full memory fence to ensure all writes are visible
        std::sync::atomic::fence(Ordering::SeqCst);
        
        header
    }
    
    /// Validate header magic and version
    #[inline]
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic != MAGIC_NUMBER {
            return Err("Invalid magic number");
        }
        if self.version != PROTOCOL_VERSION {
            return Err("Incompatible protocol version");
        }
        if self.flags.load(Ordering::Relaxed) & flags::INITIALIZED == 0 {
            return Err("Buffer not initialized");
        }
        Ok(())
    }
    
    /// Calculate available space for writing
    #[inline]
    pub fn available_write_space(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        
        if write >= read {
            // Normal case: write ahead of read
            self.capacity - write + read - 1
        } else {
            // Wrapped case: read ahead of write
            read - write - 1
        }
    }
    
    /// Calculate available data for reading
    #[inline]
    pub fn available_read_data(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }
    
    /// Advance write position with memory ordering
    #[inline]
    pub fn advance_write_pos(&self, bytes: usize) -> usize {
        let old_pos = self.write_pos.load(Ordering::Acquire);
        let new_pos = (old_pos + bytes) % self.capacity;
        
        // Increment sequence number
        self.sequence.fetch_add(1, Ordering::AcqRel);
        
        // Update position with Release ordering
        self.write_pos.store(new_pos, Ordering::Release);
        
        old_pos
    }
    
    /// Advance read position with memory ordering
    #[inline]
    pub fn advance_read_pos(&self, bytes: usize) -> usize {
        let old_pos = self.read_pos.load(Ordering::Acquire);
        let new_pos = (old_pos + bytes) % self.capacity;
        
        // Update position with Release ordering
        self.read_pos.store(new_pos, Ordering::Release);
        
        old_pos
    }
    
    /// Compare and swap write position (for lock-free operations)
    #[inline]
    pub fn cas_write_pos(&self, expected: usize, new: usize) -> Result<usize, usize> {
        self.write_pos.compare_exchange(
            expected,
            new,
            Ordering::AcqRel,  // Success: Acquire-Release
            Ordering::Acquire,  // Failure: Acquire
        )
    }
    
    /// Compare and swap read position (for lock-free operations)
    #[inline]
    pub fn cas_read_pos(&self, expected: usize, new: usize) -> Result<usize, usize> {
        self.read_pos.compare_exchange(
            expected,
            new,
            Ordering::AcqRel,  // Success: Acquire-Release  
            Ordering::Acquire,  // Failure: Acquire
        )
    }
    
    /// Check if shutdown is requested
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & flags::SHUTDOWN != 0
    }
    
    /// Request shutdown
    #[inline]
    pub fn request_shutdown(&self) {
        self.flags.fetch_or(flags::SHUTDOWN, Ordering::Release);
    }
}

// Static assertions to ensure correct layout
const _: () = {
    assert!(std::mem::size_of::<RingBufferHeader>() == 64);
    assert!(std::mem::align_of::<RingBufferHeader>() == 64);
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_header_size_and_alignment() {
        assert_eq!(std::mem::size_of::<RingBufferHeader>(), 64);
        assert_eq!(std::mem::align_of::<RingBufferHeader>(), 64);
    }
    
    #[test]
    fn test_header_initialization() {
        let mut buffer = vec![0u8; 64];
        unsafe {
            let header = RingBufferHeader::initialize(buffer.as_mut_ptr(), 1024);
            assert_eq!((*header).magic, MAGIC_NUMBER);
            assert_eq!((*header).version, PROTOCOL_VERSION);
            assert_eq!((*header).capacity, 1024);
            assert!((*header).validate().is_ok());
        }
    }
    
    #[test]
    fn test_available_space_calculations() {
        let mut buffer = vec![0u8; 64];
        unsafe {
            let header = RingBufferHeader::initialize(buffer.as_mut_ptr(), 1000);
            
            // Initially, should have capacity - 1 available
            assert_eq!((*header).available_write_space(), 999);
            assert_eq!((*header).available_read_data(), 0);
            
            // Write 100 bytes
            (*header).advance_write_pos(100);
            assert_eq!((*header).available_write_space(), 899);
            assert_eq!((*header).available_read_data(), 100);
            
            // Read 50 bytes
            (*header).advance_read_pos(50);
            assert_eq!((*header).available_write_space(), 949);
            assert_eq!((*header).available_read_data(), 50);
        }
    }
}
