# Cross-Platform IPC Architecture Design

## Objective
Implement high-performance IPC that works on **Linux, macOS, and Windows** using platform-optimized primitives while maintaining a unified API.

## Current Blocker Analysis

### What Works
✅ EventFD doorbells on Linux (145µs latency - 80x faster than polling!)
✅ FD passing via SCM_RIGHTS on Unix
✅ Shared memory mapping on all platforms
✅ Binary message protocol

### What's Broken
❌ **Cross-process atomic synchronization** - Rust volatile atomics don't provide cache coherency between processes
❌ **Platform lock-in** - EventFD is Linux-only
❌ **No macOS/Windows support** - Current implementation won't compile/run on other platforms

## Cross-Platform IPC Strategy

### Architecture: Platform-Specific Optimizations with Unified API

```rust
pub trait IpcTransport {
    fn send(&self, data: &[u8]) -> Result<()>;
    fn recv(&self, buf: &mut Vec<u8>, timeout_ms: i32) -> Result<usize>;
}

// Linux: EventFD + Futex + Shared Memory
struct LinuxIpcTransport { ... }

// macOS: Kqueue + POSIX Semaphores + Shared Memory  
struct MacOsIpcTransport { ... }

// Windows: IOCP + Windows Events + Shared Memory
struct WindowsIpcTransport { ... }
```

## Platform-Specific Implementations

### 1. Linux (Current - Needs Futex Fix)

**Components:**
- **Notification:** EventFD (kernel-level event notification)
- **Synchronization:** Linux Futex (proper cross-process atomics)
- **Memory:** POSIX shm_open + mmap
- **Control:** Unix domain sockets with SCM_RIGHTS

**Ring Buffer Synchronization:**
```rust
// Use futex for cross-process atomic operations
unsafe {
    let futex_addr = &header.write_pos as *const u32;
    syscall(SYS_futex, futex_addr, FUTEX_WAKE, 1, ...);
}
```

**Status:** ⚠️ BLOCKED - Need to replace volatile atomics with futex

### 2. macOS (New Implementation Needed)

**Components:**
- **Notification:** Kqueue (macOS equivalent of epoll/eventfd)
- **Synchronization:** POSIX semaphores (`sem_open` with `O_CREAT`)
- **Memory:** POSIX shm_open + mmap (same as Linux)
- **Control:** Unix domain sockets with SCM_RIGHTS (same as Linux)

**Ring Buffer Synchronization:**
```rust
// Named semaphores for cross-process sync
let sem = unsafe { 
    sem_open(
        b"/ipc_slot_0_sem\0".as_ptr() as *const i8,
        O_CREAT,
        0o644,
        0
    )
};

// Signal: sem_post(sem)
// Wait: sem_wait(sem) or sem_timedwait(sem)
```

**Implementation Plan:**
1. Create `macos_ipc_transport.rs`
2. Use `kqueue` with `EVFILT_USER` for notifications
3. Use named POSIX semaphores for ring buffer sync
4. Reuse Unix domain socket control logic

### 3. Windows (New Implementation Needed)

**Components:**
- **Notification:** Windows Events (`CreateEventW`, `SetEvent`, `WaitForSingleObject`)
- **Synchronization:** Windows Mutexes + Semaphores (kernel objects, cross-process by default)
- **Memory:** `CreateFileMappingW` + `MapViewOfFile`
- **Control:** Named pipes (`CreateNamedPipeW`)

**Ring Buffer Synchronization:**
```rust
// Windows kernel objects for cross-process sync
let mutex = CreateMutexW(
    &mut SECURITY_ATTRIBUTES { 
        bInheritHandle: TRUE,
        ... 
    },
    FALSE,
    w!("Global\\IPC_Slot_0_Mutex")
);

let event = CreateEventW(
    &mut SECURITY_ATTRIBUTES { ... },
    FALSE,  // auto-reset
    FALSE,  // initially non-signaled
    w!("Global\\IPC_Slot_0_Event")
);
```

**Implementation Plan:**
1. Create `windows_ipc_transport.rs`
2. Use Windows Events for notifications
3. Use Windows Mutexes for critical sections
4. Use Windows Semaphores for ring buffer counters
5. Use Named Pipes for control channel

## Unified Cross-Platform Ring Buffer Design

### Problem: Current Design Uses Rust Atomics (Broken for IPC)

**Current (Broken):**
```rust
struct VolatileRingHeader {
    read_pos: u32,   // volatile read/write
    write_pos: u32,  // volatile read/write  
    capacity: u32,
}
// ❌ Volatile doesn't guarantee cross-process cache coherency
```

### Solution: Platform-Specific Synchronization Primitives

**New Design:**
```rust
pub struct CrossProcessRingBuffer {
    // Platform-agnostic header
    header: *mut RingBufferHeader,
    data: *mut u8,
    
    // Platform-specific synchronization
    #[cfg(target_os = "linux")]
    sync: LinuxSync,
    
    #[cfg(target_os = "macos")]
    sync: MacOsSync,
    
    #[cfg(target_os = "windows")]
    sync: WindowsSync,
}

struct RingBufferHeader {
    read_pos: u32,
    write_pos: u32,
    capacity: u32,
    // NO atomics here - use platform primitives
}

#[cfg(target_os = "linux")]
struct LinuxSync {
    futex_addr: *const u32,  // Points to write_pos for futex ops
}

#[cfg(target_os = "macos")]
struct MacOsSync {
    sem: *mut libc::sem_t,  // POSIX semaphore
}

#[cfg(target_os = "windows")]
struct WindowsSync {
    event: HANDLE,  // Windows Event
    mutex: HANDLE,  // Windows Mutex
}
```

## Implementation Phases

### Phase 1: Fix Linux Implementation (CRITICAL)
**Priority: HIGH - Blocking all testing**

**Tasks:**
1. ✅ Replace volatile atomics with Linux futex syscalls
2. ✅ Implement proper FUTEX_WAIT/FUTEX_WAKE for ring buffer
3. ✅ Keep eventfd for notifications (it works!)
4. ✅ Test with 1000 concurrent clients
5. ✅ Measure memory baseline and performance

**Files to modify:**
- `src/ipc/shm_buffer_volatile.rs` - Replace atomics with futex
- `src/ipc/eventfd_doorbell.rs` - Keep as-is (works perfectly)
- Add `src/ipc/futex.rs` - Linux futex wrapper

**Estimated time:** 1-2 days

### Phase 2: Implement macOS Support
**Priority: MEDIUM**

**Tasks:**
1. ✅ Create `src/ipc/macos_ipc_transport.rs`
2. ✅ Implement kqueue-based notifications
3. ✅ Use POSIX semaphores for synchronization
4. ✅ Reuse Unix socket control channel
5. ✅ Test on macOS with concurrent clients

**New files:**
- `src/ipc/macos_ipc_transport.rs`
- `src/ipc/kqueue_doorbell.rs`
- `src/ipc/posix_sem_sync.rs`

**Estimated time:** 3-4 days

### Phase 3: Implement Windows Support
**Priority: MEDIUM**

**Tasks:**
1. ✅ Create `src/ipc/windows_ipc_transport.rs`
2. ✅ Implement Windows Events for notifications
3. ✅ Use Windows kernel objects for synchronization
4. ✅ Implement named pipes for control channel
5. ✅ Test on Windows with concurrent clients

**New files:**
- `src/ipc/windows_ipc_transport.rs`
- `src/ipc/windows_events.rs`
- `src/ipc/windows_named_pipe.rs`

**Estimated time:** 4-5 days

### Phase 4: Unified API and Testing
**Priority: HIGH**

**Tasks:**
1. ✅ Create unified `IpcTransport` trait
2. ✅ Platform-specific implementations behind feature flags
3. ✅ Cross-platform integration tests
4. ✅ Performance benchmarks on all platforms
5. ✅ Documentation and examples

**Estimated time:** 2-3 days

## Performance Expectations

### Linux (EventFD + Futex)
- **Latency:** ~100-200µs (already proven!)
- **Throughput:** >50k messages/sec
- **CPU:** Near-zero when idle (event-driven)

### macOS (Kqueue + POSIX Semaphores)
- **Latency:** ~200-500µs (kqueue slightly slower than eventfd)
- **Throughput:** >30k messages/sec
- **CPU:** Near-zero when idle

### Windows (Events + Kernel Objects)
- **Latency:** ~500-1000µs (Windows kernel objects have more overhead)
- **Throughput:** >20k messages/sec
- **CPU:** Low when idle

## Migration Path

### Step 1: Fix Linux (NOW)
```rust
// Replace this:
header.load_write_pos()  // Broken

// With this:
futex_wait(&header.write_pos, expected_value, timeout)
futex_wake(&header.write_pos, num_waiters)
```

### Step 2: Abstract Platform Layer
```rust
#[cfg(target_os = "linux")]
pub use linux_ipc_transport::IpcTransport;

#[cfg(target_os = "macos")]
pub use macos_ipc_transport::IpcTransport;

#[cfg(target_os = "windows")]
pub use windows_ipc_transport::IpcTransport;
```

### Step 3: Test All Platforms
```bash
# Linux
cargo test --features linux

# macOS  
cargo test --features macos

# Windows
cargo test --features windows
```

## Code Structure

```
lapce-ai/src/ipc/
├── mod.rs                          # Unified IPC API
├── transport.rs                    # IpcTransport trait
│
├── linux/
│   ├── mod.rs
│   ├── eventfd_doorbell.rs        # ✅ WORKS
│   ├── futex.rs                   # 🔧 NEEDS IMPLEMENTATION
│   ├── linux_ipc_transport.rs     # 🔧 NEEDS FUTEX FIX
│   └── shm_buffer_futex.rs        # 🔧 REPLACE VOLATILE WITH FUTEX
│
├── macos/
│   ├── mod.rs
│   ├── kqueue_doorbell.rs         # 📝 NEW
│   ├── posix_sem_sync.rs          # 📝 NEW
│   └── macos_ipc_transport.rs     # 📝 NEW
│
├── windows/
│   ├── mod.rs
│   ├── windows_events.rs          # 📝 NEW
│   ├── windows_sync.rs            # 📝 NEW
│   ├── windows_named_pipe.rs      # 📝 NEW
│   └── windows_ipc_transport.rs   # 📝 NEW
│
└── shared/
    ├── binary_codec.rs            # ✅ Platform-agnostic
    ├── message.rs                 # ✅ Platform-agnostic
    └── ring_buffer.rs             # 🔧 NEEDS PLATFORM ABSTRACTION
```

## Critical Next Step

**IMMEDIATE ACTION REQUIRED:** Fix Linux futex implementation

The eventfd integration proved the design works (145µs latency is excellent!). The ONLY blocker is cross-process atomic synchronization. Once we fix Linux with futex, we can:

1. Complete 1000 client stress test ✅
2. Measure memory baseline ✅
3. Run full load testing ✅
4. Then port to macOS ✅
5. Then port to Windows ✅

**Focus on Linux futex fix FIRST** - it unblocks everything else.
