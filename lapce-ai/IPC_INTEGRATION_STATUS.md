# IPC Integration Test Status

## Current Issue: Server Not Accepting Connections

### Symptoms
- Client successfully creates lock file and "connects"
- Server filesystem watcher running but not detecting client
- `ConnectionStats` shows 0 total/active connections
- Messages fail with "No data received (connection closed)"

### Root Cause Analysis

**Server Accept Flow:**
1. `IpcServer::serve()` loops with 10ms sleep
2. Calls `listener.accept().await`
3. `SharedMemoryListener::accept()` waits on channel populated by watcher
4. Watcher task polls `/tmp/<socket>_locks/` every 1ms for new `.lock` files
5. When found, sends slot_id through channel
6. Accept returns `SharedMemoryStream` for that slot

**Client Connect Flow:**
1. `SharedMemoryStream::connect()` tries random slot IDs
2. Creates lock file: `/tmp/<socket>_locks/slot_<N>.lock`
3. Opens pre-existing buffers: `<socket>_<N>_send` and `<socket>_<N>_recv`
4. Returns stream immediately

**The Gap:**
- Client expects buffers to already exist
- Server only creates buffers AFTER watcher detects lock and `accept()` is called
- But `accept()` returns pre-created slots from SlotPool
- **SlotPool has WARM_POOL_SIZE=0** → No slots pre-created!

### The Fix

Change `WARM_POOL_SIZE` from 0 to 64 in `shared_memory_complete.rs`:
```rust
const WARM_POOL_SIZE: usize = 64;    // Pre-create slots for immediate accept
const MAX_SLOTS: usize = 1000;
```

This ensures slots are ready when client tries to connect, matching the original lock-file design.

### Alternative: On-Demand Slot Creation

Currently watcher does:
```rust
if let Ok(slot_id) = slot_str.parse::<u32>() {
    seen_locks.insert(slot_id);
    let _ = accept_tx.send(slot_id);  // Send immediately
}
```

Should be:
```rust
if let Ok(slot_id) = slot_str.parse::<u32>() {
    if !seen_locks.contains(&slot_id) {
        seen_locks.insert(slot_id);
        // Create slot on-demand before sending
        if slot_pool.get_or_create_slot(slot_id).await.is_ok() {
            let _ = accept_tx.send(slot_id);
        }
    }
}
```

This ensures slot buffers exist before accept() returns them to ConnectionHandler.

---

## Next Steps

1. Fix WARM_POOL_SIZE or on-demand creation
2. Re-run integration test
3. Verify actual round-trips with proper latency
4. Stress test with 1000+ concurrent connections
5. Memory validation under load
6. Node.js comparison
