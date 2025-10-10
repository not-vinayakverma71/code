use std::sync::atomic::{AtomicUsize, Ordering};
use std::ffi::CString;
use std::ptr;

#[cfg(target_os = "macos")]
use anyhow::{Result, anyhow};
#[cfg(target_os = "macos")]
use libc::{
    shm_open, shm_unlink, mmap, munmap, ftruncate, close,
    O_CREAT, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED
};

/// Header structure for shared memory buffer
#[repr(C)]
struct BufferHeader {
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    size: usize,
    version: u32,
    lock: AtomicUsize,  // Simple spinlock for macOS
}

#[cfg(target_os = "macos")]
pub struct MacOSSharedMemory {
    fd: i32,
    ptr: *mut u8,
    size: usize,
    name: String,
    header: *mut BufferHeader,
}

#[cfg(target_os = "macos")]
unsafe impl Send for MacOSSharedMemory {}
#[cfg(target_os = "macos")]
unsafe impl Sync for MacOSSharedMemory {}

#[cfg(target_os = "macos")]
impl MacOSSharedMemory {
    /// Create or open a POSIX shared memory region
    pub fn create(name: &str, size: usize) -> Result<Self> {
        unsafe {
            // Format name for POSIX shared memory (must start with /)
            let shm_name = CString::new(format!("/{}", name))?;
            
            // Add header size to total allocation
            let total_size = size + std::mem::size_of::<BufferHeader>();
            
            // Create/open shared memory object
            let fd = shm_open(
                shm_name.as_ptr(),
                O_CREAT | O_RDWR,
                0o600,
            );
            
            if fd < 0 {
                return Err(anyhow!("Failed to create shared memory: {}", 
                    std::io::Error::last_os_error()));
            }
            
            // Set size of shared memory
            if ftruncate(fd, total_size as i64) < 0 {
                close(fd);
                return Err(anyhow!("Failed to set size: {}", 
                    std::io::Error::last_os_error()));
            }
            
            // Map into process address space
            let ptr = mmap(
                ptr::null_mut(),
                total_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            ) as *mut u8;
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                close(fd);
                return Err(anyhow!("Failed to map memory: {}", 
                    std::io::Error::last_os_error()));
            }
            
            // Initialize header
            let header = ptr as *mut BufferHeader;
            if (*header).version == 0 {
                // First time initialization
                (*header).write_pos = AtomicUsize::new(0);
                (*header).read_pos = AtomicUsize::new(0);
                (*header).size = size;
                (*header).version = 1;
                (*header).lock = AtomicUsize::new(0);
            }
            
            Ok(Self {
                fd,
                ptr,
                size: total_size,
                name: name.to_string(),
                header,
            })
        }
    }
    
    /// Acquire spinlock for synchronization
    fn acquire_lock(&self) {
        unsafe {
            let lock = &(*self.header).lock;
            while lock.compare_exchange_weak(
                0, 1,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_err() {
                std::hint::spin_loop();
            }
        }
    }
    
    /// Release spinlock
    fn release_lock(&self) {
        unsafe {
            (*self.header).lock.store(0, Ordering::Release);
        }
    }
    
    /// Write data to the shared memory buffer
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        unsafe {
            self.acquire_lock();
            let result = self.write_internal(data);
            self.release_lock();
            result
        }
    }
    
    fn write_internal(&mut self, data: &[u8]) -> Result<()> {
        unsafe {
            let header = &*self.header;
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            
            let write_pos = header.write_pos.load(Ordering::Relaxed);
            let data_len = data.len();
            
            if data_len + 4 > header.size {
                return Err(anyhow!("Data too large for buffer"));
            }
            
            // Calculate positions with wrapping
            let mut pos = write_pos;
            
            // Write length prefix
            let len_bytes = (data_len as u32).to_le_bytes();
            for &byte in &len_bytes {
                *buffer_start.add(pos) = byte;
                pos = (pos + 1) % header.size;
            }
            
            // Write data
            for &byte in data {
                *buffer_start.add(pos) = byte;
                pos = (pos + 1) % header.size;
            }
            
            // Update write position
            header.write_pos.store(pos, Ordering::Release);
            
            Ok(())
        }
    }
    
    /// Read data from the shared memory buffer
    pub fn read(&mut self) -> Result<Option<Vec<u8>>> {
        unsafe {
            self.acquire_lock();
            let result = self.read_internal();
            self.release_lock();
            result
        }
    }
    
    fn read_internal(&mut self) -> Result<Option<Vec<u8>>> {
        unsafe {
            let header = &*self.header;
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            
            let read_pos = header.read_pos.load(Ordering::Relaxed);
            let write_pos = header.write_pos.load(Ordering::Relaxed);
            
            if read_pos == write_pos {
                return Ok(None); // No data available
            }
            
            let mut pos = read_pos;
            
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            for i in 0..4 {
                len_bytes[i] = *buffer_start.add(pos);
                pos = (pos + 1) % header.size;
            }
            let data_len = u32::from_le_bytes(len_bytes) as usize;
            
            if data_len > header.size {
                return Err(anyhow!("Invalid data length"));
            }
            
            // Read data
            let mut data = Vec::with_capacity(data_len);
            for _ in 0..data_len {
                data.push(*buffer_start.add(pos));
                pos = (pos + 1) % header.size;
            }
            
            // Update read position
            header.read_pos.store(pos, Ordering::Release);
            
            Ok(Some(data))
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for MacOSSharedMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                munmap(self.ptr as _, self.size);
            }
            if self.fd >= 0 {
                close(self.fd);
                // Optionally unlink (remove) the shared memory object
                // let shm_name = CString::new(format!("/{}", self.name)).ok();
                // if let Some(name) = shm_name {
                //     shm_unlink(name.as_ptr());
                // }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacSharedMemory;

#[cfg(not(target_os = "macos"))]
impl MacSharedMemory {
    pub fn create(_name: &str, _size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        Err("macOS shared memory only available on macOS".into())
    }
    
    pub fn write(&mut self, _data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        Err("macOS shared memory only available on macOS".into())
    }
    
    pub fn read(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        Err("macOS shared memory only available on macOS".into())
    }
}
