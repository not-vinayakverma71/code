/// High-performance Single-Producer Single-Consumer ring buffer
/// Optimized for ≥1M msg/s with ≤10µs p99 latency
/// 
/// Key optimizations:
/// - 64-byte cache line alignment to prevent false sharing
/// - Minimal memory barriers (Acquire/Release only where needed)
/// - Batch API to amortize fence costs
/// - Power-of-two capacity for fast modulo via mask

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::ptr;
use anyhow::{Result, bail};

/// Cache line size on x86_64, ARM64, and most modern CPUs
const CACHE_LINE_SIZE: usize = 64;

/// Ring buffer header with cache-line aligned fields to prevent false sharing
#[repr(C)]
#[repr(align(64))]
pub struct RingHeader {
    /// Writer-owned position (updated by producer only)
    /// Aligned to its own cache line
    pub write_pos: AtomicU32,
    _padding1: [u8; CACHE_LINE_SIZE - 4],
    
    /// Reader-owned position (updated by consumer only)
    /// Aligned to its own cache line to avoid bouncing with write_pos
    pub read_pos: AtomicU32,
    _padding2: [u8; CACHE_LINE_SIZE - 4],
    
    /// Sequence number for futex/wait operations
    /// Incremented on every write to wake readers
    pub write_seq: AtomicU64,
    _padding3: [u8; CACHE_LINE_SIZE - 8],
    
    /// Ring capacity (power of 2)
    pub capacity: u32,
    _padding4: [u8; CACHE_LINE_SIZE - 4],
}

impl RingHeader {
    pub fn new(capacity: u32) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");
        Self {
            write_pos: AtomicU32::new(0),
            _padding1: [0; CACHE_LINE_SIZE - 4],
            read_pos: AtomicU32::new(0),
            _padding2: [0; CACHE_LINE_SIZE - 4],
            write_seq: AtomicU64::new(0),
            _padding3: [0; CACHE_LINE_SIZE - 8],
            capacity,
            _padding4: [0; CACHE_LINE_SIZE - 4],
        }
    }
}

/// SPSC ring buffer for lock-free, high-performance IPC
pub struct SpscRing {
    header: *mut RingHeader,
    data: *mut u8,
    capacity: usize,
    mask: usize, // capacity - 1 for fast modulo
}

unsafe impl Send for SpscRing {}
unsafe impl Sync for SpscRing {}

impl SpscRing {
    /// Create a new SPSC ring from pre-allocated shared memory
    /// 
    /// # Safety
    /// - header must point to valid, aligned RingHeader
    /// - data must point to valid buffer of at least capacity bytes
    /// - capacity must be power of 2
    pub unsafe fn from_raw(header: *mut RingHeader, data: *mut u8, capacity: usize) -> Self {
        assert!(capacity.is_power_of_two(), "Capacity must be power of 2");
        
        // Initialize header if needed
        ptr::write(header, RingHeader::new(capacity as u32));
        
        Self {
            header,
            data,
            capacity,
            mask: capacity - 1,
        }
    }
    
    /// Write a single message to the ring
    /// Returns true if written, false if full
    pub fn try_write(&self, msg: &[u8]) -> bool {
        if msg.len() > self.capacity / 2 {
            return false; // Message too large
        }
        
        unsafe {
            let header = &*self.header;
            
            // Load positions with relaxed ordering (no fence yet)
            let write_pos = header.write_pos.load(Ordering::Relaxed) as usize;
            let read_pos = header.read_pos.load(Ordering::Acquire) as usize;
            
            // Calculate available space
            let available = if write_pos >= read_pos {
                self.capacity - write_pos + read_pos
            } else {
                read_pos - write_pos
            };
            
            let total_len = 4 + msg.len(); // 4 bytes for length prefix
            if available < total_len {
                return false; // Ring full
            }
            
            // Write length prefix (4 bytes, little-endian)
            let len_bytes = (msg.len() as u32).to_le_bytes();
            let len_dst = self.data.add(write_pos & self.mask);
            ptr::copy_nonoverlapping(len_bytes.as_ptr(), len_dst, 4);
            
            // Write payload
            let data_start = (write_pos + 4) & self.mask;
            if data_start + msg.len() <= self.capacity {
                // Contiguous write
                ptr::copy_nonoverlapping(msg.as_ptr(), self.data.add(data_start), msg.len());
            } else {
                // Wrap around
                let first_part = self.capacity - data_start;
                ptr::copy_nonoverlapping(msg.as_ptr(), self.data.add(data_start), first_part);
                ptr::copy_nonoverlapping(
                    msg.as_ptr().add(first_part),
                    self.data,
                    msg.len() - first_part
                );
            }
            
            // Advance write position with Release fence
            // This ensures all writes above are visible before position update
            let new_write_pos = (write_pos + total_len) & self.mask;
            header.write_pos.store(new_write_pos as u32, Ordering::Release);
            
            // Increment sequence for waiter notification
            header.write_seq.fetch_add(1, Ordering::Release);
            
            true
        }
    }
    
    /// Try to read a single message from the ring
    /// Returns Some(message) if available, None if empty
    pub fn try_read(&self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*self.header;
            
            // Load positions with Acquire to see all writes
            let read_pos = header.read_pos.load(Ordering::Relaxed) as usize;
            let write_pos = header.write_pos.load(Ordering::Acquire) as usize;
            
            if read_pos == write_pos {
                return None; // Empty
            }
            
            // Read length prefix
            let len_src = self.data.add(read_pos & self.mask);
            let mut len_bytes = [0u8; 4];
            ptr::copy_nonoverlapping(len_src, len_bytes.as_mut_ptr(), 4);
            let msg_len = u32::from_le_bytes(len_bytes) as usize;
            
            if msg_len == 0 || msg_len > self.capacity / 2 {
                // Corrupted data - skip this position
                header.read_pos.store(write_pos as u32, Ordering::Release);
                return None;
            }
            
            // Read payload
            let mut data = vec![0u8; msg_len];
            let data_start = (read_pos + 4) & self.mask;
            
            if data_start + msg_len <= self.capacity {
                // Contiguous read
                ptr::copy_nonoverlapping(self.data.add(data_start), data.as_mut_ptr(), msg_len);
            } else {
                // Wrap around
                let first_part = self.capacity - data_start;
                ptr::copy_nonoverlapping(self.data.add(data_start), data.as_mut_ptr(), first_part);
                ptr::copy_nonoverlapping(
                    self.data,
                    data.as_mut_ptr().add(first_part),
                    msg_len - first_part
                );
            }
            
            // Advance read position with Release fence
            let new_read_pos = (read_pos + 4 + msg_len) & self.mask;
            header.read_pos.store(new_read_pos as u32, Ordering::Release);
            
            Some(data)
        }
    }
    
    /// Batch write multiple messages
    /// Returns number of messages written
    pub fn try_write_batch(&self, messages: &[&[u8]], max_batch: usize) -> usize {
        let mut written = 0;
        let batch_size = messages.len().min(max_batch);
        
        unsafe {
            let header = &*self.header;
            let write_pos = header.write_pos.load(Ordering::Relaxed) as usize;
            let read_pos = header.read_pos.load(Ordering::Acquire) as usize;
            
            let mut current_pos = write_pos;
            
            for i in 0..batch_size {
                let msg = messages[i];
                let total_len = 4 + msg.len();
                
                // Check space
                let available = if current_pos >= read_pos {
                    self.capacity - current_pos + read_pos
                } else {
                    read_pos - current_pos
                };
                
                if available < total_len || msg.len() > self.capacity / 2 {
                    break; // No more space or message too large
                }
                
                // Write length + data (same logic as try_write)
                let len_bytes = (msg.len() as u32).to_le_bytes();
                ptr::copy_nonoverlapping(len_bytes.as_ptr(), self.data.add(current_pos & self.mask), 4);
                
                let data_start = (current_pos + 4) & self.mask;
                if data_start + msg.len() <= self.capacity {
                    ptr::copy_nonoverlapping(msg.as_ptr(), self.data.add(data_start), msg.len());
                } else {
                    let first_part = self.capacity - data_start;
                    ptr::copy_nonoverlapping(msg.as_ptr(), self.data.add(data_start), first_part);
                    ptr::copy_nonoverlapping(msg.as_ptr().add(first_part), self.data, msg.len() - first_part);
                }
                
                current_pos = (current_pos + total_len) & self.mask;
                written += 1;
            }
            
            if written > 0 {
                // Single fence for entire batch
                header.write_pos.store(current_pos as u32, Ordering::Release);
                header.write_seq.fetch_add(1, Ordering::Release);
            }
            
            written
        }
    }
    
    /// Batch read up to max_batch messages
    /// Returns vector of messages read
    pub fn try_read_batch(&self, max_batch: usize) -> Vec<Vec<u8>> {
        let mut messages = Vec::with_capacity(max_batch.min(32));
        
        for _ in 0..max_batch {
            match self.try_read() {
                Some(msg) => messages.push(msg),
                None => break,
            }
        }
        
        messages
    }
    
    /// Get current write sequence (for waiter integration)
    pub fn write_seq(&self) -> u64 {
        unsafe { (*self.header).write_seq.load(Ordering::Acquire) }
    }
    
    /// Get pointer to write sequence (for futex/WaitOnAddress)
    pub fn write_seq_ptr(&self) -> *const AtomicU64 {
        unsafe { &(*self.header).write_seq as *const AtomicU64 }
    }
    
    /// Check if ring is empty
    pub fn is_empty(&self) -> bool {
        unsafe {
            let header = &*self.header;
            header.read_pos.load(Ordering::Relaxed) == header.write_pos.load(Ordering::Acquire)
        }
    }
    
    /// Get approximate occupancy (for metrics)
    pub fn occupancy(&self) -> (usize, usize) {
        unsafe {
            let header = &*self.header;
            let write_pos = header.write_pos.load(Ordering::Relaxed) as usize;
            let read_pos = header.read_pos.load(Ordering::Relaxed) as usize;
            
            let used = if write_pos >= read_pos {
                write_pos - read_pos
            } else {
                self.capacity - read_pos + write_pos
            };
            
            (used, self.capacity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc_zeroed, dealloc, Layout};
    
    #[test]
    fn test_spsc_single_message() {
        unsafe {
            let capacity = 4096;
            let header_layout = Layout::new::<RingHeader>();
            let data_layout = Layout::from_size_align(capacity, 64).unwrap();
            
            let header = alloc_zeroed(header_layout) as *mut RingHeader;
            let data = alloc_zeroed(data_layout);
            
            let ring = SpscRing::from_raw(header, data, capacity);
            
            let msg = b"Hello, SPSC ring!";
            assert!(ring.try_write(msg));
            
            let read_msg = ring.try_read().expect("Should read message");
            assert_eq!(read_msg, msg);
            
            assert!(ring.is_empty());
            
            dealloc(header as *mut u8, header_layout);
            dealloc(data, data_layout);
        }
    }
    
    #[test]
    fn test_spsc_batch() {
        unsafe {
            let capacity = 8192;
            let header_layout = Layout::new::<RingHeader>();
            let data_layout = Layout::from_size_align(capacity, 64).unwrap();
            
            let header = alloc_zeroed(header_layout) as *mut RingHeader;
            let data = alloc_zeroed(data_layout);
            
            let ring = SpscRing::from_raw(header, data, capacity);
            
            let messages: Vec<&[u8]> = vec![
                b"msg1",
                b"msg2",
                b"msg3",
                b"msg4",
            ];
            
            let written = ring.try_write_batch(&messages, 10);
            assert_eq!(written, 4);
            
            let read_msgs = ring.try_read_batch(10);
            assert_eq!(read_msgs.len(), 4);
            assert_eq!(read_msgs[0], b"msg1");
            assert_eq!(read_msgs[3], b"msg4");
            
            dealloc(header as *mut u8, header_layout);
            dealloc(data, data_layout);
        }
    }
}
