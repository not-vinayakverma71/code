/// Linux eventfd-based doorbell for efficient cross-process notifications
/// Replaces polling with kernel-based wake-up mechanism

use anyhow::{Result, bail};
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(target_os = "linux")]
use libc::{eventfd, EFD_CLOEXEC, EFD_NONBLOCK, EFD_SEMAPHORE};

/// Eventfd doorbell for cross-process notifications
pub struct EventFdDoorbell {
    fd: RawFd,
}

impl EventFdDoorbell {
    /// Create a new eventfd doorbell
    #[cfg(target_os = "linux")]
    pub fn new() -> Result<Self> {
        let fd = unsafe {
            eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK | EFD_SEMAPHORE)
        };
        
        if fd < 0 {
            bail!("Failed to create eventfd: {}", std::io::Error::last_os_error());
        }
        
        eprintln!("[EVENTFD] Created doorbell fd={}", fd);
        Ok(Self { fd })
    }
    
    /// Open existing eventfd from file descriptor
    #[cfg(target_os = "linux")]
    pub fn from_fd(fd: RawFd) -> Self {
        eprintln!("[EVENTFD] Opened doorbell from fd={}", fd);
        Self { fd }
    }
    
    /// Ring the doorbell (notify waiting thread)
    #[cfg(target_os = "linux")]
    pub fn ring(&self) -> Result<()> {
        let value: u64 = 1;
        let ret = unsafe {
            libc::write(
                self.fd,
                &value as *const u64 as *const libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        
        if ret < 0 {
            bail!("Failed to ring doorbell: {}", std::io::Error::last_os_error());
        }
        
        Ok(())
    }
    
    /// Wait for doorbell to ring (blocking)
    #[cfg(target_os = "linux")]
    pub fn wait(&self) -> Result<()> {
        let mut value: u64 = 0;
        let ret = unsafe {
            libc::read(
                self.fd,
                &mut value as *mut u64 as *mut libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock {
                // Non-blocking read, no data available
                return Ok(());
            }
            bail!("Failed to wait on doorbell: {}", err);
        }
        
        Ok(())
    }
    
    /// Wait for doorbell with timeout (using poll)
    #[cfg(target_os = "linux")]
    pub fn wait_timeout(&self, timeout_ms: i32) -> Result<bool> {
        let mut pollfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN,
            revents: 0,
        };
        
        let ret = unsafe {
            libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms)
        };
        
        if ret < 0 {
            bail!("Poll failed: {}", std::io::Error::last_os_error());
        }
        
        if ret == 0 {
            // Timeout
            return Ok(false);
        }
        
        if pollfd.revents & libc::POLLIN != 0 {
            // Data available, consume it
            self.wait()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get raw file descriptor (for sharing across processes)
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
    
    /// Duplicate fd for sharing (dup)
    #[cfg(target_os = "linux")]
    pub fn duplicate(&self) -> Result<RawFd> {
        let new_fd = unsafe { libc::dup(self.fd) };
        if new_fd < 0 {
            bail!("Failed to dup eventfd: {}", std::io::Error::last_os_error());
        }
        Ok(new_fd)
    }
}

impl AsRawFd for EventFdDoorbell {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for EventFdDoorbell {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
        eprintln!("[EVENTFD] Closed doorbell fd={}", self.fd);
    }
}

unsafe impl Send for EventFdDoorbell {}
unsafe impl Sync for EventFdDoorbell {}

#[cfg(not(target_os = "linux"))]
impl EventFdDoorbell {
    pub fn new() -> Result<Self> {
        bail!("EventFd only supported on Linux");
    }
    
    pub fn from_fd(_fd: RawFd) -> Self {
        panic!("EventFd only supported on Linux");
    }
    
    pub fn ring(&self) -> Result<()> {
        bail!("EventFd only supported on Linux");
    }
    
    pub fn wait(&self) -> Result<()> {
        bail!("EventFd only supported on Linux");
    }
    
    pub fn wait_timeout(&self, _timeout_ms: i32) -> Result<bool> {
        bail!("EventFd only supported on Linux");
    }
    
    pub fn duplicate(&self) -> Result<RawFd> {
        bail!("EventFd only supported on Linux");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(target_os = "linux")]
    fn test_eventfd_basic() {
        let doorbell = EventFdDoorbell::new().unwrap();
        
        // Ring and wait
        doorbell.ring().unwrap();
        doorbell.wait().unwrap();
        
        // Wait on empty should not block (non-blocking)
        assert!(doorbell.wait().is_ok());
    }
    
    #[test]
    #[cfg(target_os = "linux")]
    fn test_eventfd_timeout() {
        let doorbell = EventFdDoorbell::new().unwrap();
        
        // Wait with timeout on empty
        let result = doorbell.wait_timeout(10).unwrap();
        assert!(!result, "Should timeout");
        
        // Ring and wait
        doorbell.ring().unwrap();
        let result = doorbell.wait_timeout(10).unwrap();
        assert!(result, "Should have data");
    }
}
