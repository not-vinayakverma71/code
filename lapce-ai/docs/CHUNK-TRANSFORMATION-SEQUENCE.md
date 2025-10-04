# CHUNK Transformation Sequence & Priority Analysis

## 🎯 Mission (Updated with Step 29 IPC Architecture)
Transform 43 raw CHUNK files into comprehensive integration guides following:
**Codex Analysis** → **Native UI + SharedMemory IPC (5.1μs ✅) + Backend** → **Rust Translation Strategy**

### Step 29 Architecture Formula
```
UI (lapce-app/) ←→ SharedMemory IPC (5.1μs) ←→ Backend (lapce-ai-rust/)
   ~800 lines           Already built!           ~3,700 lines
```

---

## 📊 Priority System

### **Tier 1: CRITICAL PATH** (Week 5 - Must Complete First)
Core backend components that IPC dispatcher needs immediately.

**Priority 1A: Task Engine Foundation** (Days 1-2) ✅ **UPDATED with Step 29**
- `CHUNK-03-TASK.md` → `03-TASK-ORCHESTRATOR.md` ✅
  - **Why**: 2859 lines, main orchestration loop, all other components depend on it
  - **IPC Messages**: StartTask, TaskEvent, StreamToken, ToolExecution
  - **Architecture**: Backend handles orchestration, UI receives events via SharedMemory
  - **Step 29 Integration**: Task events streamed to UI at 5.1μs latency
  
- `CHUNK-01-PROMPTS.md` → `01-PROMPTS-SYSTEM.md` ✅
  - **Why**: Task engine needs prompt building for every AI request
  - **IPC Messages**: BuildPrompt, PromptReady, UpdateCustomInstructions
  - **Architecture**: UI requests → Backend builds (heavy logic) → UI receives
  - **Step 29 Integration**: Prompt building in backend, token counting included

- `CHUNK-02-TOOLS.md` → `02-TOOLS-EXECUTION.md` ✅
  - **Why**: Task engine executes 20+ tools (read_file, write_file, terminal, diff)
  - **IPC Messages**: ExecuteTool, ToolResult, ExecuteCommand, RequestDiff
  - **Architecture**: UI triggers → Backend executes → Streams results
  - **Step 29 Integration**: 
    - Terminal: OSC 633/133 parsing, streaming output
    - Diff: Line-by-line streaming updates
    - File ops: Permission system via IPC

**Priority 1B: Message Protocol** (Day 3) ✅ **UPDATED with Step 29**
- `CHUNK-30-SHARED.md` → `30-MESSAGE-PROTOCOL.md` ✅
  - **Why**: Defines unified IpcMessage protocol (consolidated from 309 variants)
  - **IPC Performance**: 5.1μs latency, 1.38M msg/sec, 1.46MB memory ✅
  - **Architecture**: SharedMemory with rkyv zero-copy serialization
  - **Step 29 Integration**: All handlers registered (Task, Terminal, Diff, Prompt, Tool)

- `CHUNK-29-VSCODE-LAPCE.md` → `29-VSCODE-LAPCE-BRIDGE-FINAL-REVISED.md` ✅
  - **Why**: Complete integration guide with IPC architecture
  - **IPC Split**: UI (~800 lines) + Bridge (~100 lines) + Backend (~3,700 lines)
  - **Architecture**: Native Floem UI + SharedMemory IPC + Rust backend
  - **Step 29 Components**:
    - Terminal integration (TerminalHandler + OSC parser)
    - Diff view (DiffHandler + streaming)
    - Workspace tracker (FileWatcher)
    - AI chat panel (Floem UI)

---

### **Tier 2: SERVICES LAYER** (Week 5 - Days 4-5)
Backend services that Task Engine calls via IPC.

**Priority 2A: Core Services** (Day 4)
- `CHUNK-25-SERVICES.md` - `25-NEW-SERVICES.md`
  - **Why**: 13 subdirectories (tree-sitter, browser, MCP, etc.)
  - **IPC Messages**: ParseFile, SearchCode, BrowserControl
  - **Complexity**: High (already done: tree-sitter ✅, semantic search ✅ ) - Already done  don`t translate the tree-sitter & semantic_search

- `CHUNK-31-UTILS.md` → `31-UTILS-LIBRARY.md`
  - **Why**: 50+ utility functions (git, shell, path, fs)
  - **IPC Messages**: GitOperation, ShellCommand, FileSystemOp
  - **Complexity**: Medium (helper functions, minimal state)

**Priority 2B: Provider Integration** (Day 5) (DONE)
- `CHUNK-27-API-PROVIDERS.md` → `27-PROVIDERS-INTEGRATION.md`
  - **Why**: Already documented in `03-REMAINING-PROVIDERS.md` ✅
  - **Status**: SKIP transformation, use existing doc
  - **Complexity**: Done

- `CHUNK-28-TRANSFORM-STREAM.md` → `28-STREAMING-INTEGRATION.md` (DONE)
  - **Why**: Already documented in `08-STREAMING-PIPELINE.md` ✅
  - **Status**: SKIP transformation, use existing doc
  - **Complexity**: Done

---

### **Tier 3: WEBVIEW COMPONENTS** (Week 6 - Phase D)
UI components that communicate with backend via IPC.

**Priority 3A: Webview Foundation** (Day 6)
- `CHUNK-04-WEBVIEW.md` → `04-WEBVIEW-PROVIDER.md`
  - **Why**: 2831 lines, manages entire webview lifecycle
  - **IPC Messages**: WebviewReady, SendMessage, UpdateState
  - **Complexity**: High (React bridge, state sync)

**Priority 3B: Core Modules** (Days 7-9)
- `CHUNK-05-HISTORY.md` → `05-HISTORY-MANAGER.md`
- `CHUNK-06-DIFF.md` → `06-DIFF-VIEWER.md`
- `CHUNK-07-STATE.md` → `07-STATE-MANAGEMENT.md`
- `CHUNK-08-AUTOCOMPLETE.md` → `08-AUTOCOMPLETE.md`
- `CHUNK-09-API-HANDLER.md` → `09-API-HANDLER.md`

**Priority 3C: Configuration** (Day 10)
- `CHUNK-10-API-CONFIG.md` → `10-API-CONFIG.md`
- `CHUNK-11-MODE-CONFIG.md` → `11-MODE-CONFIG.md`
- `CHUNK-12-CONTEXT.md` → `12-CONTEXT-MANAGER.md`

**Priority 3D: Advanced Features** (Days 11-12)
- `CHUNK-13-CACHE.md` → `13-CACHE-SYSTEM.md`
- `CHUNK-14-COST.md` → `14-COST-TRACKING.md`
- `CHUNK-15-DIAGNOSTICS.md` → `15-DIAGNOSTICS.md`
- `CHUNK-16-TERMINAL.md` → `16-TERMINAL-PROCESS.md`
- `CHUNK-17-BROWSER.md` → `17-BROWSER-INTEGRATION.md`
- `CHUNK-18-WATCHMAN.md` → `18-FILE-WATCHER.md`
- `CHUNK-19-EXPORT.md` → `19-EXPORT-MANAGER.md`

---

### **Tier 4: INTEGRATIONS** (Week 6 - Days 13-14)
- `CHUNK-26-INTEGRATIONS.md` → `26-INTEGRATIONS.md`
  - Terminal, editor, theme integrations
  - **Complexity**: Medium (VS Code → Lapce API mapping)

---

### **Tier 5: PACKAGES** (Week 6 - Days 15-17)
Supporting packages with isolated functionality.

**Priority 5A: Type System** (Day 15)
- `CHUNK-37-TYPES-PART1.md` → `37-TYPES-SYSTEM-PART1.md`
- `CHUNK-38-TYPES-PART2.md` → `38-TYPES-SYSTEM-PART2.md`
- `CHUNK-39-TYPES-PART3.md` → `39-TYPES-SYSTEM-PART3.md`
  - **Why**: 63 files, foundation for all message types
  - **Complexity**: Medium (type definitions, no runtime logic)

**Priority 5B: Infrastructure** (Day 16)
- `CHUNK-33-BUILD.md` → `33-BUILD-TOOLS.md`
- `CHUNK-34-CLOUD.md` → `34-CLOUD-INTEGRATION.md`
- `CHUNK-35-CONFIG.md` → `35-CONFIG-SYSTEM.md`

**Priority 5C: Optional Features** (Day 17)
- `CHUNK-40-TELEMETRY.md` → `40-TELEMETRY.md` (Optional)
- `CHUNK-36-EVALS.md` → `36-EVALUATION-SYSTEM.md` (Optional - can skip)

---

### **Tier 6: LOCALIZATION** (Week 6 - Optional/Deferred)
Can use English-only initially, add i18n later.

- `CHUNK-20-I18N-EN.md` → SKIP (English only)
- `CHUNK-21-I18N-ZH.md` → SKIP (Defer Chinese)
- `CHUNK-22-I18N-JA.md` → SKIP (Defer Japanese)
- `CHUNK-23-I18N-KO.md` → SKIP (Defer Korean)
- `CHUNK-24-I18N-OTHER.md` → SKIP (Defer others)

---

### **Tier 7: APPS & DOCUMENTATION** (Week 6 - Optional)
- `CHUNK-41-APPS.md` → SKIP (Already covered in webview)
- `CHUNK-42-KILOCODE.md` → SKIP (User config, no translation needed)
- `CHUNK-43-BENCHMARK.md` → SKIP (Use Criterion, not JS benchmarks)

---

## 📋 Final Transformation Order (33 files)

### **Week 5: Backend Translation** (Days 1-5)
1. CHUNK-03 → Task Engine (Day 1-2)
2. CHUNK-01 → Prompts (Day 1-2)
3. CHUNK-02 → Tools (Day 1-2)
4. CHUNK-30 → Message Protocol (Day 3)
5. CHUNK-29 → VSCode Bridge (Day 3)
6. CHUNK-25 → Services (Day 4)
7. CHUNK-31 → Utils (Day 4)

### **Week 6: UI Translation** (Days 6-17)
8. CHUNK-04 → Webview Provider (Day 6)
9. CHUNK-05 → History (Day 7)
10. CHUNK-06 → Diff (Day 7)
11. CHUNK-07 → State (Day 8)
12. CHUNK-08 → Autocomplete (Day 8)
13. CHUNK-09 → API Handler (Day 9)
14. CHUNK-10 → API Config (Day 10)
15. CHUNK-11 → Mode Config (Day 10)
16. CHUNK-12 → Context (Day 10)
17. CHUNK-13 → Cache (Day 11)
18. CHUNK-14 → Cost (Day 11)
19. CHUNK-15 → Diagnostics (Day 12)
20. CHUNK-16 → Terminal (Day 12)
21. CHUNK-17 → Browser (Day 12)
22. CHUNK-18 → Watchman (Day 12)
23. CHUNK-19 → Export (Day 12)
24. CHUNK-26 → Integrations (Day 13-14)
25. CHUNK-37 → Types Part 1 (Day 15)
26. CHUNK-38 → Types Part 2 (Day 15)
27. CHUNK-39 → Types Part 3 (Day 15)
28. CHUNK-33 → Build (Day 16)
29. CHUNK-34 → Cloud (Day 16)
30. CHUNK-35 → Config (Day 16)
31. CHUNK-40 → Telemetry (Day 17 - Optional)
32. CHUNK-36 → Evals (Day 17 - Optional)

### **Skipped (10 files)**
- CHUNK-20-24 (i18n locales) - Use English only
- CHUNK-27-28 (Providers/Streaming) - Already done ✅
- CHUNK-41-43 (Apps/Kilocode/Benchmark) - Not applicable

---

## 📐 Transformation Template

Each transformed CHUNK will follow this structure (~800-1000 lines):

```markdown
# [NUMBER]-[NAME]-INTEGRATION.md

## Part 1: Codex Analysis (30%)
### What Exists in TypeScript
- File structure from Codex
- Key functions and data structures
- Battle-tested logic to preserve
- Edge cases and quirks

## Part 2: Lapce IPC Integration (40%)
### Architecture Diagram
[Show data flow through IPC boundary]

### IPC Message Definitions
```rust
pub enum [Component]Message {
    // Request messages
    // Response messages
    // Stream messages
}
```

### Message Flow Examples
```
User Action → Lapce UI → IPC → Component → IPC → UI Update
```

### Integration Points
- How this component connects to Task Engine
- How it communicates with other components
- Shared state via IPC

## Part 3: Rust Translation (30%)
### Implementation Strategy
- TypeScript → Rust mapping
- Memory layout and optimization
- Success criteria
- Testing strategy

### Critical Code Examples
```rust
// 1:1 translation of key functions
```

### Memory Profile
- Expected memory usage
- Performance targets

## Part 4: Testing & Validation
- Unit tests
- Integration tests
- Performance benchmarks
```

---

## 🎯 Success Metrics

Each transformed CHUNK must include:

✅ **Completeness**: All major functions from TypeScript covered
✅ **IPC Design**: Clear message protocol defined
✅ **Translation Plan**: Line-by-line Rust strategy
✅ **Memory Target**: Specific MB limit stated
✅ **Performance Target**: Latency/throughput goals
✅ **Test Coverage**: Unit + integration tests outlined

---

## 📅 Timeline Estimate

- **Week 5 (Days 1-5)**: 7 backend CHUNKs = ~5,600 lines documentation
- **Week 6 (Days 6-17)**: 25 UI CHUNKs = ~20,000 lines documentation
- **Total**: 32 CHUNKs, ~25,600 lines of comprehensive integration docs

**Rate**: ~2-3 CHUNKs per day (intensive work)

---

## 🚀 Next Step

**Research Cursor AI architecture** to understand:
1. How Cursor achieves "native feel" with AI backend
2. Their IPC/communication strategy
3. UI/UX patterns to replicate in Lapce
4. Performance characteristics

Then begin transformation with **CHUNK-03 (Task Engine)**.
