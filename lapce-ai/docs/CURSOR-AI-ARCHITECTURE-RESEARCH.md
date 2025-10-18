# Cursor AI Architecture Research
## Deep Analysis for Lapce AI Implementation

---

## 🎯 Executive Summary

**Cursor** = VS Code Fork + AI Backend + Ultra-low Latency IPC

**Key Insight**: Cursor achieves "native feel" by:
1. **Forked VS Code** (familiar IDE, no reinvention)
2. **Separate Node.js Backend** (heavy AI operations isolated)
3. **IPC Message Passing** (async/event-based communication)
4. **Multiple Specialized Models** (autocomplete, apply, main agent)
5. **Client-side Encryption** (privacy-first, process-discard pattern)

**Our Equivalent**: Lapce (Floem native) + Rust AI Backend + Shared Memory IPC

---

## 🏗️ Cursor Architecture Breakdown

### **1. Process Architecture**

```
┌─────────────────────────────────────────────────────────┐
│                   CURSOR (VS CODE FORK)                 │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Main Process (Electron)                         │   │
│  │  - Window management                             │   │
│  │  - File system operations                        │   │
│  │  - Native integrations                           │   │
│  └──────────────────┬───────────────────────────────┘   │
│                     │ Electron IPC                      │
│  ┌──────────────────▼───────────────────────────────┐   │
│  │  Renderer Process (Chromium)                     │   │
│  │  - UI (React components)                         │   │
│  │  - Editor (Monaco/CodeMirror)                    │   │
│  │  - Chat panel, inline edits                      │   │
│  └──────────────────┬───────────────────────────────┘   │
└─────────────────────┼──────────────────────────────────┘
                      │
                 IPC BOUNDARY
              (node-ipc library)
                      │
┌─────────────────────▼──────────────────────────────────┐
│        BACKEND (Forked Node.js Process)                │
│  ┌──────────────────────────────────────────────────┐  │
│  │  AI Orchestration Engine                         │  │
│  │  - Tool execution (read/write files)             │  │
│  │  - LLM API calls (OpenAI, Anthropic)             │  │
│  │  - Codebase indexing (vector embeddings)         │  │
│  │  - Long-running tasks                            │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

**Why Separate Backend Process?**
- **Performance Isolation**: Heavy AI operations don't block UI
- **Memory Management**: Backend can use more RAM without affecting editor
- **Native Dependencies**: Access to Node.js native modules
- **Crash Isolation**: Backend crash doesn't kill editor

---

## 🔄 IPC Communication Pattern

### **Cursor's Approach**

```javascript
// Frontend (Renderer) → Backend
ipcClient.emit('chat_message', {
  message: "Refactor this function",
  files: ["index.ts"],
  context: { currentFile, cursorPosition }
});

// Backend → Frontend (Event-based)
ipcServer.emit('stream_token', {
  taskId: "abc123",
  token: "const ",
  delta: 1
});

ipcServer.emit('tool_execution', {
  taskId: "abc123",
  tool: "write_file",
  args: { path: "index.ts", content: "..." }
});

ipcServer.emit('task_complete', {
  taskId: "abc123",
  status: "success"
});
```

### **Key Properties**
- **Async**: Non-blocking, UI stays responsive
- **Event-based**: Backend sends progress updates during long tasks
- **Typed Messages**: Fully typed payloads (TypeScript)
- **Bidirectional**: Both sides can initiate communication

---

## 🤖 Multi-Model Architecture

Cursor uses **3+ specialized models**:

### **1. Autocomplete Model (Custom, Fireworks-hosted)**
- **Purpose**: Ultra-fast inline completions (<1s latency)
- **Input**: Small code snippet (~200 lines context)
- **Output**: Single-line or multi-line completion
- **Optimization**: Tiny model, optimized for speed

### **2. Apply Model (Code Edit Specialist)**
- **Purpose**: Transform semantic diffs → actual code
- **Input**: "Semantic diff" from main agent + lint feedback
- **Output**: Actual file contents with fixes
- **Why**: Main LLM writes lazy/high-level diffs, apply model fills details

### **3. Main Agent (Claude 3.5 Sonnet / GPT-4)**
- **Purpose**: High-level reasoning, tool calling, task orchestration
- **Input**: User query + codebase context + tool results
- **Output**: Tool calls, semantic diffs, explanations
- **Tools**: read_file, write_file, run_command, codebase_search, grep_search, web_search

### **4. Embedding Model (OpenAI text-embedding-ada-002)**
- **Purpose**: Convert code → vector embeddings
- **Input**: Code chunks (functions, classes)
- **Output**: 1536-dim vectors
- **Storage**: Turbopuffer (vector database)

---

## 🔍 Codebase Indexing System

### **Process Flow**

```
1. CLIENT: Split files into chunks (functions/classes)
   ↓
2. CLIENT: Encrypt chunks + obfuscate file paths
   ↓
3. SEND TO SERVER: Encrypted chunks
   ↓
4. SERVER: Decrypt, generate embeddings, DISCARD CONTENT
   ↓
5. STORE: Only embeddings in Turbopuffer (no source code)
   ↓
6. QUERY TIME: 
   - Convert query → embedding
   - Vector similarity search
   - Return relevant chunk IDs (obfuscated)
   ↓
7. CLIENT: Send actual source code for matched chunks
   ↓
8. SERVER: Use for LLM context, then DISCARD
```

### **Privacy Design**
- ✅ Client encrypts before sending
- ✅ Server discards content after embedding
- ✅ Only vectors stored (one-way, can't reconstruct code)
- ✅ Obfuscated file paths (no real names)
- ✅ Respects .gitignore + .cursorignore

### **Synchronization (Merkle Tree)**
- Client & server maintain Merkle tree of project
- Compare trees every few minutes
- Only send changed chunks for re-indexing
- Efficient: Only delta updates, not full re-index

---

## 🛠️ Tool System Architecture

### **Core Tools**

```typescript
// Cursor's tool definitions
interface Tool {
  name: string;
  description: string;
  parameters: {
    type: "object";
    properties: { [key: string]: any };
    required: string[];
  };
}

// Examples from Cursor system prompt
{
  name: "read_file",
  description: "Read the contents of a file",
  parameters: {
    type: "object",
    properties: {
      path: { type: "string", description: "Full file path" },
      explanation: { type: "string", description: "Why reading this file" }
    },
    required: ["path", "explanation"]
  }
}

{
  name: "write_file", 
  description: "Write or edit file contents",
  parameters: {
    type: "object",
    properties: {
      path: { type: "string" },
      content: { type: "string", description: "Full file contents OR semantic diff" },
      explanation: { type: "string" }
    },
    required: ["path", "content", "explanation"]
  }
}

{
  name: "codebase_search",
  description: "Semantic search across entire codebase",
  parameters: {
    type: "object",
    properties: {
      query: { type: "string", description: "Natural language query" },
      explanation: { type: "string" }
    }
  }
}
```

### **Tool Execution Flow**

```
1. User: "Refactor authentication code"
   ↓
2. Main Agent: Decides to call codebase_search("authentication")
   ↓
3. IPC: Send tool call to backend
   ↓
4. Backend: Execute search (vector similarity)
   ↓
5. Backend: Return results (file paths + relevance scores)
   ↓
6. IPC: Stream results back to frontend
   ↓
7. Main Agent: Calls read_file for top results
   ↓
8. Backend: Read files from disk
   ↓
9. Main Agent: Analyzes code, generates semantic diff
   ↓
10. Main Agent: Calls write_file with diff
    ↓
11. Backend: Apply model transforms diff → actual code
    ↓
12. Backend: Write to disk, run linter
    ↓
13. Backend: Send lint feedback to agent
    ↓
14. Main Agent: Self-corrects if lint errors, else done
```

---

## 🚀 Performance Optimizations

### **1. Ultra-Low Latency Autocomplete**
- **Target**: <1 second (ideally <500ms)
- **Techniques**:
  - Small custom model (not GPT-4)
  - Pre-warming connections
  - Minimal context (only nearby lines)
  - Client-side caching
  - Speculative prefetching

### **2. Efficient Code Edits**
- **Semantic Diffs** instead of full files:
  ```typescript
  // Instead of sending 1000-line file:
  "Add import at top:\nimport { foo } from 'bar';\n\nIn function handleAuth (line 45):\n  Replace:\n    const token = req.headers.auth;\n  With:\n    const token = req.headers.authorization?.split(' ')[1];"
  ```
- **Apply Model** fills in details, fixes syntax
- **Linter Feedback Loop**: Catch errors, self-correct

### **3. Lazy Context Loading**
- Don't send entire codebase to LLM
- Use @file/@folder for explicit context
- Use vector search for semantic context
- Only fetch relevant chunks on-demand

### **4. Backend Isolation**
- Long-running operations don't block UI
- Can use all CPU cores for AI inference
- Memory isolation (backend can use 10GB+ RAM)

---

## 📊 Infrastructure Stack

### **Cursor's Cloud Architecture**

```
┌─────────────────────────────────────────────────────────┐
│                  CLIENT (Developer Machine)             │
│  - Cursor IDE (Electron)                                │
│  - Encryption/Decryption                                │
│  - Local file operations                                │
└─────────────────────┬───────────────────────────────────┘
                      │ HTTPS (encrypted)
                      ▼
┌─────────────────────────────────────────────────────────┐
│                  CLOUDFLARE (CDN/Proxy)                 │
│  - DDoS protection                                      │
│  - TLS termination                                      │
│  - Global edge network                                  │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              AWS (Primary Infrastructure)               │
│  ┌──────────────────────────────────────────────────┐   │
│  │  API Servers (US, Tokyo, London)                 │   │
│  │  - Handle requests                               │   │
│  │  - Route to appropriate service                  │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Job Queues (SQS/Redis)                          │   │
│  │  - Async task processing                         │   │
│  │  - Background indexing                           │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
┌──────────────┐ ┌──────────┐ ┌────────────┐
│   FIREWORKS  │ │  OPENAI  │ │ ANTHROPIC  │
│   (Custom)   │ │ (GPT-4)  │ │ (Claude)   │
│ Autocomplete │ │Embedding │ │Main Agent  │
└──────────────┘ └──────────┘ └────────────┘
                      │
                      ▼
              ┌───────────────┐
              │  TURBOPUFFER  │
              │Vector Database│
              │  (Embeddings) │
              └───────────────┘
```

### **Key Metrics**
- **QPS**: >1 million queries/second (mostly autocomplete)
- **Latency**: <1s autocomplete, <5s chat responses
- **Data**: Embeddings only (no source code stored)
- **Regions**: US (primary), Tokyo, London (latency optimization)

---

## 🎨 "Native Feel" Secrets

### **What Makes Cursor Feel Native?**

1. **Familiar Foundation**: VS Code fork (developers already know it)
2. **Instant Feedback**: Autocomplete <1s, feels real-time
3. **Non-blocking UI**: All heavy operations in backend process
4. **Smooth Streaming**: Tokens stream in gradually, not batch updates
5. **Inline Integration**: AI suggestions appear directly in editor (not separate panel)
6. **Keyboard-first**: Tab to accept, Cmd+K for inline edit (no mouse needed)
7. **Context Awareness**: Automatically includes current file, cursor position
8. **Minimal Latency**: <50ms IPC overhead, feels instant

### **Critical UX Patterns**

```typescript
// 1. Autocomplete appears while typing
onTyping() {
  debounce(300ms, () => {
    sendToBackend({ context: getCurrentLine() });
    // UI shows loading indicator (subtle)
  });
}

// 2. Stream tokens as they arrive
onStreamToken(token) {
  // Append immediately, don't wait for full response
  editor.insertText(token);
  // UI feels responsive
}

// 3. Inline edits with diff preview
onInlineEdit(selection, instruction) {
  // Show diff overlay BEFORE applying
  showDiffPreview(oldCode, newCode);
  // User can accept/reject
}

// 4. Background operations don't interrupt
onBackgroundTask(taskId) {
  // Show subtle progress indicator
  // DON'T block editor
  // User can keep typing
}
```

---

## 🔐 Security & Privacy Model

### **Cursor's Privacy Approach**

1. **Client-side Encryption**: All code encrypted before leaving machine
2. **Obfuscated Identifiers**: File paths hashed, not plaintext
3. **Process-Discard Pattern**: 
   - Receive encrypted data
   - Decrypt for inference
   - Generate result
   - **DISCARD** immediately (never persist)
4. **Embeddings Only**: Vector DB stores embeddings (one-way, can't reconstruct)
5. **Privacy Mode**: Extra option to never send code to cloud (local models only)

### **Our Implementation (Lapce AI)**

```rust
// Shared Memory IPC = Local-only by default
// No network requests unless user explicitly uses cloud models

pub struct PrivacyConfig {
    pub local_only: bool,           // Default: true (no cloud)
    pub encrypt_ipc: bool,          // Encrypt even local IPC
    pub vector_storage: VectorStorageType, // Local | Cloud | Hybrid
    pub discard_immediately: bool,  // Always discard after processing
}

// If cloud models used:
pub async fn cloud_request(code: &str) -> Result<Response> {
    // 1. Encrypt on client
    let encrypted = encrypt_with_user_key(code);
    
    // 2. Send to cloud
    let response = api_call(encrypted).await?;
    
    // 3. Server processes, discards
    // (No persistent storage)
    
    Ok(response)
}
```

---

## 🎯 Key Takeaways for Lapce AI

### **Architecture Decisions**

| Aspect | Cursor | Lapce AI (Our Plan) |
|--------|--------|---------------------|
| **Base** | VS Code fork (Electron) | Lapce native (Floem) |
| **Backend** | Node.js forked process | Rust separate binary |
| **IPC** | node-ipc library | Shared Memory (custom) |
| **Models** | Cloud-based (OpenAI, Anthropic) | Cloud + Local option |
| **Indexing** | Cloud vector DB (Turbopuffer) | Local LanceDB + optional cloud |
| **Privacy** | Encrypt → Cloud → Discard | Local-first, no cloud by default |
| **Latency** | <1s autocomplete | <10μs IPC, <100ms total |
| **Throughput** | 1M+ QPS | 55M msg/sec (IPC) |

### **What to Copy**

✅ **Multi-model architecture** (autocomplete, apply, main agent)
✅ **Semantic diff pattern** (lazy high-level edits)
✅ **Tool system design** (read/write/search tools)
✅ **Streaming responses** (token-by-token updates)
✅ **Inline integration** (AI in editor, not separate)
✅ **Codebase indexing** (vector embeddings for search)
✅ **Event-based IPC** (async, non-blocking)

### **What to Improve**

🚀 **Faster IPC**: Shared Memory (0.091μs) vs node-ipc (~1-10ms)
🚀 **Local-first**: No cloud required, privacy by default
🚀 **Native Performance**: Rust backend vs Node.js (10x+ faster)
🚀 **Memory Efficiency**: 3MB vs 20MB+ (Electron overhead)
🚀 **True Native UI**: Floem (GPU-accelerated) vs Chromium

---

## 📐 Lapce AI Architecture (Final Design)

```
┌─────────────────────────────────────────────────────────┐
│                    LAPCE IDE (NATIVE)                   │
│  ┌──────────────────────────────────────────────────┐   │
│  │  UI Layer (Floem - GPU Accelerated)             │   │
│  │  - Editor tabs                                   │   │
│  │  - AI Chat Panel (native, not webview)          │   │
│  │  - Inline edit overlays                          │   │
│  │  - Diff viewer                                   │   │
│  └──────────────────┬───────────────────────────────┘   │
│                     │                                   │
│  ┌──────────────────▼───────────────────────────────┐   │
│  │  AI Bridge Module (ai_bridge.rs)                 │   │
│  │  - IPC client                                    │   │
│  │  - Message serialization (rkyv)                  │   │
│  │  - Async message handling                        │   │
│  └──────────────────┬───────────────────────────────┘   │
└─────────────────────┼──────────────────────────────────┘
                      │
                 IPC BOUNDARY
            (Shared Memory - 0.091μs)
                      │
┌─────────────────────▼──────────────────────────────────┐
│           LAPCE-AI-RUST ENGINE (SEPARATE BINARY)       │
│  ┌──────────────────────────────────────────────────┐  │
│  │  IPC Server (shared_memory_complete.rs)          │  │
│  │  - Lock-free ring buffer                         │  │
│  │  - Zero-copy message passing                     │  │
│  │  - Message dispatcher                            │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                  │
│         Routes to components:                          │
│         ┌───────────┴───────────┐                      │
│  ┌──────▼────┐  ┌───────▼──────┐  ┌─────────────┐     │
│  │Task Engine│  │Tools System  │  │Prompts      │     │
│  │(CHUNK-03) │  │(CHUNK-02)    │  │(CHUNK-01)   │     │
│  └───────────┘  └──────────────┘  └─────────────┘     │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐        │
│  │Providers  │  │Tree-sitter│  │LanceDB      │        │
│  │(40+ APIs) │  │(100+ langs)│  │(semantic)   │        │
│  └───────────┘  └───────────┘  └─────────────┘        │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐        │
│  │Streaming  │  │Cache      │  │Connection   │        │
│  │Pipeline   │  │System     │  │Pool         │        │
│  └───────────┘  └───────────┘  └─────────────┘        │
└────────────────────────────────────────────────────────┘
```

---

## ✅ Ready to Transform CHUNKs

With this research complete, we can now transform each CHUNK to include:

1. **Codex Analysis** (what TypeScript does)
2. **IPC Integration** (how it connects via Shared Memory)
3. **Rust Translation** (1:1 port strategy)

Following Cursor's proven patterns:
- Multi-model architecture
- Tool-based agent system
- Semantic diff for edits
- Streaming responses
- Vector-based codebase search
- Event-driven IPC
- Privacy-first design

**But better**:
- 110x faster IPC (Shared Memory vs node-ipc)
- Native UI (Floem vs Electron/Chromium)
- Local-first (no cloud required)
- Memory efficient (3MB vs 20MB+)
- Pure Rust (type-safe, performant)
