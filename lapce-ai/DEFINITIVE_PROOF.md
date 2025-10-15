# DEFINITIVE PROOF: Different Physical Memory

## Evidence

**Writer (client) sees:**
```
[BUFFER WRITE] w=45
[BUFFER WRITE] RAW MEMORY: raw_bytes=[45, 0, 0, 0, 0, 0, 0, 0], raw_value=45
```

**Reader (handler) sees:**
```
[BUFFER READ] w=0
[BUFFER READ] RAW MEMORY: raw_bytes=[0, 0, 0, 0, 0, 0, 0, 0], raw_value=0
```

## What This Proves

Both processes read the **raw physical memory** at the same offset in their mmap region. The writer's memory contains `45`, but the reader's memory contains `0`.

This is NOT a cache coherency issue or atomic ordering issue. The actual bytes in physical RAM are different between the two processes.

## Root Cause

They must be accessing DIFFERENT shared memory objects. Possibilities:

1. **Race condition**: Server creates `sock_X_recv`, then client ALSO creates `sock_X_recv` which overwrites/replaces it
2. **Double shm_open with O_CREAT**: If client calls `shm_open(..., O_CREAT | O_RDWR, ...)` it might recreate the object
3. **shm_unlink between create and open**: Something is unlinking the shm object

## Investigation

Need to check if the client's buffer opening code is using `O_CREAT` flag when it should use `O_RDWR` only.
