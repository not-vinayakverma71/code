/// Simple multi-process IPC test to validate shared memory atomics work
/// Uses fork() to create true separate processes

use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr;

#[repr(C)]
struct TestHeader {
    counter: AtomicUsize,
    data: [u8; 56],
}

#[test]
fn test_multiprocess_atomics() {
    unsafe {
        let shm_name = std::ffi::CString::new("/test_mp_atomics").unwrap();
        
        // Cleanup
        libc::shm_unlink(shm_name.as_ptr());
        
        // Create shared memory
        const O_EXCL: std::os::raw::c_int = 0x80;
        let fd = libc::shm_open(
            shm_name.as_ptr(),
            (libc::O_CREAT as std::os::raw::c_int) | O_EXCL | (libc::O_RDWR as std::os::raw::c_int),
            0o600
        );
        assert!(fd != -1, "shm_open failed");
        
        let size = std::mem::size_of::<TestHeader>();
        assert!(libc::ftruncate(fd, size as i64) == 0, "ftruncate failed");
        
        let ptr = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0
        ) as *mut TestHeader;
        assert!(ptr != libc::MAP_FAILED as *mut TestHeader, "mmap failed");
        libc::close(fd);
        
        // Initialize
        (*ptr).counter.store(0, Ordering::SeqCst);
        libc::msync(ptr as *mut libc::c_void, size, 1); // MS_SYNC
        
        eprintln!("[PARENT] Created shared memory, counter=0");
        
        // Fork
        let pid = libc::fork();
        
        if pid == 0 {
            // Child process
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // Open existing shared memory
            let fd_child = libc::shm_open(
                shm_name.as_ptr(),
                libc::O_RDWR as std::os::raw::c_int,
                0
            );
            assert!(fd_child != -1, "child shm_open failed");
            
            let ptr_child = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd_child,
                0
            ) as *mut TestHeader;
            assert!(ptr_child != libc::MAP_FAILED as *mut TestHeader, "child mmap failed");
            libc::close(fd_child);
            
            // Sync and read
            libc::msync(ptr_child as *mut libc::c_void, size, 3); // MS_SYNC | MS_INVALIDATE
            let value = (*ptr_child).counter.load(Ordering::SeqCst);
            eprintln!("[CHILD] Read counter={}", value);
            
            // Write
            (*ptr_child).counter.store(42, Ordering::SeqCst);
            libc::msync(ptr_child as *mut libc::c_void, size, 1); // MS_SYNC
            eprintln!("[CHILD] Wrote counter=42");
            
            libc::munmap(ptr_child as *mut libc::c_void, size);
            libc::exit(0);
        } else {
            // Parent process
            eprintln!("[PARENT] Forked child pid={}", pid);
            
            // Write from parent
            (*ptr).counter.store(100, Ordering::SeqCst);
            libc::msync(ptr as *mut libc::c_void, size, 1); // MS_SYNC
            eprintln!("[PARENT] Wrote counter=100");
            
            // Wait for child
            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            eprintln!("[PARENT] Child exited");
            
            // Read child's write
            libc::msync(ptr as *mut libc::c_void, size, 3); // MS_SYNC | MS_INVALIDATE
            let final_value = (*ptr).counter.load(Ordering::SeqCst);
            eprintln!("[PARENT] Final counter={}", final_value);
            
            assert_eq!(final_value, 42, "Parent should see child's write");
            
            // Cleanup
            libc::munmap(ptr as *mut libc::c_void, size);
            libc::shm_unlink(shm_name.as_ptr());
            
            println!("✅ Multi-process atomic test PASSED");
        }
    }
}
