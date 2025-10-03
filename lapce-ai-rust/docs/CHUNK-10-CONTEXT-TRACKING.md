# CHUNK-10: CONTEXT-TRACKING (FILE MODIFICATION & CONVERSATION STATE)

## 📁 Complete System Analysis

```
Context Tracking System:
├── Codex/src/core/context-tracking/
│   ├── FileContextTracker.ts              (228 lines) - File modification tracking
│   └── FileContextTrackerTypes.ts         (29 lines)  - Type definitions
├── Codex/src/core/task-persistence/
│   └── apiMessages.ts                     (84 lines)  - API message persistence
└── Codex/src/core/task/Task.ts            (2,859 lines) - Conversation management
    └── apiConversationHistory: ApiMessage[]

TOTAL: 3,200+ lines conversation & file tracking
```

---

## 🎯 PURPOSE

**Dual Tracking System**:

1. **File Context Tracking**: Detect when files are modified outside of AI's control (user edits) to prevent stale context and diff conflicts
2. **Conversation History**: Persist API messages (user/assistant) to disk for session resumption and debugging

**Critical for**:
- Preventing diff application failures on stale files
- Resuming conversations from history
- Debug analysis of AI behavior
- Context window management
- Checkpoint coordination

---

## 📊 ARCHITECTURE OVERVIEW

```
FileContextTracker:
┌─────────────────────────────────────────────────┐
│ File Tracking Lifecycle                         │
│                                                  │
│ 1. File mentioned/read → trackFileContext()     │
│ 2. Setup VSCode FileSystemWatcher               │
│ 3. Save metadata entry (state: "active")        │
│ 4. Detect changes:                              │
│    - Roo edit → Mark as "edited_by_roo"         │
│    - User edit → Mark as "stale"                │
│ 5. Before diff → Check if stale → Re-read file  │
└─────────────────────────────────────────────────┘

API Message History:
┌─────────────────────────────────────────────────┐
│ Persistence Flow                                 │
│                                                  │
│ Task.apiConversationHistory[] (in-memory)       │
│         ↓                                        │
│ saveApiMessages() after each API call           │
│         ↓                                        │
│ ~/.config/Code/User/globalStorage/.../tasks/    │
│   {taskId}/api_conversation_history.json        │
│         ↓                                        │
│ readApiMessages() on task resume                │
└─────────────────────────────────────────────────┘

Metadata Storage:
{
  "files_in_context": [
    {
      "path": "src/main.rs",
      "record_state": "active",      // or "stale"
      "record_source": "read_tool",   // or "roo_edited", "user_edited", "file_mentioned"
      "roo_read_date": 1696234567890,
      "roo_edit_date": 1696234567900,
      "user_edit_date": null
    }
  ]
}
```

---

## 🔧 FILE 1: FileContextTracker.ts (228 lines)

### Purpose: Detect Stale Context

**Problem**: User edits file after AI reads it → AI's context becomes stale → diff application fails.

**Solution**: Track file operations, watch for external changes, mark entries as stale.

### Class Structure

```typescript
export class FileContextTracker {
    readonly taskId: string
    private providerRef: WeakRef<ClineProvider>
    
    // Tracking sets
    private fileWatchers = new Map<string, vscode.FileSystemWatcher>()
    private recentlyModifiedFiles = new Set<string>()
    private recentlyEditedByRoo = new Set<string>()
    private checkpointPossibleFiles = new Set<string>()
    
    constructor(provider: ClineProvider, taskId: string) {
        this.providerRef = new WeakRef(provider)
        this.taskId = taskId
    }
}
```

**WeakRef pattern**: Prevents memory leaks (provider can be GC'd).

---

### Method 1: trackFileContext() - Lines 81-95

**Main entry point** - Called whenever AI interacts with a file.

```typescript
async trackFileContext(filePath: string, operation: RecordSource) {
    try {
        const cwd = this.getCwd()
        if (!cwd) return
        
        // Add to metadata
        await this.addFileToFileContextTracker(this.taskId, filePath, operation)
        
        // Setup VSCode file watcher
        await this.setupFileWatcher(filePath)
    } catch (error) {
        console.error("Failed to track file operation:", error)
    }
}
```

**RecordSource types**:
- `"read_tool"` - AI used read_file tool
- `"file_mentioned"` - User mentioned file in @file
- `"roo_edited"` - AI edited file (write_to_file, apply_diff)
- `"user_edited"` - User edited file externally

---

### Method 2: setupFileWatcher() - Lines 48-77

**Creates VSCode file system watcher** for each tracked file.

```typescript
async setupFileWatcher(filePath: string) {
    // Prevent duplicate watchers
    if (this.fileWatchers.has(filePath)) {
        return
    }
    
    const cwd = this.getCwd()
    if (!cwd) return
    
    // Create watcher for specific file
    const fileUri = vscode.Uri.file(path.resolve(cwd, filePath))
    const watcher = vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(
            path.dirname(fileUri.fsPath),
            path.basename(fileUri.fsPath)
        )
    )
    
    // Track changes
    watcher.onDidChange(() => {
        if (this.recentlyEditedByRoo.has(filePath)) {
            // AI just edited this file, ignore the change event
            this.recentlyEditedByRoo.delete(filePath)
        } else {
            // User edited this file externally!
            this.recentlyModifiedFiles.add(filePath)
            this.trackFileContext(filePath, "user_edited")
        }
    })
    
    this.fileWatchers.set(filePath, watcher)
}
```

**Change detection logic**:
1. AI edits file → `recentlyEditedByRoo` flag set → watcher fires → flag checked → ignore
2. User edits file → watcher fires → no flag → add to `recentlyModifiedFiles` → update metadata

**Why track AI edits?** Prevent false positives (AI's own edits trigger watchers).

---

### Method 3: addFileToFileContextTracker() - Lines 143-200

**Core business logic** - Updates metadata with file operation.

```typescript
async addFileToFileContextTracker(taskId: string, filePath: string, source: RecordSource) {
    try {
        const metadata = await this.getTaskMetadata(taskId)
        const now = Date.now()
        
        // CRITICAL: Mark all existing entries for this file as stale
        metadata.files_in_context.forEach((entry) => {
            if (entry.path === filePath && entry.record_state === "active") {
                entry.record_state = "stale"
            }
        })
        
        // Helper to get latest timestamp for a field
        const getLatestDateForField = (path: string, field: keyof FileMetadataEntry) => {
            const relevantEntries = metadata.files_in_context
                .filter(entry => entry.path === path && entry[field])
                .sort((a, b) => (b[field] as number) - (a[field] as number))
            
            return relevantEntries.length > 0 ? relevantEntries[0][field] : null
        }
        
        // Create new entry (starts as "active")
        let newEntry: FileMetadataEntry = {
            path: filePath,
            record_state: "active",
            record_source: source,
            roo_read_date: getLatestDateForField(filePath, "roo_read_date"),
            roo_edit_date: getLatestDateForField(filePath, "roo_edit_date"),
            user_edit_date: getLatestDateForField(filePath, "user_edit_date"),
        }
        
        // Update timestamps based on operation type
        switch (source) {
            case "user_edited":
                newEntry.user_edit_date = now
                this.recentlyModifiedFiles.add(filePath)
                break
                
            case "roo_edited":
                newEntry.roo_read_date = now
                newEntry.roo_edit_date = now
                this.checkpointPossibleFiles.add(filePath)
                this.markFileAsEditedByRoo(filePath)
                break
                
            case "read_tool":
            case "file_mentioned":
                newEntry.roo_read_date = now
                break
        }
        
        // Append new entry (keeps history)
        metadata.files_in_context.push(newEntry)
        await this.saveTaskMetadata(taskId, metadata)
    } catch (error) {
        console.error("Failed to add file to metadata:", error)
    }
}
```

**Key insight**: Each file operation creates a NEW entry rather than updating existing entry. This maintains a timeline of file interactions.

**Example timeline**:
```json
{
  "files_in_context": [
    {
      "path": "src/main.rs",
      "record_state": "stale",          // Marked stale when next entry added
      "record_source": "read_tool",
      "roo_read_date": 1696234567890,
      "roo_edit_date": null,
      "user_edit_date": null
    },
    {
      "path": "src/main.rs",
      "record_state": "stale",          // Marked stale when user edited
      "record_source": "roo_edited",
      "roo_read_date": 1696234570000,
      "roo_edit_date": 1696234570000,
      "user_edit_date": null
    },
    {
      "path": "src/main.rs",
      "record_state": "active",         // Current state
      "record_source": "user_edited",
      "roo_read_date": 1696234570000,  // Inherited from previous
      "roo_edit_date": 1696234570000,  // Inherited
      "user_edit_date": 1696234580000  // New timestamp
    }
  ]
}
```

**Timestamp inheritance**: `getLatestDateForField()` preserves latest timestamp across entries.

---

### Method 4: getAndClearRecentlyModifiedFiles() - Lines 203-207

**Consumed by Task** before applying diffs.

```typescript
getAndClearRecentlyModifiedFiles(): string[] {
    const files = Array.from(this.recentlyModifiedFiles)
    this.recentlyModifiedFiles.clear()
    return files
}
```

**Usage in Task.ts**:
```typescript
async presentAssistantMessage() {
    // Check for stale files before applying diff
    const staleFiles = this.fileContextTracker.getAndClearRecentlyModifiedFiles()
    
    if (staleFiles.length > 0 && currentBlock.type === "tool_use" && currentBlock.name === "apply_diff") {
        await this.say("text", `File ${staleFiles[0]} was modified externally. Re-reading...`)
        await this.executeTool({ name: "read_file", input: { path: staleFiles[0] } })
        // Now context is fresh, can apply diff
    }
}
```

**Prevents**: `diff hunk failed` errors due to line number mismatches.

---

### Method 5: getAndClearCheckpointPossibleFile() - Lines 209-213

**Signals checkpoint system** when files are edited.

```typescript
getAndClearCheckpointPossibleFile(): string[] {
    const files = Array.from(this.checkpointPossibleFiles)
    this.checkpointPossibleFiles.clear()
    return files
}
```

**Usage**: Task calls this after successful tool execution to auto-checkpoint edited files.

---

## 🔧 FILE 2: apiMessages.ts (84 lines)

### Purpose: Persist API Conversation History

**Storage location**: `~/.config/Code/User/globalStorage/roo-coder.roo-cline/tasks/{taskId}/api_conversation_history.json`

### Type Definition

```typescript
export type ApiMessage = Anthropic.MessageParam & { 
    ts?: number         // Timestamp of when message was sent
    isSummary?: boolean // True if this message is a condensed summary
}

// Anthropic.MessageParam = { role: "user" | "assistant", content: [...] }
```

**Extended fields**:
- `ts`: Used for debugging/analysis
- `isSummary`: Marks condensed messages (from conversation summarization)

---

### Function 1: readApiMessages() - Lines 14-69

**Loads conversation history** from disk.

```typescript
export async function readApiMessages({
    taskId,
    globalStoragePath,
}: {
    taskId: string
    globalStoragePath: string
}): Promise<ApiMessage[]> {
    const taskDir = await getTaskDirectoryPath(globalStoragePath, taskId)
    const filePath = path.join(taskDir, GlobalFileNames.apiConversationHistory)
    
    // Try new filename first
    if (await fileExistsAtPath(filePath)) {
        const fileContent = await fs.readFile(filePath, "utf8")
        try {
            const parsedData = JSON.parse(fileContent)
            if (Array.isArray(parsedData) && parsedData.length === 0) {
                console.error(
                    `[Roo-Debug] Found API conversation history file, but it's empty. TaskId: ${taskId}`
                )
            }
            return parsedData
        } catch (error) {
            console.error(
                `[Roo-Debug] Error parsing API conversation history. TaskId: ${taskId}, Error: ${error}`
            )
            throw error
        }
    }
    
    // Fallback: Try old filename (migration path)
    const oldPath = path.join(taskDir, "claude_messages.json")
    if (await fileExistsAtPath(oldPath)) {
        const fileContent = await fs.readFile(oldPath, "utf8")
        try {
            const parsedData = JSON.parse(fileContent)
            if (Array.isArray(parsedData) && parsedData.length === 0) {
                console.error(
                    `[Roo-Debug] Found OLD API conversation history file (claude_messages.json), but it's empty.`
                )
            }
            await fs.unlink(oldPath)  // Delete old file after successful read
            return parsedData
        } catch (error) {
            console.error(
                `[Roo-Debug] Error parsing OLD API conversation history. Error: ${error}`
            )
            throw error  // DO NOT delete old file if parsing failed
        }
    }
    
    // No history file found
    console.error(
        `[Roo-Debug] API conversation history file not found for taskId: ${taskId}`
    )
    return []
}
```

**Migration handling**: Transparently upgrades from `claude_messages.json` → `api_conversation_history.json`.

**Error handling**: Extensive logging for debugging empty/corrupt history files.

---

### Function 2: saveApiMessages() - Lines 71-83

**Writes conversation history** to disk.

```typescript
export async function saveApiMessages({
    messages,
    taskId,
    globalStoragePath,
}: {
    messages: ApiMessage[]
    taskId: string
    globalStoragePath: string
}) {
    const taskDir = await getTaskDirectoryPath(globalStoragePath, taskId)
    const filePath = path.join(taskDir, GlobalFileNames.apiConversationHistory)
    await safeWriteJson(filePath, messages)
}
```

**safeWriteJson**: Atomic write (temp file + rename) to prevent corruption.

**Called after**:
- Each API request completes
- Conversation is condensed (summarized)
- User sends new message

---

## 🔧 FILE 3: Task.ts - Conversation Management (2,859 lines)

### Key Properties - Lines 248-249

```typescript
export class Task extends EventEmitter<TaskEvents> implements TaskLike {
    // Conversation tracking
    apiConversationHistory: ApiMessage[] = []      // API messages (for LLM)
    clineMessages: ClineMessage[] = []            // UI messages (for webview)
    
    fileContextTracker: FileContextTracker
}
```

**Two parallel histories**:
1. `apiConversationHistory`: Raw API messages sent to/from LLM
2. `clineMessages`: Formatted messages displayed in UI

**Why separate?**
- API history: Includes full tool calls, can be condensed
- UI history: Includes user-friendly tool summaries, never condensed

---

### Conversation Loading - Task.init()

```typescript
async init(historyItem?: HistoryItem) {
    if (historyItem) {
        // Load from disk
        this.apiConversationHistory = await readApiMessages({
            taskId: this.taskId,
            globalStoragePath: this.globalStoragePath,
        })
        
        this.clineMessages = await readTaskMessages({
            taskId: this.taskId,
            globalStoragePath: this.globalStoragePath,
        })
    } else {
        // New task, start fresh
        this.apiConversationHistory = []
        this.clineMessages = []
    }
}
```

---

### Conversation Saving

**Called after each API response**:

```typescript
async saveClineMessages() {
    await saveTaskMessages({
        messages: this.clineMessages,
        taskId: this.taskId,
        globalStoragePath: this.globalStoragePath,
    })
}

async saveApiConversationHistory() {
    await saveApiMessages({
        messages: this.apiConversationHistory,
        taskId: this.taskId,
        globalStoragePath: this.globalStoragePath,
    })
}
```

---

### Context Window Management

**When context window exceeded**:

```typescript
async handleContextWindowError(error: unknown) {
    if (checkContextWindowExceededError(error)) {
        // Get messages since last summary
        const messagesToCondense = getMessagesSinceLastSummary(this.apiConversationHistory)
        
        if (messagesToCondense.length > 0) {
            // Summarize with LLM
            const summary = await summarizeConversation(messagesToCondense, this.api)
            
            // Replace old messages with summary
            const summaryMessage: ApiMessage = {
                role: "user",
                content: `[Previous conversation summary]\n${summary}`,
                ts: Date.now(),
                isSummary: true
            }
            
            // Remove old messages, keep summary
            const beforeSummary = this.apiConversationHistory.slice(0, lastSummaryIndex)
            const afterSummary = this.apiConversationHistory.slice(messagesToCondense.length)
            
            this.apiConversationHistory = [
                ...beforeSummary,
                summaryMessage,
                ...afterSummary
            ]
            
            await this.saveApiConversationHistory()
            
            // Retry API call with condensed history
            return await this.recursivelyMakeApiRequest(userContent)
        }
    }
}
```

**FORCED_CONTEXT_REDUCTION_PERCENT = 75%**: Keep most recent 75% of messages if summarization fails.

---

## 🎯 STALE CONTEXT DETECTION FLOW

```
Timeline of Events:

1. User mentions file:
   → fileContextTracker.trackFileContext("src/main.rs", "file_mentioned")
   → Metadata entry created: { state: "active", roo_read_date: T1 }
   → VSCode watcher created

2. AI reads file:
   → Tool execution: read_file
   → fileContextTracker.trackFileContext("src/main.rs", "read_tool")
   → Previous entry marked "stale", new entry: { state: "active", roo_read_date: T2 }

3. User edits file in VSCode:
   → VSCode watcher fires onDidChange
   → recentlyEditedByRoo not set → User edit detected!
   → fileContextTracker.trackFileContext("src/main.rs", "user_edited")
   → Previous entry marked "stale", new entry: { state: "active", user_edit_date: T3 }

4. AI attempts apply_diff:
   → Task checks: staleFiles = fileContextTracker.getAndClearRecentlyModifiedFiles()
   → staleFiles = ["src/main.rs"]
   → Task says: "File was modified externally. Re-reading..."
   → Task executes: read_file("src/main.rs")
   → Context now fresh, diff can proceed

5. AI edits file:
   → Tool execution: apply_diff
   → fileContextTracker.markFileAsEditedByRoo("src/main.rs")
   → VSCode watcher fires (due to file write)
   → recentlyEditedByRoo IS set → Ignore change event
   → fileContextTracker.trackFileContext("src/main.rs", "roo_edited")
   → checkpointPossibleFiles.add("src/main.rs")
```

---

## 🎯 RUST TRANSLATION PATTERNS

```rust
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use notify::{Watcher, RecursiveMode, watcher};

#[derive(Debug, Clone)]
pub enum RecordSource {
    ReadTool,
    UserEdited,
    RooEdited,
    FileMentioned,
}

#[derive(Debug, Clone)]
pub struct FileMetadataEntry {
    pub path: String,
    pub record_state: RecordState,  // Active or Stale
    pub record_source: RecordSource,
    pub roo_read_date: Option<i64>,
    pub roo_edit_date: Option<i64>,
    pub user_edit_date: Option<i64>,
}

pub struct FileContextTracker {
    task_id: String,
    metadata: Arc<RwLock<TaskMetadata>>,
    recently_modified: Arc<RwLock<HashSet<String>>>,
    recently_edited_by_roo: Arc<RwLock<HashSet<String>>>,
    checkpoint_possible: Arc<RwLock<HashSet<String>>>,
    file_watchers: HashMap<String, notify::RecommendedWatcher>,
}

impl FileContextTracker {
    pub async fn track_file_context(&mut self, file_path: &str, operation: RecordSource) -> Result<(), Error> {
        // Add to metadata
        self.add_file_to_tracker(file_path, operation).await?;
        
        // Setup watcher if not exists
        self.setup_file_watcher(file_path).await?;
        
        Ok(())
    }
    
    async fn setup_file_watcher(&mut self, file_path: &str) -> Result<(), Error> {
        if self.file_watchers.contains_key(file_path) {
            return Ok(());
        }
        
        let (tx, mut rx) = mpsc::channel(100);
        let mut watcher = watcher(tx, std::time::Duration::from_secs(1))?;
        watcher.watch(file_path, RecursiveMode::NonRecursive)?;
        
        let recently_edited = Arc::clone(&self.recently_edited_by_roo);
        let recently_modified = Arc::clone(&self.recently_modified);
        let path = file_path.to_string();
        
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Ok(event) = event {
                    if event.kind.is_modify() {
                        let was_roo_edit = recently_edited.write().unwrap().remove(&path);
                        if !was_roo_edit {
                            recently_modified.write().unwrap().insert(path.clone());
                        }
                    }
                }
            }
        });
        
        self.file_watchers.insert(file_path.to_string(), watcher);
        Ok(())
    }
    
    async fn add_file_to_tracker(&self, file_path: &str, source: RecordSource) -> Result<(), Error> {
        let mut metadata = self.metadata.write().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        
        // Mark existing entries as stale
        for entry in &mut metadata.files_in_context {
            if entry.path == file_path && entry.record_state == RecordState::Active {
                entry.record_state = RecordState::Stale;
            }
        }
        
        // Get latest timestamps
        let latest_read = metadata.files_in_context.iter()
            .filter(|e| e.path == file_path)
            .filter_map(|e| e.roo_read_date)
            .max();
        
        let mut new_entry = FileMetadataEntry {
            path: file_path.to_string(),
            record_state: RecordState::Active,
            record_source: source.clone(),
            roo_read_date: latest_read,
            roo_edit_date: None,
            user_edit_date: None,
        };
        
        match source {
            RecordSource::UserEdited => {
                new_entry.user_edit_date = Some(now);
                self.recently_modified.write().unwrap().insert(file_path.to_string());
            }
            RecordSource::RooEdited => {
                new_entry.roo_read_date = Some(now);
                new_entry.roo_edit_date = Some(now);
                self.checkpoint_possible.write().unwrap().insert(file_path.to_string());
                self.recently_edited_by_roo.write().unwrap().insert(file_path.to_string());
            }
            _ => {
                new_entry.roo_read_date = Some(now);
            }
        }
        
        metadata.files_in_context.push(new_entry);
        Ok(())
    }
    
    pub fn get_and_clear_recently_modified(&self) -> Vec<String> {
        let mut modified = self.recently_modified.write().unwrap();
        let files: Vec<String> = modified.iter().cloned().collect();
        modified.clear();
        files
    }
}

// API Message Persistence
pub async fn save_api_messages(
    messages: &[ApiMessage],
    task_id: &str,
    storage_path: &Path,
) -> Result<(), Error> {
    let task_dir = storage_path.join("tasks").join(task_id);
    tokio::fs::create_dir_all(&task_dir).await?;
    
    let file_path = task_dir.join("api_conversation_history.json");
    let json = serde_json::to_string_pretty(messages)?;
    
    // Atomic write
    let temp_path = file_path.with_extension("tmp");
    tokio::fs::write(&temp_path, json).await?;
    tokio::fs::rename(temp_path, file_path).await?;
    
    Ok(())
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] FileContextTracker architecture explained
- [x] Stale detection logic traced
- [x] VSCode watcher integration shown
- [x] Metadata timeline structure documented
- [x] API message persistence detailed
- [x] Task conversation management covered
- [x] Context window handling explained
- [x] Rust translation patterns provided

**STATUS**: CHUNK-10 COMPLETE (4,000+ words, deep technical analysis)
