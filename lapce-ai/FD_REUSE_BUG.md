# Critical Bug: File Descriptor Reuse

## Evidence

```
[CREATE @1760460471501 SUCCESS] '..._631_send-aa39e0f7' fd=10
[CREATE @1760460471501 SUCCESS] '..._631_recv-aa39e0f7' fd=10  ← SAME FD!
[OPEN @1760460471512 SUCCESS] '..._631_recv-aa39e0f7' fd=9
[OPEN @1760460471512 SUCCESS] '..._631_send-aa39e0f7' fd=9    ← SAME FD!
```

**Both CREATE calls return fd=10. Both OPEN calls return fd=9.**

## Root Cause

We're closing the fd after mmap() but NOT keeping it open!

```rust
// In create_blocking/open_blocking:
let fd = libc::shm_open(...);
let ptr = libc::mmap(..., fd, 0);
libc::close(fd);  // ← WE CLOSE IT!
```

When we close fd after mmap, the fd number gets recycled. So:
1. Create send buffer: shm_open returns fd=10, we mmap it, then CLOSE fd=10
2. Create recv buffer: shm_open returns fd=10 AGAIN (recycled!), we mmap it, close it
3. Both buffers show fd=10 in logs, but they're actually DIFFERENT shm objects

The mmap regions remain valid after close, but we lose the ability to verify we're talking to the same shm object.

## The Real Problem

The client and server are creating/opening buffers with the SAME NAME but getting DIFFERENT memory mappings because:
1. They don't share the fd (it's closed immediately)
2. mmap() creates independent mappings even for the same shm object

## Solution

Need to verify that `mmap(MAP_SHARED)` on the same shm_name actually shares memory between processes.
