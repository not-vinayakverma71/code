# Full-Stack IPC Integration Test Plan

## Overview
This document describes the complete, production-grade IPC test implementation with **NO MOCKS**.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Lapce UI (lapce-app)                     │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────────────┐  │
│  │ Terminal UI  │→ │TerminalBridge│→ │ BridgeClient           │  │
│  └──────────────┘  └─────────────┘  │ ┌────────────────────┐ │  │
│                                      │ │ ShmTransport       │ │  │
│                                      │ │ ┌────────────────┐ │ │  │
│                                      │ │ │IpcClientVolatile││ │  │
│                                      │ │ └────────────────┘ │ │  │
│                                      │ └────────────────────┘ │  │
│                                      └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              ↕ Unix Domain Socket
┌─────────────────────────────────────────────────────────────────┐
│                   Lapce AI Backend (lapce-ai)                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ IpcServer                                                   │  │
│  │ ┌─────────────────────────────────────────────────────────┐ │  │
│  │ │ SharedMemoryListener                                     │ │  │
│  │ │  → Accepts connections                                   │ │  │
│  │ │  → BinaryCodec decode                                    │ │  │
│  │ │  → Routes to handlers                                    │ │  │
│  │ │  → Encode response                                       │ │  │
│  │ └─────────────────────────────────────────────────────────┘ │  │
│  │                                                               │  │
│  │ Registered Handlers:                                         │  │
│  │  - Context routes (truncate, condense, track files)         │  │
│  │  - Provider routes (list models, stream completions)        │  │
│  │  - Tool routes (execute tools, stream results)              │  │
│  │  - Terminal routes (command lifecycle, output)              │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Test Suite: `integration_test.rs`

### Test 1: Server Startup
**Purpose**: Verify IPC server can bind to socket

```rust
async fn test_01_server_startup()
```

**Steps**:
1. Clean up any existing socket file
2. Start `IpcServer::new(socket_path).await`
3. Call `server.serve().await` in background task
4. Verify socket file exists
5. Cleanup

**Expected**: Socket file created, server listening

---

### Test 2: Client Connection  
**Purpose**: Verify client can establish IPC connection

```rust
async fn test_02_client_connection()
```

**Steps**:
1. Start server
2. Create `ShmTransport::new(socket_path)`
3. Call `transport.connect()`
4. Verify connection succeeds
5. Disconnect and cleanup

**Expected**: Connection established, no errors

---

### Test 3: Message Roundtrip
**Purpose**: End-to-end message send/receive

```rust
async fn test_03_message_roundtrip()
```

**Steps**:
1. Start server with echo handler
2. Connect client
3. Send `OutboundMessage::TerminalCommandStarted`
4. Wait for response in inbound queue
5. Verify message content matches

**Expected**: Message serialized, sent, processed, response received

---

### Test 4: Terminal Bridge Integration
**Purpose**: Test complete terminal event flow

```rust
async fn test_04_terminal_bridge_integration()
```

**Steps**:
1. Start server
2. Create `TerminalBridge`
3. Send sequence of events:
   - `send_command_started()`
   - `send_command_completed()`
   - `send_output_chunk()`
   - `send_injection_result()`
4. Verify all succeed

**Expected**: All terminal events processed correctly

---

### Test 5: Concurrent Connections
**Purpose**: Test server handles multiple simultaneous clients

```rust
async fn test_05_concurrent_connections()
```

**Steps**:
1. Start server
2. Spawn 5 concurrent client tasks
3. Each client connects and sends message
4. Wait for all to complete

**Expected**: All 5 clients succeed, no race conditions

---

### Test 6: Connection Recovery
**Purpose**: Test disconnect and reconnect flow

```rust
async fn test_06_connection_recovery()
```

**Steps**:
1. Connect client
2. Disconnect explicitly
3. Reconnect
4. Verify connection re-established

**Expected**: Reconnection works, no stale state

---

## Running the Tests

### Prerequisites
```bash
# Ensure dependencies are up to date
cargo update

# Build lapce-ai backend first
cargo build --lib -p lapce-ai-rust

# Build lapce-app with IPC support
cargo build --lib -p lapce-app
```

### Execute Full Suite
```bash
# Run all IPC integration tests
cargo test --lib -p lapce-app ai_bridge::integration_test -- --nocapture

# Run specific test
cargo test --lib -p lapce-app test_04_terminal_bridge_integration -- --nocapture
```

### Expected Output
```
🧪 TEST 1: IPC Server Startup
✅ Server started and socket created

🧪 TEST 2: Client Connection
✅ Client connected successfully

🧪 TEST 3: Message Roundtrip
✅ Message sent successfully

🧪 TEST 4: Terminal Bridge Integration
✅ Command started event sent
✅ Command completed event sent
✅ Output chunk sent
✅ Injection result sent

🧪 TEST 5: Concurrent Connections
  Client 0 sent message
  Client 1 sent message
  Client 2 sent message
  Client 3 sent message
  Client 4 sent message
✅ All 5 clients completed successfully

🧪 TEST 6: Connection Recovery
✅ Initial connection successful
✅ Disconnected successfully
✅ Reconnection successful

📊 ========== FULL-STACK IPC TEST SUMMARY ==========
✅ All integration tests validate:
  1. IPC server startup and socket creation
  2. Client connection establishment
  3. Message serialization and roundtrip
  4. Terminal bridge event flow
  5. Concurrent client handling
  6. Connection recovery (disconnect/reconnect)

🎉 FULL IPC STACK VALIDATED
====================================================
```

## Key Differences from Mock Tests

### Mock Approach ❌
```rust
// Fake transport that doesn't actually send
struct NoTransport {}
impl Transport for NoTransport {
    fn send(&self, _msg: OutboundMessage) -> Result<(), BridgeError> {
        Ok(()) // Does nothing!
    }
}
```

### Real Approach ✅
```rust
// Actual IPC with Unix domain sockets
struct ShmTransport {
    client: Arc<Mutex<Option<IpcClientHandle>>>,
    // ...
}

impl Transport for ShmTransport {
    fn send(&self, message: OutboundMessage) -> Result<(), BridgeError> {
        let serialized = serde_json::to_vec(&message)?;
        let ipc_client = handle.client.clone();
        // REAL IPC CALL:
        let response = runtime.block_on(async move {
            ipc_client.send_bytes(&serialized).await
        })?;
        // Process actual response
        Ok(())
    }
}
```

## Debugging

### Enable Verbose Logging
```bash
RUST_LOG=debug cargo test --lib test_03_message_roundtrip -- --nocapture
```

### Check Socket Files
```bash
# List IPC sockets
ls -la /tmp/lapce-ai-test-*.sock

# Monitor socket activity
sudo lsof -U | grep lapce-ai-test
```

### Server Logs
The IPC server prints diagnostic info:
```
[SHM_TRANSPORT] Connecting to: /tmp/lapce-ai-test-03.sock
[TEST] IPC server starting on /tmp/lapce-ai-test-03.sock
[HANDLER 1] Stream conn_id=0
[HANDLER 1] Got message: 142 bytes
```

## Performance Benchmarks

Expected latencies (on reasonable hardware):

| Operation | Target | Acceptable |
|-----------|--------|------------|
| Connection establishment | < 10ms | < 50ms |
| Single message roundtrip | < 5ms | < 20ms |
| 5 concurrent clients | < 50ms | < 200ms |
| Disconnect/reconnect | < 15ms | < 100ms |

## Troubleshooting

### Test hangs on connection
**Cause**: Server not started or socket path mismatch  
**Fix**: Verify socket path consistent, check server logs

### "Address already in use"
**Cause**: Previous test didn't clean up socket  
**Fix**: `rm /tmp/lapce-ai-test-*.sock` before rerun

### Compilation errors
**Cause**: Dependency version conflicts  
**Fix**: Update git2 to 0.20, tree-sitter to 0.23

### "Connection refused"
**Cause**: Server crashed during startup  
**Fix**: Check server logs for panic/error

## Next Steps

1. ✅ Restore full ShmTransport implementation
2. ✅ Create integration test suite
3. ⏳ Compile and verify (in progress)
4. ⏳ Run tests and fix any issues
5. ⏳ Add streaming handler tests
6. ⏳ Add context/provider route tests
7. ⏳ Performance profiling

## Success Criteria

- [ ] All 6 tests pass reliably
- [ ] No memory leaks (valgrind clean)
- [ ] Latencies within target ranges
- [ ] Handles connection failures gracefully
- [ ] Server supports 100+ concurrent connections
- [ ] Message throughput > 1000 msg/sec

---

**Status**: Implementation complete, awaiting compilation & test execution
**Last Updated**: 2025-10-18 09:42 IST
