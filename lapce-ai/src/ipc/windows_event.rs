/// Windows Event-based doorbell for cross-process notifications
/// Replaces Linux eventfd and macOS kqueue with Windows kernel Events

use anyhow::{Result, bail};

#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    CreateEventW, SetEvent, WaitForSingleObject, OpenEventW,
    EVENT_ALL_ACCESS, INFINITE,
};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Windows Event wrapper for cross-process notifications
pub struct WindowsEvent {
    handle: HANDLE,
    name: String,
    owned: bool,
}

unsafe impl Send for WindowsEvent {}
unsafe impl Sync for WindowsEvent {}

impl WindowsEvent {
    /// Create a new named Event
    #[cfg(windows)]
    pub fn create(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            CreateEventW(
                None,
                false, // Auto-reset
                false, // Initially non-signaled
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_EVENT] Created event: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: true,
        })
    }
    
    /// Open existing named Event
    #[cfg(windows)]
    pub fn open(name: &str) -> Result<Self> {
        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        let handle = unsafe {
            OpenEventW(
                EVENT_ALL_ACCESS,
                false,
                PCWSTR(wide_name.as_ptr()),
            )?
        };
        
        eprintln!("[WIN_EVENT] Opened event: {}", name);
        Ok(Self {
            handle,
            name: name.to_string(),
            owned: false,
        })
    }
    
    /// Signal the event (notify waiting threads)
    #[cfg(windows)]
    pub fn ring(&self) -> Result<()> {
        unsafe {
            SetEvent(self.handle)?;
        }
        Ok(())
    }
    
    /// Wait for event with timeout
    #[cfg(windows)]
    pub fn wait_timeout(&self, timeout_ms: i32) -> Result<bool> {
        let timeout = if timeout_ms < 0 {
            INFINITE
        } else {
            timeout_ms as u32
        };
        
        let result = unsafe { WaitForSingleObject(self.handle, timeout) };
        
        match result {
            WAIT_OBJECT_0 => Ok(true), // Signaled
            WAIT_TIMEOUT => Ok(false), // Timeout
            _ => bail!("WaitForSingleObject failed"),
        }
    }
    
    /// Get raw handle (for duplication)
    #[cfg(windows)]
    pub fn as_raw_handle(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for WindowsEvent {
    #[cfg(windows)]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
        eprintln!("[WIN_EVENT] Closed event: {}", self.name);
    }
}

#[cfg(not(windows))]
impl WindowsEvent {
    pub fn create(_name: &str) -> Result<Self> {
        bail!("WindowsEvent only available on Windows")
    }
    
    pub fn open(_name: &str) -> Result<Self> {
        bail!("WindowsEvent only available on Windows")
    }
    
    pub fn ring(&self) -> Result<()> {
        bail!("WindowsEvent only available on Windows")
    }
    
    pub fn wait_timeout(&self, _timeout_ms: i32) -> Result<bool> {
        bail!("WindowsEvent only available on Windows")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    #[cfg(windows)]
    fn test_windows_event_signal() {
        let event = WindowsEvent::create("test_event_1").unwrap();
        let event2 = WindowsEvent::open("test_event_1").unwrap();
        
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            event2.ring().unwrap();
        });
        
        let result = event.wait_timeout(5000).unwrap();
        assert!(result, "Should receive event signal");
        
        handle.join().unwrap();
    }
    
    #[test]
    #[cfg(windows)]
    fn test_windows_event_timeout() {
        let event = WindowsEvent::create("test_event_2").unwrap();
        
        let result = event.wait_timeout(100).unwrap();
        assert!(!result, "Should timeout without signal");
    }
}
