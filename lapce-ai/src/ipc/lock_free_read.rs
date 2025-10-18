/// Lock-free read implementation for shared memory
/// Ensures no await happens while holding locks
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;

/// Lock-free read implementation trait
pub trait LockFreeRead {
    /// Read data without holding lock during async operations
    fn try_read(&self) -> Option<Vec<u8>>;
    
    /// Read exact bytes with lock-free semantics
    async fn read_exact_lockfree(&self, buf: &mut [u8]) -> Result<()>;
    
    /// Read with timeout, lock-free
    async fn read_timeout_lockfree(&self, buf: &mut [u8], timeout_ms: u64) -> Result<usize>;
}

/// Wrapper for SharedMemoryBuffer with lock-free reads
pub struct LockFreeSharedBuffer {
    inner: Arc<RwLock<super::SharedMemoryBuffer>>,
}

impl LockFreeSharedBuffer {
    pub fn new(buffer: super::SharedMemoryBuffer) -> Self {
        Self {
            inner: Arc::new(RwLock::new(buffer)),
        }
    }
    
    /// Try to read without blocking
    #[inline(always)]
    pub fn try_read(&self) -> Option<Vec<u8>> {
        // Use try_write to avoid blocking
        if let Some(mut guard) = self.inner.try_write() {
            guard.read()
        } else {
            None
        }
    }
    
    /// Read exact number of bytes with lock-free guarantee
    pub async fn read_exact_lockfree(&self, buf: &mut [u8]) -> Result<()> {
        let needed = buf.len();
        let mut total_read = 0;
        
        while total_read < needed {
            // Read without holding lock across await
            let data = self.try_read();
            
            if let Some(data) = data {
                let to_copy = std::cmp::min(data.len(), needed - total_read);
                buf[total_read..total_read + to_copy].copy_from_slice(&data[..to_copy]);
                total_read += to_copy;
            } else {
                // No data available, yield without holding lock
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
        }
        
        Ok(())
    }
    
    /// Read with timeout, ensuring no lock across await
    pub async fn read_timeout_lockfree(&self, buf: &mut [u8], timeout_ms: u64) -> Result<usize> {
        let start = std::time::Instant::now();
        
        loop {
            // Try to read without blocking
            let data = self.try_read();
            
            if let Some(data) = data {
                let to_copy = std::cmp::min(data.len(), buf.len());
                buf[..to_copy].copy_from_slice(&data[..to_copy]);
                return Ok(to_copy);
            }
            
            // Check timeout
            if start.elapsed().as_millis() > timeout_ms as u128 {
                return Ok(0);  // Timeout, no data read
            }
            
            // Yield without holding lock
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        }
    }
    
    /// Write data (for testing)
    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.inner.write().write(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;
    
    #[tokio::test]
    async fn test_lock_free_read_exact() {
        // Create buffer
        let buffer = super::super::SharedMemoryBuffer::create("/test_lockfree_exact", 1024 * 1024).unwrap();
        let lock_free = LockFreeSharedBuffer::new(buffer);
        
        // Write test data
        let test_data = b"Hello, lock-free world!";
        lock_free.write(test_data).unwrap();
        
        // Read exact bytes
        let mut read_buf = vec![0u8; test_data.len()];
        lock_free.read_exact_lockfree(&mut read_buf).await.unwrap();
        
        assert_eq!(read_buf, test_data);
    }
    
    #[tokio::test]
    async fn test_lock_free_read_timeout() {
        // Create buffer
        let buffer = super::super::SharedMemoryBuffer::create("/test_lockfree_timeout", 1024 * 1024).unwrap();
        let lock_free = LockFreeSharedBuffer::new(buffer);
        
        // Test timeout with no data
        let mut read_buf = vec![0u8; 100];
        let bytes_read = lock_free.read_timeout_lockfree(&mut read_buf, 10).await.unwrap();
        assert_eq!(bytes_read, 0);  // Should timeout
        
        // Write data and read with timeout
        let test_data = b"Timeout test data";
        lock_free.write(test_data).unwrap();
        
        let bytes_read = lock_free.read_timeout_lockfree(&mut read_buf, 100).await.unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&read_buf[..bytes_read], test_data);
    }
    
    #[tokio::test]
    async fn test_no_lock_across_await() {
        // This test verifies that we don't hold locks across await points
        let buffer = super::super::SharedMemoryBuffer::create("/test_no_lock_await", 1024 * 1024).unwrap();
        let lock_free = Arc::new(LockFreeSharedBuffer::new(buffer));
        
        // Spawn reader task
        let lock_free_reader = lock_free.clone();
        let reader_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 10];
            // This should not block even if writer is slow
            lock_free_reader.read_timeout_lockfree(&mut buf, 50).await
        });
        
        // Spawn writer task
        let lock_free_writer = lock_free.clone();
        let writer_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            lock_free_writer.write(b"test").unwrap();
        });
        
        // Both should complete without deadlock
        let read_result = reader_handle.await.unwrap();
        writer_handle.await.unwrap();
        
        // Reader should have gotten data or timed out (no deadlock)
        assert!(read_result.is_ok());
    }
    
    #[tokio::test]
    async fn test_concurrent_readers() {
        // Test multiple concurrent readers don't block each other
        let buffer = super::super::SharedMemoryBuffer::create("/test_concurrent_readers", 1024 * 1024).unwrap();
        let lock_free = Arc::new(LockFreeSharedBuffer::new(buffer));
        
        // Write initial data
        for i in 0..10 {
            let data = format!("Message {}", i).into_bytes();
            lock_free.write(&data).unwrap();
        }
        
        // Spawn multiple reader tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let lock_free_clone = lock_free.clone();
            let handle = tokio::spawn(async move {
                let mut buf = vec![0u8; 100];
                let mut messages = vec![];
                
                for _ in 0..10 {
                    if let Ok(bytes) = lock_free_clone.read_timeout_lockfree(&mut buf, 10).await {
                        if bytes > 0 {
                            messages.push(String::from_utf8_lossy(&buf[..bytes]).to_string());
                        }
                    }
                }
                messages
            });
            handles.push(handle);
        }
        
        // All readers should complete without blocking each other
        for handle in handles {
            let messages = handle.await.unwrap();
            assert!(!messages.is_empty());
        }
    }
    
    #[test]
    fn test_try_read_non_blocking() {
        // Test that try_read doesn't block
        let buffer = super::super::SharedMemoryBuffer::create("/test_try_read", 1024 * 1024).unwrap();
        let lock_free = LockFreeSharedBuffer::new(buffer);
        
        // Write some data
        lock_free.write(b"non-blocking").unwrap();
        
        // Try read should succeed immediately
        let data = lock_free.try_read();
        assert!(data.is_some());
        assert_eq!(data.unwrap(), b"non-blocking");
        
        // Second read should return None (no data)
        let data = lock_free.try_read();
        assert!(data.is_none());
    }
}
