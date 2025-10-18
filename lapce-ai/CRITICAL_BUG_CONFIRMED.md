# CRITICAL BUG: Stream Buffers Don't Match conn_id

## Evidence

```
[SLOT_POOL] Creating slot 741
[ACCEPT SLOT=741] ✓ Handler will write to 'sock_741_send', read from 'sock_741_recv'
[HANDLER 1759841780] Stream conn_id=741
[CLIENT connect()] Created stream with conn_id=741
[BUFFER WRITE] 'sock_741_recv' - SUCCESS ✓
[BUFFER READ] 'sock_876_recv' - WRONG BUFFER! ✗
```

## The Bug

The handler's `stream.conn_id()` returns 741 (correct), but the actual buffers inside the stream are from slot 876 (wrong!).

This means:
1. `accept()` creates a stream saying it's slot 741
2. The stream's `conn_id` field = 741
3. But the stream's `recv_buffer` and `send_buffer` are actually from slot 876

## Root Cause

In `accept()` we do:
```rust
let slot = self.slot_pool.get_or_create_slot(slot_id).await?;

let stream = SharedMemoryStream {
    send_buffer: slot.send_buffer.clone(),  // ← These are slot 876 buffers!
    recv_buffer: slot.recv_buffer.clone(),  // ← Not slot 741!
    conn_id: slot_id as u64,                // ← But conn_id = 741!
    ...
};
```

`get_or_create_slot(741)` is returning buffers from a DIFFERENT slot (876).

## Investigation

Need to check:
1. Is `get_or_create_slot` returning the wrong slot from the HashMap?
2. Is there a race where slot 741 gets replaced with slot 876?
3. Are we accidentally reusing buffer Arc pointers from a previous slot?
