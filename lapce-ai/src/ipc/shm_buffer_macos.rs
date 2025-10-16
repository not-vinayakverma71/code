/// macOS shared memory ring buffer with POSIX semaphore synchronization
/// Uses kqueue for notifications and POSIX semaphores for atomics

use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::sync::{Arc, Mutex};
use anyhow::{Result, bail};

#[cfg(target_os = "macos")]
use crate::ipc::kqueue_doorbell::KqueueDoorbell;
#[cfg(target_os = "macos")]
use crate::ipc::posix_sem_sync::PosixSemaphore;

/// Ring buffer header with POSIX semaphore-synchronized positions
#[repr(C)]
pub struct MacOsRingHeader {
    pub read_pos: u32,
    pub write_pos: u32,
    pub capacity: u32,
    pub max_message_len: u32,
}

impl MacOsRingHeader {
    pub fn init(&mut self, capacity: u32) {
        unsafe {
            ptr::write_volatile(&mut self.read_pos, 0);
            ptr::write_volatile(&mut self.write_pos, 0);
            ptr::write_volatile(&mut self.capacity, capacity);
            ptr::write_volatile(&mut self.max_message_len, 64 * 1024);
        }
    }
    
    pub fn capacity(&self) -> u32 {
        unsafe { ptr::read_volatile(&self.capacity) }
    }
}

/// Shared memory ring buffer using POSIX semaphores for synchronization
pub struct MacOsSharedMemoryBuffer {
    header: *mut MacOsRingHeader,
    data: *mut u8,
    shm_name: String,
    shm_fd: i32,
    shm_size: usize,
    doorbell: Mutex<Option<Arc<KqueueDoorbell>>>,
    read_sem: PosixSemaphore,
    write_sem: PosixSemaphore,
}

unsafe impl Send for MacOsSharedMemoryBuffer {}
unsafe impl Sync for MacOsSharedMemoryBuffer {}

impl MacOsSharedMemoryBuffer {
    /// Create a new shared memory buffer with POSIX semaphore synchronization
    #[cfg(target_os = "macos")]
    pub fn create(name: &str, capacity: u32) -> Result<Self> {
        use std::ffi::CString;
        use libc::{shm_open, ftruncate, mmap, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
        
        let header_size = std::mem::size_of::<MacOsRingHeader>();
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
        
        let header = ptr as *mut MacOsRingHeader;
        let data = unsafe { ptr.add(header_size) as *mut u8 };
        
        // Initialize header
        unsafe {
            (*header).init(capacity);
        }
        
        // Create POSIX semaphores for read/write position synchronization
        let read_sem = PosixSemaphore::create(&format!("{}_read_sem", name), 1)?;
        let write_sem = PosixSemaphore::create(&format!("{}_write_sem", name), 1)?;
        
        eprintln!("[BUFFER CREATE MACOS] '{}' fd={} size={} header@{:p} data@{:p}",
            name, fd, total_size, header, data);
        
        Ok(Self {
            header,
            data,
            shm_name: name.to_string(),
            shm_fd: fd,
            shm_size: total_size,
            doorbell: Mutex::new(None),
            read_sem,
            write_sem,
        })
    }
    
    /// Open existing shared memory buffer
    #[cfg(target_os = "macos")]
    pub fn open(name: &str) -> Result<Self> {
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
        
        let header_size = std::mem::size_of::<MacOsRingHeader>();
        let header = ptr as *mut MacOsRingHeader;
        let data = unsafe { ptr.add(header_size) as *mut u8 };
        
        // Open existing POSIX semaphores
        let read_sem = PosixSemaphore::open(&format!("{}_read_sem", name))?;
        let write_sem = PosixSemaphore::open(&format!("{}_write_sem", name))?;
        
        eprintln!("[BUFFER OPEN MACOS] '{}' fd={} size={} header@{:p} data@{:p}",
            name, fd, total_size, header, data);
        
        Ok(Self {
            header,
            data,
            shm_name: name.to_string(),
            shm_fd: fd,
            shm_size: total_size,
            doorbell: Mutex::new(None),
            read_sem,
            write_sem,
        })
    }
    
    /// Attach a kqueue doorbell for notifications
    pub fn attach_doorbell(&self, doorbell: Arc<KqueueDoorbell>) {
        eprintln!("[BUF MACOS] Attached kqueue doorbell fd={} to {}", doorbell.as_raw_fd(), self.shm_name);
        *self.doorbell.lock().unwrap() = Some(doorbell);
    }
    
    /// Attach a kqueue doorbell from raw fd (for client use)
    pub fn attach_doorbell_fd(&self, fd: i32) {
        let doorbell = KqueueDoorbell::from_fd(fd).unwrap();
        eprintln!("[BUF MACOS] Attached kqueue doorbell fd={} to {}", fd, self.shm_name);
        *self.doorbell.lock().unwrap() = Some(Arc::new(doorbell));
    }
    
    /// Ring the doorbell to notify reader
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
    
    /// Write data to ring buffer using POSIX semaphore synchronization
    #[cfg(target_os = "macos")]
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let header = unsafe { &*self.header };
        
        // Lock write position
        self.write_sem.wait()?;
        let write_pos = unsafe { ptr::read_volatile(&header.write_pos) };
        self.write_sem.post()?;
        
        // Lock read position to check available space
        self.read_sem.wait()?;
        let read_pos = unsafe { ptr::read_volatile(&header.read_pos) };
        self.read_sem.post()?;
        
        let capacity = header.capacity();
        
        // Calculate available space
        let available = if write_pos >= read_pos {
            capacity - (write_pos - read_pos) - 1
        } else {
            read_pos - write_pos - 1
        };
        
        if data.len() as u32 > available {
            bail!("Not enough space: need {}, have {}", data.len(), available);
        }
        
        // Write data
        let len = data.len();
        let end_pos = write_pos + len as u32;
        
        if end_pos <= capacity {
            // No wrap
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.data.add(write_pos as usize),
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
                    self.data.add(write_pos as usize),
                    first_chunk,
                );
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_chunk),
                    self.data,
                    second_chunk,
                );
            }
        }
        
        // Update write position
        self.write_sem.wait()?;
        let new_write_pos = end_pos % capacity;
        unsafe { ptr::write_volatile(&mut (*self.header).write_pos as *mut u32, new_write_pos) };
        self.write_sem.post()?;
        
        // Ring doorbell to notify reader
        self.ring_doorbell();
        
        Ok(())
    }
    
    /// Read data from ring buffer using POSIX semaphore synchronization
    #[cfg(target_os = "macos")]
    pub fn read(&self, buf: &mut Vec<u8>, max_len: usize) -> Result<usize> {
        let header = unsafe { &*self.header };
        
        // Lock positions to read available data
        self.read_sem.wait()?;
        let read_pos = unsafe { ptr::read_volatile(&header.read_pos) };
        self.read_sem.post()?;
        
        self.write_sem.wait()?;
        let write_pos = unsafe { ptr::read_volatile(&header.write_pos) };
        self.write_sem.post()?;
        
        // Calculate available data
        let available = if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            header.capacity() - read_pos + write_pos
        };
        
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
        
        // Update read position
        self.read_sem.wait()?;
        let new_read_pos = end_pos % capacity;
        unsafe { ptr::write_volatile(&mut (*self.header).read_pos as *mut u32, new_read_pos) };
        self.read_sem.post()?;
        
        Ok(to_read)
    }
    
    pub fn header(&self) -> &MacOsRingHeader {
        unsafe { &*self.header }
    }
}

impl Drop for MacOsSharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.header as *mut libc::c_void, self.shm_size);
            libc::close(self.shm_fd);
        }
    }
}
