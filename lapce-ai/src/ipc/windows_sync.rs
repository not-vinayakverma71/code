/// Windows Mutex/Semaphore-based synchronization
/// Replaces futex (Linux) and POSIX semaphores (macOS) with Windows kernel objects

use anyhow::{Result, bail};
use std::ptr;

#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    CreateMutexW, ReleaseMutex, WaitForSingleObject, OpenMutexW,
    CreateSemaphoreW, ReleaseSemaphore, OpenSemaphoreW,
    SYNCHRONIZATION_SYNCHRONIZE, INFINITE,
};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Windows Mutex wrapper for cross-process synchronization
pub struct WindowsMutex {
    handle: HANDLE,
    name: String,
    owned: bool,
}

unsafe impl Send for WindowsMutex {}
unsafe impl Sync for WindowsMutex {}

impl WindowsMutex {
    /// Create a new named mutex
    #[cfg(windows)]
    pub fn create(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            CreateMutexW(
                None,
                false, // Not initially owned
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_MUTEX] Created mutex: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: true,
        })
    }
    
    /// Open existing named mutex
    #[cfg(windows)]
    pub fn open(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            OpenMutexW(
                SYNCHRONIZATION_SYNCHRONIZE.0,
                false,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_MUTEX] Opened mutex: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: false,
        })
    }
    
    /// Acquire the mutex
    #[cfg(windows)]
    pub fn lock(&self) -> Result<()> {
        let result = unsafe { WaitForSingleObject(self.handle, INFINITE) };
        
        if result != WAIT_OBJECT_0 {
            bail!("Failed to acquire mutex");
        }
        
        Ok(())
    }
    
    /// Release the mutex
    #[cfg(windows)]
    pub fn unlock(&self) -> Result<()> {
        unsafe {
            ReleaseMutex(self.handle)?;
        }
        Ok(())
    }
}

impl Drop for WindowsMutex {
    #[cfg(windows)]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
        eprintln!("[WIN_MUTEX] Closed mutex: {}", self.name);
    }
}

/// Windows Semaphore wrapper
pub struct WindowsSemaphore {
    handle: HANDLE,
    name: String,
    owned: bool,
}

unsafe impl Send for WindowsSemaphore {}
unsafe impl Sync for WindowsSemaphore {}

impl WindowsSemaphore {
    /// Create a new named semaphore
    #[cfg(windows)]
    pub fn create(name: &str, initial_count: i32, max_count: i32) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            CreateSemaphoreW(
                None,
                initial_count,
                max_count,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_SEM] Created semaphore: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: true,
        })
    }
    
    /// Open existing named semaphore
    #[cfg(windows)]
    pub fn open(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            OpenSemaphoreW(
                SYNCHRONIZATION_SYNCHRONIZE.0,
                false,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_SEM] Opened semaphore: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: false,
        })
    }
    
    /// Wait (decrement) the semaphore
    #[cfg(windows)]
    pub fn wait(&self) -> Result<()> {
        let result = unsafe { WaitForSingleObject(self.handle, INFINITE) };
        
        if result != WAIT_OBJECT_0 {
            bail!("Failed to wait on semaphore");
        }
        
        Ok(())
    }
    
    /// Wait with timeout
    #[cfg(windows)]
    pub fn wait_timeout(&self, timeout_ms: i32) -> Result<bool> {
        let timeout = if timeout_ms < 0 {
            INFINITE
        } else {
            timeout_ms as u32
        };
        
        let result = unsafe { WaitForSingleObject(self.handle, timeout) };
        
        match result {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => bail!("WaitForSingleObject failed"),
        }
    }
    
    /// Post (increment) the semaphore
    #[cfg(windows)]
    pub fn post(&self) -> Result<()> {
        unsafe {
            ReleaseSemaphore(self.handle, 1, None)?;
        }
        Ok(())
    }
}

impl Drop for WindowsSemaphore {
    #[cfg(windows)]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
        eprintln!("[WIN_SEM] Closed semaphore: {}", self.name);
    }
}

/// Atomic operations using Windows Mutex for synchronization
pub struct WindowsAtomicU32 {
    addr: *mut u32,
    mutex: WindowsMutex,
}

unsafe impl Send for WindowsAtomicU32 {}
unsafe impl Sync for WindowsAtomicU32 {}

impl WindowsAtomicU32 {
    /// Create atomic with mutex protection
    #[cfg(windows)]
    pub fn new(addr: *mut u32, mutex_name: &str) -> Result<Self> {
        let mutex = WindowsMutex::create(mutex_name)?;
        Ok(Self { addr, mutex })
    }
    
    /// Open existing atomic with mutex
    #[cfg(windows)]
    pub fn open(addr: *mut u32, mutex_name: &str) -> Result<Self> {
        let mutex = WindowsMutex::open(mutex_name)?;
        Ok(Self { addr, mutex })
    }
    
    /// Load value with acquire semantics
    #[cfg(windows)]
    pub fn load(&self) -> Result<u32> {
        self.mutex.lock()?;
        let val = unsafe { ptr::read_volatile(self.addr) };
        self.mutex.unlock()?;
        Ok(val)
    }
    
    /// Store value with release semantics
    #[cfg(windows)]
    pub fn store(&self, val: u32) -> Result<()> {
        self.mutex.lock()?;
        unsafe { ptr::write_volatile(self.addr, val) };
        self.mutex.unlock()?;
        Ok(())
    }
    
    /// Compare and swap
    #[cfg(windows)]
    pub fn compare_exchange(&self, expected: u32, new: u32) -> Result<Result<u32, u32>> {
        self.mutex.lock()?;
        let current = unsafe { ptr::read_volatile(self.addr) };
        
        if current == expected {
            unsafe { ptr::write_volatile(self.addr, new) };
            self.mutex.unlock()?;
            Ok(Ok(expected))
        } else {
            self.mutex.unlock()?;
            Ok(Err(current))
        }
    }
}

#[cfg(not(windows))]
impl WindowsMutex {
    pub fn create(_name: &str) -> Result<Self> {
        bail!("WindowsMutex only available on Windows")
    }
    pub fn open(_name: &str) -> Result<Self> {
        bail!("WindowsMutex only available on Windows")
    }
    pub fn lock(&self) -> Result<()> {
        bail!("WindowsMutex only available on Windows")
    }
    pub fn unlock(&self) -> Result<()> {
        bail!("WindowsMutex only available on Windows")
    }
}

#[cfg(not(windows))]
impl WindowsSemaphore {
    pub fn create(_name: &str, _initial_count: i32, _max_count: i32) -> Result<Self> {
        bail!("WindowsSemaphore only available on Windows")
    }
    pub fn open(_name: &str) -> Result<Self> {
        bail!("WindowsSemaphore only available on Windows")
    }
    pub fn wait(&self) -> Result<()> {
        bail!("WindowsSemaphore only available on Windows")
    }
    pub fn wait_timeout(&self, _timeout_ms: i32) -> Result<bool> {
        bail!("WindowsSemaphore only available on Windows")
    }
    pub fn post(&self) -> Result<()> {
        bail!("WindowsSemaphore only available on Windows")
    }
}
