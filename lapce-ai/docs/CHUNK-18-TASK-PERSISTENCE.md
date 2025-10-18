# CHUNK-18: CORE/TASK-PERSISTENCE - CONVERSATION STATE MANAGEMENT

## 📁 MODULE STRUCTURE

```
Codex/src/core/task-persistence/
├── index.ts              (4 lines)   - Public exports
├── apiMessages.ts        (84 lines)  - API conversation history
├── taskMessages.ts       (43 lines)  - UI messages storage
└── taskMetadata.ts       (102 lines) - Task statistics & metadata
```

**Total**: 233 lines analyzed

---

## 🎯 PURPOSE

Persist AI conversation state across sessions, enabling:
1. **Resume conversations** after restart
2. **View conversation history** in UI
3. **Calculate task statistics** (tokens, cost, size)
4. **Migrate legacy formats** (claude_messages.json → api_conversation_history.json)

**Critical for**: Multi-session tasks, cost tracking, history browser

---

## 🗂️ FILE 1: apiMessages.ts (84 lines)

### Purpose
Store conversation messages in API format (Anthropic.MessageParam) for resuming AI conversations.

### Data Structure

```typescript
export type ApiMessage = Anthropic.MessageParam & { 
    ts?: number           // Timestamp (milliseconds)
    isSummary?: boolean   // Flag for condensed messages
}

// Anthropic.MessageParam:
{
    role: "user" | "assistant",
    content: string | Array<ContentBlock>
}

// ContentBlock examples:
{ type: "text", text: "Hello" }
{ type: "image", source: { type: "base64", media_type: "image/png", data: "..." } }
```

### Read Operation

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
    
    // Try new format first
    if (await fileExistsAtPath(filePath)) {
        const fileContent = await fs.readFile(filePath, "utf8")
        try {
            const parsedData = JSON.parse(fileContent)
            if (Array.isArray(parsedData) && parsedData.length === 0) {
                console.error(`[Roo-Debug] API conversation history empty. TaskId: ${taskId}`)
            }
            return parsedData
        } catch (error) {
            console.error(`[Roo-Debug] Error parsing API conversation history: ${error}`)
            throw error
        }
    } 
    
    // Fallback: Try legacy format (claude_messages.json)
    const oldPath = path.join(taskDir, "claude_messages.json")
    if (await fileExistsAtPath(oldPath)) {
        const fileContent = await fs.readFile(oldPath, "utf8")
        try {
            const parsedData = JSON.parse(fileContent)
            if (Array.isArray(parsedData) && parsedData.length === 0) {
                console.error(`[Roo-Debug] OLD API history (claude_messages.json) empty`)
            }
            await fs.unlink(oldPath)  // Delete after successful migration
            return parsedData
        } catch (error) {
            console.error(`[Roo-Debug] Error parsing OLD API history: ${error}`)
            throw error  // DO NOT delete on error
        }
    }
    
    // Not found
    console.error(`[Roo-Debug] API conversation history not found for taskId: ${taskId}`)
    return []
}
```

**File Path Structure**:
```
{globalStoragePath}/
└── tasks/
    └── {taskId}/
        ├── api_conversation_history.json  ← NEW FORMAT
        └── claude_messages.json           ← LEGACY (auto-deleted after migration)
```

**Migration Strategy**:
1. Try reading new format
2. If not found, try legacy format
3. If legacy exists, read it and delete file
4. If both missing, return empty array

### Save Operation

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

**safeWriteJson**: Atomic write with temp file + rename (prevents corruption)

---

## 🗂️ FILE 2: taskMessages.ts (43 lines)

### Purpose
Store UI-friendly messages for displaying conversation history in the webview.

### Data Structure

```typescript
export type ClineMessage = {
    type: "say" | "ask" | "error" | "api_req_started" | ...
    ts: number
    text?: string
    images?: string[]
    partial?: boolean
    ask?: "followup" | "command" | "completion_result" | ...
    // ... many more fields for different message types
}
```

**Difference from ApiMessage**:
- **ApiMessage**: Minimal format for AI API (role + content)
- **ClineMessage**: Rich format for UI (type, ask state, tool results, errors, etc.)

### Read Operation

```typescript
export async function readTaskMessages({
    taskId,
    globalStoragePath,
}: ReadTaskMessagesOptions): Promise<ClineMessage[]> {
    const taskDir = await getTaskDirectoryPath(globalStoragePath, taskId)
    const filePath = path.join(taskDir, GlobalFileNames.uiMessages)
    const fileExists = await fileExistsAtPath(filePath)
    
    if (fileExists) {
        return JSON.parse(await fs.readFile(filePath, "utf8"))
    }
    
    return []
}
```

**File Path**:
```
{globalStoragePath}/tasks/{taskId}/ui_messages.json
```

**No migration logic**: This is a newer format with no legacy equivalent

### Save Operation

```typescript
export async function saveTaskMessages({
    messages,
    taskId,
    globalStoragePath,
}: SaveTaskMessagesOptions) {
    const taskDir = await getTaskDirectoryPath(globalStoragePath, taskId)
    const filePath = path.join(taskDir, GlobalFileNames.uiMessages)
    await safeWriteJson(filePath, messages)
}
```

---

## 🗂️ FILE 3: taskMetadata.ts (102 lines)

### Purpose
Calculate task statistics for history browser UI: tokens used, cost, task size, timestamp.

### Dependencies

```typescript
import NodeCache from "node-cache"              // In-memory cache
import getFolderSize from "get-folder-size"     // Disk usage calculation

import { combineApiRequests } from "../../shared/combineApiRequests"
import { combineCommandSequences } from "../../shared/combineCommandSequences"
import { getApiMetrics } from "../../shared/getApiMetrics"
```

### Cache Strategy

```typescript
const taskSizeCache = new NodeCache({ 
    stdTTL: 30,           // Cache for 30 seconds
    checkperiod: 5 * 60   // Check for expired keys every 5 minutes
})
```

**Why cache?**: `getFolderSize` is expensive (scans directory tree)

### Main Function

```typescript
export async function taskMetadata({
    messages,
    taskId,
    taskNumber,
    globalStoragePath,
    workspace,
    mode,
}: TaskMetadataOptions): Promise<{
    historyItem: HistoryItem
    tokenUsage: ApiMetrics
}> {
    const taskDir = await getTaskDirectoryPath(globalStoragePath, taskId)
    const hasMessages = messages && messages.length > 0
    
    // Initialize variables
    let timestamp: number
    let tokenUsage: ReturnType<typeof getApiMetrics>
    let taskDirSize: number
    let taskMessage: ClineMessage | undefined
    
    if (!hasMessages) {
        // Empty task
        timestamp = Date.now()
        tokenUsage = {
            totalTokensIn: 0,
            totalTokensOut: 0,
            totalCacheWrites: 0,
            totalCacheReads: 0,
            totalCost: 0,
            contextTokens: 0,
        }
        taskDirSize = 0
    } else {
        // Task with messages
        taskMessage = messages[0]  // First message = task description
        
        // Find last relevant message (ignore resume actions)
        const lastRelevantMessage = 
            messages[findLastIndex(messages, 
                (m) => !(m.ask === "resume_task" || m.ask === "resume_completed_task")
            )] || taskMessage
        
        timestamp = lastRelevantMessage.ts
        
        // Calculate token usage (skip first message which is just task description)
        tokenUsage = getApiMetrics(
            combineApiRequests(
                combineCommandSequences(messages.slice(1))
            )
        )
        
        // Get task directory size (with caching)
        const cachedSize = taskSizeCache.get<number>(taskDir)
        
        if (cachedSize === undefined) {
            try {
                taskDirSize = await getFolderSize.loose(taskDir)
                taskSizeCache.set<number>(taskDir, taskDirSize)
            } catch (error) {
                taskDirSize = 0
            }
        } else {
            taskDirSize = cachedSize
        }
    }
    
    // Build history item
    const historyItem: HistoryItem = {
        id: taskId,
        number: taskNumber,
        ts: timestamp,
        task: hasMessages
            ? taskMessage!.text?.trim() || t("common:tasks.incomplete", { taskNumber })
            : t("common:tasks.no_messages", { taskNumber }),
        tokensIn: tokenUsage.totalTokensIn,
        tokensOut: tokenUsage.totalTokensOut,
        cacheWrites: tokenUsage.totalCacheWrites,
        cacheReads: tokenUsage.totalCacheReads,
        totalCost: tokenUsage.totalCost,
        size: taskDirSize,
        workspace,
        mode,
    }
    
    return { historyItem, tokenUsage }
}
```

### Key Calculations

**1. Timestamp**: From last non-resume message
```typescript
// Skip "resume_task" and "resume_completed_task" asks
const lastRelevantMessage = messages[findLastIndex(messages, 
    (m) => !(m.ask === "resume_task" || m.ask === "resume_completed_task")
)]
```

**2. Token Usage**: Combine all API requests and calculate totals
```typescript
// Process messages:
messages.slice(1)                    // Skip first (task description)
→ combineCommandSequences()          // Merge related commands
→ combineApiRequests()               // Merge API calls
→ getApiMetrics()                    // Sum tokens & calculate cost
```

**3. Task Size**: Directory size in bytes (cached for 30s)
```typescript
taskDirSize = await getFolderSize.loose(taskDir)
taskSizeCache.set<number>(taskDir, taskDirSize)
```

**Output Type**:
```typescript
type HistoryItem = {
    id: string              // Task ID (UUID)
    number: number          // Task number (sequential)
    ts: number              // Timestamp (ms)
    task: string            // Task description
    tokensIn: number        // Input tokens
    tokensOut: number       // Output tokens
    cacheWrites: number     // Prompt cache writes
    cacheReads: number      // Prompt cache reads
    totalCost: number       // Cost in USD
    size: number            // Disk usage in bytes
    workspace: string       // Workspace path
    mode?: string           // AI mode (code/chat/research)
}
```

---

## 🔄 INTEGRATION POINTS

### 1. Task Creation

```typescript
// When new task starts
const taskId = generateUUID()
await saveTaskMessages({
    messages: [initialTaskMessage],
    taskId,
    globalStoragePath,
})
await saveApiMessages({
    messages: [initialApiMessage],
    taskId,
    globalStoragePath,
})
```

### 2. Task Execution Loop

```typescript
// After each AI turn
conversationHistory.push(newMessage)

await saveApiMessages({
    messages: conversationHistory,
    taskId,
    globalStoragePath,
})

await saveTaskMessages({
    messages: uiMessages,
    taskId,
    globalStoragePath,
})
```

### 3. Task Resume

```typescript
// On "Resume Task" command
const apiMessages = await readApiMessages({ taskId, globalStoragePath })
const uiMessages = await readTaskMessages({ taskId, globalStoragePath })

// Restore conversation state
conversationHistory = apiMessages
this.webview.postMessage({ type: "restoreMessages", messages: uiMessages })
```

### 4. History Browser

```typescript
// Load all tasks for history UI
const taskIds = await getAllTaskIds(globalStoragePath)

const historyItems = await Promise.all(
    taskIds.map(async (taskId, index) => {
        const messages = await readTaskMessages({ taskId, globalStoragePath })
        const { historyItem } = await taskMetadata({
            messages,
            taskId,
            taskNumber: index + 1,
            globalStoragePath,
            workspace: cwd,
        })
        return historyItem
    })
)

// Display in UI sorted by timestamp
historyItems.sort((a, b) => b.ts - a.ts)
```

---

## 🦀 RUST TRANSLATION

### Data Structures

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,  // "user" | "assistant"
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_summary: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<String>,
    // ... other fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub number: usize,
    pub ts: u64,
    pub task: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cache_writes: u64,
    pub cache_reads: u64,
    pub total_cost: f64,
    pub size: u64,
    pub workspace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}
```

### API Messages Operations

```rust
use tokio::fs;
use std::path::Path;
use anyhow::{Context, Result};

pub async fn read_api_messages(
    task_id: &str,
    global_storage_path: &Path,
) -> Result<Vec<ApiMessage>> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join("api_conversation_history.json");
    
    // Try new format
    if file_path.exists() {
        let content = fs::read_to_string(&file_path).await
            .context("Failed to read API conversation history")?;
        
        let messages: Vec<ApiMessage> = serde_json::from_str(&content)
            .context("Failed to parse API conversation history")?;
        
        if messages.is_empty() {
            log::error!("[Roo-Debug] API conversation history empty. TaskId: {}", task_id);
        }
        
        return Ok(messages);
    }
    
    // Try legacy format
    let old_path = task_dir.join("claude_messages.json");
    if old_path.exists() {
        let content = fs::read_to_string(&old_path).await
            .context("Failed to read OLD API conversation history")?;
        
        let messages: Vec<ApiMessage> = serde_json::from_str(&content)
            .context("Failed to parse OLD API conversation history")?;
        
        if messages.is_empty() {
            log::error!("[Roo-Debug] OLD API history empty. TaskId: {}", task_id);
        }
        
        // Delete legacy file after successful read
        fs::remove_file(&old_path).await
            .context("Failed to delete legacy claude_messages.json")?;
        
        return Ok(messages);
    }
    
    // Not found
    log::error!("[Roo-Debug] API conversation history not found for taskId: {}", task_id);
    Ok(Vec::new())
}

pub async fn save_api_messages(
    messages: &[ApiMessage],
    task_id: &str,
    global_storage_path: &Path,
) -> Result<()> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join("api_conversation_history.json");
    
    safe_write_json(&file_path, messages).await
}
```

### Task Messages Operations

```rust
pub async fn read_task_messages(
    task_id: &str,
    global_storage_path: &Path,
) -> Result<Vec<ClineMessage>> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join("ui_messages.json");
    
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&file_path).await?;
    let messages = serde_json::from_str(&content)?;
    
    Ok(messages)
}

pub async fn save_task_messages(
    messages: &[ClineMessage],
    task_id: &str,
    global_storage_path: &Path,
) -> Result<()> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join("ui_messages.json");
    
    safe_write_json(&file_path, messages).await
}
```

### Task Metadata with Caching

```rust
use moka::future::Cache;
use std::time::Duration;

lazy_static! {
    static ref TASK_SIZE_CACHE: Cache<PathBuf, u64> = Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build();
}

pub async fn task_metadata(
    messages: &[ClineMessage],
    task_id: &str,
    task_number: usize,
    global_storage_path: &Path,
    workspace: &str,
    mode: Option<String>,
) -> Result<(HistoryItem, TokenUsage)> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    
    let (timestamp, token_usage, task_dir_size) = if messages.is_empty() {
        // Empty task
        (
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
            TokenUsage::default(),
            0,
        )
    } else {
        // Task with messages
        let task_message = &messages[0];
        
        // Find last relevant message
        let last_relevant = messages.iter().rposition(|m| {
            m.ask.as_ref().map_or(true, |ask| {
                ask != "resume_task" && ask != "resume_completed_task"
            })
        }).unwrap_or(0);
        
        let timestamp = messages[last_relevant].ts;
        
        // Calculate token usage (skip first message)
        let token_usage = get_api_metrics(
            &combine_api_requests(
                &combine_command_sequences(&messages[1..])
            )
        );
        
        // Get task directory size (with caching)
        let task_dir_size = match TASK_SIZE_CACHE.get(&task_dir).await {
            Some(cached) => cached,
            None => {
                let size = calculate_folder_size(&task_dir).await.unwrap_or(0);
                TASK_SIZE_CACHE.insert(task_dir.clone(), size).await;
                size
            }
        };
        
        (timestamp, token_usage, task_dir_size)
    };
    
    let task_text = if messages.is_empty() {
        format!("Task {} (No messages)", task_number)
    } else {
        messages[0].text.as_ref()
            .map(|t| t.trim().to_string())
            .unwrap_or_else(|| format!("Task {} (Incomplete)", task_number))
    };
    
    let history_item = HistoryItem {
        id: task_id.to_string(),
        number: task_number,
        ts: timestamp,
        task: task_text,
        tokens_in: token_usage.total_tokens_in,
        tokens_out: token_usage.total_tokens_out,
        cache_writes: token_usage.total_cache_writes,
        cache_reads: token_usage.total_cache_reads,
        total_cost: token_usage.total_cost,
        size: task_dir_size,
        workspace: workspace.to_string(),
        mode,
    };
    
    Ok((history_item, token_usage))
}

async fn calculate_folder_size(path: &Path) -> Result<u64> {
    use tokio::fs;
    use futures::stream::{self, StreamExt};
    
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    
    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            
            if metadata.is_file() {
                total += metadata.len();
            } else if metadata.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    
    Ok(total)
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Two Message Formats

**Why separate ApiMessage and ClineMessage?**

**ApiMessage** (84 bytes overhead):
```json
{
  "role": "user",
  "content": "Hello"
}
```

**ClineMessage** (~500 bytes overhead):
```json
{
  "type": "say",
  "ts": 1234567890,
  "text": "Hello",
  "partial": false,
  "ask": null,
  "tool": null,
  "api_req_started": null,
  ...
}
```

**Reasoning**:
- **ApiMessage**: Minimal for AI API resumption
- **ClineMessage**: Rich for UI rendering (colors, icons, state)
- Separation reduces API payload size

### 2. Legacy Migration Strategy

**Auto-delete on success**:
```typescript
if (await fileExistsAtPath(oldPath)) {
    const data = JSON.parse(await fs.readFile(oldPath, "utf8"))
    await fs.unlink(oldPath)  // Delete legacy file
    return data
}
```

**Why delete immediately?**
- Prevents confusion (which file is current?)
- Saves disk space
- One-time migration per task

**Safe deletion**: Only delete if parse succeeds

### 3. Caching Strategy

**30-second TTL for folder sizes**:
```typescript
const taskSizeCache = new NodeCache({ stdTTL: 30 })
```

**Why cache?**
- `getFolderSize` scans entire directory tree (slow)
- Size doesn't change frequently
- History browser loads many tasks at once

**Trade-off**: Slightly stale data vs 10-100x performance improvement

### 4. Empty Array on Missing Files

```typescript
if (!fileExists) {
    return []  // Not throw error
}
```

**Why return empty instead of error?**
- New tasks have no history yet
- Missing file is normal state, not error
- Caller can handle empty gracefully

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `node-cache` (^5.1.2) - In-memory caching
- `get-folder-size` (^3.0.0) - Directory size calculation
- `@anthropic-ai/sdk` - Type definitions

**Internal Modules**:
- `../../utils/safeWriteJson` - Atomic file writes
- `../../utils/fs` - File existence checks
- `../../utils/storage` - Task directory paths
- `../../shared/combineApiRequests` - API request aggregation
- `../../shared/combineCommandSequences` - Command merging
- `../../shared/getApiMetrics` - Token/cost calculation
- `../../i18n` - Internationalization

**Rust Crates**:
- `serde` (1.0) - Serialization
- `serde_json` (1.0) - JSON parsing
- `tokio` (1.35) - Async runtime
- `moka` (0.12) - Async cache
- `lazy_static` (1.4) - Static cache initialization
- `anyhow` (1.0) - Error handling

---

## 📊 PERFORMANCE CHARACTERISTICS

### Read Operations
- **API Messages**: O(1) file read + O(n) JSON parse
- **Task Messages**: O(1) file read + O(n) JSON parse
- **Typical size**: 10-100 KB per task

### Write Operations
- **safeWriteJson**: Write temp file + atomic rename
- **Cost**: 2× disk I/O vs direct write
- **Benefit**: No corruption on crash

### Metadata Calculation
- **Token aggregation**: O(n) where n = messages
- **Folder size**: O(f) where f = files in task dir
- **With cache**: O(1) for repeated calls within 30s

### Memory Usage
- **Messages in memory**: ~1-10 MB per active task
- **Cache overhead**: ~100 bytes per cached task size
- **Total**: Negligible unless 1000+ concurrent tasks

---

## 🧪 TESTING CONSIDERATIONS

### Unit Tests

```typescript
describe("readApiMessages", () => {
    it("should read new format", async () => {
        const messages = await readApiMessages({ taskId, globalStoragePath })
        expect(messages).toBeInstanceOf(Array)
    })
    
    it("should migrate from legacy format", async () => {
        // Create claude_messages.json
        await fs.writeFile(oldPath, JSON.stringify(legacyMessages))
        
        const messages = await readApiMessages({ taskId, globalStoragePath })
        
        expect(messages).toEqual(legacyMessages)
        expect(await fileExistsAtPath(oldPath)).toBe(false)  // Deleted
    })
    
    it("should return empty array for missing file", async () => {
        const messages = await readApiMessages({ taskId: "nonexistent", globalStoragePath })
        expect(messages).toEqual([])
    })
    
    it("should not delete legacy file on parse error", async () => {
        await fs.writeFile(oldPath, "invalid json")
        
        await expect(readApiMessages({ taskId, globalStoragePath }))
            .rejects.toThrow()
        
        expect(await fileExistsAtPath(oldPath)).toBe(true)  // Still exists
    })
})

describe("taskMetadata", () => {
    it("should calculate correct token usage", async () => {
        const { tokenUsage } = await taskMetadata({
            messages: messagesWithApiCalls,
            taskId, taskNumber: 1, globalStoragePath,
            workspace: "/test",
        })
        
        expect(tokenUsage.totalTokensIn).toBeGreaterThan(0)
        expect(tokenUsage.totalCost).toBeGreaterThan(0)
    })
    
    it("should cache folder size", async () => {
        const call1 = await taskMetadata({ ... })
        const call2 = await taskMetadata({ ... })  // Within 30s
        
        // Second call should be faster (cached)
        expect(call2.duration).toBeLessThan(call1.duration * 0.5)
    })
})
```

---

## 🎓 KEY TAKEAWAYS

✅ **Dual Format**: API messages (minimal) + UI messages (rich)

✅ **Automatic Migration**: Legacy format auto-deleted on success

✅ **Performance**: 30s cache for expensive folder size calculations

✅ **Error Handling**: Empty array on missing (not error), preserve on parse failure

✅ **Atomic Writes**: safeWriteJson prevents corruption

✅ **Metadata Aggregation**: Combines API requests for accurate cost tracking

✅ **Internationalization**: Uses i18n for user-facing task labels

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Medium
**Estimated Effort**: 3-4 hours
**Lines of Rust**: ~300 lines (more verbose than TypeScript)
**Dependencies**: `serde_json`, `tokio`, `moka` (cache)
**Key Challenge**: Async file I/O, cache initialization
**Risk**: Low - straightforward file operations

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-19 (kilocode/)
