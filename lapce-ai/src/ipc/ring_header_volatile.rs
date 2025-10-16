/// Cross-process safe ring buffer header using volatile operations
/// Replaces AtomicUsize which is NOT safe across processes
/// Uses Release/Acquire fences for proper cross-process visibility

use std::cell::UnsafeCell;
use std::sync::atomic::{fence, Ordering};

/// Cache-line aligned header for cross-process ring buffer
/// MUST be mapped with MAP_SHARED for cross-process visibility
#[repr(C, align(64))]
pub struct VolatileRingHeader {
    /// Write position (updated by writer)
    write_pos: UnsafeCell<u32>,
    
    /// Read position (updated by reader)
    read_pos: UnsafeCell<u32>,
    
    /// Ring capacity (immutable after creation)
    capacity: u32,
    
    /// Sequence number for debugging
    seq: UnsafeCell<u64>,
    
    /// Padding to full cache line
    _pad: [u8; 64 - 4 - 4 - 4 - 8],
}

impl VolatileRingHeader {
    /// Create new header (MUST be in MAP_SHARED memory)
    pub fn new(capacity: u32) -> Self {
        Self {
            write_pos: UnsafeCell::new(0),
            read_pos: UnsafeCell::new(0),
            capacity,
            seq: UnsafeCell::new(0),
            _pad: [0; 64 - 4 - 4 - 4 - 8],
        }
    }
    
    /// Writer: Load write position
    /// No fence needed - writer owns write_pos
    #[inline]
    pub fn load_write_pos(&self) -> u32 {
        unsafe { std::ptr::read_volatile(self.write_pos.get()) }
    }
    
    /// Writer: Store write position with Release semantics
    /// Ensures all prior writes (payload data) are visible before position update
    #[inline]
    pub fn store_write_pos(&self, pos: u32) {
        fence(Ordering::Release);
        unsafe { std::ptr::write_volatile(self.write_pos.get(), pos) }
    }
    
    /// Writer: Load read position (to check available space)
    /// Use Acquire to see reader's updates
    #[inline]
    pub fn load_read_pos_acquire(&self) -> u32 {
        let pos = unsafe { std::ptr::read_volatile(self.read_pos.get()) };
        fence(Ordering::Acquire);
        pos
    }
    
    /// Reader: Load read position
    /// No fence needed - reader owns read_pos
    #[inline]
    pub fn load_read_pos(&self) -> u32 {
        unsafe { std::ptr::read_volatile(self.read_pos.get()) }
    }
    
    /// Reader: Store read position with Release semantics
    /// Ensures reader has consumed data before updating position
    #[inline]
    pub fn store_read_pos(&self, pos: u32) {
        fence(Ordering::Release);
        unsafe { std::ptr::write_volatile(self.read_pos.get(), pos) }
    }
    
    /// Reader: Load write position with Acquire semantics
    /// Ensures we see all writes that happened before writer updated write_pos
    #[inline]
    pub fn load_write_pos_acquire(&self) -> u32 {
        let pos = unsafe { std::ptr::read_volatile(self.write_pos.get()) };
        fence(Ordering::Acquire);
        pos
    }
    
    /// Get capacity (immutable, no synchronization needed)
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    
    /// Calculate available space for writing
    #[inline]
    pub fn available_write(&self) -> u32 {
        let w = self.load_write_pos();
        let r = self.load_read_pos_acquire();
        
        if w >= r {
            self.capacity - (w - r) - 1
        } else {
            r - w - 1
        }
    }
    
    /// Calculate available data for reading
    #[inline]
    pub fn available_read(&self) -> u32 {
        let w = self.load_write_pos_acquire();
        let r = self.load_read_pos();
        
        if w >= r {
            w - r
        } else {
            self.capacity - r + w
        }
    }
    
    /// Increment sequence number (for debugging/tracing)
    pub fn increment_seq(&self) {
        unsafe {
            let seq = std::ptr::read_volatile(self.seq.get());
            std::ptr::write_volatile(self.seq.get(), seq + 1);
        }
    }
    
    /// Load sequence number
    pub fn load_seq(&self) -> u64 {
        unsafe { std::ptr::read_volatile(self.seq.get()) }
    }
}

// SAFETY: VolatileRingHeader is designed for cross-process use
// with proper volatile operations and memory fences
unsafe impl Send for VolatileRingHeader {}
unsafe impl Sync for VolatileRingHeader {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_header_size() {
        // Must be exactly 64 bytes (one cache line)
        assert_eq!(std::mem::size_of::<VolatileRingHeader>(), 64);
        assert_eq!(std::mem::align_of::<VolatileRingHeader>(), 64);
    }
    
    #[test]
    fn test_basic_operations() {
        let header = VolatileRingHeader::new(1024);
        
        assert_eq!(header.load_write_pos(), 0);
        assert_eq!(header.load_read_pos(), 0);
        assert_eq!(header.capacity(), 1024);
        
        header.store_write_pos(100);
        assert_eq!(header.load_write_pos(), 100);
        
        header.store_read_pos(50);
        assert_eq!(header.load_read_pos(), 50);
    }
    
    #[test]
    fn test_available_space() {
        let header = VolatileRingHeader::new(1024);
        
        // Empty: can write capacity - 1
        assert_eq!(header.available_write(), 1023);
        assert_eq!(header.available_read(), 0);
        
        // Write 100 bytes
        header.store_write_pos(100);
        assert_eq!(header.available_read(), 100);
        assert_eq!(header.available_write(), 1023 - 100);
        
        // Read 50 bytes
        header.store_read_pos(50);
        assert_eq!(header.available_read(), 50);
        assert_eq!(header.available_write(), 1023 - 50);
    }
}
