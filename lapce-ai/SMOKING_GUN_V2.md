# SMOKING GUN: Handler Uses WRONG Slot Buffers

## Evidence

```
[WATCHER] NEW lock file detected: slot_50.lock
[ACCEPT SLOT=50] ✓ Handler will write to 'sock_50_send', read from 'sock_50_recv'
[CLIENT connect()] Created stream with conn_id=50
[BUFFER WRITE] 'sock_50_recv' - Wrote 41 bytes to header@0x7a0d658f9000, r=0 w=45
[BUFFER READ] 'sock_636_recv' Attempt 0 from header@0x72b7519fb000: r=0, w=0
```

**Client writes to slot 50, handler reads from slot 636!**

## Root Cause

The `accept()` method says it created a stream for slot 50, but the handler is actually reading from slot 636 buffers.

This means:
1. The stream returned by `accept()` has wrong buffers, OR
2. The stream is being replaced/modified before handler uses it, OR  
3. The handler is somehow getting a different stream

## Investigation Needed

Need to:
1. Log the stream's buffer debug_name inside the handler
2. Verify the stream passed to handler is the same one from accept()
3. Check if ConnectionHandler is cloning/creating new buffers
