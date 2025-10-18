/// POSIX semaphore-based synchronization for macOS
/// Replaces Linux futex with POSIX semaphores for cross-process atomics

use anyhow::{Result, bail};
use std::ptr;
use std::ffi::CString;

#[cfg(unix)]
#[cfg(target_os = "linux")]
use libc::{sem_t, sem_open, sem_close, sem_unlink, sem_post, sem_wait, sem_timedwait};

#[cfg(target_os = "macos")]
use libc::{sem_t, sem_open, sem_close, sem_unlink, sem_post, sem_wait, sem_trywait};
#[cfg(unix)]
use libc::{O_CREAT, O_EXCL, timespec, CLOCK_REALTIME};

/// POSIX semaphore wrapper for cross-process synchronization
pub struct PosixSemaphore {
    sem: *mut sem_t,
    name: String,
    owned: bool, // Whether we created it (and should unlink on drop)
}

unsafe impl Send for PosixSemaphore {}
unsafe impl Sync for PosixSemaphore {}

impl PosixSemaphore {
    /// Create a new named semaphore
    #[cfg(unix)]
    pub fn create(name: &str, initial_value: u32) -> Result<Self> {
        let c_name = CString::new(name)?;
        
        // Create semaphore with O_CREAT | O_EXCL
        let sem = unsafe {
            sem_open(c_name.as_ptr(), O_CREAT | O_EXCL, 0o600, initial_value)
        };
        
        if sem == libc::SEM_FAILED {
            // If already exists, try to open it
            let sem = unsafe { sem_open(c_name.as_ptr(), 0) };
            if sem == libc::SEM_FAILED {
                bail!("sem_open failed: {}", std::io::Error::last_os_error());
            }
            return Ok(Self {
                sem,
                name: name.to_string(),
                owned: false,
            });
        }
        
        eprintln!("[POSIX_SEM] Created semaphore: {}", name);
        Ok(Self {
            sem,
            name: name.to_string(),
            owned: true,
        })
    }
    
    /// Open existing named semaphore
    #[cfg(unix)]
    pub fn open(name: &str) -> Result<Self> {
        let c_name = CString::new(name)?;
        
        let sem = unsafe { sem_open(c_name.as_ptr(), 0) };
        
        if sem == libc::SEM_FAILED {
            bail!("sem_open failed for {}: {}", name, std::io::Error::last_os_error());
        }
        
        eprintln!("[POSIX_SEM] Opened semaphore: {}", name);
        Ok(Self {
            sem,
            name: name.to_string(),
            owned: false,
        })
    }
    
    /// Post (increment) the semaphore
    #[cfg(unix)]
    pub fn post(&self) -> Result<()> {
        let ret = unsafe { sem_post(self.sem) };
        if ret != 0 {
            bail!("sem_post failed: {}", std::io::Error::last_os_error());
        }
        Ok(())
    }
    
    /// Wait (decrement) the semaphore
    #[cfg(unix)]
    pub fn wait(&self) -> Result<()> {
        let ret = unsafe { sem_wait(self.sem) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                return Ok(()); // EINTR is not an error
            }
            bail!("sem_wait failed: {}", err);
        }
        Ok(())
    }
    
    /// Wait with timeout
    #[cfg(unix)]
    pub fn wait_timeout(&self, timeout_ms: i32) -> Result<bool> {
        // Get absolute time for timeout
        let mut now: timespec = unsafe { std::mem::zeroed() };
        
        #[cfg(target_os = "macos")]
        unsafe {
            let mut tv: libc::timeval = std::mem::zeroed();
            libc::gettimeofday(&mut tv, ptr::null_mut());
            now.tv_sec = tv.tv_sec;
            now.tv_nsec = (tv.tv_usec * 1000) as i64;
        }
        
        #[cfg(target_os = "linux")]
        unsafe {
            libc::clock_gettime(CLOCK_REALTIME, &mut now);
        }
        
        // Add timeout
        let timeout_sec = timeout_ms / 1000;
        let timeout_nsec = ((timeout_ms % 1000) * 1_000_000) as i64;
        
        let mut abs_timeout = timespec {
            tv_sec: now.tv_sec + timeout_sec as libc::time_t,
            tv_nsec: now.tv_nsec + timeout_nsec,
        };
        
        // Handle nanosecond overflow
        if abs_timeout.tv_nsec >= 1_000_000_000 {
            abs_timeout.tv_sec += 1;
            abs_timeout.tv_nsec -= 1_000_000_000;
        }
        
        #[cfg(target_os = "linux")]
        {
            let ret = unsafe { sem_timedwait(self.sem, &abs_timeout) };
            
            if ret != 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::ETIMEDOUT) => return Ok(false), // Timeout
                    Some(libc::EINTR) => return Ok(false), // Interrupted
                    _ => bail!("sem_timedwait failed: {}", err),
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS doesn't have sem_timedwait, use polling with sem_trywait
            let start = std::time::Instant::now();
            let timeout_duration = std::time::Duration::from_millis(timeout_ms as u64);
            
            loop {
                let ret = unsafe { sem_trywait(self.sem) };
                if ret == 0 {
                    return Ok(true); // Acquired
                }
                
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EAGAIN) => {
                        // Not available, check timeout
                        if start.elapsed() >= timeout_duration {
                            return Ok(false); // Timeout
                        }
                        // Sleep briefly before retry
                        std::thread::sleep(std::time::Duration::from_micros(100));
                        continue;
                    }
                    Some(libc::EINTR) => continue, // Interrupted, retry
                    _ => bail!("sem_trywait failed: {}", err),
                }
            }
        }
        
        Ok(true)
    }
}

impl Drop for PosixSemaphore {
    fn drop(&mut self) {
        unsafe {
            sem_close(self.sem);
            
            if self.owned {
                let c_name = CString::new(self.name.as_str()).unwrap();
                sem_unlink(c_name.as_ptr());
                eprintln!("[POSIX_SEM] Unlinked semaphore: {}", self.name);
            }
        }
    }
}

/// Atomic operations using POSIX semaphores for synchronization
pub struct PosixAtomicU32 {
    addr: *mut u32,
    sem: PosixSemaphore,
}

unsafe impl Send for PosixAtomicU32 {}
unsafe impl Sync for PosixAtomicU32 {}

impl PosixAtomicU32 {
    /// Create atomic with semaphore protection
    pub fn new(addr: *mut u32, sem_name: &str) -> Result<Self> {
        let sem = PosixSemaphore::create(sem_name, 1)?; // Binary semaphore (mutex)
        Ok(Self { addr, sem })
    }
    
    /// Open existing atomic with semaphore
    pub fn open(addr: *mut u32, sem_name: &str) -> Result<Self> {
        let sem = PosixSemaphore::open(sem_name)?;
        Ok(Self { addr, sem })
    }
    
    /// Load value with acquire semantics
    pub fn load(&self) -> Result<u32> {
        self.sem.wait()?;
        let val = unsafe { ptr::read_volatile(self.addr) };
        self.sem.post()?;
        Ok(val)
    }
    
    /// Store value with release semantics
    pub fn store(&self, val: u32) -> Result<()> {
        self.sem.wait()?;
        unsafe { ptr::write_volatile(self.addr, val) };
        self.sem.post()?;
        Ok(())
    }
    
    /// Compare and swap
    pub fn compare_exchange(&self, expected: u32, new: u32) -> Result<Result<u32, u32>> {
        self.sem.wait()?;
        let current = unsafe { ptr::read_volatile(self.addr) };
        
        if current == expected {
            unsafe { ptr::write_volatile(self.addr, new) };
            self.sem.post()?;
            Ok(Ok(expected))
        } else {
            self.sem.post()?;
            Ok(Err(current))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    #[cfg(unix)]
    fn test_posix_sem_post_wait() {
        let sem = PosixSemaphore::create("/test_posix_sem_1", 0).unwrap();
        
        let sem_name = sem.name.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let sem2 = PosixSemaphore::open(&sem_name).unwrap();
            sem2.post().unwrap();
        });
        
        let result = sem.wait_timeout(5000).unwrap();
        assert!(result, "Should receive semaphore signal");
        
        handle.join().unwrap();
    }
    
    #[test]
    #[cfg(unix)]
    fn test_posix_sem_timeout() {
        let sem = PosixSemaphore::create("/test_posix_sem_2", 0).unwrap();
        
        let result = sem.wait_timeout(100).unwrap();
        assert!(!result, "Should timeout without signal");
    }
}
