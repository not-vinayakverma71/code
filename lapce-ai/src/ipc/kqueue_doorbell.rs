/// macOS kqueue-based doorbell for efficient cross-process notifications
/// Replaces Linux eventfd with kqueue EVFILT_USER for macOS compatibility

use anyhow::{Result, bail};
use std::os::unix::io::{AsRawFd, RawFd};

#[cfg(target_os = "macos")]
use libc::{kqueue, kevent, EV_ADD, EV_ENABLE, EV_CLEAR, EVFILT_USER, NOTE_TRIGGER};

/// Kqueue doorbell for cross-process notifications on macOS
pub struct KqueueDoorbell {
    kq_fd: RawFd,
    ident: usize,
}

impl KqueueDoorbell {
    /// Create a new kqueue doorbell
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self> {
        let kq_fd = unsafe { kqueue() };
        
        if kq_fd < 0 {
            bail!("Failed to create kqueue: {}", std::io::Error::last_os_error());
        }
        
        // Use unique identifier for this doorbell
        let ident = kq_fd as usize;
        
        // Register EVFILT_USER event
        let mut kev: libc::kevent = unsafe { std::mem::zeroed() };
        kev.ident = ident;
        kev.filter = EVFILT_USER;
        kev.flags = EV_ADD | EV_ENABLE | EV_CLEAR;
        kev.fflags = 0;
        kev.data = 0;
        kev.udata = std::ptr::null_mut();
        
        let ret = unsafe {
            kevent(
                kq_fd,
                &kev as *const libc::kevent,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if ret < 0 {
            unsafe { libc::close(kq_fd) };
            bail!("Failed to register kqueue event: {}", std::io::Error::last_os_error());
        }
        
        eprintln!("[KQUEUE] Created doorbell kq_fd={} ident={}", kq_fd, ident);
        Ok(Self { kq_fd, ident })
    }
    
    /// Open existing kqueue from file descriptor
    #[cfg(target_os = "macos")]
    pub fn from_fd(kq_fd: RawFd) -> Result<Self> {
        let ident = kq_fd as usize;
        eprintln!("[KQUEUE] Opened doorbell from kq_fd={} ident={}", kq_fd, ident);
        Ok(Self { kq_fd, ident })
    }
    
    /// Ring the doorbell (notify waiting thread)
    #[cfg(target_os = "macos")]
    pub fn ring(&self) -> Result<()> {
        let mut kev: libc::kevent = unsafe { std::mem::zeroed() };
        kev.ident = self.ident;
        kev.filter = EVFILT_USER;
        kev.flags = 0;
        kev.fflags = NOTE_TRIGGER;
        kev.data = 0;
        kev.udata = std::ptr::null_mut();
        
        let ret = unsafe {
            kevent(
                self.kq_fd,
                &kev as *const libc::kevent,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if ret < 0 {
            bail!("Failed to trigger kqueue: {}", std::io::Error::last_os_error());
        }
        
        Ok(())
    }
    
    /// Wait for doorbell to ring with timeout
    #[cfg(target_os = "macos")]
    pub fn wait_timeout(&self, timeout_ms: i32) -> Result<bool> {
        let timeout_spec = libc::timespec {
            tv_sec: (timeout_ms / 1000) as libc::time_t,
            tv_nsec: ((timeout_ms % 1000) * 1_000_000) as libc::c_long,
        };
        
        let mut kev: libc::kevent = unsafe { std::mem::zeroed() };
        
        let ret = unsafe {
            kevent(
                self.kq_fd,
                std::ptr::null(),
                0,
                &mut kev as *mut libc::kevent,
                1,
                &timeout_spec as *const libc::timespec,
            )
        };
        
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                return Ok(false); // Interrupted, no event
            }
            bail!("Failed to wait on kqueue: {}", err);
        }
        
        if ret == 0 {
            return Ok(false); // Timeout
        }
        
        if ret > 0 && kev.filter == EVFILT_USER {
            return Ok(true); // Event received
        }
        
        Ok(false)
    }
    
    /// Duplicate the kqueue fd (for passing to child process)
    #[cfg(target_os = "macos")]
    pub fn duplicate(&self) -> Result<RawFd> {
        let new_fd = unsafe { libc::dup(self.kq_fd) };
        if new_fd < 0 {
            bail!("Failed to duplicate kqueue fd: {}", std::io::Error::last_os_error());
        }
        Ok(new_fd)
    }
}

impl AsRawFd for KqueueDoorbell {
    fn as_raw_fd(&self) -> RawFd {
        self.kq_fd
    }
}

impl Drop for KqueueDoorbell {
    fn drop(&mut self) {
        eprintln!("[KQUEUE] Closed doorbell kq_fd={}", self.kq_fd);
        unsafe {
            libc::close(self.kq_fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    #[cfg(target_os = "macos")]
    fn test_kqueue_ring_wait() {
        let doorbell = KqueueDoorbell::new().unwrap();
        
        let doorbell_clone = KqueueDoorbell::from_fd(doorbell.duplicate().unwrap()).unwrap();
        
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            doorbell_clone.ring().unwrap();
        });
        
        let result = doorbell.wait_timeout(5000).unwrap();
        assert!(result, "Should receive doorbell notification");
        
        handle.join().unwrap();
    }
    
    #[test]
    #[cfg(target_os = "macos")]
    fn test_kqueue_timeout() {
        let doorbell = KqueueDoorbell::new().unwrap();
        
        let result = doorbell.wait_timeout(100).unwrap();
        assert!(!result, "Should timeout without notification");
    }
}
