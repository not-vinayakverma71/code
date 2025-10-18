/// Cross-platform wait/notify primitives for low-latency IPC
/// 
/// Platform implementations:
/// - Linux: futex (FUTEX_WAIT_PRIVATE / FUTEX_WAKE_PRIVATE)
/// - Windows: WaitOnAddress / WakeByAddressSingle (Windows 8+)
/// - macOS: ulock_wait / ulock_wake (macOS 10.12+) with kevent fallback

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use anyhow::Result;

const SPIN_ITERATIONS: u32 = 100; // Bounded spin before syscall (~5-10µs)

/// Cross-platform waiter for low-latency synchronization
pub struct ShmWaiter {
    #[cfg(target_os = "linux")]
    _phantom: std::marker::PhantomData<()>,
    
    #[cfg(target_os = "windows")]
    _phantom: std::marker::PhantomData<()>,
    
    #[cfg(target_os = "macos")]
    fallback: Option<MacOsFallback>,
}

#[cfg(target_os = "macos")]
struct MacOsFallback {
    condvar: parking_lot::Condvar,
    mutex: parking_lot::Mutex<()>,
}

impl ShmWaiter {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                _phantom: std::marker::PhantomData,
            })
        }
        
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                _phantom: std::marker::PhantomData,
            })
        }
        
        #[cfg(target_os = "macos")]
        {
            // Try ulock first, fallback to condvar
            Ok(Self {
                fallback: Some(MacOsFallback {
                    condvar: parking_lot::Condvar::new(),
                    mutex: parking_lot::Mutex::new(()),
                }),
            })
        }
    }
    
    /// Wait for sequence to change from expected value
    /// Returns true if woken by notify, false if timeout
    pub fn wait(&self, seq_ptr: *const AtomicU64, expected: u64, timeout: Duration) -> bool {
        // Bounded spin first to avoid syscall for short waits
        for _ in 0..SPIN_ITERATIONS {
            let current = unsafe { (*seq_ptr).load(Ordering::Acquire) };
            if current != expected {
                return true;
            }
            std::hint::spin_loop();
        }
        
        // Still same value, do OS-specific wait
        self.wait_impl(seq_ptr, expected, timeout)
    }
    
    /// Wake one waiter
    pub fn wake_one(&self, seq_ptr: *const AtomicU64) {
        self.wake_impl(seq_ptr, 1)
    }
    
    /// Wake all waiters
    pub fn wake_all(&self, seq_ptr: *const AtomicU64) {
        self.wake_impl(seq_ptr, i32::MAX)
    }
    
    #[cfg(target_os = "linux")]
    fn wait_impl(&self, seq_ptr: *const AtomicU64, expected: u64, timeout: Duration) -> bool {
        use std::os::raw::c_int;
        
        // Futex constants (from linux/futex.h)
        const FUTEX_WAIT_PRIVATE: c_int = 128; // FUTEX_WAIT | FUTEX_PRIVATE_FLAG
        const FUTEX_WAKE_PRIVATE: c_int = 129; // FUTEX_WAKE | FUTEX_PRIVATE_FLAG
        
        unsafe {
            let timeout_spec = libc::timespec {
                tv_sec: timeout.as_secs() as libc::time_t,
                tv_nsec: timeout.subsec_nanos() as libc::c_long,
            };
            
            // FUTEX_WAIT_PRIVATE: wait if *seq_ptr == expected
            let ret = libc::syscall(
                libc::SYS_futex,
                seq_ptr as *const u64,
                FUTEX_WAIT_PRIVATE,
                expected,
                &timeout_spec as *const libc::timespec,
                0 as *const c_int,
                0 as c_int,
            );
            
            if ret == 0 {
                true // Woken by wake
            } else {
                let err = std::io::Error::last_os_error();
                // EAGAIN means value changed, ETIMEDOUT means timeout
                err.raw_os_error() == Some(libc::EAGAIN)
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    fn wake_impl(&self, seq_ptr: *const AtomicU64, count: i32) {
        const FUTEX_WAKE_PRIVATE: i32 = 129; // FUTEX_WAKE | FUTEX_PRIVATE_FLAG
        
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                seq_ptr as *const u64,
                FUTEX_WAKE_PRIVATE,
                count,
                0,
                0,
                0,
            );
        }
    }
    
    #[cfg(target_os = "windows")]
    fn wait_impl(&self, seq_ptr: *const AtomicU64, expected: u64, timeout: Duration) -> bool {
        use windows_sys::Win32::System::Threading::{WaitOnAddress, INFINITE};
        
        unsafe {
            let timeout_ms = if timeout.as_secs() > (INFINITE as u64 / 1000) {
                INFINITE
            } else {
                timeout.as_millis() as u32
            };
            
            let expected_bytes = expected.to_le_bytes();
            
            // WaitOnAddress: wait while *seq_ptr == expected
            let ret = WaitOnAddress(
                seq_ptr as *const std::ffi::c_void,
                expected_bytes.as_ptr() as *const std::ffi::c_void,
                8, // size of u64
                timeout_ms,
            );
            
            ret != 0 // Non-zero means success (woken or value changed)
        }
    }
    
    #[cfg(target_os = "windows")]
    fn wake_impl(&self, seq_ptr: *const AtomicU64, count: i32) {
        use windows_sys::Win32::System::Threading::{WakeByAddressSingle, WakeByAddressAll};
        
        unsafe {
            if count == 1 {
                WakeByAddressSingle(seq_ptr as *mut std::ffi::c_void);
            } else {
                WakeByAddressAll(seq_ptr as *mut std::ffi::c_void);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn wait_impl(&self, seq_ptr: *const AtomicU64, expected: u64, timeout: Duration) -> bool {
        // Try ulock_wait first (macOS 10.12+)
        #[cfg(target_arch = "x86_64")]
        {
            if let Some(result) = self.try_ulock_wait(seq_ptr, expected, timeout) {
                return result;
            }
        }
        
        // Fallback to condvar
        if let Some(ref fallback) = self.fallback {
            let _guard = fallback.mutex.lock();
            let current = unsafe { (*seq_ptr).load(Ordering::Acquire) };
            if current != expected {
                return true;
            }
            
            fallback.condvar.wait_for(&mut fallback.mutex.lock(), timeout).timed_out()
        } else {
            false
        }
    }
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn try_ulock_wait(&self, seq_ptr: *const AtomicU64, expected: u64, timeout: Duration) -> Option<bool> {
        // ulock_wait syscall number on macOS x86_64
        const SYS_ULOCK_WAIT: i32 = 515;
        const UL_COMPARE_AND_WAIT64: u32 = 5;
        
        unsafe {
            let timeout_us = timeout.as_micros() as u32;
            
            let ret = libc::syscall(
                SYS_ULOCK_WAIT,
                UL_COMPARE_AND_WAIT64,
                seq_ptr as *const u64,
                expected,
                timeout_us,
            );
            
            if ret == 0 {
                Some(true) // Success
            } else {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ETIMEDOUT) {
                    Some(false)
                } else {
                    None // ulock not available, use fallback
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn wake_impl(&self, seq_ptr: *const AtomicU64, count: i32) {
        // Try ulock_wake first
        #[cfg(target_arch = "x86_64")]
        {
            if self.try_ulock_wake(seq_ptr, count) {
                return;
            }
        }
        
        // Fallback to condvar
        if let Some(ref fallback) = self.fallback {
            if count == 1 {
                fallback.condvar.notify_one();
            } else {
                fallback.condvar.notify_all();
            }
        }
    }
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn try_ulock_wake(&self, seq_ptr: *const AtomicU64, count: i32) -> bool {
        const SYS_ULOCK_WAKE: i32 = 516;
        const UL_COMPARE_AND_WAIT64: u32 = 5;
        const ULF_WAKE_ALL: u32 = 0x00000100;
        
        unsafe {
            let flags = if count > 1 { ULF_WAKE_ALL } else { 0 };
            
            let ret = libc::syscall(
                SYS_ULOCK_WAKE,
                UL_COMPARE_AND_WAIT64 | flags,
                seq_ptr as *const u64,
                0,
            );
            
            ret == 0
        }
    }
}

unsafe impl Send for ShmWaiter {}
unsafe impl Sync for ShmWaiter {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[test]
    fn test_waiter_basic() {
        let waiter = ShmWaiter::new().unwrap();
        let seq = Arc::new(AtomicU64::new(0));
        let seq_ptr = Arc::as_ptr(&seq);
        
        // Should timeout when waiting for value that won't change
        let result = waiter.wait(seq_ptr, 0, Duration::from_millis(10));
        assert!(!result); // Timeout
        
        // Change value and wake
        seq.store(1, Ordering::Release);
        waiter.wake_one(seq_ptr);
        
        // Should return immediately since value changed
        let result = waiter.wait(seq_ptr, 0, Duration::from_millis(100));
        assert!(result); // Value changed
    }
    
    #[test]
    fn test_waiter_multi_thread() {
        let waiter = Arc::new(ShmWaiter::new().unwrap());
        let seq = Arc::new(AtomicU64::new(0));
        
        let waiter_clone = waiter.clone();
        let seq_clone = seq.clone();
        
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            seq_clone.store(1, Ordering::Release);
            let ptr = Arc::as_ptr(&seq_clone);
            waiter_clone.wake_one(ptr);
        });
        
        let start = std::time::Instant::now();
        let seq_ptr = Arc::as_ptr(&seq);
        let result = waiter.wait(seq_ptr, 0, Duration::from_secs(1));
        let elapsed = start.elapsed();
        
        assert!(result); // Should wake
        assert!(elapsed < Duration::from_millis(100)); // Should wake quickly
        
        handle.join().unwrap();
    }
}
