/// Windows Native Shared Memory Implementation
/// Uses CreateFileMapping/MapViewOfFile for high performance IPC

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr;
use std::ffi::OsStr;

// Windows-specific imports
#[cfg(target_os = "windows")]
use anyhow::{Result, anyhow};
#[cfg(target_os = "windows")]
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(target_os = "windows")]
use winapi::um::memoryapi::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile,
    FILE_MAP_ALL_ACCESS,
};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{HANDLE, PAGE_READWRITE};
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

/// Header structure for shared memory buffer
#[repr(C)]
struct BufferHeader {
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    size: usize,
    version: u32,
}

#[cfg(target_os = "windows")]
pub struct WindowsSharedMemory {
    handle: HANDLE,
    ptr: *mut u8,
    size: usize,
    name: String,
    header: *mut BufferHeader,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsSharedMemory {}
#[cfg(target_os = "windows")]
unsafe impl Sync for WindowsSharedMemory {}

#[cfg(target_os = "windows")]
impl WindowsSharedMemory {
    /// Create or open a shared memory region
    pub fn create(name: &str, size: usize) -> Result<Self> {
        unsafe {
            // Convert name to wide string for Windows API
            let wide_name = Self::to_wide_string(&format!("Local\\{}", name));
            
            // Add header size to total allocation
            let total_size = size + std::mem::size_of::<BufferHeader>();
            
            // Create file mapping object
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null_mut(),
                PAGE_READWRITE,
                (total_size >> 32) as DWORD,
                total_size as DWORD,
                wide_name.as_ptr(),
            );
            
            if handle.is_null() {
                return Err(anyhow!("Failed to create file mapping"));
            }
            
            // Map view of file into process address space
            let ptr = MapViewOfFile(
                handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                total_size,
            ) as *mut u8;
            
            if ptr.is_null() {
                CloseHandle(handle);
                return Err(anyhow!("Failed to map view of file"));
            }
            
            // Initialize header
            let header = ptr as *mut BufferHeader;
            if (*header).version == 0 {
                // First time initialization
                (*header).write_pos = AtomicUsize::new(0);
                (*header).read_pos = AtomicUsize::new(0);
                (*header).size = size;
                (*header).version = 1;
            }
            
            Ok(Self {
                handle,
                ptr,
                size: total_size,
                name: name.to_string(),
                header,
            })
        }
    }
    
    /// Write data to the shared memory buffer
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        unsafe {
            let header = &*self.header;
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            
            // Simple ring buffer implementation
            let write_pos = header.write_pos.load(Ordering::Acquire);
            let data_len = data.len();
            
            if data_len > header.size {
                return Err(anyhow!("Data too large for buffer"));
            }
            
            // Write length prefix
            let len_bytes = (data_len as u32).to_le_bytes();
            let new_write_pos = (write_pos + 4 + data_len) % header.size;
            
            // Copy length
            ptr::copy_nonoverlapping(
                len_bytes.as_ptr(),
                buffer_start.add(write_pos),
                4,
            );
            
            // Copy data
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                buffer_start.add(write_pos + 4),
                data_len,
            );
            
            // Update write position
            header.write_pos.store(new_write_pos, Ordering::Release);
            
            Ok(())
        }
    }
    
    /// Read data from the shared memory buffer
    pub fn read(&mut self) -> Result<Option<Vec<u8>>> {
        unsafe {
            let header = &*self.header;
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            
            let read_pos = header.read_pos.load(Ordering::Acquire);
            let write_pos = header.write_pos.load(Ordering::Acquire);
            
            if read_pos == write_pos {
                return Ok(None); // No data available
            }
            
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            ptr::copy_nonoverlapping(
                buffer_start.add(read_pos),
                len_bytes.as_mut_ptr(),
                4,
            );
            let data_len = u32::from_le_bytes(len_bytes) as usize;
            
            if data_len > header.size {
                return Err(anyhow!("Invalid data length"));
            }
            
            // Read data
            let mut data = vec![0u8; data_len];
            ptr::copy_nonoverlapping(
                buffer_start.add(read_pos + 4),
                data.as_mut_ptr(),
                data_len,
            );
            
            // Update read position
            let new_read_pos = (read_pos + 4 + data_len) % header.size;
            header.read_pos.store(new_read_pos, Ordering::Release);
            
            Ok(Some(data))
        }
    }
    
    /// Convert string to wide string for Windows API
    fn to_wide_string(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsSharedMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                UnmapViewOfFile(self.ptr as _);
            }
            if !self.handle.is_null() {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(not(windows))]
pub struct WindowsSharedMemory;

#[cfg(not(windows))]
impl WindowsSharedMemory {
    pub fn create(_name: &str, _size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        Err("Windows shared memory only available on Windows".into())
    }
    
    pub fn write(&mut self, _data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        Err("Windows shared memory only available on Windows".into())
    }
    
    pub fn read(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        Err("Windows shared memory only available on Windows".into())
    }
}
