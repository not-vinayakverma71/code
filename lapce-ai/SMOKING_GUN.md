# SMOKING GUN: Client and Server Use DIFFERENT Shared Memory Buffers

## Evidence

Client writes successfully:
```
[BUFFER WRITE] SUCCESS - Wrote 41 bytes, write_pos now at 45
```

Handler reads from empty buffer:
```
[BUFFER READ] Attempt 0: read_pos=0, write_pos=0
```

**They're reading from DIFFERENT physical shared memory regions!**

## Root Cause

Both `create_blocking()` and `open_blocking()` convert paths to `shm_name`:

```rust
// In create_blocking():
#[cfg(not(target_os = "macos"))]
let namespaced_path = create_namespaced_path(path);
let shm_name_str = format!("/{}", namespaced_path.trim_start_matches('/').replace('/', "_"));

// In open_blocking():
#[cfg(not(target_os = "macos"))]
let namespaced_path = create_namespaced_path(path);  
let shm_name_str = {
    let without_leading = namespaced_path.trim_start_matches('/');
    format!("/{}", without_leading.replace('/', "_"))
};
```

**The problem:** `shm_open()` with `O_RDWR` (no `O_CREAT`) should FAIL if buffer doesn't exist, but it's not failing. This means:

1. Either `shm_open()` is silently creating new buffers (should not happen without `O_CREAT`)
2. Or the paths are subtly different so they create distinct SHM objects

## The Fix

Need to verify `shm_name` is IDENTICAL between create and open. Add logging to both to compare.
