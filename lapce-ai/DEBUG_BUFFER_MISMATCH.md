# IPC Buffer Mismatch Root Cause

## The Problem

Handler starts, waits for message, but NEVER receives it even though client sends.

## Buffer Flow Analysis

### Server Side (accept)
```rust
// Slot creates buffers:
send_buffer: SharedMemoryBuffer::create(format!("{}_{}_send", path, slot_id))
recv_buffer: SharedMemoryBuffer::create(format!("{}_{}_recv", path, slot_id))

// Server stream gets:
send_buffer: slot.send_buffer  // = <path>_<slot>_send
recv_buffer: slot.recv_buffer  // = <path>_<slot>_recv
```

### Client Side (connect)
```rust
// Client OPENS existing buffers:
send_path = format!("{}_{}_recv", path, slot_id);  // Opens <path>_<slot>_recv
recv_path = format!("{}_{}_send", path, slot_id);  // Opens <path>_<slot>_send

send_buffer = SharedMemoryBuffer::open(send_path)  // <path>_<slot>_recv
recv_buffer = SharedMemoryBuffer::open(recv_path)  // <path>_<slot>_send
```

## Data Flow
1. Client writes to `send_buffer` = `<path>_<slot>_recv`
2. Server handler reads from `recv_buffer` = `<path>_<slot>_recv`
3. **SHOULD WORK!**

## Actual Issue

The problem is that `SharedMemoryBuffer::open()` likely CREATES a NEW buffer instead of opening the existing one created by the server!

When slot pool creates buffers with `SharedMemoryBuffer::create()`, they exist in memory.
When client calls `SharedMemoryBuffer::open()`, it might create a DIFFERENT buffer with same name!

## Solution

Need to check if `SharedMemoryBuffer::open()` actually exists or if we're calling `::create()` which makes a fresh buffer.
