# IPC Shared Memory Root Cause Analysis

## Summary
The IPC system fails because the handler never sees client writes to shared memory, despite both mapping the same file (verified by identical inodes).

## Evidence
1. **Same shared memory object**: Both processes open files with same inode (dev=30, ino=211152)
2. **Client writes successfully**: write_pos advances (0→45→90→...→4500)  
3. **Handler sees nothing**: write_pos always reads as 0
4. **Memory addresses differ**: Expected with mmap (virtual addresses are process-local)

## Root Causes Identified

### 1. Atomic Initialization Issue (PARTIALLY FIXED)
- Originally used `AtomicUsize::new()` which creates process-local atomics
- Fixed to use `.store()` directly on shared memory atomics
- Still not working - suggests deeper issue

### 2. Missing Memory Synchronization (ATTEMPTED)
- Added `msync()` calls with MS_SYNC flag
- Added memory barriers (SeqCst fences)
- Still not working - atomics should work without explicit msync

### 3. Buffer Creation Race (FIXED)
- Server was recreating buffers that already existed
- Fixed by checking existence before creating
- Client now successfully opens server-created buffers

### 4. CRITICAL: Shared Memory Not Actually Shared
Despite same inode, the memory regions act isolated. Possible causes:
- **Copy-on-write semantics**: MAP_SHARED should prevent this
- **Cache coherency**: Memory barriers should fix this
- **Struct layout issue**: AtomicUsize in shared memory might not work as expected

## Next Steps

1. **Test with raw pointers**: Skip atomics, use volatile reads/writes
2. **Verify with simpler test**: Our simple_shm_test works, so issue is in our implementation
3. **Check struct alignment**: Ensure RingBufferHeader is properly aligned for shared memory
4. **Use futex or eventfd**: For explicit cross-process synchronization

## Conclusion
The issue appears to be that Rust's `AtomicUsize` doesn't work correctly when the struct is placed in POSIX shared memory. The atomic operations may be using CPU-specific instructions that don't cross process boundaries properly.
