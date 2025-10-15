/// Shared memory implementation using volatile pointers instead of atomics
/// This works around the issue where Rust atomics don't sync properly in shared memory

use anyhow::{Result, bail};
use std::ptr;

/// Ring buffer header using volatile reads/writes instead of atomics
#[repr(C)]
pub struct VolatileRingBufferHeader {
    pub magic: u64,
    pub write_pos: usize,
    pub read_pos: usize,
    pub capacity: usize,
    pub version: u32,
    pub flags: u32,
    pub sequence: u64,
    pub last_error: u64,
    _padding: [u8; 8],  // Align to 64 bytes
}

impl VolatileRingBufferHeader {
    /// Initialize header in shared memory
    pub unsafe fn initialize(ptr: *mut u8, capacity: usize) -> *mut Self {
        let header = ptr as *mut Self;
        
        // Zero the entire header
        ptr::write_bytes(ptr, 0, std::mem::size_of::<Self>());
        
        // Initialize fields using volatile writes
        ptr::write_volatile(&mut (*header).magic, 0xDEADBEEF);
        ptr::write_volatile(&mut (*header).write_pos, 0);
        ptr::write_volatile(&mut (*header).read_pos, 0);
        ptr::write_volatile(&mut (*header).capacity, capacity);
        ptr::write_volatile(&mut (*header).version, 1);
        ptr::write_volatile(&mut (*header).flags, 1); // INITIALIZED
        ptr::write_volatile(&mut (*header).sequence, 0);
        ptr::write_volatile(&mut (*header).last_error, 0);
        
        // Sync to ensure visibility
        libc::msync(ptr as *mut core::ffi::c_void, 64, 1); // MS_SYNC
        
        header
    }
    
    /// Read write position with volatile read
    #[inline]
    pub unsafe fn get_write_pos(&self) -> usize {
        ptr::read_volatile(&self.write_pos)
    }
    
    /// Set write position with volatile write
    #[inline]
    pub unsafe fn set_write_pos(&mut self, pos: usize) {
        ptr::write_volatile(&mut self.write_pos, pos);
    }
    
    /// Read read position with volatile read
    #[inline]
    pub unsafe fn get_read_pos(&self) -> usize {
        ptr::read_volatile(&self.read_pos)
    }
    
    /// Set read position with volatile write
    #[inline]
    pub unsafe fn set_read_pos(&mut self, pos: usize) {
        ptr::write_volatile(&mut self.read_pos, pos);
    }
}

pub struct VolatileSharedMemoryBuffer {
    ptr: *mut u8,
    size: usize,
    capacity: usize,
    header: *mut VolatileRingBufferHeader,
    data_ptr: *mut u8,
    debug_name: String,
}

unsafe impl Send for VolatileSharedMemoryBuffer {}
unsafe impl Sync for VolatileSharedMemoryBuffer {}

impl VolatileSharedMemoryBuffer {
    /// Create or open shared memory buffer
    pub async fn create(path: &str, size: usize) -> Result<Self> {
        let path = path.to_string();
        let size = size;
        tokio::task::spawn_blocking(move || Self::create_blocking(&path, size))
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
    }
    
    fn create_blocking(path: &str, size: usize) -> Result<Self> {
        let header_size = std::mem::size_of::<VolatileRingBufferHeader>();
        let total_size = header_size + size;
        
        // Create shared memory name
        let shm_name_str = format!("/{}", path.trim_start_matches('/').replace('/', "_"));
        let shm_name = std::ffi::CString::new(shm_name_str.clone())
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        
        // Try to open existing first
        let fd = unsafe {
            let fd = libc::shm_open(shm_name.as_ptr(), libc::O_RDWR as i32, 0);
            if fd == -1 {
                // Doesn't exist, create it
                let fd = libc::shm_open(
                    shm_name.as_ptr(),
                    (libc::O_CREAT | libc::O_RDWR) as i32,
                    0o600
                );
                if fd == -1 {
                    bail!("shm_open failed: {}", std::io::Error::last_os_error());
                }
                
                // Set size
                if libc::ftruncate(fd, total_size as i64) == -1 {
                    libc::close(fd);
                    bail!("ftruncate failed: {}", std::io::Error::last_os_error());
                }
                
                eprintln!("[VOLATILE] Created new shared memory: {}", shm_name_str);
                fd
            } else {
                eprintln!("[VOLATILE] Opened existing shared memory: {}", shm_name_str);
                fd
            }
        };
        
        // Map the memory
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                total_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0
            )
        };
        
        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd); }
            bail!("mmap failed: {}", std::io::Error::last_os_error());
        }
        
        unsafe { libc::close(fd); }
        
        // Initialize or validate header
        let header = unsafe {
            let h = ptr as *mut VolatileRingBufferHeader;
            if (*h).magic != 0xDEADBEEF {
                // New buffer, initialize
                VolatileRingBufferHeader::initialize(ptr as *mut u8, size)
            } else {
                h
            }
        };
        
        Ok(Self {
            ptr: ptr as *mut u8,
            size: total_size,
            capacity: size,
            header,
            data_ptr: unsafe { (ptr as *mut u8).add(header_size) },
            debug_name: path.to_string(),
        })
    }
    
    /// Write data to buffer
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        unsafe {
            let write_pos = (*self.header).get_write_pos();
            let read_pos = (*self.header).get_read_pos();
            
            // Check space
            let available = if write_pos >= read_pos {
                self.capacity - write_pos + read_pos
            } else {
                read_pos - write_pos
            };
            
            let needed = 4 + data.len();
            if available <= needed {
                bail!("Buffer full");
            }
            
            // Write length prefix
            let len_bytes = (data.len() as u32).to_le_bytes();
            let dst = self.data_ptr.add(write_pos);
            ptr::copy_nonoverlapping(len_bytes.as_ptr(), dst, 4);
            
            // Write data
            let data_dst = dst.add(4);
            ptr::copy_nonoverlapping(data.as_ptr(), data_dst, data.len());
            
            // Update write position
            let new_pos = (write_pos + needed) % self.capacity;
            (*self.header).set_write_pos(new_pos);
            
            // Sync memory
            libc::msync(
                self.ptr as *mut core::ffi::c_void,
                self.size,
                1 // MS_SYNC
            );
            
            eprintln!("[VOLATILE WRITE] {} bytes, write_pos {} -> {}", data.len(), write_pos, new_pos);
        }
        
        Ok(())
    }
    
    /// Read data from buffer
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        unsafe {
            // Sync memory to see writes
            libc::msync(
                self.ptr as *mut core::ffi::c_void,
                self.size,
                3 // MS_SYNC | MS_INVALIDATE
            );
            
            let write_pos = (*self.header).get_write_pos();
            let read_pos = (*self.header).get_read_pos();
            
            eprintln!("[VOLATILE READ] read_pos={}, write_pos={}", read_pos, write_pos);
            
            if read_pos == write_pos {
                return None; // Empty
            }
            
            // Read length prefix
            let src = self.data_ptr.add(read_pos);
            let mut len_bytes = [0u8; 4];
            ptr::copy_nonoverlapping(src, len_bytes.as_mut_ptr(), 4);
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            if len > 1024 * 1024 {
                eprintln!("[VOLATILE READ] Invalid length: {}", len);
                return None;
            }
            
            // Read data
            let mut data = vec![0u8; len];
            let data_src = src.add(4);
            ptr::copy_nonoverlapping(data_src, data.as_mut_ptr(), len);
            
            // Update read position
            let new_pos = (read_pos + 4 + len) % self.capacity;
            (*self.header).set_read_pos(new_pos);
            
            // Sync memory
            libc::msync(
                self.ptr as *mut core::ffi::c_void,
                self.size,
                1 // MS_SYNC
            );
            
            eprintln!("[VOLATILE READ] Read {} bytes, read_pos {} -> {}", len, read_pos, new_pos);
            
            Some(data)
        }
    }
}

impl Drop for VolatileSharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut core::ffi::c_void, self.size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_volatile_shared_memory() {
        let path = "/test_volatile_shm";
        
        // Create buffer
        let mut writer = VolatileSharedMemoryBuffer::create(path, 1024 * 1024).await.unwrap();
        
        // Write data
        let data = b"Hello from volatile shared memory!";
        writer.write(data).await.unwrap();
        
        // Create reader (simulates different process)
        let mut reader = VolatileSharedMemoryBuffer::create(path, 1024 * 1024).await.unwrap();
        
        // Read data
        if let Some(read_data) = reader.read().await {
            assert_eq!(read_data, data);
            println!("✅ Volatile shared memory test passed!");
        } else {
            panic!("Failed to read data");
        }
        
        // Cleanup
        unsafe {
            let shm_name = std::ffi::CString::new(format!("/{}", path.trim_start_matches('/'))).unwrap();
            libc::shm_unlink(shm_name.as_ptr());
        }
    }
}
