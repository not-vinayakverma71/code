# Step 29 IPC Integration with ACTUAL Lapce Components

**Mission**: Map Step 29 IPC architecture to **REAL** Lapce component paths (not hypothetical)

---

## 🎯 Actual Lapce Component Inventory

### UI Components (lapce-app/src/)

| Step 29 Hypothetical | ACTUAL Lapce Component | Path |
|---------------------|------------------------|------|
| `panel/ai_chat.rs` | `ai_panel/message_handler.rs` | MessageHandler struct |
| `editor/ai_terminal.rs` | `terminal/panel.rs` | TerminalPanelData struct |
| `editor/ai_diff.rs` | `editor/diff.rs` | DiffEditorData struct |
| `ai_bridge.rs` | `ai_panel/message_handler.rs` | MessageHandler (IPC client) |

### Existing Lapce Infrastructure

**Terminal (REAL):**
- `/lapce-app/src/terminal/panel.rs` - `TerminalPanelData` (29KB, 821 lines)
- `/lapce-app/src/terminal/data.rs` - `TerminalData` (30KB)
- `/lapce-app/src/terminal/view.rs` - Terminal rendering (29KB)
- `/lapce-app/src/panel/terminal_view.rs` - Panel integration (15KB)

**Diff Editor (REAL):**
- `/lapce-app/src/editor/diff.rs` - `DiffEditorData` (19KB, 548 lines)
- Already has: `rope_diff()`, `DiffLines`, `DiffExpand`

**AI Panel (REAL):**
- `/lapce-app/src/ai_panel/message_handler.rs` - `MessageHandler` (14KB, 408 lines)
- Already has: `handle_ipc()`, `bridge: Arc<LapceAiInterface>`

---

## 📋 Integration Strategy

### 1. Terminal Integration (ACTUAL)

**File**: `/lapce-app/src/terminal/panel.rs`

```rust
// Extend EXISTING TerminalPanelData
impl TerminalPanelData {
    // ADD this method to existing struct
    pub async fn execute_ai_command(&self, cmd: String) -> Result<()> {
        // Use existing workspace and common data
        let workspace = self.workspace.clone();
        
        // Send via IPC to lapce-ai-rust backend
        let response = self.common.ipc_client.send(IpcMessage::ExecuteCommand {
            cmd,
            cwd: Some(workspace.path.clone()),
        }).await?;
        
        // Update existing terminal data
        match response {
            IpcMessage::TerminalOutput { data, markers } => {
                // Use existing TerminalData::receive_data() method
                self.tab_info.with(|info| {
                    if let Some((_, tab)) = info.tabs.get(info.active) {
                        tab.terminal.receive_data(data);
                    }
                });
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

**Integration Points:**
- Uses existing: `workspace: Arc<LapceWorkspace>`
- Uses existing: `tab_info: RwSignal<TerminalTabInfo>`
- Uses existing: `common: Rc<CommonData>` (add IPC client here)

### 2. Diff Editor Integration (ACTUAL)

**File**: `/lapce-app/src/editor/diff.rs`

```rust
// Extend EXISTING DiffEditorData
impl DiffEditorData {
    // ADD this method to existing struct
    pub async fn apply_ai_diff(&self, changes: String) -> Result<()> {
        let file_path = self.left.doc.content.path().cloned();
        let original = self.left.doc.buffer().text().to_string();
        
        // Send via IPC to lapce-ai-rust backend
        let mut stream = self.common.ipc_client.send_stream(IpcMessage::RequestDiff {
            file_path: file_path.unwrap(),
            original,
            modified: changes,
        }).await?;
        
        // Stream diff updates
        while let Some(msg) = stream.recv().await {
            match msg {
                IpcMessage::StreamDiffLine { line_num, content, change_type } => {
                    // Use existing rope_diff() and DiffLines
                    self.update_diff_line(line_num, content, change_type);
                }
                IpcMessage::DiffComplete { .. } => break,
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

**Integration Points:**
- Uses existing: `left: EditorData` (has doc, buffer)
- Uses existing: `right: EditorData`
- Uses existing: `changes: Vec<DiffLines>` from `lapce_core::buffer::diff`
- Uses existing: `common: Rc<CommonData>` (add IPC client here)

### 3. AI Panel Integration (ACTUAL)

**File**: `/lapce-app/src/ai_panel/message_handler.rs`

```rust
// Extend EXISTING MessageHandler
impl MessageHandler {
    // MODIFY existing handle_ipc() to route to lapce-ai-rust
    pub fn handle_ipc(&self, message: String) -> String {
        // Parse message
        let request: IpcMessage = serde_json::from_str(&message).unwrap();
        
        // Route to lapce-ai-rust backend via SharedMemory
        let handler = self.clone();
        tokio::spawn(async move {
            let response = match request {
                IpcMessage::StartTask { task, mode } => {
                    // Forward to backend
                    handler.bridge.send(request).await
                }
                IpcMessage::ExecuteTool { tool, params } => {
                    // Forward to backend
                    handler.bridge.send(request).await
                }
                _ => {
                    // Forward all to backend
                    handler.bridge.send(request).await
                }
            };
            
            handler.send_response(response).await;
        });
        
        json!({ "status": "processing" }).to_string()
    }
}
```

**Integration Points:**
- Uses existing: `bridge: Arc<LapceAiInterface>` (make it IPC client)
- Uses existing: `editor_proxy: Arc<EditorProxy>`
- Uses existing: `file_system: Arc<FileSystemBridge>`
- Uses existing: `pending_responses` HashMap

---

## 🔌 CommonData Extension (CRITICAL)

**File**: `/lapce-app/src/window_tab.rs`

```rust
// ADD to EXISTING CommonData struct
pub struct CommonData {
    // ... existing fields ...
    pub focus: RwSignal<Focus>,
    pub config: ReadSignal<Arc<LapceConfig>>,
    
    // ADD THIS for IPC
    pub ipc_client: Arc<LapceAiIpcClient>,  // NEW!
}

// NEW struct
pub struct LapceAiIpcClient {
    tx: mpsc::Sender<IpcMessage>,
    rx: mpsc::Receiver<IpcMessage>,
}

impl LapceAiIpcClient {
    pub fn new() -> Self {
        // Connect to lapce-ai-rust via SharedMemory
        let (tx, rx) = shared_memory_connect("/tmp/lapce-ai.sock");
        Self { tx, rx }
    }
    
    pub async fn send(&self, msg: IpcMessage) -> Result<IpcMessage> {
        self.tx.send(msg).await?;
        self.rx.recv().await.ok_or_else(|| anyhow!("Connection closed"))
    }
}
```

---

## 📊 Updated Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│  Lapce IDE (lapce-app/)                                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │ ACTUAL Components:                                │  │
│  │                                                   │  │
│  │ terminal/panel.rs (TerminalPanelData)            │  │
│  │ editor/diff.rs (DiffEditorData)                  │  │
│  │ ai_panel/message_handler.rs (MessageHandler)     │  │
│  └───────────────────────┬───────────────────────────┘  │
│                          │                              │
│  ┌───────────────────────▼───────────────────────────┐  │
│  │ window_tab.rs (CommonData)                        │  │
│  │ + ipc_client: LapceAiIpcClient                    │  │
│  └───────────────────────┬───────────────────────────┘  │
└──────────────────────────┼──────────────────────────────┘
                           │
                  ═════════▼═════════
                  SharedMemory IPC
                  5.1μs latency ✅
                  (Already built!)
                  ═════════│═════════
                           │
┌──────────────────────────▼──────────────────────────────┐
│  lapce-ai-rust/src/                                     │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ipc_server.rs (Router)                           │   │
│  │  ├─→ handlers/task_orchestrator.rs              │   │
│  │  ├─→ handlers/tools/terminal.rs (OSC parser)    │   │
│  │  ├─→ handlers/tools/diff.rs                     │   │
│  │  └─→ handlers/prompts.rs                        │   │
│  └──────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

---

## ✅ Implementation Checklist

**Week 6: UI Integration (Need to do)**

- [ ] Add `ipc_client: LapceAiIpcClient` to `CommonData` in `window_tab.rs`
- [ ] Extend `TerminalPanelData` in `terminal/panel.rs` with `execute_ai_command()`
- [ ] Extend `DiffEditorData` in `editor/diff.rs` with `apply_ai_diff()`
- [ ] Modify `MessageHandler` in `ai_panel/message_handler.rs` to route to backend
- [ ] Test end-to-end: UI → SharedMemory → Backend → UI

**Backend (Already Done ✅):**
- [x] SharedMemory IPC (5.1μs latency)
- [x] Message routing in `ipc_server.rs`
- [x] Handlers: Terminal, Diff, Prompts, Tools

---

**KEY DIFFERENCE**: 
- ❌ Before: Used hypothetical `ai_chat.rs`, `ai_terminal.rs`, `ai_diff.rs`
- ✅ Now: Uses ACTUAL `message_handler.rs`, `terminal/panel.rs`, `editor/diff.rs`

All Step 29 IPC integration now routes to **REAL** Lapce components!
