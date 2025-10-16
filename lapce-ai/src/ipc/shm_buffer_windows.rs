/// Windows shared memory ring buffer with kernel synchronization
/// Uses Windows Events for notifications and Mutexes for atomics

use anyhow::{Result, bail};
use std::sync::Arc;
use std::ptr;

#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, OpenFileMappingW,
    FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};
#[cfg(windows)]
use windows::core::PCWSTR;

#[cfg(windows)]
use crate::ipc::windows_event::WindowsEvent;
#[cfg(windows)]
use crate::ipc::windows_sync::WindowsMutex;

/// Ring buffer header with Windows Mutex-synchronized positions
#[repr(C)]
pub struct WindowsRingHeader {
    pub read_pos: u32,
    pub write_pos: u32,
    pub capacity: u32,
    pub max_message_len: u32,
}

impl WindowsRingHeader {
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

/// Shared memory ring buffer using Windows kernel objects
pub struct WindowsSharedMemoryBuffer {
    header: *mut WindowsRingHeader,
    data: *mut u8,
    mapping_name: String,
    mapping_handle: HANDLE,
    total_size: usize,
    doorbell: Option<Arc<WindowsEvent>>,
    read_mutex: WindowsMutex,
    write_mutex: WindowsMutex,
}

unsafe impl Send for WindowsSharedMemoryBuffer {}
unsafe impl Sync for WindowsSharedMemoryBuffer {}

impl WindowsSharedMemoryBuffer {
    /// Create a new shared memory buffer with Windows synchronization
    #[cfg(windows)]
    pub fn create(name: &str, capacity: u32) -> Result<Self> {
        let header_size = std::mem::size_of::<WindowsRingHeader>();
        let total_size = header_size + capacity as usize;
        
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let mapping_handle = unsafe {
            CreateFileMappingW(
                HANDLE::default(), // Use paging file
                None,
                PAGE_READWRITE,
                0,
                total_size as u32,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        let ptr = unsafe {
            MapViewOfFile(
                mapping_handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                total_size,
            )
        };
        
        if ptr.Value.is_null() {
            unsafe { CloseHandle(mapping_handle)? };
            bail!("MapViewOfFile failed");
        }
        
        let header = ptr.Value as *mut WindowsRingHeader;
        let data = unsafe { (ptr.Value as *mut u8).add(header_size) };
        
        // Initialize header
        unsafe {
            (*header).init(capacity);
        }
        
        // Create Windows Mutexes for synchronization
        let read_mutex = WindowsMutex::create(&format!("{}_read_mtx", name))?;
        let write_mutex = WindowsMutex::create(&format!("{}_write_mtx", name))?;
        
        eprintln!("[BUFFER CREATE WINDOWS] '{}' size={} header@{:p} data@{:p}",
            name, total_size, header, data);
        
        Ok(Self {
            header,
            data,
            mapping_name: name.to_string(),
            mapping_handle,
            total_size,
            doorbell: None,
            read_mutex,
            write_mutex,
        })
    }
    
    /// Open existing shared memory buffer
    #[cfg(windows)]
    pub fn open(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let mapping_handle = unsafe {
            OpenFileMappingW(
                FILE_MAP_ALL_ACCESS.0,
                false,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        let ptr = unsafe {
            MapViewOfFile(
                mapping_handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                0, // Map entire object
            )
        };
        
        if ptr.Value.is_null() {
            unsafe { CloseHandle(mapping_handle)? };
            bail!("MapViewOfFile failed");
        }
        
        let header_size = std::mem::size_of::<WindowsRingHeader>();
        let header = ptr.Value as *mut WindowsRingHeader;
        let data = unsafe { (ptr.Value as *mut u8).add(header_size) };
        
        let capacity = unsafe { (*header).capacity() };
        let total_size = header_size + capacity as usize;
        
        // Open existing Windows Mutexes
        let read_mutex = WindowsMutex::open(&format!("{}_read_mtx", name))?;
        let write_mutex = WindowsMutex::open(&format!("{}_write_mtx", name))?;
        
        eprintln!("[BUFFER OPEN WINDOWS] '{}' size={} header@{:p} data@{:p}",
            name, total_size, header, data);
        
        Ok(Self {
            header,
            data,
            mapping_name: name.to_string(),
            mapping_handle,
            total_size,
            doorbell: None,
            read_mutex,
            write_mutex,
        })
    }
    
    /// Attach a Windows Event doorbell for notifications
    pub fn attach_doorbell(&mut self, doorbell: Arc<WindowsEvent>) {
        eprintln!("[BUF WINDOWS] Attached event doorbell to {}", self.mapping_name);
        self.doorbell = Some(doorbell);
    }
    
    /// Attach a Windows Event doorbell by name (for client use)
    pub fn attach_doorbell_name(&mut self, event_name: &str) -> Result<()> {
        let doorbell = WindowsEvent::open(event_name)?;
        eprintln!("[BUF WINDOWS] Attached event doorbell '{}' to {}", event_name, self.mapping_name);
        self.doorbell = Some(Arc::new(doorbell));
        Ok(())
    }
    
    /// Ring the doorbell to notify reader
    fn ring_doorbell(&self) {
        if let Some(ref doorbell) = self.doorbell {
            let _ = doorbell.ring();
        }
    }
    
    /// Wait on doorbell with timeout
    pub fn wait_doorbell(&self, timeout_ms: i32) -> Result<bool> {
        if let Some(ref doorbell) = self.doorbell {
            doorbell.wait_timeout(timeout_ms)
        } else {
            Ok(false)
        }
    }
    
    /// Write data to ring buffer using Windows Mutex synchronization
    #[cfg(windows)]
    pub fn write(&self, data: &[u8]) -> Result<()> {
        let header = unsafe { &*self.header };
        
        // Lock and read positions
        self.write_mutex.lock()?;
        let write_pos = unsafe { ptr::read_volatile(&header.write_pos) };
        self.write_mutex.unlock()?;
        
        self.read_mutex.lock()?;
        let read_pos = unsafe { ptr::read_volatile(&header.read_pos) };
        self.read_mutex.unlock()?;
        
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
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.data.add(write_pos as usize),
                    len,
                );
            }
        } else {
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
        self.write_mutex.lock()?;
        let new_write_pos = end_pos % capacity;
        unsafe { ptr::write_volatile(&(*self.header).write_pos, new_write_pos) };
        self.write_mutex.unlock()?;
        
        // Ring doorbell
        self.ring_doorbell();
        
        Ok(())
    }
    
    /// Read data from ring buffer using Windows Mutex synchronization
    #[cfg(windows)]
    pub fn read(&self, buf: &mut Vec<u8>, max_len: usize) -> Result<usize> {
        let header = unsafe { &*self.header };
        
        // Lock and read positions
        self.read_mutex.lock()?;
        let read_pos = unsafe { ptr::read_volatile(&header.read_pos) };
        self.read_mutex.unlock()?;
        
        self.write_mutex.lock()?;
        let write_pos = unsafe { ptr::read_volatile(&header.write_pos) };
        self.write_mutex.unlock()?;
        
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
            unsafe {
                buf.set_len(to_read);
                ptr::copy_nonoverlapping(
                    self.data.add(read_pos as usize),
                    buf.as_mut_ptr(),
                    to_read,
                );
            }
        } else {
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
        self.read_mutex.lock()?;
        let new_read_pos = end_pos % capacity;
        unsafe { ptr::write_volatile(&(*self.header).read_pos, new_read_pos) };
        self.read_mutex.unlock()?;
        
        Ok(to_read)
    }
    
    pub fn header(&self) -> &WindowsRingHeader {
        unsafe { &*self.header }
    }
}

impl Drop for WindowsSharedMemoryBuffer {
    #[cfg(windows)]
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(windows::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.header as *mut _,
            });
            let _ = CloseHandle(self.mapping_handle);
        }
    }
}

#[cfg(not(windows))]
impl WindowsSharedMemoryBuffer {
    pub fn create(_name: &str, _capacity: u32) -> Result<Self> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn open(_name: &str) -> Result<Self> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn attach_doorbell(&mut self, _doorbell: Arc<WindowsEvent>) {}
    
    pub fn attach_doorbell_name(&mut self, _event_name: &str) -> Result<()> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn wait_doorbell(&self, _timeout_ms: i32) -> Result<bool> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn write(&self, _data: &[u8]) -> Result<()> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn read(&self, _buf: &mut Vec<u8>, _max_len: usize) -> Result<usize> {
        bail!("WindowsSharedMemoryBuffer only available on Windows")
    }
    
    pub fn header(&self) -> &WindowsRingHeader {
        unimplemented!("WindowsSharedMemoryBuffer only available on Windows")
    }
}
