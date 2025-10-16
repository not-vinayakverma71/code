/// Futex-based shared memory ring buffer for cross-process IPC
/// Uses Linux futex for proper cache-coherent atomics between processes

use std::os::unix::io::RawFd;
use std::ptr;
use std::sync::{Arc, Mutex};
use anyhow::{Result, bail};

#[cfg(target_os = "linux")]
use crate::ipc::futex::{atomic_load, atomic_store, atomic_cas, futex_wait, futex_wake};
use crate::ipc::eventfd_doorbell::EventFdDoorbell;

/// Ring buffer header with futex-synchronized positions
#[repr(C)]
pub struct FutexRingHeader {
    pub read_pos: u32,
    pub write_pos: u32,
    pub capacity: u32,
    pub max_message_len: u32,
}

impl FutexRingHeader {
    pub fn init(&mut self, capacity: u32) {
        unsafe {
            ptr::write_volatile(&mut self.read_pos, 0);
            ptr::write_volatile(&mut self.write_pos, 0);
            ptr::write_volatile(&mut self.capacity, capacity);
            ptr::write_volatile(&mut self.max_message_len, 64 * 1024);
        }
    }
    
    #[cfg(target_os = "linux")]
    pub fn load_read_pos(&self) -> u32 {
        atomic_load(&self.read_pos as *const u32)
    }
    
    #[cfg(target_os = "linux")]
    pub fn load_write_pos(&self) -> u32 {
        atomic_load(&self.write_pos as *const u32)
    }
    
    #[cfg(target_os = "linux")]
    pub fn store_read_pos(&mut self, val: u32) {
        atomic_store(&mut self.read_pos as *mut u32, val);
        // Wake any threads waiting on read_pos change
        let _ = futex_wake(&self.read_pos as *const u32, 1);
    }
    
    #[cfg(target_os = "linux")]
    pub fn store_write_pos(&mut self, val: u32) {
        atomic_store(&mut self.write_pos as *mut u32, val);
        // Wake any threads waiting on write_pos change
        let _ = futex_wake(&self.write_pos as *const u32, 1);
    }
    
    pub fn capacity(&self) -> u32 {
        unsafe { ptr::read_volatile(&self.capacity) }
    }
    
    pub fn available_read(&self) -> u32 {
        let read = self.load_read_pos();
        let write = self.load_write_pos();
        let cap = self.capacity();
        
        if write >= read {
            write - read
        } else {
            cap - read + write
        }
    }
    
    pub fn available_write(&self) -> u32 {
        let cap = self.capacity();
        let used = self.available_read();
        cap.saturating_sub(used).saturating_sub(1)
    }
}

/// Shared memory ring buffer using futex for synchronization
pub struct FutexSharedMemoryBuffer {
    header: *mut FutexRingHeader,
    data: *mut u8,
    shm_name: String,
    shm_fd: i32,
    shm_size: usize,
    doorbell: Mutex<Option<Arc<EventFdDoorbell>>>,
}

unsafe impl Send for FutexSharedMemoryBuffer {}
unsafe impl Sync for FutexSharedMemoryBuffer {}

impl FutexSharedMemoryBuffer {
    /// Create a new shared memory buffer with futex synchronization
    #[cfg(target_os = "linux")]
    pub fn create(name: &str, capacity: u32) -> Result<Arc<Self>> {
        use std::ffi::CString;
        use libc::{shm_open, ftruncate, mmap, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
        
        let header_size = std::mem::size_of::<FutexRingHeader>();
        let total_size = header_size + capacity as usize;
        
        let c_name = CString::new(name)?;
        let fd = unsafe { shm_open(c_name.as_ptr(), O_CREAT | O_RDWR, 0o600) };
        
        if fd < 0 {
            bail!("shm_open failed");
        }
        
        let truncate_result = unsafe { ftruncate(fd, total_size as i64) };
        if truncate_result < 0 {
            unsafe { libc::close(fd) };
            bail!("ftruncate failed");
        }
        
        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                total_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };
        
        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            bail!("mmap failed");
        }
        
        let header = ptr as *mut FutexRingHeader;
        let data = unsafe { ptr.add(header_size) as *mut u8 };
        
        // Initialize header
        unsafe {
            (*header).init(capacity);
        }
        
        eprintln!("[BUFFER CREATE] '{}' fd={} size={} header@{:p} data@{:p}",
            name, fd, total_size, header, data);
        
        Ok(Arc::new(Self {
            header,
            data,
            shm_name: name.to_string(),
            shm_fd: fd,
            shm_size: total_size,
            doorbell: Mutex::new(None),
        }))
    }
    
    /// Open existing shared memory buffer
    #[cfg(target_os = "linux")]
    pub fn open(name: &str, _capacity: u32) -> Result<Arc<Self>> {
        use std::ffi::CString;
        use libc::{shm_open, mmap, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
        
        let c_name = CString::new(name)?;
        let fd = unsafe { shm_open(c_name.as_ptr(), O_RDWR, 0o600) };
        
        if fd < 0 {
            bail!("shm_open failed for {}", name);
        }
        
        // Get size
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut stat) } < 0 {
            unsafe { libc::close(fd) };
            bail!("fstat failed");
        }
        
        let total_size = stat.st_size as usize;
        
        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                total_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };
        
        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            bail!("mmap failed");
        }
        
        let header_size = std::mem::size_of::<FutexRingHeader>();
        let header = ptr as *mut FutexRingHeader;
        let data = unsafe { ptr.add(header_size) as *mut u8 };
        
        eprintln!("[BUFFER OPEN] '{}' fd={} size={} header@{:p} data@{:p}",
            name, fd, total_size, header, data);
        
        Ok(Arc::new(Self {
            header,
            data,
            shm_name: name.to_string(),
            shm_fd: fd,
            shm_size: total_size,
            doorbell: Mutex::new(None),
        }))
    }
    
    /// Attach an eventfd doorbell for notifications
    pub fn attach_doorbell(&self, doorbell: Arc<EventFdDoorbell>) {
        eprintln!("[BUF] Attached doorbell fd={} to {}", doorbell.as_raw_fd(), self.shm_name);
        *self.doorbell.lock().unwrap() = Some(doorbell);
    }
    
    /// Attach an eventfd doorbell from raw fd (for client use)
    pub fn attach_doorbell_fd(&self, fd: i32) {
        let doorbell = EventFdDoorbell::from_fd(fd);
        eprintln!("[BUF] Attached doorbell fd={} to {}", fd, self.shm_name);
        *self.doorbell.lock().unwrap() = Some(Arc::new(doorbell));
    }
    
    /// Ring the doorbell (notify reader)
    fn ring_doorbell(&self) {
        if let Some(ref doorbell) = *self.doorbell.lock().unwrap() {
            let _ = doorbell.ring();
        }
    }
    
    /// Wait on doorbell with timeout
    pub fn wait_doorbell(&self, timeout_ms: i32) -> Result<bool> {
        if let Some(ref doorbell) = *self.doorbell.lock().unwrap() {
            doorbell.wait_timeout(timeout_ms)
        } else {
            Ok(false)
        }
    }
    
    /// Write data to ring buffer using futex synchronization
    #[cfg(target_os = "linux")]
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let header = unsafe { &mut *self.header };
        
        let read_pos = header.load_read_pos();
        let write_pos = header.load_write_pos();
        let available = header.available_write();
        let capacity = header.capacity();
        
        if data.len() as u32 > available {
            bail!("Not enough space: need {}, have {}", data.len(), available);
        }
        
        // Write data
        let data_ptr = self.data;
        let len = data.len();
        
        // Handle wrap-around
        let end_pos = write_pos + len as u32;
        
        if end_pos <= capacity {
            // No wrap
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    data_ptr.add(write_pos as usize),
                    len,
                );
            }
        } else {
            // Wrap around
            let first_chunk = (capacity - write_pos) as usize;
            let second_chunk = len - first_chunk;
            
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    data_ptr.add(write_pos as usize),
                    first_chunk,
                );
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_chunk),
                    data_ptr,
                    second_chunk,
                );
            }
        }
        
        // Update write position with futex wake
        let new_write_pos = end_pos % capacity;
        header.store_write_pos(new_write_pos);
        
        // Ring doorbell to notify reader
        self.ring_doorbell();
        
        Ok(())
    }
    
    /// Read data from ring buffer using futex synchronization
    #[cfg(target_os = "linux")]
    pub fn read(&self, buf: &mut Vec<u8>, max_len: usize) -> Result<usize> {
        let header = unsafe { &mut *self.header };
        
        let read_pos = header.load_read_pos();
        let write_pos = header.load_write_pos();
        let available = header.available_read();
        
        if available == 0 {
            return Ok(0);
        }
        
        let to_read = available.min(max_len as u32) as usize;
        let capacity = header.capacity();
        
        // Read data
        buf.clear();
        buf.reserve(to_read);
        
        let end_pos = read_pos + to_read as u32;
        
        if end_pos <= capacity {
            // No wrap
            unsafe {
                buf.set_len(to_read);
                ptr::copy_nonoverlapping(
                    self.data.add(read_pos as usize),
                    buf.as_mut_ptr(),
                    to_read,
                );
            }
        } else {
            // Wrap around
            let first_chunk = (capacity - read_pos) as usize;
            let second_chunk = to_read - first_chunk;
            
            unsafe {
                buf.set_len(to_read);
                ptr::copy_nonoverlapping(
                    self.data.add(read_pos as usize),
                    buf.as_mut_ptr(),
                    first_chunk,
                );
                ptr::copy_nonoverlapping(
                    self.data,
                    buf.as_mut_ptr().add(first_chunk),
                    second_chunk,
                );
            }
        }
        
        // Update read position with futex wake
        let new_read_pos = end_pos % capacity;
        header.store_read_pos(new_read_pos);
        
        Ok(to_read)
    }
    
    pub fn header(&self) -> &FutexRingHeader {
        unsafe { &*self.header }
    }
}

impl Drop for FutexSharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.header as *mut libc::c_void, self.shm_size);
            libc::close(self.shm_fd);
        }
    }
}
