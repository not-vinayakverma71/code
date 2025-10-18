/// Simple test to verify POSIX shared memory works correctly
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn test_posix_shm_basic() {
    unsafe {
        // Create shared memory
        let name = std::ffi::CString::new("/test_shm_basic").unwrap();
        let fd = libc::shm_open(name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
        assert!(fd >= 0, "shm_open failed");
        
        let size = 4096;
        assert_eq!(libc::ftruncate(fd, size), 0);
        
        // Map it
        let ptr = libc::mmap(
            ptr::null_mut(),
            size as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0
        );
        assert_ne!(ptr, libc::MAP_FAILED);
        
        // Write a value
        let atomic_ptr = ptr as *mut AtomicU32;
        (*atomic_ptr).store(42, Ordering::SeqCst);
        
        // Sync
        libc::msync(ptr, size as usize, libc::MS_SYNC);
        
        // Fork to test cross-process visibility
        let pid = libc::fork();
        
        if pid == 0 {
            // Child process - read the value
            let value = (*atomic_ptr).load(Ordering::SeqCst);
            if value == 42 {
                std::process::exit(0); // Success
            } else {
                eprintln!("Child read wrong value: {}", value);
                std::process::exit(1); // Failure
            }
        } else {
            // Parent - wait for child
            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            
            // Cleanup
            libc::munmap(ptr, size as usize);
            libc::close(fd);
            libc::shm_unlink(name.as_ptr());
            
            assert_eq!(libc::WEXITSTATUS(status), 0, "Child process failed");
        }
    }
}

#[test] 
fn test_shm_separate_open() {
    unsafe {
        let name = std::ffi::CString::new("/test_shm_separate").unwrap();
        
        // Process 1: Create and write
        let fd1 = libc::shm_open(name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
        assert!(fd1 >= 0);
        assert_eq!(libc::ftruncate(fd1, 4096), 0);
        
        let ptr1 = libc::mmap(
            ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd1,
            0
        );
        assert_ne!(ptr1, libc::MAP_FAILED);
        
        let atomic1 = ptr1 as *mut AtomicU32;
        (*atomic1).store(123, Ordering::SeqCst);
        libc::msync(ptr1, 4096, libc::MS_SYNC);
        
        // Process 2: Open existing and read
        let fd2 = libc::shm_open(name.as_ptr(), libc::O_RDWR, 0);
        assert!(fd2 >= 0);
        
        let ptr2 = libc::mmap(
            ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd2,
            0
        );
        assert_ne!(ptr2, libc::MAP_FAILED);
        
        let atomic2 = ptr2 as *mut AtomicU32;
        let value = (*atomic2).load(Ordering::SeqCst);
        
        println!("ptr1={:p}, ptr2={:p}", ptr1, ptr2);
        println!("Written: 123, Read: {}", value);
        
        // Cleanup
        libc::munmap(ptr1, 4096);
        libc::munmap(ptr2, 4096);
        libc::close(fd1);
        libc::close(fd2);
        libc::shm_unlink(name.as_ptr());
        
        assert_eq!(value, 123, "Shared memory not working!");
    }
}
