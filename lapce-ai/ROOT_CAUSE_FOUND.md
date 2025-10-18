# ROOT CAUSE: Client and Handler Use DIFFERENT Slot IDs

## Evidence

**Client writes:**
```
[BUFFER WRITE] '/tmp/lapce-ipc-integration-test.sock_350_recv' 
```

**Handler reads:**
```
[BUFFER READ] '/tmp/lapce-ipc-integration-test.sock_772_recv'
```

**350 ≠ 772** - They're accessing COMPLETELY DIFFERENT buffers!

## Flow Analysis

Expected:
1. Client creates lock file for slot N
2. Watcher detects slot N, creates buffers
3. accept() returns stream for slot N
4. Client opens buffers for slot N
5. **BOTH use slot N**

Actual:
1. Client creates lock for one slot
2. Server accepts THAT slot
3. But somewhere the slot IDs diverge
4. Client sends to slot 350, handler reads from slot 772

## Hypothesis

The test might be:
- Starting server which pre-populates some slots
- Then client connects with a different random slot
- Handler is stuck reading from a stale slot

OR the SharedMemoryStream created in accept() is using wrong buffers.

Need to trace EXACT slot ID from lock file creation → watcher → accept → handler usage.
