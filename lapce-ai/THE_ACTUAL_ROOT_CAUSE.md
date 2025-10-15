# ROOT CAUSE: Client and Handler Use DIFFERENT Slots

## The Evidence

```
[WATCHER] NEW lock file detected: slot_933.lock
[ACCEPT SLOT=933] ✓ Handler spawned for slot 933
[HANDLER 1759838651] Stream conn_id=933
[BUFFER READ] 'sock_933_recv' - Empty (correct, no one is writing here)

[CLIENT connect()] Created stream with conn_id=136
[BUFFER WRITE] (presumably to sock_136_recv, but we don't see it)
```

**Handler waits on slot 933. Client writes to slot 136. They never communicate!**

## The Flow

1. Client calls `SharedMemoryStream::connect()`
2. Client creates lock file `/tmp/..._locks/slot_136.lock`
3. Watcher detects lock file `/tmp/..._locks/slot_933.lock` (DIFFERENT!)
4. Server calls `accept()` for slot 933
5. Handler spawns for slot 933
6. Client opens buffers for slot 136
7. **Handler reads slot 933 (empty), Client writes to slot 136 (unseen)**

## Why This Happens

The watcher is detecting STALE lock files from previous test runs, or there's a race where:
- Client creates slot_136.lock
- But watcher had already detected slot_933.lock from earlier
- Accept returns slot 933 to handler
- Client proceeds with slot 136

## The Fix

Need to ensure:
1. Client waits for its specific slot to be accepted
2. OR clean up ALL lock files before test starts
3. OR watcher processes events in strict order and doesn't skip slots
