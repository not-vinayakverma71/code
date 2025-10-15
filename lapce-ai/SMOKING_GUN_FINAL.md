# SMOKING GUN: Different Virtual Memory Regions = Different Shm Objects

## The Evidence

**Writer (client):**
```
[BUFFER WRITE] MMAP base_ptr=0x7371464f9000
[BUFFER WRITE] RAW MEMORY: raw_value=45
```

**Reader (handler):**
```
[BUFFER READ] MMAP base_ptr=0x7ca872dfb000
[BUFFER READ] RAW MEMORY: raw_value=0
```

## What This Proves

They have **completely different mmap base addresses**:
- Writer: `0x7371464f9000`
- Reader: `0x7ca872dfb000`

This means they called `mmap()` on **different file descriptors** from `shm_open()`, which means they opened **different shared memory objects**, despite using the same name string.

## The Root Cause

The only way this can happen:
1. Server creates shm object `/tmp_..._recv-aa39e0f7`
2. Something deletes/unlinks it
3. Client creates NEW shm object `/tmp_..._recv-aa39e0f7` (same name, different object)
4. Both mmap their respective objects
5. They see different memory because they're in different physical pages

## Next Step

Check if there's a Drop/cleanup that calls `shm_unlink` prematurely, or if the server's buffer creation is being undone before the client opens it.
