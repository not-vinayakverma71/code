/// Cross-process safe shared memory buffer using volatile ring header
/// Replaces shared_memory_complete.rs atomic-based approach

use std::sync::Arc;
use std::ptr;
use anyhow::{Result, bail};
use crate::ipc::ring_header_volatile::VolatileRingHeader;
use crate::ipc::eventfd_doorbell::EventFdDoorbell;
use std::os::unix::io::RawFd;
use crate::ipc::shm_namespace::create_namespaced_path;
use std::sync::{Arc, Mutex};

const SLOT_SIZE: usize = 1024;
const NUM_SLOTS: usize = 1024;

/// Shared memory buffer with volatile header
pub struct VolatileSharedMemoryBuffer {
    ptr: *mut u8,
    size: usize,
    fd: i32,
    shm_name: String,
    header: *mut VolatileRingHeader,
    data: *mut u8,
    // Optional doorbell for efficient notifications
    doorbell: Mutex<Option<Arc<EventFdDoorbell>>>,
}

impl VolatileSharedMemoryBuffer {
    /// Create new shared memory buffer
    pub fn create(path: &str, capacity: u32) -> Result<Arc<Self>> {
        let data_size = capacity as usize;
        let header_size = std::mem::size_of::<VolatileRingHeader>();
        let total_size = header_size + data_size;
        
        #[cfg(not(target_os = "macos"))]
        let namespaced_path = create_namespaced_path(path);
        
        #[cfg(target_os = "macos")]
        let namespaced_path = path.to_string();
        
        // Create shm name with macOS length handling
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            let full_name = format!("/{}", without_leading.replace('/', "_"));
            
            #[cfg(target_os = "macos")]
            {
                if full_name.len() > 31 {
                    let chars: Vec<char> = full_name.chars().collect();
                    let prefix: String = chars.iter().take(15).collect();
                    let suffix: String = chars.iter().skip(chars.len().saturating_sub(15)).collect();
                    format!("{}_", prefix) + suffix.as_str()
                } else {
                    full_name
                }
            }
            
            #[cfg(not(target_os = "macos"))]
            full_name
        };
        
        let shm_name = std::ffi::CString::new(shm_name_str.clone())?;
        
        let fd = unsafe {
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o600,
            );
            
            if fd < 0 {
                bail!("shm_open failed for '{}': {}", shm_name_str, std::io::Error::last_os_error());
            }
            
            // Set size
            if libc::ftruncate(fd, total_size as i64) != 0 {
                libc::close(fd);
                bail!("ftruncate failed");
            }
            
            fd
        };
        
        // mmap with MAP_SHARED for cross-process visibility
        let ptr = unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                total_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,  // CRITICAL: MAP_SHARED for cross-process
                fd,
                0,
            );
            
            if ptr == libc::MAP_FAILED {
                libc::close(fd);
                bail!("mmap failed");
            }
            
            ptr as *mut u8
        };
        
        // Initialize header
        let header = ptr as *mut VolatileRingHeader;
        unsafe {
            std::ptr::write(header, VolatileRingHeader::new(capacity));
        }
        
        let data = unsafe { ptr.add(header_size) };
        
        eprintln!("[BUFFER CREATE] '{}' fd={} size={} header@{:p} data@{:p}", 
            shm_name_str, fd, total_size, header, data);
        
        Ok(Arc::new(Self {
            ptr,
            size: total_size,
            fd,
            shm_name: shm_name_str,
            header,
            data,
            doorbell: Mutex::new(None),
        }))
    }
    
    /// Open existing shared memory buffer
    pub fn open(path: &str, capacity: u32) -> Result<Arc<Self>> {
        let data_size = capacity as usize;
        let header_size = std::mem::size_of::<VolatileRingHeader>();
        let total_size = header_size + data_size;
        
        #[cfg(not(target_os = "macos"))]
        let namespaced_path = create_namespaced_path(path);
        
        #[cfg(target_os = "macos")]
        let namespaced_path = path.to_string();
        
        let shm_name_str = {
            let without_leading = namespaced_path.trim_start_matches('/');
            let full_name = format!("/{}", without_leading.replace('/', "_"));
            
            #[cfg(target_os = "macos")]
            {
                if full_name.len() > 31 {
                    let chars: Vec<char> = full_name.chars().collect();
                    let prefix: String = chars.iter().take(15).collect();
                    let suffix: String = chars.iter().skip(chars.len().saturating_sub(15)).collect();
                    format!("{}_", prefix) + suffix.as_str()
                } else {
                    full_name
                }
            }
            
            #[cfg(not(target_os = "macos"))]
            full_name
        };
        
        let shm_name = std::ffi::CString::new(shm_name_str.clone())?;
        
        let fd = unsafe {
            let fd = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_RDWR,
                0o600,
            );
            
            if fd < 0 {
                bail!("shm_open failed for '{}': {}", shm_name_str, std::io::Error::last_os_error());
            }
            
            fd
        };
        
        let ptr = unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                total_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            
            if ptr == libc::MAP_FAILED {
                libc::close(fd);
                bail!("mmap failed");
            }
            
            ptr as *mut u8
        };
        
        let header = ptr as *mut VolatileRingHeader;
        let data = unsafe { ptr.add(header_size) };
        
        eprintln!("[BUFFER OPEN] '{}' fd={} size={} header@{:p} data@{:p}", 
            shm_name_str, fd, total_size, header, data);
        
        Ok(Arc::new(Self {
            ptr,
            size: total_size,
            fd,
            shm_name: shm_name_str,
            header,
            data,
            doorbell: Mutex::new(None),
        }))
    }
    
    /// Write data to ring buffer
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let header = unsafe { &*self.header };
        
        let read_pos = header.load_read_pos();
        let write_pos = header.load_write_pos();
        let available = header.available_write();
        let capacity = header.capacity();
        
        // Verbose logging disabled for performance
        // eprintln!("[BUF WRITE {}] BEFORE: read={} write={} avail_write={} capacity={} data_len={}",
        //     &self.shm_name[self.shm_name.len().saturating_sub(20)..],
        //     read_pos, write_pos, available, capacity, data.len());
        
        if data.len() as u32 > available {
            bail!("Not enough space: need {}, have {}", data.len(), available);
        }
        
        // Write data
        let data_ptr = self.data;
        let len = data.len();
        
        // Handle wrap-around
        let end_pos = write_pos + len as u32;
        if end_pos <= capacity {
            // Contiguous write
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    data_ptr.add(write_pos as usize),
                    len,
                );
            }
        } else {
            // Wrapped write
            let first_chunk = (capacity - write_pos) as usize;
            let second_chunk = len - first_chunk;
            
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    data_ptr.add(write_pos as usize),
                    first_chunk,
                );
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_chunk),
                    data_ptr,
                    second_chunk,
                );
            }
        }
        
        // Update write position with Release fence
        let new_write_pos = end_pos % capacity;
        header.store_write_pos(new_write_pos);
        
        // eprintln!("[BUF WRITE {}] AFTER: new_write={} bytes_written={}",
        //     &self.shm_name[self.shm_name.len().saturating_sub(20)..],
        //     new_write_pos, data.len());
        
        // Ring doorbell to notify reader
        self.ring_doorbell();
        
        Ok(())
    }
    
    /// Read data from ring buffer
    pub fn read(&self, buf: &mut Vec<u8>, max_len: usize) -> Result<usize> {
        let header = unsafe { &*self.header };
        
        let read_pos = header.load_read_pos();
        let write_pos = header.load_write_pos_acquire();
        let available = header.available_read();
        let capacity = header.capacity();
        
        // Verbose logging disabled for performance
        // eprintln!("[BUF READ {}] BEFORE: read={} write={} avail_read={} capacity={} max_len={}",
        //     &self.shm_name[self.shm_name.len().saturating_sub(20)..],
        //     read_pos, write_pos, available, capacity, max_len);
        
        if available == 0 {
            return Ok(0);
        }
        
        let to_read = available.min(max_len as u32) as usize;
        let capacity = header.capacity();
        let read_pos = header.load_read_pos();
        
        // Read data
        buf.clear();
        buf.reserve(to_read);
        
        let data_ptr = self.data;
        
        // Handle wrap-around
        let end_pos = read_pos + to_read as u32;
        if end_pos <= capacity {
            // Contiguous read
            unsafe {
                buf.set_len(to_read);
                std::ptr::copy_nonoverlapping(
                    data_ptr.add(read_pos as usize),
                    buf.as_mut_ptr(),
                    to_read,
                );
            }
        } else {
            // Wrapped read
            let first_chunk = (capacity - read_pos) as usize;
            let second_chunk = to_read - first_chunk;
            
            unsafe {
                buf.set_len(to_read);
                std::ptr::copy_nonoverlapping(
                    data_ptr.add(read_pos as usize),
                    buf.as_mut_ptr(),
                    first_chunk,
                );
                std::ptr::copy_nonoverlapping(
                    data_ptr,
                    buf.as_mut_ptr().add(first_chunk),
                    second_chunk,
                );
            }
        }
        
        // Update read position with Release fence
        let new_read_pos = end_pos % capacity;
        header.store_read_pos(new_read_pos);
        
        // eprintln!("[BUF READ {}] AFTER: new_read={} bytes_read={}",
        //     &self.shm_name[self.shm_name.len().saturating_sub(20)..],
        //     new_read_pos, to_read);
        
        Ok(to_read)
    }
    
    /// Get header (for inspection)
    pub fn header(&self) -> &VolatileRingHeader {
        unsafe { &*self.header }
    }
    
    /// Attach an eventfd doorbell for notifications
    pub fn attach_doorbell(&self, doorbell: Arc<EventFdDoorbell>) {
        eprintln!("[BUF] Attached doorbell to {}", self.shm_name);
        *self.doorbell.lock().unwrap() = Some(doorbell);
    }
    
    /// Attach doorbell from raw fd
    pub fn attach_doorbell_fd(&self, fd: RawFd) {
        let doorbell = EventFdDoorbell::from_fd(fd);
        *self.doorbell.lock().unwrap() = Some(Arc::new(doorbell));
        eprintln!("[BUF] Attached doorbell fd={} to {}", fd, self.shm_name);
    }
    
    /// Ring the doorbell (notify reader)
    fn ring_doorbell(&self) {
        if let Some(ref doorbell) = *self.doorbell.lock().unwrap() {
            let _ = doorbell.ring();
        }
    }
    
    /// Wait for doorbell notification
    pub fn wait_doorbell(&self, timeout_ms: i32) -> Result<bool> {
        if let Some(ref doorbell) = *self.doorbell.lock().unwrap() {
            doorbell.wait_timeout(timeout_ms)
        } else {
            // No doorbell, just return false (timeout)
            Ok(false)
        }
    }
}

unsafe impl Send for VolatileSharedMemoryBuffer {}
unsafe impl Sync for VolatileSharedMemoryBuffer {}

impl Drop for VolatileSharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
            libc::close(self.fd);
            
            // Unlink on drop (last owner cleans up)
            let shm_name = std::ffi::CString::new(self.shm_name.clone()).unwrap();
            libc::shm_unlink(shm_name.as_ptr());
        }
    }
}
