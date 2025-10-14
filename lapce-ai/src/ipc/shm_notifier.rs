/// EventFD-based notifier for low-latency wakeups (Linux)
/// Replaces micro-sleep polling to achieve ≤10µs p99 latency

use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::Arc;
use anyhow::{Result, bail};

#[cfg(target_os = "linux")]
pub struct EventNotifier {
    event_fd: RawFd,
}

#[cfg(target_os = "linux")]
impl EventNotifier {
    pub fn new() -> Result<Self> {
        unsafe {
            let fd = libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK);
            if fd < 0 {
                bail!("Failed to create eventfd: {}", std::io::Error::last_os_error());
            }
            Ok(Self { event_fd: fd })
        }
    }
    
    /// Signal the waiter (writer calls this after advancing write_pos)
    pub fn notify(&self) -> Result<()> {
        unsafe {
            let val: u64 = 1;
            let ret = libc::write(
                self.event_fd,
                &val as *const u64 as *const libc::c_void,
                std::mem::size_of::<u64>(),
            );
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() != std::io::ErrorKind::WouldBlock {
                    bail!("Failed to write to eventfd: {}", err);
                }
            }
        }
        Ok(())
    }
    
    /// Wait for notification with timeout (reader calls this when buffer empty)
    pub async fn wait_timeout(&self, timeout_ms: u64) -> Result<bool> {
        use tokio::io::unix::AsyncFd;
        use tokio::time::{timeout, Duration};
        
        let async_fd = AsyncFd::new(self.event_fd)?;
        
        match timeout(Duration::from_millis(timeout_ms), async_fd.readable()).await {
            Ok(Ok(mut guard)) => {
                // Clear the eventfd
                unsafe {
                    let mut val: u64 = 0;
                    let ret = libc::read(
                        self.event_fd,
                        &mut val as *mut u64 as *mut libc::c_void,
                        std::mem::size_of::<u64>(),
                    );
                    if ret < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock {
                        bail!("Failed to read from eventfd: {}", std::io::Error::last_os_error());
                    }
                }
                guard.clear_ready();
                Ok(true)
            }
            Ok(Err(e)) => bail!("Wait error: {}", e),
            Err(_) => Ok(false), // Timeout
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for EventNotifier {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.event_fd);
        }
    }
}

#[cfg(target_os = "linux")]
impl AsRawFd for EventNotifier {
    fn as_raw_fd(&self) -> RawFd {
        self.event_fd
    }
}

// Fallback for non-Linux: use condvar-based notifier
#[cfg(not(target_os = "linux"))]
pub struct EventNotifier {
    pair: Arc<(parking_lot::Mutex<bool>, parking_lot::Condvar)>,
}

#[cfg(not(target_os = "linux"))]
impl EventNotifier {
    pub fn new() -> Result<Self> {
        Ok(Self {
            pair: Arc::new((parking_lot::Mutex::new(false), parking_lot::Condvar::new())),
        })
    }
    
    pub fn notify(&self) -> Result<()> {
        let (lock, cvar) = &*self.pair;
        let mut notified = lock.lock();
        *notified = true;
        cvar.notify_all();
        Ok(())
    }
    
    pub async fn wait_timeout(&self, timeout_ms: u64) -> Result<bool> {
        let pair = self.pair.clone();
        tokio::task::spawn_blocking(move || {
            let (lock, cvar) = &*pair;
            let mut notified = lock.lock();
            
            if *notified {
                *notified = false;
                return Ok(true);
            }
            
            let timeout = std::time::Duration::from_millis(timeout_ms);
            let result = cvar.wait_for(&mut notified, timeout);
            
            if result.timed_out() {
                Ok(false)
            } else {
                *notified = false;
                Ok(true)
            }
        })
        .await?
    }
}

#[cfg(not(target_os = "linux"))]
impl Clone for EventNotifier {
    fn clone(&self) -> Self {
        Self {
            pair: self.pair.clone(),
        }
    }
}

/// Shared notifier pair for bidirectional communication
pub struct NotifierPair {
    pub read_notifier: Arc<EventNotifier>,
    pub write_notifier: Arc<EventNotifier>,
}

impl NotifierPair {
    pub fn new() -> Result<Self> {
        Ok(Self {
            read_notifier: Arc::new(EventNotifier::new()?),
            write_notifier: Arc::new(EventNotifier::new()?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_eventfd_notification() {
        let notifier = EventNotifier::new().unwrap();
        
        // Should timeout when no notification
        assert!(!notifier.wait_timeout(10).await.unwrap());
        
        // Should wake up when notified
        notifier.notify().unwrap();
        assert!(notifier.wait_timeout(100).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_multiple_notifications() {
        let notifier = Arc::new(EventNotifier::new().unwrap());
        let n1 = notifier.clone();
        let n2 = notifier.clone();
        
        // Writer task
        tokio::spawn(async move {
            for _ in 0..5 {
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                n1.notify().unwrap();
            }
        });
        
        // Reader should get all notifications
        let mut count = 0;
        for _ in 0..5 {
            if n2.wait_timeout(100).await.unwrap() {
                count += 1;
            }
        }
        assert_eq!(count, 5);
    }
}
