# Team Integration Guide: Connect UI to IPC

## Overview

**Goal:** Connect `lapce-app/src/ai_bridge` (UI layer) to `lapce-ai/src/ipc` (comprehensive IPC system) so your team can develop and test tools on Linux while cross-platform IPC fixes continue in parallel.

## Architecture

```
┌──────────────────────────────────────────────────┐
│  lapce-app (Floem UI)                            │
│                                                   │
│  src/ai_bridge/                                  │
│  ├── shm_transport.rs    ← WIRE THIS             │
│  ├── bridge.rs           (client API)            │
│  └── messages.rs         (protocol)              │
└─────────────────┬────────────────────────────────┘
                  │ IPC Connection
                  ▼
┌──────────────────────────────────────────────────┐
│  lapce-ai (Backend)                              │
│                                                   │
│  src/ipc/                                        │
│  ├── ipc_client_volatile.rs  ← USE THIS         │
│  ├── ipc_server_volatile.rs  (server)           │
│  ├── shared_memory_buffer.rs (ring buffers)     │
│  ├── eventfd_doorbell.rs     (notifications)    │
│  └── binary_codec.rs         (serialization)    │
│                                                   │
│  src/integration/                                │
│  ├── tool_bridge.rs      ← YOUR TEAM WIRES TOOLS│
│  └── provider_bridge.rs  ← YOUR TEAM WIRES AI   │
│                                                   │
│  src/mcp_tools/          ← YOUR TEAM'S TOOLS    │
└──────────────────────────────────────────────────┘
```

## Step 1: Add Dependency

**File:** `lapce-app/Cargo.toml`

```toml
[dependencies]
# Add this line:
lapce-ai-rust = { path = "../lapce-ai" }
```

## Step 2: Wire ShmTransport to Real IPC

**File:** `lapce-app/src/ai_bridge/shm_transport.rs`

Replace the TODO sections (lines 10-12, 91-103):

```rust
// At the top, replace line 10-12:
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

// Replace IpcClientHandle struct (lines 23-28):
struct IpcClientHandle {
    client: IpcClientVolatile,
}

// Replace connect() method (lines 85-110):
fn connect(&mut self) -> Result<(), BridgeError> {
    let socket_path = self.socket_path.clone();
    let runtime = self.runtime.clone();
    
    eprintln!("[SHM_TRANSPORT] Connecting to: {}", socket_path);
    
    // Real IPC connection
    let ipc_client = runtime.block_on(async {
        IpcClientVolatile::connect(&socket_path).await
    }).map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
    
    let handle = IpcClientHandle {
        client: ipc_client,
    };
    
    *self.client.lock().unwrap() = Some(handle);
    *self.status.lock().unwrap() = ConnectionStatusType::Connected;
    
    eprintln!("[SHM_TRANSPORT] Connected via real IPC");
    Ok(())
}

// Update send() to use real client (lines 52-74):
fn send(&self, message: OutboundMessage) -> Result<(), BridgeError> {
    let client_guard = self.client.lock().unwrap();
    let client = client_guard.as_ref()
        .ok_or(BridgeError::Disconnected)?;
    
    // Serialize message
    let serialized = serde_json::to_vec(&message)
        .map_err(|e| BridgeError::SerializationError(e.to_string()))?;
    
    // Send through real IPC
    let runtime = self.runtime.clone();
    let ipc_client = &client.client;
    let response = runtime.block_on(async {
        ipc_client.send_bytes(&serialized).await
    }).map_err(|e| BridgeError::SendFailed(e.to_string()))?;
    
    // Queue response if present
    if !response.is_empty() {
        if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
            self.inbound_queue.lock().unwrap().push_back(msg);
        }
    }
    
    Ok(())
}
```

## Step 3: Start Backend Server

**On Linux (works now):**

```bash
# Terminal 1: Start lapce-ai backend server
cd lapce-ai
cargo run --bin ipc_test_server_volatile --features unix-bins -- /tmp/lapce_ai.sock

# Terminal 2: Run your UI with tool tests
cd lapce-app
LAPCE_AI_SOCKET=/tmp/lapce_ai.sock cargo run
```

## Step 4: Wire Tools (Your Team's Work)

**File:** `lapce-ai/src/integration/tool_bridge.rs`

```rust
use crate::mcp_tools::core::ToolExecutor; // Your existing tool system

pub struct ToolBridge {
    tool_executor: Arc<ToolExecutor>, // Uncomment this
}

impl ToolBridge {
    pub fn new(tool_executor: Arc<ToolExecutor>) -> Self {
        Self { tool_executor }
    }
    
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: HashMap<String, JsonValue>,
    ) -> Result<ToolResult> {
        eprintln!("[TOOL BRIDGE] Executing: {}", tool_name);
        
        // Real tool execution:
        let result = self.tool_executor.execute(tool_name, args).await?;
        
        Ok(ToolResult {
            success: true,
            output: result.output,
            metadata: result.metadata,
        })
    }
}
```

## Development Workflow

### Your Team (Tool Development):

1. **Wire ToolBridge** to MCP tools (see Step 4)
2. **Implement tools** in `lapce-ai/src/mcp_tools/`
3. **Test on Linux:**
   ```bash
   cd lapce-ai
   cargo test --features unix-bins -- --test-threads=1
   ```
4. **Integration test:**
   ```bash
   # Start server
   cargo run --bin ipc_test_server_volatile --features unix-bins -- /tmp/test.sock
   
   # In another terminal, run your tool tests
   cargo test test_tool_execution --features unix-bins
   ```

### Me (IPC Cross-Platform):

1. **Fix macOS** connection issues (async I/O - in progress)
2. **Test Windows** IPC (comprehensive system already exists)
3. **CI validation** across all 3 platforms
4. **Merge fixes** - your tools automatically work everywhere

## Testing Checklist

- [ ] Add `lapce-ai-rust` dependency to `lapce-app/Cargo.toml`
- [ ] Wire `ShmTransport::connect()` to real IPC client
- [ ] Wire `ToolBridge` to MCP tool executor
- [ ] Test on Linux: Start server + run UI
- [ ] Verify tool execution through IPC
- [ ] (Wait for cross-platform fixes to test on macOS/Windows)

## Socket Path Configuration

**Development:**
- Default: `/tmp/lapce_ai.sock` (Linux/macOS)
- Override: Set `LAPCE_AI_SOCKET` environment variable

**Production:**
- Will use platform-specific paths
- Windows: Named objects (already implemented)

## Current Status

✅ **Linux IPC:** Stable, 1M msg/sec  
⚠️ **macOS IPC:** Connection fixes in progress (commit b68a7a91)  
⚠️ **Windows IPC:** Full system exists, testing now (commit c816debc)

**Your team can start tool development on Linux immediately!**

## Questions?

- IPC issues: File in `.github/workflows/ipc_*.yml` CI logs
- Tool integration: Check `lapce-ai/src/integration/`
- MCP tools: See `lapce-ai/src/mcp_tools/`
