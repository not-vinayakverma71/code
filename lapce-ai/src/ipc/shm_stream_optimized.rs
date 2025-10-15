/// Optimized SharedMemory transport using SPSC rings and cross-OS waiters
/// Target: ≥1M msg/s throughput, ≤10µs p99 latency across Linux/Windows/macOS

use std::sync::Arc;
use std::ptr;
use anyhow::{Result, bail};
use crate::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use crate::ipc::shm_waiter_cross_os::ShmWaiter;
use crate::ipc::shm_namespace::create_namespaced_path;
use crate::ipc::shm_permissions::create_fd_0600;
use std::time::Duration;

const DEFAULT_RING_SIZE: usize = 2 * 1024 * 1024; // 2MB per ring
const BATCH_SIZE: usize = 16; // Messages per batch

/// Optimized connection with two SPSC rings (send/recv)
pub struct OptimizedShmStream {
    send_ring: Arc<SpscRing>,
    recv_ring: Arc<SpscRing>,
    send_waiter: Arc<ShmWaiter>,
    recv_waiter: Arc<ShmWaiter>,
    conn_id: u64,
    _send_shm: ShmSegment,
    _recv_shm: ShmSegment,
}

struct ShmSegment {
    ptr: *mut u8,
    size: usize,
    name: String,
}

impl Drop for ShmSegment {
    fn drop(&mut self) {
        unsafe {
            #[cfg(unix)]
            {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
                let name_cstr = std::ffi::CString::new(self.name.as_bytes()).unwrap();
                libc::shm_unlink(name_cstr.as_ptr());
            }
            
            #[cfg(windows)]
            {
                use windows_sys::Win32::System::Memory::{UnmapViewOfFile, VirtualFree, MEM_RELEASE};
                UnmapViewOfFile(self.ptr as *const std::ffi::c_void);
                VirtualFree(self.ptr as *mut std::ffi::c_void, 0, MEM_RELEASE);
            }
        }
    }
}

impl OptimizedShmStream {
    /// Connect to server using optimized SPSC transport
    pub async fn connect(path: &str) -> Result<Self> {
        let conn_id = rand::random::<u64>();
        
        // Create two SPSC rings: send (client→server) and recv (server→client)
        let send_shm = Self::create_shm_segment(&format!("{}_send_{}", path, conn_id), DEFAULT_RING_SIZE)?;
        let recv_shm = Self::create_shm_segment(&format!("{}_recv_{}", path, conn_id), DEFAULT_RING_SIZE)?;
        
        unsafe {
            let send_header = send_shm.ptr as *mut RingHeader;
            let send_data = send_shm.ptr.add(std::mem::size_of::<RingHeader>());
            // Calculate data capacity as power of 2
            let header_size = std::mem::size_of::<RingHeader>();
            let data_capacity = (DEFAULT_RING_SIZE - header_size).next_power_of_two() / 2; // Ensure power of 2
            let send_ring = Arc::new(SpscRing::from_raw(
                send_header,
                send_data,
                data_capacity,
            ));
            
            let recv_header = recv_shm.ptr as *mut RingHeader;
            let recv_data = recv_shm.ptr.add(std::mem::size_of::<RingHeader>());
            let recv_ring = Arc::new(SpscRing::from_raw(
                recv_header,
                recv_data,
                data_capacity,
            ));
            
            Ok(Self {
                send_ring,
                recv_ring,
                send_waiter: Arc::new(ShmWaiter::new()?),
                recv_waiter: Arc::new(ShmWaiter::new()?),
                conn_id,
                _send_shm: send_shm,
                _recv_shm: recv_shm,
            })
        }
    }
    
    #[cfg(unix)]
    fn create_shm_segment(name: &str, size: usize) -> Result<ShmSegment> {
        let namespaced = create_namespaced_path(name);
        let shm_name = std::ffi::CString::new(namespaced.as_bytes())?;
        
        unsafe {
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR,
                0o600,
            );
            
            if fd == -1 {
                bail!("shm_open failed: {}", std::io::Error::last_os_error());
            }
            
            if libc::ftruncate(fd, size as i64) == -1 {
                libc::close(fd);
                bail!("ftruncate failed: {}", std::io::Error::last_os_error());
            }
            
            create_fd_0600(fd)?;
            
            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            
            libc::close(fd);
            
            if ptr == libc::MAP_FAILED {
                bail!("mmap failed: {}", std::io::Error::last_os_error());
            }
            
            // Pre-touch pages and advise huge pages
            Self::setup_memory_advice(ptr as *mut u8, size)?;
            
            Ok(ShmSegment {
                ptr: ptr as *mut u8,
                size,
                name: namespaced,
            })
        }
    }
    
    #[cfg(windows)]
    fn create_shm_segment(name: &str, size: usize) -> Result<ShmSegment> {
        use windows_sys::Win32::System::Memory::{
            CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS,
            PAGE_READWRITE, VirtualAlloc, MEM_COMMIT, MEM_RESERVE,
        };
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        
        unsafe {
            let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null(),
                PAGE_READWRITE,
                0,
                size as u32,
                wide_name.as_ptr(),
            );
            
            if handle == 0 {
                bail!("CreateFileMappingW failed");
            }
            
            let ptr = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size);
            
            if ptr.is_null() {
                bail!("MapViewOfFile failed");
            }
            
            Self::setup_memory_advice(ptr as *mut u8, size)?;
            
            Ok(ShmSegment {
                ptr: ptr as *mut u8,
                size,
                name: name.to_string(),
            })
        }
    }
    
    #[cfg(unix)]
    fn setup_memory_advice(ptr: *mut u8, size: usize) -> Result<()> {
        unsafe {
            // Pre-touch all pages to avoid faults in hot path
            for offset in (0..size).step_by(4096) {
                ptr.add(offset).write_volatile(0);
            }
            
            #[cfg(target_os = "linux")]
            {
                // Advise huge pages on Linux
                libc::madvise(
                    ptr as *mut libc::c_void,
                    size,
                    libc::MADV_HUGEPAGE,
                );
            }
            
            #[cfg(target_os = "macos")]
            {
                // Best-effort advice on macOS
                libc::madvise(
                    ptr as *mut libc::c_void,
                    size,
                    libc::MADV_WILLNEED,
                );
            }
        }
        Ok(())
    }
    
    #[cfg(windows)]
    fn setup_memory_advice(ptr: *mut u8, size: usize) -> Result<()> {
        unsafe {
            // Pre-touch pages
            for offset in (0..size).step_by(4096) {
                ptr.add(offset).write_volatile(0);
            }
        }
        Ok(())
    }
    
    /// Write single message with batching support
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        loop {
            if self.send_ring.try_write(data) {
                // Wake receiver only if it might be waiting
                if self.send_ring.occupancy().0 == data.len() + 4 {
                    self.send_waiter.wake_one(self.send_ring.write_seq_ptr());
                }
                return Ok(());
            }
            
            // Ring full, wait for space
            let seq = self.send_ring.write_seq();
            tokio::time::sleep(Duration::from_micros(10)).await;
        }
    }
    
    /// Write batch of messages (amortize fences)
    pub async fn write_batch(&self, messages: &[&[u8]]) -> Result<usize> {
        let written = self.send_ring.try_write_batch(messages, BATCH_SIZE);
        
        if written > 0 {
            self.send_waiter.wake_one(self.send_ring.write_seq_ptr());
        }
        
        Ok(written)
    }
    
    /// Read single message with low-latency wait
    pub async fn read(&self) -> Result<Vec<u8>> {
        loop {
            if let Some(msg) = self.recv_ring.try_read() {
                return Ok(msg);
            }
            
            // Ring empty, wait with bounded timeout
            let seq = self.recv_ring.write_seq();
            
            // Try a few yields before blocking
            for _ in 0..5 {
                tokio::task::yield_now().await;
                if let Some(msg) = self.recv_ring.try_read() {
                    return Ok(msg);
                }
            }
            
            // Block wait (waiter handles bounded spin internally)
            let waiter = self.recv_waiter.clone();
            let seq_ptr = self.recv_ring.write_seq_ptr() as usize; // Convert to usize for Send
            let _woken = tokio::task::spawn_blocking(move || {
                waiter.wait(seq_ptr as *const std::sync::atomic::AtomicU64, seq, Duration::from_millis(10))
            }).await.unwrap_or(false);
        }
    }
    
    /// Read batch (drain available messages)
    pub async fn read_batch(&self, max: usize) -> Vec<Vec<u8>> {
        self.recv_ring.try_read_batch(max)
    }
    
    /// Read exact bytes (for compatibility)
    pub async fn read_exact(&self, buf: &mut [u8]) -> Result<()> {
        let mut total = 0;
        
        while total < buf.len() {
            let msg = self.read().await?;
            let to_copy = msg.len().min(buf.len() - total);
            buf[total..total + to_copy].copy_from_slice(&msg[..to_copy]);
            total += to_copy;
        }
        
        Ok(())
    }
    
    /// Write all bytes (for compatibility)
    pub async fn write_all(&self, buf: &[u8]) -> Result<()> {
        self.write(buf).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_optimized_stream_roundtrip() {
        let stream = OptimizedShmStream::connect("/test_optimized_stream")
            .await
            .expect("Failed to create stream");
        
        let msg = b"Hello, optimized IPC!";
        stream.write(msg).await.expect("Write failed");
        
        // For testing, write to recv ring to simulate server response
        stream.recv_ring.try_write(msg);
        stream.recv_waiter.wake_one(stream.recv_ring.write_seq_ptr());
        
        let received = stream.read().await.expect("Read failed");
        assert_eq!(received, msg);
    }
    
    #[tokio::test]
    async fn test_batch_operations() {
        let stream = OptimizedShmStream::connect("/test_batch_ops")
            .await
            .expect("Failed to create stream");
        
        let messages: Vec<&[u8]> = vec![b"msg1", b"msg2", b"msg3"];
        let written = stream.write_batch(&messages).await.expect("Batch write failed");
        assert_eq!(written, 3);
        
        // Simulate server echo
        for msg in &messages {
            stream.recv_ring.try_write(msg);
        }
        stream.recv_waiter.wake_one(stream.recv_ring.write_seq_ptr());
        
        let received = stream.read_batch(10).await;
        assert_eq!(received.len(), 3);
    }
}
