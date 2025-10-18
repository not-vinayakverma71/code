/// Linux futex wrapper for cross-process atomic synchronization
/// Futex provides proper cache-coherent atomics between processes

use anyhow::{Result, bail};
use std::time::Duration;

#[cfg(target_os = "linux")]
use libc::{syscall, SYS_futex, FUTEX_WAIT, FUTEX_WAKE};

// FUTEX_PRIVATE constants not in libc crate
#[cfg(target_os = "linux")]
const FUTEX_PRIVATE_FLAG: i32 = 128;
#[cfg(target_os = "linux")]
const FUTEX_WAIT_PRIVATE: i32 = FUTEX_WAIT | FUTEX_PRIVATE_FLAG;
#[cfg(target_os = "linux")]
const FUTEX_WAKE_PRIVATE: i32 = FUTEX_WAKE | FUTEX_PRIVATE_FLAG;

/// Wait on a futex until it changes from expected value or timeout
#[cfg(target_os = "linux")]
pub fn futex_wait(futex_addr: *const u32, expected: u32, timeout_ms: Option<i32>) -> Result<()> {
    let timeout_spec = timeout_ms.map(|ms| libc::timespec {
        tv_sec: (ms / 1000) as libc::time_t,
        tv_nsec: ((ms % 1000) * 1_000_000) as libc::c_long,
    });
    
    let timeout_ptr = match &timeout_spec {
        Some(ts) => ts as *const libc::timespec,
        None => std::ptr::null(),
    };
    
    let ret = unsafe {
        syscall(
            SYS_futex,
            futex_addr,
            FUTEX_WAIT,
            expected,
            timeout_ptr,
            0,
            0,
        )
    };
    
    if ret == -1 {
        let errno = unsafe { *libc::__errno_location() };
        match errno {
            libc::EAGAIN => Ok(()), // Value changed, not an error
            libc::ETIMEDOUT => Ok(()), // Timeout, not an error
            libc::EINTR => Ok(()), // Interrupted, not an error
            _ => bail!("futex_wait failed: errno {}", errno),
        }
    } else {
        Ok(())
    }
}

/// Wake up threads waiting on a futex
#[cfg(target_os = "linux")]
pub fn futex_wake(futex_addr: *const u32, num_waiters: i32) -> Result<()> {
    let ret = unsafe {
        syscall(
            SYS_futex,
            futex_addr,
            FUTEX_WAKE,
            num_waiters,
            0,
            0,
            0,
        )
    };
    
    if ret == -1 {
        let errno = unsafe { *libc::__errno_location() };
        bail!("futex_wake failed: errno {}", errno);
    }
    
    Ok(())
}

/// Wait on a private futex (process-private, faster)
#[cfg(target_os = "linux")]
pub fn futex_wait_private(futex_addr: *const u32, expected: u32, timeout_ms: Option<i32>) -> Result<()> {
    let timeout_spec = timeout_ms.map(|ms| libc::timespec {
        tv_sec: (ms / 1000) as libc::time_t,
        tv_nsec: ((ms % 1000) * 1_000_000) as libc::c_long,
    });
    
    let timeout_ptr = match &timeout_spec {
        Some(ts) => ts as *const libc::timespec,
        None => std::ptr::null(),
    };
    
    let ret = unsafe {
        syscall(
            SYS_futex,
            futex_addr,
            FUTEX_WAIT_PRIVATE,
            expected,
            timeout_ptr,
            0,
            0,
        )
    };
    
    if ret == -1 {
        let errno = unsafe { *libc::__errno_location() };
        match errno {
            libc::EAGAIN | libc::ETIMEDOUT | libc::EINTR => Ok(()),
            _ => bail!("futex_wait_private failed: errno {}", errno),
        }
    } else {
        Ok(())
    }
}

/// Wake up threads waiting on a private futex
#[cfg(target_os = "linux")]
pub fn futex_wake_private(futex_addr: *const u32, num_waiters: i32) -> Result<()> {
    let ret = unsafe {
        syscall(
            SYS_futex,
            futex_addr,
            FUTEX_WAKE_PRIVATE,
            num_waiters,
            0,
            0,
            0,
        )
    };
    
    if ret == -1 {
        let errno = unsafe { *libc::__errno_location() };
        bail!("futex_wake_private failed: errno {}", errno);
    }
    
    Ok(())
}

/// Atomic compare-and-swap for shared memory
#[cfg(target_os = "linux")]
pub fn atomic_cas(addr: *mut u32, expected: u32, new: u32) -> bool {
    unsafe {
        let result = std::sync::atomic::AtomicU32::from_ptr(addr)
            .compare_exchange(
                expected,
                new,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            );
        result.is_ok()
    }
}

/// Atomic load with acquire ordering
#[cfg(target_os = "linux")]
pub fn atomic_load(addr: *const u32) -> u32 {
    unsafe {
        std::sync::atomic::AtomicU32::from_ptr(addr as *mut u32)
            .load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Atomic store with release ordering
#[cfg(target_os = "linux")]
pub fn atomic_store(addr: *mut u32, value: u32) {
    unsafe {
        std::sync::atomic::AtomicU32::from_ptr(addr)
            .store(value, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[test]
    #[cfg(target_os = "linux")]
    fn test_futex_wake_wait() {
        let value = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let value_clone = value.clone();
        
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            value_clone.store(1, std::sync::atomic::Ordering::SeqCst);
            futex_wake(value_clone.as_ptr(), 1).unwrap();
        });
        
        futex_wait(value.as_ptr(), 0, Some(5000)).unwrap();
        assert_eq!(value.load(std::sync::atomic::Ordering::SeqCst), 1);
        
        handle.join().unwrap();
    }
}
