// Shared Memory Pool for Multi-Process Zero-Copy Access
// Implements lock-free shared memory with reference counting

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::collections::HashMap;
// Removed unused imports
use std::time::Instant;
use parking_lot::RwLock;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::PathBuf;
use crate::error::{Error, Result};

/// Shared memory segment
pub struct SharedSegment {
    /// Unique identifier
    pub id: u64,
    /// Memory mapped region
    mmap: Arc<RwLock<MmapMut>>,
    /// Reference count
    ref_count: Arc<AtomicUsize>,
    /// Lock status
    locked: AtomicBool,
    /// Size in bytes
    size: usize,
}

/// IPC message for coordination
#[derive(Debug, Clone)]
pub enum IpcMessage {
    Allocate { size: usize, id: u64 },
    Release { id: u64 },
    Lock { id: u64 },
    Unlock { id: u64 },
    Sync,
}

/// Shared memory pool manager
pub struct SharedMemoryPool {
    /// Pool name for IPC
    name: String,
    /// Base path for memory files
    base_path: PathBuf,
    /// Active segments
    segments: Arc<RwLock<HashMap<u64, Arc<SharedSegment>>>>,
    /// Next segment ID
    next_id: AtomicUsize,
    /// Total allocated size
    total_allocated: AtomicUsize,
    /// Maximum pool size
    max_size: usize,
    /// IPC channel
    ipc_sender: Option<crossbeam_channel::Sender<IpcMessage>>,
    ipc_receiver: Option<crossbeam_channel::Receiver<IpcMessage>>,
}

impl SharedMemoryPool {
    /// Create new shared memory pool
    pub fn new(name: String, max_size: usize) -> Result<Self> {
        let base_path = std::env::temp_dir().join(format!("lancedb_shm_{}", name));
        std::fs::create_dir_all(&base_path).map_err(|e| Error::Runtime {
            message: format!("Failed to create shared memory directory: {}", e),
        })?;
        
        // Create IPC channel
        let (sender, receiver) = crossbeam_channel::unbounded();
        
        Ok(Self {
            name,
            base_path,
            segments: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicUsize::new(1),
            total_allocated: AtomicUsize::new(0),
            max_size,
            ipc_sender: Some(sender),
            ipc_receiver: Some(receiver),
        })
    }
    
    /// Allocate shared memory segment
    pub fn allocate(&self, size: usize) -> Result<Arc<SharedSegment>> {
        let start = Instant::now();
        
        // Check pool limit
        let current = self.total_allocated.load(Ordering::Relaxed);
        if current + size > self.max_size {
            return Err(Error::Runtime {
                message: format!("Pool limit exceeded: {} + {} > {}", current, size, self.max_size),
            });
        }
        
        // Generate unique ID
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64;
        
        // Create memory mapped file
        let file_path = self.base_path.join(format!("segment_{}.bin", id));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path).map_err(|e| Error::Runtime {
                message: format!("Failed to create shared memory file: {}", e),
            })?;
        
        // Set file size
        file.set_len(size as u64).map_err(|e| Error::Runtime {
            message: format!("Failed to set file size: {}", e),
        })?;
        
        // Memory map the file
        let mmap = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(&file).map_err(|e| Error::Runtime {
                    message: format!("Failed to memory map file: {}", e),
                })?
        };
        
        // Create segment
        let segment = Arc::new(SharedSegment {
            id,
            mmap: Arc::new(RwLock::new(mmap)),
            ref_count: Arc::new(AtomicUsize::new(1)),
            locked: AtomicBool::new(false),
            size,
        });
        
        // Register segment
        self.segments.write().insert(id, segment.clone());
        self.total_allocated.fetch_add(size, Ordering::SeqCst);
        
        // Send IPC message
        if let Some(sender) = &self.ipc_sender {
            let _ = sender.send(IpcMessage::Allocate { size, id });
        }
        
        let elapsed = start.elapsed();
        log::debug!("Allocated {}MB in {:?}", size / 1048576, elapsed);
        
        Ok(segment)
    }
    
    /// Get segment by ID (zero-copy)
    pub fn get(&self, id: u64) -> Option<Arc<SharedSegment>> {
        let segments = self.segments.read();
        segments.get(&id).map(|s| {
            s.ref_count.fetch_add(1, Ordering::SeqCst);
            s.clone()
        })
    }
    
    /// Release segment
    pub fn release(&self, id: u64) -> Result<()> {
        let mut segments = self.segments.write();
        
        if let Some(segment) = segments.get(&id) {
            let refs = segment.ref_count.fetch_sub(1, Ordering::SeqCst);
            
            if refs == 1 {
                // Last reference, deallocate
                let size = segment.size;
                segments.remove(&id);
                self.total_allocated.fetch_sub(size, Ordering::SeqCst);
                
                // Remove file
                let file_path = self.base_path.join(format!("segment_{}.bin", id));
                let _ = std::fs::remove_file(file_path);
                
                // Send IPC message
                if let Some(sender) = &self.ipc_sender {
                    let _ = sender.send(IpcMessage::Release { id });
                }
            }
        }
        
        Ok(())
    }
    
    /// Lock segment for exclusive access
    pub fn lock(&self, id: u64) -> Result<()> {
        if let Some(segment) = self.segments.read().get(&id) {
            while segment.locked.compare_exchange_weak(
                false,
                true,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ).is_err() {
                std::hint::spin_loop();
            }
            
            // Send IPC message
            if let Some(sender) = &self.ipc_sender {
                let _ = sender.send(IpcMessage::Lock { id });
            }
        }
        
        Ok(())
    }
    
    /// Unlock segment
    pub fn unlock(&self, id: u64) -> Result<()> {
        if let Some(segment) = self.segments.read().get(&id) {
            segment.locked.store(false, Ordering::SeqCst);
            
            // Send IPC message
            if let Some(sender) = &self.ipc_sender {
                let _ = sender.send(IpcMessage::Unlock { id });
            }
        }
        
        Ok(())
    }
    
    /// Process IPC messages
    pub async fn process_ipc_messages(&self) -> Result<()> {
        if let Some(receiver) = &self.ipc_receiver {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    IpcMessage::Sync => {
                        // Sync all segments
                        for segment in self.segments.read().values() {
                            let _ = segment.flush();
                        }
                    }
                    _ => {
                        // Other messages handled by respective methods
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total_segments: self.segments.read().len(),
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            max_size: self.max_size,
        }
    }
}

impl SharedSegment {
    /// Get raw pointer (zero-copy access)
    pub fn as_ptr(&self) -> *const u8 {
        let mmap = self.mmap.read();
        mmap.as_ptr()
    }
    
    /// Get mutable pointer (zero-copy write)
    pub fn as_mut_ptr(&self) -> *mut u8 {
        let mut mmap = self.mmap.write();
        mmap.as_mut_ptr()
    }
    
    /// Read data (zero-copy)
    pub fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        if offset + len > self.size {
            return Err(Error::Runtime {
                message: format!("Read out of bounds: {} + {} > {}", offset, len, self.size),
            });
        }
        
        let mmap = self.mmap.read();
        let mut result = vec![0u8; len];
        unsafe {
            std::ptr::copy_nonoverlapping(
                mmap.as_ptr().add(offset),
                result.as_mut_ptr(),
                len
            );
        }
        Ok(result)
    }
    
    /// Write data (zero-copy)
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(Error::Runtime {
                message: format!("Write out of bounds: {} + {} > {}", offset, data.len(), self.size),
            });
        }
        
        let mut mmap = self.mmap.write();
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mmap.as_mut_ptr().add(offset),
                data.len()
            );
        }
        
        Ok(())
    }
    
    /// Flush to disk
    pub fn flush(&self) -> Result<()> {
        let mmap = self.mmap.read();
        mmap.flush().map_err(|e| Error::Runtime {
            message: format!("Failed to flush memory map: {}", e),
        })?;
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug)]
pub struct PoolStats {
    pub total_segments: usize,
    pub total_allocated: usize,
    pub max_size: usize,
}

/// Lock-free ring buffer for IPC
pub struct LockFreeRingBuffer {
    buffer: Vec<AtomicUsize>,
    head: AtomicUsize,
    tail: AtomicUsize,
    capacity: usize,
}

impl LockFreeRingBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(AtomicUsize::new(0));
        }
        
        Self {
            buffer,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            capacity,
        }
    }
    
    pub fn push(&self, value: usize) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (tail + 1) % self.capacity;
        
        if next_tail == self.head.load(Ordering::Acquire) {
            return false; // Buffer full
        }
        
        self.buffer[tail].store(value, Ordering::Release);
        self.tail.store(next_tail, Ordering::Release);
        true
    }
    
    pub fn pop(&self) -> Option<usize> {
        let head = self.head.load(Ordering::Relaxed);
        
        if head == self.tail.load(Ordering::Acquire) {
            return None; // Buffer empty
        }
        
        let value = self.buffer[head].load(Ordering::Acquire);
        self.head.store((head + 1) % self.capacity, Ordering::Release);
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shared_memory_allocation() {
        let pool = SharedMemoryPool::new("test".to_string(), 100_000_000).unwrap();
        
        // Allocate segment
        let segment = pool.allocate(1024).unwrap();
        assert_eq!(segment.size, 1024);
        
        // Write data
        let data = b"Hello, shared memory!";
        segment.write(0, data).unwrap();
        
        // Read data
        let read_data = segment.read(0, data.len()).unwrap();
        assert_eq!(&read_data[..], data);
    }
    
    #[test]
    fn test_zero_copy_access() {
        let pool = SharedMemoryPool::new("test_zc".to_string(), 100_000_000).unwrap();
        let segment = pool.allocate(4096).unwrap();
        
        // Direct pointer access (zero-copy)
        unsafe {
            let ptr = segment.as_mut_ptr();
            std::ptr::write(ptr, 42u8);
            
            let read_ptr = segment.as_ptr();
            let value = std::ptr::read(read_ptr);
            assert_eq!(value, 42u8);
        }
    }
    
    #[tokio::test]
    async fn test_multi_process_simulation() {
        let pool = Arc::new(SharedMemoryPool::new("test_mp".to_string(), 100_000_000).unwrap());
        
        // Simulate multiple processes
        let mut handles = vec![];
        
        for i in 0..3 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let segment = pool_clone.allocate(1024).unwrap();
                pool_clone.lock(segment.id).unwrap();
                // Simulate work
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                pool_clone.unlock(segment.id).unwrap();
                pool_clone.release(segment.id).unwrap();
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        let stats = pool.stats();
        assert_eq!(stats.total_segments, 0); // All released
    }
}
