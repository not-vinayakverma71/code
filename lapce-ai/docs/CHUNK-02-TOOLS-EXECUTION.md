# CHUNK 02: src/core/tools/ - 43 FILES TOOL EXECUTION

## Overview
These files implement the ACTUAL tool execution logic (not descriptions). Each tool receives AI requests and performs operations on the filesystem, terminal, or editor.

## File Inventory
```
tools/
├── readFileTool.ts (726 lines!) - Multi-file reading with line ranges
├── writeToFileTool.ts (319 lines) - File creation/modification
├── executeCommandTool.ts (365 lines) - Terminal command execution
├── applyDiffTool.ts (256 lines) - Surgical file edits
├── multiApplyDiffTool.ts - Batch diff operations
├── editFileTool.ts - Morph AI fast editing
├── searchFilesTool.ts - Ripgrep file search
├── listFilesTool.ts - Directory listing
├── codebaseSearchTool.ts - Semantic code search
├── browserActionTool.ts - Puppeteer automation
├── useMcpToolTool.ts - Dynamic MCP tool execution
├── askFollowupQuestionTool.ts - User interaction
├── attemptCompletionTool.ts - Task completion
├── insertContentTool.ts - Line insertion
├── searchAndReplaceTool.ts - Find/replace operations
├── updateTodoListTool.ts - TODO management
├── switchModeTool.ts - Mode switching
├── newTaskTool.ts - Subtask creation
├── listCodeDefinitionNamesTool.ts - Symbol extraction
├── accessMcpResourceTool.ts - MCP resource access
├── fetchInstructionsTool.ts - Instruction fetching
├── condenseTool.ts - Context condensation
├── ToolRepetitionDetector.ts - Anti-loop detection
├── validateToolUse.ts - Input validation
├── helpers/imageHelpers.ts - Image processing
└── __tests__/ (15 test files)
```

## Critical Architecture Patterns

### Step 29 IPC Architecture: ACTUAL Lapce Components

```
┌──────────────────────────────────────────────────────────┐
│  Lapce IDE (lapce-app/src/) - ACTUAL COMPONENTS         │
│  ┌────────────────────────────────────────────────────┐  │
│  │ EXISTING Lapce Infrastructure:                     │  │
│  │                                                    │  │
│  │ terminal/panel.rs (TerminalPanelData)             │  │
│  │   ├─ workspace: Arc<LapceWorkspace>                │  │
│  │   ├─ tab_info: RwSignal<TerminalTabInfo>           │  │
│  │   └─ common: Rc<CommonData>                        │  │
│  │                                                    │  │
│  │ editor/diff.rs (DiffEditorData)                   │  │
│  │   ├─ left: EditorData                              │  │
│  │   ├─ right: EditorData                             │  │
│  │   └─ changes: Vec<DiffLines>                       │  │
│  │                                                    │  │
│  │ ai_panel/message_handler.rs (MessageHandler)      │  │
│  │   ├─ bridge: Arc<LapceAiInterface>                 │  │
│  │   ├─ editor_proxy: Arc<EditorProxy>                │  │
│  │   └─ file_system: Arc<FileSystemBridge>            │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                              │
│  ┌────────────────────────▼───────────────────────────┐  │
│  │ window_tab.rs (CommonData) - ADD THIS:            │  │
│  │ pub ai_ipc: Arc<LapceAiIpcClient>                 │  │
│  │ - Shared by ALL components                        │  │
│  └────────────────────────┬───────────────────────────┘  │
└─────────────────────────────┼────────────────────────────┘
                              │
                     ═════════▼═════════
                     SharedMemory IPC
                     
                     ═════════│═════════
                               │
┌─────────────────────────────▼──────────────────────────────┐
│  lapce-ai/src/ (Backend)                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ipc_server.rs (MessageRouter)                        │  │
│  │ - Listens on SharedMemory                            │  │
│  │ - Routes to tool handlers                            │  │
│  └──────────┬───────────────────────────────────────────┘  │
│             │                                               │
│  ┌──────────▼───────────────────────────────────────────┐  │
│  │ handlers/tools/ (NEW - Need to create)               │  │
│  │ ├─ terminal.rs (ExecuteCommandTool + OSC parser)     │  │
│  │ ├─ diff.rs (ApplyDiffTool + 3 strategies)           │  │
│  │ ├─ file.rs (ReadFileTool, WriteFileTool)            │  │
│  │ ├─ search.rs (SearchFilesTool + ripgrep)            │  │
│  │ └─ mod.rs (ToolRegistry + ToolHandler trait)        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 1. Tool Execution via IPC - ACTUAL Lapce Integration

#### File: `lapce-app/src/ai_panel/message_handler.rs` (EXISTING, 408 lines)

**MODIFY existing MessageHandler** to add tool execution:

```rust
// EXISTING struct (lines 8-14)
pub struct MessageHandler {
    bridge: Arc<LapceAiInterface>,  // CHANGE to ipc_client
    editor_proxy: Arc<EditorProxy>,
    file_system: Arc<FileSystemBridge>,
    pending_responses: Arc<RwLock<HashMap<String, ResponseChannel>>>,
}

// ADD new methods
impl MessageHandler {
    /// Execute any tool via backend
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Get IPC client from CommonData
        let ipc_client = self.get_ipc_client()?;
        
        // Send via SharedMemory IPC
        let response = ipc_client.send(IpcMessage::ExecuteTool {
            tool: tool_name.to_string(),
            params,
        }).await?;
        
        match response {
            IpcMessage::ToolResult { output, .. } => Ok(output),
            IpcMessage::Error { message, .. } => Err(anyhow!(message)),
            _ => Err(anyhow!("Unexpected response")),
        }
    }
    
    /// Execute file read tool
    pub async fn read_file(
        &self,
        path: String,
        line_ranges: Option<Vec<(usize, usize)>>,
    ) -> Result<String> {
        let params = json!({
            "path": path,
            "line_ranges": line_ranges,
        });
        
        let result = self.execute_tool("read_file", params).await?;
        Ok(result.as_str().unwrap().to_string())
    }
    
    /// Execute file write tool
    pub async fn write_file(&self, path: String, content: String) -> Result<()> {
        let params = json!({
            "path": path,
            "content": content,
        });
        
        self.execute_tool("write_to_file", params).await?;
        Ok(())
    }
    
    fn get_ipc_client(&self) -> Result<Arc<LapceAiIpcClient>> {
        // Access from CommonData (shared across all components)
        todo!("Get from CommonData.ai_ipc")
    }
}
```

**Backend Side (lapce-ai/src/handlers/tools/mod.rs)**:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(
        &self,
        task: &mut Task,
        block: &ToolUse,
        ask_approval: &dyn AskApproval,
        handle_error: &dyn HandleError,
        push_result: &dyn PushToolResult,
    ) -> Result<(), ToolError>;
}

pub struct ToolHandler {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolHandler {
    pub async fn handle_execute(
        &self,
        tool: String,
        params: ToolParams,
    ) -> Result<IpcMessage> {
        let tool_impl = self.tools.get(&tool)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool))?;
        
        let result = tool_impl.execute(params).await?;
        
        Ok(IpcMessage::ToolResult {
            tool,
            output: result,
        })
    }
}
```

### 2. Terminal Integration (from Step 29)

**Terminal Integration - ACTUAL Lapce Component:**

#### File: `lapce-app/src/terminal/panel.rs` (EXISTING, 821 lines)

**EXTEND existing TerminalPanelData** (currently at line 36):

```rust
// EXISTING struct (lines 36-44)
#[derive(Clone)]
pub struct TerminalPanelData {
    pub cx: Scope,
    pub workspace: Arc<LapceWorkspace>,
    pub tab_info: RwSignal<TerminalTabInfo>,
    pub debug: RunDebugData,
    pub breakline: Memo<Option<(usize, PathBuf)>>,
    pub common: Rc<CommonData>,  // <- Has ai_ipc!
    pub main_split: MainSplitData,
}

// ADD new method for AI command execution
impl TerminalPanelData {
    /// Execute command via AI backend (with OSC parsing)
    pub async fn execute_ai_command(&self, cmd: String) -> Result<()> {
        // Get IPC client from CommonData
        let ipc_client = self.common.ai_ipc.clone();
        
        // Get current workspace directory
        let cwd = Some(self.workspace.path.clone());
        
        // Send via SharedMemory IPC
        let mut stream = ipc_client.send_stream(IpcMessage::ExecuteCommand {
            cmd: cmd.clone(),
            cwd,
        }).await?;
        
        // Stream output updates to active terminal
        while let Some(msg) = stream.recv().await {
            match msg {
                IpcMessage::TerminalOutput { data, markers } => {
                    // Update existing terminal data
                    self.tab_info.update(|info| {
                        if let Some((_, tab_data)) = info.tabs.get(info.active) {
                            // Use existing TerminalData::receive_data()
                            tab_data.terminal.with_untracked(|term| {
                                if let Some(term) = term.raw.as_ref() {
                                    // Feed data to existing alacritty terminal
                                    term.lock().unwrap().pty_write(data);
                                }
                            });
                        }
                    });
                }
                IpcMessage::CommandComplete { exit_code, .. } => {
                    log::info!("Command completed with exit code: {:?}", exit_code);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

**Backend: OSC 633/133 Shell Integration (from Step 29)**:
```rust
// lapce-ai/src/handlers/tools/terminal.rs
pub struct TerminalTool {
    parser: Arc<OscParser>,  // From Step 29
}

impl TerminalTool {
    pub async fn execute_command(&self, cmd: String) -> Result<IpcMessage> {
        let mut pty = create_pty().await?;
        pty.write_all(format!("{}
", cmd).as_bytes()).await?;
        
        let mut output = Vec::new();
        let mut markers = Vec::new();
        
        loop {
            let data = pty.read().await?;
            
            // Parse OSC markers (500x faster with memchr)
            let chunk_markers = self.parser.parse(&data);
            markers.extend(chunk_markers);
            
            output.extend_from_slice(&data);
            
            // Check for end marker
            if markers.iter().any(|m| matches!(m, ShellMarker::CommandOutputEnd(_))) {
                break;
            }
        }
        
        Ok(IpcMessage::CommandComplete { exit_code: 0, duration_ms: 0 })
    }
}
```

### 3. Diff View Streaming (from Step 29)

**Diff View Integration - ACTUAL Lapce Component:**

#### File: `lapce-app/src/editor/diff.rs` (EXISTING, 548 lines)

**EXTEND existing DiffEditorData** (currently at line 31):

```rust
// EXISTING imports (use these)
use lapce_core::buffer::diff::{DiffExpand, DiffLines, expand_diff_lines, rope_diff};
use lapce_core::buffer::rope_text::RopeText;
use lapce_xi_rope::Rope;

// EXISTING struct (lines 31-35)
#[derive(Clone)]
pub struct DiffInfo {
    pub is_right: bool,
    pub changes: Vec<DiffLines>,  // Already has diff infrastructure!
}

// ADD new method for AI diff streaming
impl DiffEditorData {
    /// Apply AI-generated diff with streaming updates
    pub async fn apply_ai_diff(&self, changes: String) -> Result<()> {
        // Get file path from left editor
        let file_path = self.left.doc.content.path()
            .ok_or_else(|| anyhow!("No file path"))?;
        
        // Get original content
        let original = self.left.doc.buffer.with(|buffer| {
            buffer.text().to_string()
        });
        
        // Get IPC client from CommonData
        let ipc_client = self.common.ai_ipc.clone();
        
        // Request diff from backend
        let mut stream = ipc_client.send_stream(IpcMessage::RequestDiff {
            file_path: file_path.clone(),
            original,
            modified: changes,
        }).await?;
        
        // Stream line-by-line diff updates
        while let Some(msg) = stream.recv().await {
            match msg {
                IpcMessage::StreamDiffLine { line_num, content, change_type } => {
                    // Use EXISTING rope_diff() infrastructure
                    let new_rope = Rope::from(content);
                    
                    // Update right editor with new content
                    self.right.doc.reload(DocContent::History(
                        file_path.clone(),
                    ), false);
                    
                    // Compute diff using existing Lapce infrastructure
                    let diff = rope_diff(
                        self.left.doc.buffer.get_untracked().text(),
                        &new_rope,
                        None,
                        None,
                    );
                    
                    // Update changes vector (EXISTING field)
                    self.diff_info.update(|info| {
                        info.changes = diff;
                    });
                }
                IpcMessage::DiffComplete { total_lines } => {
                    log::info!("Diff complete: {} lines changed", total_lines);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

**Backend - Diff Generation:**
```rust
// lapce-ai/src/handlers/tools/diff.rs
impl DiffTool {
    pub async fn apply_diff(&self, file: PathBuf, changes: String) -> Vec<IpcMessage> {
        let mut messages = Vec::new();
        
        // Compute diff
        let diff = self.diff_engine.compute(&self.original, &changes);
        
        // Stream each line
        for (line_num, change) in diff.changes.iter().enumerate() {
            messages.push(IpcMessage::StreamDiffLine {
                line_num,
                content: change.content.clone(),
                change_type: change.change_type,
            });
        }
        
        messages.push(IpcMessage::DiffComplete {
            total_lines: diff.changes.len(),
        });
        
        messages
    }
}
```

### 4. Partial Message Streaming
Tools support streaming progress updates BEFORE completion:

**RUST:**
```rust
if block.partial {
    let partial_msg = IpcMessage::ToolProgress {
        tool: "readFile",
        path: Some(path.clone()),
        content: None,
        partial: true,
    };
    task.ask(AskType::Tool, &serde_json::to_string(&partial_msg)?, true).await?;
    return Ok(());
}
```

### 3. Permission System (Critical UX)
Every destructive operation requires approval:

```typescript
const didApprove = await askApproval("command", command)
if (!didApprove) {
    return // User rejected
}
```

**Approval Types:**
- `command` - Terminal commands
- `tool` - File operations
- `api_req_start` - API requests
- `followup` - Questions to user

**RUST:**
```rust
let approved = ask_approval.request(ApprovalType::Command, &command).await?;
if !approved {
    return Ok(()); // Early return, no error
}
```

### 4. .rooignore Integration
Files can be protected from AI access:

```typescript
const accessAllowed = cline.rooIgnoreController?.validateAccess(relPath)
if (!accessAllowed) {
    await cline.say("rooignore_error", relPath)
    pushToolResult(formatResponse.toolError(formatResponse.rooIgnoreError(relPath)))
    return
}
```

**RUST:**
```rust
if !task.roo_ignore.validate_access(&rel_path)? {
    task.say(SayType::RooIgnoreError, &rel_path).await?;
    push_result.push(&format_response::roo_ignore_error(&rel_path))?;
    return Ok(());
}
```

### 5. Error Tracking & Recovery
```typescript
cline.consecutiveMistakeCount++
cline.recordToolError("write_to_file")

// Per-tool error tracking
const currentCount = (cline.consecutiveMistakeCountForApplyDiff.get(relPath) || 0) + 1
cline.consecutiveMistakeCountForApplyDiff.set(relPath, currentCount)

if (currentCount >= 2) {
    await cline.say("diff_error", formattedError)
}
```

**Purpose:** Detect when AI is stuck in a loop trying the same failing operation.

**RUST:**
```rust
task.consecutive_mistake_count += 1;
task.record_tool_error("write_to_file");

let count = task.consecutive_mistakes_per_tool
    .entry(tool_name.to_string())
    .and_modify(|c| *c += 1)
    .or_insert(1);

if *count >= 2 {
    task.say(SayType::DiffError, &error_msg).await?;
}
```

## Deep Dive: readFileTool.ts (726 lines!)

### Features
1. **Multi-file reading** - Read up to 5 files in one request
2. **Line range support** - Read specific line ranges `1-50, 100-150`
3. **Binary file handling** - Extract text from PDF, DOCX
4. **Image support** - Base64 encode images for vision models
5. **Memory tracking** - Prevent excessive image memory usage

### XML Parsing
```typescript
const parsed = parseXml(argsXmlTag) as any
const files = Array.isArray(parsed.file) ? parsed.file : [parsed.file].filter(Boolean)

for (const file of files) {
    const fileEntry: FileEntry = {
        path: file.path,
        lineRanges: [],
    }
    
    if (file.line_range) {
        const ranges = Array.isArray(file.line_range) ? file.line_range : [file.line_range]
        for (const range of ranges) {
            const match = String(range).match(/(\d+)-(\d+)/)
            if (match) {
                fileEntry.lineRanges?.push({ start, end })
            }
        }
    }
}
```

### Binary File Handling
```typescript
const isBinary = await isBinaryFile(absolutePath)
if (isBinary) {
    const supportedFormats = getSupportedBinaryFormats()
    if (isSupportedImageFormat(absolutePath)) {
        // Handle images
        const imageData = await processImageFile(absolutePath, ...)
        result.imageDataUrl = imageData.dataUrl
    } else if (supportedFormats.includes(ext)) {
        // Extract text from PDF/DOCX
        fileContent = await extractTextFromFile(absolutePath)
    }
}
```

### Line Numbering
```typescript
fileContent = addLineNumbers(fileContent, startLine)
// Output: "1 | const x = 1\n2 | const y = 2"
```

**CRITICAL:** This format is used throughout prompts. AI expects line numbers for diffs.

### Image Memory Tracking
```typescript
const imageMemoryTracker = new ImageMemoryTracker(
    DEFAULT_MAX_IMAGE_FILE_SIZE_MB,
    DEFAULT_MAX_TOTAL_IMAGE_SIZE_MB
)

for (const result of results) {
    if (result.imageDataUrl) {
        imageMemoryTracker.addImage(result.path, result.imageDataUrl)
    }
}

if (imageMemoryTracker.exceedsLimits()) {
    // Reject image processing
}
```

## Deep Dive: executeCommandTool.ts (365 lines)

### Shell Integration
Uses VS Code's Terminal API for capturing output:

```typescript
const terminal = new Terminal(cline.terminalManager, ...)
const process = terminal.runCommand(command, {
    onLine: (line) => output.push(line),
    onCompleted: (fullOutput) => { /* done */ },
    onShellExecutionStarted: (pid) => { /* track PID */ }
})
```

### Output Limiting
```typescript
const {
    terminalOutputLineLimit = 500,
    terminalOutputCharacterLimit = DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT,
} = providerState ?? {}

// Truncate output if too large
if (output.length > terminalOutputLineLimit) {
    output = output.slice(-terminalOutputLineLimit)
}
```

### Timeout Support
```typescript
const commandExecutionTimeout = isCommandAllowlisted ? 0 : commandExecutionTimeoutSeconds * 1000

// Allowlist for long-running commands
const commandTimeoutAllowlist = ["npm install", "cargo build", ...]
```

### Fallback Without Shell Integration
```typescript
try {
    const [rejected, result] = await executeCommand(task, options)
} catch (error) {
    if (error instanceof ShellIntegrationError) {
        // Retry without shell integration
        await executeCommand(task, {
            ...options,
            terminalShellIntegrationDisabled: true
        })
    }
}
```

## Deep Dive: applyDiffTool.ts (256 lines)

### Diff Strategy Pattern
```typescript
const diffResult = await cline.diffStrategy?.applyDiff(
    originalContent,
    diffContent,
    parseInt(block.params.start_line ?? "")
)

if (!diffResult.success) {
    // Track consecutive failures
    const currentCount = (cline.consecutiveMistakeCountForApplyDiff.get(relPath) || 0) + 1
    
    if (currentCount >= 2) {
        await cline.say("diff_error", formattedError)
    }
}
```

### Diff Strategies Available
1. **Unified Diff** - Standard patch format
2. **Search/Replace** - Find exact text, replace
3. **Line Range** - Replace lines N-M

**RUST REQUIREMENT:** Implement all 3 diff strategies with exact matching behavior.

## Deep Dive: writeToFileTool.ts (319 lines)

### Pre-processing Content
Handle weaker model artifacts:

```typescript
// Remove markdown code blocks
if (newContent.startsWith("```")) {
    newContent = newContent.split("\n").slice(1).join("\n")
}
if (newContent.endsWith("```")) {
    newContent = newContent.split("\n").slice(0, -1).join("\n")
}

// Unescape HTML entities (non-Claude models)
if (!cline.api.getModel().id.includes("claude")) {
    newContent = unescapeHtmlEntities(newContent)
}

// Strip line numbers if present
if (everyLineHasLineNumbers(newContent)) {
    newContent = stripLineNumbers(newContent)
}
```

### Streaming Edits to UI
```typescript
if (block.partial) {
    if (!cline.diffViewProvider.isEditing) {
        await cline.diffViewProvider.open(relPath)
    }
    await cline.diffViewProvider.update(newContent, false)
    return
}
```

### Omission Detection
```typescript
const omissionInfo = detectCodeOmission(newContent, predictedLineCount)
if (omissionInfo.hasOmission) {
    // Warn about truncated content
    await cline.say("warning", "Content appears truncated")
}
```

## Tool Registry System

```typescript
// In Task.ts
const toolHandlers = {
    read_file: readFileTool,
    write_to_file: writeToFileTool,
    execute_command: executeCommandTool,
    apply_diff: applyDiffTool,
    search_files: searchFilesTool,
    list_files: listFilesTool,
    // ... 20+ tools
}

const handler = toolHandlers[toolName]
await handler(this, block, askApproval, handleError, pushToolResult, removeClosingTag)
```

**RUST:**
```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
        tools.insert("read_file".to_string(), Box::new(ReadFileTool));
        tools.insert("write_to_file".to_string(), Box::new(WriteToFileTool));
        tools.insert("execute_command".to_string(), Box::new(ExecuteCommandTool));
        // ... 20+ tools
        Self { tools }
    }
    
    pub async fn execute(
        &self,
        tool_name: &str,
        task: &mut Task,
        block: &ToolUse,
        ...
    ) -> Result<(), ToolError> {
        let tool = self.tools.get(tool_name)
            .ok_or(ToolError::NotFound(tool_name.to_string()))?;
        tool.execute(task, block, ...).await
    }
}
```

## VS Code Dependencies to Replace

### For Lapce:
```rust
// VS Code Terminal → Lapce Process
vscode.window.createTerminal() → tokio::process::Command

// VS Code File Operations → Tokio FS
vscode.workspace.fs.readFile() → tokio::fs::read()
vscode.workspace.fs.writeFile() → tokio::fs::write()

// VS Code Workspace Config → Lapce Config
vscode.workspace.getConfiguration() → ctx.get_config::<Config>()

// VS Code Diff View → Lapce Diff
vscode.commands.executeCommand("vscode.diff", ...) → Custom diff UI
```

## Translation Requirements

### Phase 1: Tool Trait System
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, ctx: ToolContext) -> Result<ToolResult, ToolError>;
    fn validate(&self, params: &ToolParams) -> Result<(), ValidationError>;
}

pub struct ToolContext<'a> {
    task: &'a mut Task,
    block: &'a ToolUse,
    ask_approval: &'a dyn AskApproval,
    handle_error: &'a dyn HandleError,
    push_result: &'a dyn PushToolResult,
}
```

### Phase 2: 20+ Tool Implementations
Each tool needs:
1. Parameter parsing from XML
2. Permission checking (rooignore, write-protected)
3. Error handling with retry logic
4. Partial message streaming
5. Result formatting

### Phase 3: Helper Systems
1. **XML Parser** - Parse tool use blocks
2. **Line Numbering** - Add/strip line numbers
3. **Binary File Handler** - PDF/DOCX/Image extraction
4. **Diff Engine** - Apply surgical edits
5. **Terminal Manager** - Command execution

---

# Part 3: Lapce Integration Points

## 3.1 Integration Discovery

Lapce has **terminal infrastructure** but no tool execution bridge. Found:
- `TerminalData` - Terminal state management
- `TerminalPanelData` - UI panel for terminals
- `tokio::process::Command` - Process execution

Missing:
- Tool execution service
- File operation bridge
- Permission system
- Diff view integration

## 3.2 File Operations Bridge

```rust
// lapce-app/src/ai/tools/file_ops.rs (NEW)

use tokio::fs;
use std::path::PathBuf;

pub struct FileOperations {
    workspace_dir: PathBuf,
    roo_ignore: Arc<RooIgnoreController>,
}

impl FileOperations {
    pub async fn read_file(
        &self,
        rel_path: &str,
        line_ranges: Option<Vec<(usize, usize)>>,
    ) -> Result<String, FileError> {
        // Validate access
        if !self.roo_ignore.validate_access(rel_path)? {
            return Err(FileError::AccessDenied(rel_path.to_string()));
        }
        
        let abs_path = self.workspace_dir.join(rel_path);
        let content = fs::read_to_string(&abs_path).await?;
        
        // Apply line ranges if specified
        if let Some(ranges) = line_ranges {
            Ok(self.extract_line_ranges(&content, &ranges))
        } else {
            Ok(self.add_line_numbers(&content))
        }
    }
    
    pub async fn write_file(
        &self,
        rel_path: &str,
        content: &str,
    ) -> Result<(), FileError> {
        // Validate access
        if !self.roo_ignore.validate_access(rel_path)? {
            return Err(FileError::AccessDenied(rel_path.to_string()));
        }
        
        let abs_path = self.workspace_dir.join(rel_path);
        
        // Create parent directories
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(&abs_path, content).await?;
        Ok(())
    }
}
```

## 3.3 Terminal Bridge

```rust
// lapce-app/src/ai/tools/terminal_ops.rs (NEW)

use crate::terminal::data::TerminalData;

pub struct TerminalOperations {
    terminal_panel: Arc<TerminalPanelData>,
}

impl TerminalOperations {
    pub async fn execute_command(
        &self,
        command: &str,
        cwd: Option<PathBuf>,
    ) -> Result<CommandResult, TerminalError> {
        // Create or reuse terminal
        let term_id = self.get_or_create_terminal().await?;
        
        // Execute command
        let (tx, mut rx) = mpsc::channel(100);
        
        let handle = tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c").arg(command);
            
            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }
            
            let mut child = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            
            // Stream output
            let mut output = String::new();
            // ... collect output with line limits
            
            let status = child.wait().await?;
            Ok(CommandResult { output, exit_code: status.code() })
        });
        
        handle.await?
    }
}
```

---

# Part 4: IPC Message Protocol

## 4.1 Tool Execution Messages

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionMessage {
    // ... existing from CHUNK-03
    
    // === TOOL EXECUTION ===
    
    /// Tool execution started
    ToolExecutionStarted {
        tool_name: String,
        params: serde_json::Value,
    },
    
    /// Tool execution progress (partial)
    ToolExecutionProgress {
        tool_name: String,
        partial_result: String,
    },
    
    /// Tool execution completed
    ToolExecutionCompleted {
        tool_name: String,
        result: String,
        duration_ms: u64,
    },
    
    /// Tool execution failed
    ToolExecutionFailed {
        tool_name: String,
        error: String,
        consecutive_failures: u32,
    },
    
    /// Command execution status
    CommandExecutionStatus {
        execution_id: String,
        status: CommandStatus,
        output: Option<String>,
    },
    
    /// File operation result
    FileOperationResult {
        operation: String, // "read", "write", "diff"
        path: String,
        success: bool,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandStatus {
    Started,
    Output,
    Completed { exit_code: Option<i32> },
    Timeout,
    Fallback, // Retry without shell integration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    // ... existing from CHUNK-03
    
    // === TOOL APPROVAL ===
    
    /// Approve tool execution
    ApproveToolExecution {
        tool_name: String,
        approved: bool,
    },
    
    /// Cancel running command
    CancelCommand {
        execution_id: String,
    },
    
    /// Retry failed tool
    RetryTool {
        tool_name: String,
        params: serde_json::Value,
    },
}
```

## 4.2 Message Flow

```
[AI generates tool use]
    ↓
[Task parses XML]
    ↓
[Send ToolExecutionStarted to UI]
    ↓
[Request approval if needed]
    ↓ (ApproveToolExecution from UI)
[Execute tool]
    ↓ (stream partial results)
[Send ToolExecutionProgress]
    ↓
[Tool completes]
    ↓
[Send ToolExecutionCompleted]
    ↓
[Push result to conversation]
```

---

# Part 5: Error Recovery Patterns

## 5.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied for {operation} on {path}")]
    PermissionDenied { operation: String, path: String },
    
    #[error("File access blocked by .rooignore: {0}")]
    RooIgnoreBlocked(String),
    
    #[error("Parameter missing: {tool} requires {param}")]
    MissingParameter { tool: String, param: String },
    
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("Diff application failed: {0}")]
    DiffFailed(String),
    
    #[error("Tool execution timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Repetition limit reached: {0} called {1} times")]
    RepetitionLimit(String, u32),
    
    #[error("Binary file not supported: {0}")]
    UnsupportedBinary(String),
}
```

## 5.2 Recovery Strategies

```rust
impl ToolExecutor {
    pub async fn execute_with_recovery(
        &self,
        tool_name: &str,
        block: &ToolUse,
        task: &mut Task,
    ) -> Result<String, ToolError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        loop {
            match self.execute_tool(tool_name, block, task).await {
                Ok(result) => return Ok(result),
                
                Err(ToolError::RooIgnoreBlocked(path)) => {
                    // Not recoverable - inform user
                    task.say(SayType::RooIgnoreError, &path).await?;
                    return Err(ToolError::RooIgnoreBlocked(path));
                }
                
                Err(ToolError::CommandFailed(e)) if attempts < MAX_ATTEMPTS => {
                    warn!("Command failed (attempt {}): {}", attempts + 1, e);
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempts))).await;
                }
                
                Err(ToolError::DiffFailed(e)) => {
                    // Track consecutive failures
                    task.consecutive_mistake_count += 1;
                    let count = task.consecutive_mistakes_per_tool
                        .entry(tool_name.to_string())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    
                    if *count >= 2 {
                        task.say(SayType::DiffError, &e).await?;
                    }
                    
                    return Err(ToolError::DiffFailed(e));
                }
                
                Err(ToolError::Timeout(_)) => {
                    // Offer to retry without timeout
                    task.ask_followup(
                        "Command timed out. Retry without timeout?",
                        &["Yes", "No"],
                    ).await?;
                    // ... handle response
                }
                
                Err(e) => return Err(e),
            }
        }
    }
}
```

## 5.3 Repetition Detection

```rust
pub struct ToolRepetitionDetector {
    previous_tool: Option<String>,
    consecutive_count: u32,
    limit: u32,
}

impl ToolRepetitionDetector {
    pub fn check(&mut self, tool_name: &str, params: &ToolParams) -> Result<(), ToolError> {
        let tool_signature = format!("{}:{}", tool_name, serde_json::to_string(params)?);
        
        if self.previous_tool.as_ref() == Some(&tool_signature) {
            self.consecutive_count += 1;
        } else {
            self.consecutive_count = 0;
            self.previous_tool = Some(tool_signature);
        }
        
        // Exception: browser scrolling can repeat 10x
        let limit = if tool_name == "browser_action" {
            self.limit * 10
        } else {
            self.limit
        };
        
        if self.consecutive_count >= limit {
            self.consecutive_count = 0;
            self.previous_tool = None;
            return Err(ToolError::RepetitionLimit(tool_name.to_string(), limit));
        }
        
        Ok(())
    }
}
```

---

# Part 6: Benchmark Specifications

```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn bench_read_file(c: &mut Criterion) {
        c.bench_function("read_single_file", |b| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let tool = ReadFileTool::new();
            let block = ToolUse { /* ... */ };
            
            b.iter(|| {
                runtime.block_on(async {
                    let start = Instant::now();
                    let result = tool.execute(&mut task, &block, ...).await.unwrap();
                    let elapsed = start.elapsed();
                    
                    // Target: <50ms for single file
                    assert!(elapsed < Duration::from_millis(50));
                    black_box(result);
                });
            });
        });
        
        c.bench_function("read_multiple_files", |b| {
            b.iter(|| {
                // Target: <200ms for 5 files
                // ...
            });
        });
    }
    
    fn bench_execute_command(c: &mut Criterion) {
        c.bench_function("command_overhead", |b| {
            b.iter(|| {
                // Measure overhead without actual command
                // Target: <5ms for setup
            });
        });
    }
    
    fn bench_apply_diff(c: &mut Criterion) {
        c.bench_function("unified_diff", |b| {
            b.iter(|| {
                // Target: <100ms for 1000-line file
            });
        });
    }
    
    fn bench_tool_registry(c: &mut Criterion) {
        c.bench_function("tool_lookup", |b| {
            let registry = ToolRegistry::new();
            b.iter(|| {
                let tool = registry.get("read_file").unwrap();
                // Target: <1μs for lookup
                black_box(tool);
            });
        });
    }
}
```

| Test | Target | Notes |
|------|--------|-------|
| Single file read | <50ms | Including line numbering |
| Multi-file read (5) | <200ms | Parallel reads |
| Command setup | <5ms | Excludes actual execution |
| Diff application | <100ms | 1000-line file |
| Tool lookup | <1μs | HashMap access |
| Binary file detect | <10ms | isBinaryFile check |
| XML parsing | <5ms | Tool use block |
| Permission check | <1ms | .rooignore validation |

---

# Summary: CHUNK-02 Complete

## Files Analyzed: 28 tools (5,643 lines)

### Top 5 by complexity:
1. **readFileTool.ts** (726 lines) - Multi-file, line ranges, images, binary
2. **executeCommandTool.ts** (365 lines) - Terminal integration, timeouts
3. **writeToFileTool.ts** (319 lines) - Streaming edits, omission detection
4. **applyDiffTool.ts** (256 lines) - Diff strategies, retry logic
5. **multiApplyDiffTool.ts** (230 lines) - Batch operations

## Key Patterns Extracted:
1. **Standard tool signature** (6 parameters)
2. **Partial message streaming** for real-time UI
3. **Permission system** (askApproval callback)
4. **.rooignore integration** for file protection
5. **Error tracking** (consecutive mistakes, per-tool counters)
6. **Repetition detection** (anti-loop with 3x limit)

## Lapce Integration:
- File operations: NEW `FileOperations` service
- Terminal: Bridge to existing `TerminalPanelData`
- Permissions: NEW `PermissionManager`
- Diff: NEW `DiffEngine` (3 strategies)

## IPC Protocol:
- 4 new ExtensionMessage types (Started, Progress, Completed, Failed)
- 3 new WebviewMessage types (Approve, Cancel, Retry)
- CommandStatus enum (Started, Output, Completed, Timeout, Fallback)

## Error Recovery:
- 9 error types defined
- 3 recovery strategies: retry with backoff, fallback, user prompt
- Repetition detection with 3x limit (30x for browser)

## Benchmarks:
- 8 performance targets specified
- Range: <1μs (lookup) to <200ms (multi-file read)

**Result**: Production-ready specification for 28 tool implementations in Rust.
