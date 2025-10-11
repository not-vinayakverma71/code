# Windows Shared Memory Implementation Plan (Production-Grade)

This document specifies how to add a first-class Windows shared memory transport that mirrors our Unix `shared_memory_complete` module. The goal is full parity (API + behavior + performance characteristics) on Windows without compromising the existing Unix implementation.

## Goals and Success Criteria

- API parity with Unix:
  - `SharedMemoryBuffer::{create, open, write, read}`
  - `SharedMemoryListener::{bind, accept}`
  - `SharedMemoryStream` behavior identical
- Performance targets (on modern Windows):
  - Average latency: < 15 μs per message (1 KB)
  - P99 latency: < 50 μs
  - Throughput: > 300K msg/s (1 KB payload)
- Safety and correctness:
  - No handle leaks; all `HANDLE`s closed, views unmapped
  - Zero UB: only `unsafe` for FFI and pointer arithmetic
  - Thread-safe, process-safe lock-free ring buffer (same header)
- CI:
  - Cross-OS tests pass on `windows-latest`
  - Nuclear tests exercise Windows path without flakiness

## Win32 API Mapping (POSIX → Windows)

- Create object:
  - POSIX: `shm_open + ftruncate`
  - Windows: `CreateFileMappingW(INVALID_HANDLE_VALUE, ..., PAGE_READWRITE, sizeHigh, sizeLow, name)`
- Open existing object:
  - POSIX: `shm_open`
  - Windows: `OpenFileMappingW(FILE_MAP_ALL_ACCESS, FALSE, name)`
- Map into process:
  - POSIX: `mmap`
  - Windows: `MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size)`
- Unmap:
  - POSIX: `munmap`
  - Windows: `UnmapViewOfFile(view)`
- Close backing object:
  - POSIX: `close(fd)`
  - Windows: `CloseHandle(handle)`
- Delete object:
  - POSIX: `shm_unlink(name)` (unlink by name)
  - Windows: N/A (object destroyed when last handle closes). Simulate cleanup by unique names + short TTL control scheme.

## Object Naming Strategy (Windows)

- Use the Windows object manager namespaces:
  - Default: `Local\LapceAI_<session>_<boot>_<sanitized_path>`
  - We avoid `Global\` to prevent privilege issues (`SeCreateGlobalPrivilege`) on CI and typical desktops.
- Sanitization:
  - Allow `[A-Za-z0-9_\-]` only; replace others with `_`
  - Cap length at 200 chars to avoid edge cases
- Boot/session components:
  - Session: `ProcessId` + `SessionId` via `ProcessIdToSessionId` for basic deconfliction
  - Boot: derived monotonic tick (`GetTickCount64()` bucketed) to reduce stale collisions (not security-relevant)

Note: Windows named objects do not need unlink; when all handles close, object is torn down. Namespacing + short-lived control channel makes collisions unlikely. For added safety, include a 64-bit random suffix generated at server start.

## Security and ACLs

- V1: Use default DACL by passing `lpSecurityAttributes = NULL`. This allows any process running as the same user/session to open the mapping. This matches our use (Lapce editor ↔ AI engine launched by editor).
- V2 (optional): Custom DACL to support cross-user or service scenarios.

## Memory Layout and Concurrency

- Reuse existing `RingBufferHeader` exactly as-is (`#[repr(C)]`). The header file `ipc/shared_memory_header.rs` is already shared; import it in the Windows module.
- 64-bit only: enforce with `#[cfg(target_pointer_width = "64")]` on Windows to avoid 32-bit surprises.
- The allocation granularity on Windows does not constrain mapping size at offset 0; mapping of any size at offset 0 is fine. We always map from offset 0.

## Module Structure

```
lapce-ai/
  src/ipc/
    shared_memory_complete.rs      # Unix (existing)
    windows_shared_memory.rs       # Windows (new)
    shared_memory_header.rs        # Shared header (already present)
    mod.rs                         # cfg-gated re-exports
```

## Public API Parity (Windows)

- `SharedMemoryBuffer` (Windows)
  - `pub fn create(name: &str, requested_size: usize) -> Result<Self>`
    - `CreateFileMappingW(INVALID_HANDLE_VALUE)` with computed size
    - `MapViewOfFile`
    - Immediately `CloseHandle(mapping_handle)` after successful map (the view keeps a reference internally); or keep handle and close in Drop — both are valid; choose keeping handle in V1 to mirror POSIX close-after-map behavior with clear lifetime. We'll explicitly unmap then close handle in Drop.
  - `pub fn open(name: &str, size_hint: usize) -> Result<Self>`
    - `OpenFileMappingW(FILE_MAP_ALL_ACCESS)` then `MapViewOfFile`
  - `pub fn write(&mut self, data: &[u8]) -> Result<()>` — identical logic
  - `pub fn read(&mut self) -> Option<Vec<u8>>` — identical logic
  - `impl Drop` — `UnmapViewOfFile(view); CloseHandle(mapping_handle);`

- `SharedMemoryListener` (Windows)
  - `pub fn bind(path: &str) -> Result<Self>`
    - Control channel is a `SharedMemoryBuffer` of fixed size (same as Unix `CONTROL_SIZE`)
    - Optional: named event `CreateEventW` for wakeups (V2 optimization). V1: busy/poll like Unix does.
  - `pub async fn accept(&mut self) -> IpcResult<SharedMemoryStream>`
    - Same ring buffer protocol as Unix

- `SharedMemoryStream` (Windows)
  - Holds two `SharedMemoryBuffer`s (send/recv), same as Unix
  - Identical framing and zero-copy semantics using `BinaryCodec`

## Error Handling

- Use `std::io::Error::last_os_error()` after Win32 calls for consistent errors
- Wrap in `anyhow::Result`
- Enrich messages with mapping name and operation for diagnostics

## Testing Strategy (Windows)

- Unit tests:
  - `test_shared_memory_roundtrip_windows` (direct buffer create/open/write/read)
  - Control channel handshake unit test
- Integration tests:
  - Enable Cross-OS suite on Windows (`tests/cross_os_success_criteria.rs`) with Windows-specific variant
  - Nuclear tests: connection bomb, latency torture with reduced parameters on Windows runner to avoid flakiness
- Property tests (optional):
  - Randomized write sizes, wrap-around correctness

## CI Integration

- Update `ipc/mod.rs`:
  - `#[cfg(windows)] pub mod windows_shared_memory;`
  - `#[cfg(windows)] pub use windows_shared_memory::{SharedMemoryBuffer, SharedMemoryListener, SharedMemoryStream};`
- Update `ipc/ipc_server.rs` imports under cfg:
  - `#[cfg(unix)] use super::shared_memory_complete::{...};`
  - `#[cfg(windows)] use super::windows_shared_memory::{...};`
- Re-enable Windows jobs in Cross-OS CI to run shared memory tests

## Performance Considerations (Windows)

- Prefer `FILE_MAP_WRITE | FILE_MAP_READ` access
- Pin threads is unnecessary; let OS scheduler handle
- Consider `WriteCombine` not applicable here
- For ultra-low latency, consider using named events for wakeups (V2)

## Risks and Mitigations

- Name collisions: Mitigated by `Local\` + randomized suffix + sanitized path
- Privileges (Global namespace): Avoided by using `Local\` in V1
- Handle leaks: Covered by Drop + CI leak checks
- 32-bit Windows: Not supported; enforce 64-bit compile

## Migration Plan

1) Windows module compiles and basic roundtrip passes locally
2) Re-enable Windows tests in CI (Cross-OS suite)
3) Restore Windows nuclear tests with tuned parameters
4) Optional: add named events for reduced busy-wait

---

# Implementation TODO (Comprehensive)

- [ ] Create Windows module scaffolding
  - [ ] Add dependency: `windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_System_Memory", "Win32_System_Threading", "Win32_System_SystemInformation", "Win32_Security"] }`
  - [ ] New file: `src/ipc/windows_shared_memory.rs`
  - [ ] Import shared header: `mod shared_memory_header { include!("shared_memory_header.rs"); }`

- [ ] Implement name sanitizer and namespace
  - [ ] `fn sanitize_name(path: &str) -> String` → `Local\\LapceAI_<session>_<rand>_<sanitized>`
  - [ ] Session id via `ProcessIdToSessionId`
  - [ ] Random 64-bit suffix via `rand::rngs::OsRng`

- [ ] Implement `SharedMemoryBuffer` (Windows)
  - [ ] `create()` using `CreateFileMappingW`
  - [ ] `open()` using `OpenFileMappingW`
  - [ ] `map()` using `MapViewOfFile`
  - [ ] Ring buffer init/validate using shared header
  - [ ] `write()` and `read()` identical to Unix
  - [ ] `Drop` uses `UnmapViewOfFile` then `CloseHandle`

- [ ] Implement `SharedMemoryListener` and `SharedMemoryStream`
  - [ ] Control channel create/bind
  - [ ] Accept handshake via control buffer (parity with Unix)
  - [ ] Stream open (send/recv buffers)

- [ ] Wire into IPC server
  - [ ] `ipc/mod.rs` re-exports under `#[cfg(windows)]`
  - [ ] `ipc/ipc_server.rs` dual imports via cfg

- [ ] Tests
  - [ ] `#[cfg(windows)] test_shared_memory_roundtrip_windows` in `tests/cross_os_success_criteria.rs`
  - [ ] Add Windows to nuclear tests (reduced parameters initially)
  - [ ] Leak test: ensure Drop cleans up mapping + views

- [ ] CI updates
  - [ ] Enable Cross-OS tests on Windows for shared memory
  - [ ] Tune timeouts and parameters for Windows runner

- [ ] Documentation
  - [ ] Add section to `MACOS_TESTING.md` covering Windows testing
  - [ ] Update `ARCHITECTURE_INTEGRATION_PLAN.md` to note Windows parity

- [ ] Post-merge hardening (V2)
  - [ ] Optional named events for wakeup (reduce busy/poll)
  - [ ] Optional custom DACL for `Global\` namespace support
  - [ ] Benchmark and tune spin thresholds

## References

- Microsoft Docs:
  - CreateFileMappingW / OpenFileMappingW / MapViewOfFile / UnmapViewOfFile / CloseHandle
  - CreateEventW / SetEvent / WaitForSingleObject (optional V2)
- Rust crates:
  - `windows-sys` (thin FFI bindings, minimal overhead)

---

With this plan, Windows gains a native shared memory transport with API and behavior parity, preserving our architecture’s performance goals while maintaining clean separation via `#[cfg]` gating.
