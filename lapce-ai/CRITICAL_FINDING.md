# CRITICAL: Shared Memory Works But Handler Blocks

## Evidence
```
[CREATE fstat] name='sock_316_recv' dev=30 ino=211153  ← Same inode
[OPEN fstat] name='sock_316_recv' dev=30 ino=211153    ← Same inode
[HANDLER] reads from sock_316_recv at 0x7ca872dfb000
[CLIENT] writes to sock_316_recv at 0x7371464f9000
```

Both processes open the SAME shm object (same inode) but get different virtual addresses (expected with mmap).

## The Problem

The handler calls `read_exact()` which calls `recv_buffer.read()` which checks:
```rust
let write_pos = header.write_pos.load(Ordering::Acquire);
```

But this atomic load returns 0 even though the client has written data and updated write_pos to 855.

## Root Cause

This is likely a **cache coherency issue** in single-process tests where:
1. Client and server run as tokio tasks in the SAME process
2. CPU caches the atomic value per-thread
3. Memory barriers aren't sufficient for intra-process shared memory

## Solution

Need to test with actual separate processes (fork or separate binaries) to verify true IPC.
