# CHUNK-29 Step 3: Deep API Usage Analysis

**Generated:** 2025-10-02  
**Status:** Complete

## Executive Summary

Deep dive into VS Code API usage patterns across 28 integration files, revealing complex dependencies on 80+ VS Code APIs with critical focus on terminal shell integration, diff view streaming, and workspace tracking.

**IPC Architecture Split:**
```
UI (lapce-app/)              IPC Boundary           Backend (lapce-ai-rust/)
─────────────────            ════════════           ─────────────────────────
Diff Renderer         ←─── SharedMemory ───→      Diff Content Generator
Terminal UI           ←─── 5.1μs latency ───→     Shell Integration Parser
Chat Panel            ←─── 1.38M msg/s  ───→      AI Providers
File Explorer         ←─── Binary Proto ───→      Workspace Tracker
```

---

## 1. TERMINAL SHELL INTEGRATION APIS

### 1.1 Core Terminal APIs

#### `vscode.window.createTerminal`
**Location:** Terminal.ts:21
```typescript
vscode.window.createTerminal({ 
  cwd, 
  name: "Kilo Code", 
  iconPath: new vscode.ThemeIcon("rocket"), 
  env 
})
```
**Purpose:** Creates VS Code integrated terminal with custom environment
**Lapce Equivalent:** Need native terminal creation with PTY support

#### `terminal.shellIntegration`
**Critical Locations:** 
- TerminalProcess.ts:52-53 (check availability)
- TerminalProcess.ts:132 (executeCommand)
- Terminal.ts:72-74 (wait for integration)

**Purpose:** Access to shell integration features for command tracking
**Key Methods:**
- `shellIntegration.executeCommand(command)` - Execute with tracking
- `shellIntegration.cwd` - Get current working directory
- Stream handling for output capture

#### Shell Integration Event Handlers
**Location:** TerminalRegistry.ts:49-124
```typescript
vscode.window.onDidStartTerminalShellExecution
vscode.window.onDidEndTerminalShellExecution  
vscode.window.onDidCloseTerminal
```
**Purpose:** Lifecycle tracking of terminal commands
**Challenge:** Must implement equivalent event system in Rust

### 1.2 Escape Sequence Parsing

**Critical Pattern:** OSC 633/133 markers
**Location:** TerminalProcess.ts:164-241

```typescript
// VS Code markers
"\x1b]633;A\x07"  // Prompt start
"\x1b]633;B\x07"  // Prompt end  
"\x1b]633;C\x07"  // Command output start ← CRITICAL
"\x1b]633;D\x07"  // Command output end   ← CRITICAL

// iTerm2 fallback markers
"\x1b]133;C\x07"  // Alternative command start
"\x1b]133;D\x07"  // Alternative command end
```

**Parsing Implementation:**
- Uses `indexOf` not regex (500x faster)
- Handles partial chunks
- Supports both marker types
- Stream-based processing

**Rust Translation Need:** High-performance byte-level parsing

---

## 2. DIFF VIEW & EDITOR APIS

### 2.1 Diff View Creation

#### `vscode.commands.executeCommand("vscode.diff")`
**Location:** DiffViewProvider.ts:546-554
```typescript
vscode.commands.executeCommand(
  "vscode.diff",
  vscode.Uri.parse(`${DIFF_VIEW_URI_SCHEME}:${fileName}`).with({
    query: Buffer.from(this.originalContent ?? "").toString("base64"),
  }),
  uri,
  `${fileName}: ${DIFF_VIEW_LABEL_CHANGES} (Editable)`,
  { preserveFocus: true }
)
```

**Lapce IPC Split:**
```rust
// UI: lapce-app/src/editor/ai_diff.rs
impl AiDiffView {
    pub async fn show_diff(&mut self, file: PathBuf) {
        // Request diff content via IPC
        let diff_data = self.bridge.request_diff(file).await?;
        
        // Render with Floem
        self.render_diff(diff_data);
    }
}

// Backend: lapce-ai-rust/src/handlers/diff.rs
impl DiffHandler {
    pub async fn generate_diff(&self, file: PathBuf) -> IpcResponse {
        // Generate diff content (heavy logic)
        let diff = self.compute_changes(file).await?;
        IpcResponse::DiffData(diff)
    }
}
```

### 2.2 Text Editing APIs

#### `vscode.WorkspaceEdit`
**Location:** DiffViewProvider.ts:139-144, 161-164, 174-182
```typescript
const edit = new vscode.WorkspaceEdit()
edit.replace(document.uri, rangeToReplace, contentToReplace)
await vscode.workspace.applyEdit(edit)
```
**Purpose:** Apply text changes to documents
**Pattern:** Batch edits with ranges

#### Decoration APIs
**Location:** DecorationController.ts:3-14
```typescript
vscode.window.createTextEditorDecorationType({
  backgroundColor: "rgba(255, 255, 0, 0.1)",
  opacity: "0.4",
  isWholeLine: true
})
```
**Purpose:** Visual highlighting during streaming
**Lapce Need:** Custom rendering decorations

### 2.3 Document Management

#### `vscode.workspace.openTextDocument`
**Locations:** Multiple files
**Purpose:** Open files in memory
**Variations:**
- With URI: `openTextDocument(uri)`
- From content: `openTextDocument({ content })`

#### `vscode.window.showTextDocument`
**Locations:** DiffViewProvider.ts:210, open-file.ts:142
**Purpose:** Display document in editor
**Options:** preview, selection, viewColumn, preserveFocus

---

## 3. WORKSPACE & FILE SYSTEM APIS

### 3.1 File System Operations

#### `vscode.workspace.fs`
**Methods Used:**
```typescript
fs.readFile(uri)           // Read file contents
fs.writeFile(uri, content) // Write file
fs.stat(uri)              // Get file metadata
fs.createDirectory(uri)   // Create directory
fs.delete(uri)           // Delete file/directory
```
**Locations:** Throughout misc files
**Lapce Need:** Direct std::fs operations

### 3.2 File Watching

#### `vscode.workspace.createFileSystemWatcher`
**Location:** WorkspaceTracker.ts:42
```typescript
const watcher = vscode.workspace.createFileSystemWatcher("**")
watcher.onDidCreate(uri => ...)
watcher.onDidDelete(uri => ...)
```
**Purpose:** Track file system changes
**Challenge:** Efficient file watching in Rust (notify crate)

### 3.3 Workspace Information

#### `vscode.workspace.workspaceFolders`
**Multiple locations**
**Purpose:** Get workspace root paths
**Lapce:** Direct access to project root

#### `vscode.workspace.getWorkspaceFolder(uri)`
**Location:** EditorUtils.ts:113
**Purpose:** Find workspace for a file
**Pattern:** Relative path computation

---

## 4. UI INTERACTION APIS

### 4.1 Tab Management

#### `vscode.window.tabGroups`
**Location:** WorkspaceTracker.ts:64, DiffViewProvider.ts:90-102
```typescript
vscode.window.tabGroups.all
  .map(group => group.tabs)
  .flat()
  .filter(tab => tab.input instanceof vscode.TabInputText)
```
**Purpose:** Track open files and active tabs
**Lapce Need:** Tab/buffer management API

### 4.2 User Dialogs

#### `vscode.window.showSaveDialog`
**Location:** export-markdown.ts:32-35
```typescript
vscode.window.showSaveDialog({
  filters: { Markdown: ["md"] },
  defaultUri: vscode.Uri.file(path)
})
```
**Purpose:** File save dialogs
**Lapce:** Native file dialogs

#### `vscode.window.showOpenDialog`
**Location:** process-images.ts:6-17
**Purpose:** File/image selection
**Options:** canSelectMany, filters

### 4.3 Messages & Notifications

#### `vscode.window.showErrorMessage`
**Location:** Multiple files
**Purpose:** Error notifications
**Lapce:** Status bar or notification system

#### `vscode.window.showInformationMessage`
**Location:** image-handler.ts:35
**Purpose:** Info notifications

---

## 5. LANGUAGE & DIAGNOSTIC APIS

### 5.1 Diagnostics

#### `vscode.languages.getDiagnostics`
**Location:** DiffViewProvider.ts:68, 244
```typescript
const preDiagnostics = vscode.languages.getDiagnostics()
// ... after edit ...
const postDiagnostics = vscode.languages.getDiagnostics()
const newProblems = getNewDiagnostics(preDiagnostics, postDiagnostics)
```
**Purpose:** Track compilation/lint errors
**Pattern:** Before/after comparison
**Lapce Need:** LSP diagnostic integration

### 5.2 Diagnostic Structure
```typescript
interface Diagnostic {
  message: string
  severity: DiagnosticSeverity
  range: Range
  source?: string
  code?: string | number
}
```

---

## 6. EXTENSION & ENVIRONMENT APIS

### 6.1 Extension APIs

#### `vscode.extensions.all`
**Location:** getTheme.ts:41
```typescript
for (const extension of vscode.extensions.all) {
  if (extension.packageJSON?.contributes?.themes?.length > 0) {
    // Theme processing
  }
}
```
**Purpose:** Access installed extensions
**Lapce:** Plugin system access

### 6.2 Environment APIs

#### `vscode.env`
```typescript
vscode.env.clipboard.readText()    // Clipboard read
vscode.env.clipboard.writeText()   // Clipboard write
vscode.env.appRoot                 // VS Code installation path
```
**Lapce Need:** System clipboard integration

---

## 7. COMMAND APIS

### 7.1 Command Execution

#### `vscode.commands.executeCommand`
**Common Commands:**
```typescript
"vscode.diff"                    // Open diff view
"vscode.open"                    // Open file
"revealInExplorer"              // Show in file tree
"workbench.action.terminal.*"   // Terminal commands
```
**Pattern:** String-based command dispatch
**Lapce:** Command palette integration

---

## 8. EVENT-DRIVEN PATTERNS

### 8.1 VS Code Event Subscriptions

| Event | Location | Purpose |
|-------|----------|---------|
| `onDidCloseTerminal` | TerminalRegistry:38 | Cleanup on terminal close |
| `onDidStartTerminalShellExecution` | TerminalRegistry:49 | Track command start |
| `onDidEndTerminalShellExecution` | TerminalRegistry:76 | Track command end |
| `onDidChangeTabs` | WorkspaceTracker:64 | Tab change tracking |
| `onDidCreate` (file) | WorkspaceTracker:45 | File creation |
| `onDidDelete` (file) | WorkspaceTracker:53 | File deletion |
| `onDidChangeVisibleTextEditors` | DiffViewProvider:531 | Editor visibility |
| `onDidOpenTextDocument` | DiffViewProvider:511 | Document open |

### 8.2 Custom Event Emitters

**BaseTerminalProcess Events:**
```typescript
'line': [line: string]
'continue': []
'completed': [output?: string]
'stream_available': [stream: AsyncIterable<string>]
'shell_execution_started': [pid: number | undefined]
'shell_execution_complete': [exitDetails: ExitCodeDetails]
'error': [error: Error]
'no_shell_integration': [message: string]
```

**Pattern:** Typed events with payloads
**Rust Translation:** tokio::sync::mpsc channels or crossbeam

---

## 9. ASYNC/PROMISE PATTERNS

### 9.1 Common Async Patterns

#### Promise.race for Timeouts
```typescript
Promise.race([
  pWaitFor(() => terminal.shellIntegration !== undefined),
  timeout(5000)
])
```
**Purpose:** Shell integration timeout
**Rust:** tokio::time::timeout

#### Promise.all for Parallel Operations
```typescript
await Promise.all(
  fileUris.map(async (uri) => {
    const buffer = await fs.readFile(uri.fsPath)
    return buffer.toString('base64')
  })
)
```
**Rust:** futures::future::join_all

#### Async Iteration
```typescript
for await (const line of stream) {
  // Process streaming data
}
```
**Rust:** tokio::stream::StreamExt

---

## 10. CRITICAL IMPLEMENTATION DETAILS

### 10.1 Terminal Output Processing Pipeline

1. **Command Execution** (Terminal.ts:80)
   - Send command via shellIntegration.executeCommand
   
2. **Stream Capture** (TerminalRegistry.ts:52)
   - onDidStartTerminalShellExecution gets stream

3. **Marker Detection** (TerminalProcess.ts:176-187)
   - Wait for OSC 633;C or 133;C start marker
   - Begin accumulating output

4. **Output Processing** (TerminalProcess.ts:193-206)
   - Accumulate chunks
   - Emit lines with throttling (100ms)
   - Track "hot" status for compilation

5. **Completion** (TerminalProcess.ts:242-257)
   - Detect OSC 633;D or 133;D end marker
   - Remove escape sequences
   - Emit completed event

### 10.2 Diff View Update Pipeline

1. **Open Diff** (DiffViewProvider.ts:48-111)
   - Create directories if needed
   - Get original content
   - Open diff editor

2. **Stream Updates** (DiffViewProvider.ts:113-188)
   - Receive accumulated content
   - Apply WorkspaceEdit
   - Update decorations (active line, faded overlay)
   - Manage scroll position

3. **Save Changes** (DiffViewProvider.ts:190-297)
   - Compare diagnostics before/after
   - Detect user edits
   - Apply final content

### 10.3 Workspace Tracking Pipeline

1. **Initialize** (WorkspaceTracker.ts:27-39)
   - List initial files (max 1000)
   - Start file watchers

2. **Track Changes** (WorkspaceTracker.ts:44-58)
   - File create/delete events
   - Update Set<string> of paths

3. **Debounced Updates** (WorkspaceTracker.ts:112-129)
   - 300ms debounce timer
   - Compute relative paths
   - Post to webview

---

## 11. PERFORMANCE CONSIDERATIONS

### 11.1 String Operations
- **indexOf vs regex:** 500x faster for escape sequences
- **Buffer.from(base64):** Memory-intensive for large files
- **stripAnsi:** Called on every terminal output

### 11.2 Memory Management
- **WeakRef:** Terminal/Task references (prevent leaks)
- **Set vs Array:** File path deduplication
- **Stream processing:** Avoid full content loading

### 11.3 Throttling & Debouncing
- **Terminal output:** 100ms throttle
- **Workspace updates:** 300ms debounce
- **Compilation detection:** 15s vs 2s hot timer

---

## 12. TRANSLATION CHALLENGES

### 12.1 High Complexity APIs

| VS Code API | Complexity | Rust Challenge |
|-------------|------------|----------------|
| shellIntegration | **10/10** | Must parse escape sequences, handle streams |
| vscode.diff | **8/10** | Custom diff view component needed |
| FileSystemWatcher | **7/10** | Cross-platform file watching |
| WorkspaceEdit | **6/10** | Transaction-based editing |
| TabGroups | **6/10** | Buffer/tab management |
| Diagnostics | **7/10** | LSP integration required |

### 12.2 Missing Lapce Equivalents

1. **Shell Integration Protocol**
   - No built-in OSC 633/133 support
   - Need custom PTY handling

2. **Diff View Component**
   - No native diff editor
   - Must build custom UI

3. **Decoration System**
   - Different rendering model
   - Need custom highlight system

4. **Extension/Theme Access**
   - Different plugin architecture
   - Need plugin API bridge

---

## 13. API USAGE FREQUENCY MATRIX

### 13.1 Top 20 Most Used APIs

| Rank | API | Count | Files Using | Critical |
|------|-----|-------|-------------|----------|
| 1 | `vscode.Uri` | 80+ | 15 | ✅ |
| 2 | `vscode.workspace.fs` | 60+ | 8 | ✅ |
| 3 | `vscode.window.showTextDocument` | 40+ | 5 | ✅ |
| 4 | `vscode.Range` | 35+ | 4 | ✅ |
| 5 | `vscode.Position` | 30+ | 4 | ✅ |
| 6 | `vscode.workspace.workspaceFolders` | 25+ | 6 | ✅ |
| 7 | `vscode.WorkspaceEdit` | 20+ | 2 | ✅ |
| 8 | `vscode.window.tabGroups` | 18+ | 2 | ⚠️ |
| 9 | `vscode.languages.getDiagnostics` | 15+ | 1 | ⚠️ |
| 10 | `vscode.window.createTerminal` | 12+ | 1 | ✅ |
| 11 | `vscode.commands.executeCommand` | 10+ | 4 | ⚠️ |
| 12 | `vscode.TextEditor` | 10+ | 2 | ✅ |
| 13 | `vscode.window.activeTextEditor` | 8+ | 2 | ⚠️ |
| 14 | `vscode.env.clipboard` | 6+ | 2 | ⚠️ |
| 15 | `vscode.Selection` | 6+ | 2 | ⚠️ |
| 16 | `vscode.FileType` | 5+ | 2 | ✅ |
| 17 | `vscode.Diagnostic` | 5+ | 1 | ⚠️ |
| 18 | `vscode.TabInputText` | 4+ | 2 | ⚠️ |
| 19 | `vscode.ThemeIcon` | 3+ | 1 | ❌ |
| 20 | `vscode.extensions.all` | 2+ | 1 | ❌ |

**Legend:** ✅ Critical | ⚠️ Important | ❌ Optional

---

## 14. KEY FINDINGS

### 14.1 Core Dependencies
1. **Terminal Integration:** Absolutely critical for command execution
2. **File System:** Heavy reliance on vscode.workspace.fs
3. **Text Editing:** WorkspaceEdit for all document changes
4. **Event System:** Deep integration with VS Code events
5. **Async Everywhere:** Promises/async-await throughout

### 14.2 Architecture Patterns
1. **Streaming:** Line-by-line processing for performance
2. **Debouncing:** Prevent UI flooding (300ms typical)
3. **WeakRef:** Memory leak prevention
4. **Event-Driven:** Pub-sub for loose coupling
5. **Escape Sequence Parsing:** Performance-critical string ops

### 14.3 Translation Strategy Requirements
1. **Must Have:** Terminal PTY, File System, Text Editing
2. **Should Have:** Diagnostics, Tab Management, Commands
3. **Nice to Have:** Themes, Extensions, Clipboard
4. **Can Skip:** VS Code specific commands

---

## 15. NEXT STEPS (Step 4 Preview)

Based on this API analysis, **Step 4: Lapce API Mapping** will map:

1. **Terminal APIs → Lapce Terminal/PTY**
2. **FileSystem APIs → std::fs + notify**
3. **WorkspaceEdit → Lapce Buffer Operations**
4. **Event System → tokio::sync channels**
5. **Diagnostics → LSP Client Integration**
6. **UI Dialogs → Native Lapce Dialogs**

---

**Step 3 Status:** ✅ **COMPLETE**  
**Analysis Depth:** Deep implementation-level  
**APIs Analyzed:** 80+ VS Code APIs  
**Next:** Step 4 - Lapce API Mapping
