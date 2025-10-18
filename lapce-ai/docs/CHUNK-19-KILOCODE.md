# CHUNK-19: CORE/KILOCODE - KILOCODE-SPECIFIC FEATURES

## 📁 MODULE STRUCTURE

```
Codex/src/core/kilocode/
└── webview/
    └── webviewMessageHandlerUtils.ts    (194 lines)
```

**Total**: 194 lines analyzed

---

## 🎯 PURPOSE

Implement Kilocode cloud service specific features:
1. **Message Editing**: Allow users to edit and resend previous messages
2. **Checkpoint Revert**: Restore code to previous checkpoint when editing
3. **Cloud Notifications**: Fetch notifications from Kilocode API
4. **Message Resend Logic**: Delete subsequent messages and continue from edited point

**Integration**: Works with Kilocode cloud backend for user notifications and authentication.

---

## 🔧 CORE FUNCTIONS

### 1. Message Editing with Resend

#### deleteMessagesForResend()

```typescript
const deleteMessagesForResend = async (
    cline: Task, 
    originalMessageIndex: number, 
    originalMessageTs: number
) => {
    // Delete UI messages after the edited message
    const newClineMessages = cline.clineMessages.slice(0, originalMessageIndex)
    await cline.overwriteClineMessages(newClineMessages)
    
    // Delete API messages after the edited message
    const apiHistory = [...cline.apiConversationHistory]
    const timeCutoff = originalMessageTs - 1000  // 1 second before original
    const apiHistoryIndex = apiHistory.findIndex(
        (entry) => entry.ts && entry.ts >= timeCutoff
    )
    
    if (apiHistoryIndex !== -1) {
        const newApiHistory = apiHistory.slice(0, apiHistoryIndex)
        await cline.overwriteApiConversationHistory(newApiHistory)
    }
}
```

**Logic**:
1. Truncate UI messages array at edit point
2. Find API messages with timestamp >= (editTimestamp - 1s)
3. Truncate API messages array before that point
4. Persist both truncated arrays

**Why -1000ms cutoff?**
- UI message timestamp might not exactly match API message timestamp
- 1-second buffer ensures we catch the corresponding API message
- Prevents off-by-one errors in timestamp matching

#### resendMessageSequence()

```typescript
const resendMessageSequence = async (
    provider: ClineProvider,
    taskId: string,
    originalMessageIndex: number,
    originalMessageTimestamp: number,
    editedText: string,
    images?: string[],
): Promise<boolean> => {
    // 1. Get current task instance
    const currentCline = provider.getCurrentTask()
    if (!currentCline || currentCline.taskId !== taskId) {
        provider.log(`[Edit Message] Error: Could not get current cline instance`)
        vscode.window.showErrorMessage(t("kilocode:userFeedback.message_update_failed"))
        return false
    }
    
    // 2. Delete messages from edit point onwards
    await deleteMessagesForResend(currentCline, originalMessageIndex, originalMessageTimestamp)
    await provider.postStateToWebview()
    
    // 3. Re-initialize task with truncated history
    const { historyItem } = await provider.getTaskWithId(taskId)
    if (!historyItem) {
        provider.log(`[Edit Message] Error: Failed to retrieve history item`)
        vscode.window.showErrorMessage(t("kilocode:userFeedback.message_update_failed"))
        return false
    }
    
    const newCline = await provider.createTaskWithHistoryItem(historyItem)
    if (!newCline) {
        provider.log(`[Edit Message] Error: Failed to re-initialize Cline`)
        vscode.window.showErrorMessage(t("kilocode:userFeedback.message_update_failed"))
        return false
    }
    
    // 4. Send edited message with new task instance
    await new Promise((resolve) => setTimeout(resolve, 100))  // Race condition mitigation
    await newCline.handleWebviewAskResponse("messageResponse", editedText, images)
    
    return true
}
```

**Flow**:
```
User edits message #5
↓
Delete messages 5, 6, 7, 8
↓
Re-initialize task with history [0-4]
↓
Send edited message as new #5
↓
AI responds with new #6
```

**Why re-initialize task?**
- Clean slate for execution state
- Prevents stale references to deleted messages
- Ensures checkpoint tracking is consistent

**Race condition mitigation**: 100ms delay before sending edited message
- Allows task re-initialization to complete
- Prevents message sending before task is ready

---

### 2. Checkpoint Revert

#### editMessageHandler() with revert

```typescript
export const editMessageHandler = async (
    provider: ClineProvider, 
    message: WebviewMessage
) => {
    const timestamp = message.values.ts
    const newText = message.values.text
    const revert = message.values.revert || false  // Flag for checkpoint restore
    const images = message.values.images
    
    const currentCline = provider.getCurrentTask()
    if (!currentCline) {
        provider.log("[Edit Message] Error: No active Cline instance found.")
        return
    }
    
    // Find message by timestamp
    const messageIndex = currentCline.clineMessages.findIndex(
        (msg) => msg.ts && msg.ts === timestamp
    )
    
    if (messageIndex === -1) {
        provider.log(`[Edit Message] Error: Message with timestamp ${timestamp} not found.`)
        return
    }
    
    if (revert) {
        // Find most recent checkpoint before this message
        const checkpointMessage = currentCline.clineMessages
            .filter((msg) => msg.say === "checkpoint_saved")
            .filter((msg) => msg.ts && msg.ts <= timestamp)
            .sort((a, b) => (b.ts || 0) - (a.ts || 0))[0]
        
        if (checkpointMessage && checkpointMessage.text) {
            // Cancel current task
            await provider.cancelTask()
            
            // Wait for cancellation to complete
            try {
                await pWaitFor(() => currentCline.isInitialized === true, { 
                    timeout: 3_000 
                })
            } catch (error) {
                vscode.window.showErrorMessage(t("common:errors.checkpoint_timeout"))
            }
            
            // Restore git shadow to checkpoint commit
            try {
                await currentCline.checkpointRestore({
                    commitHash: checkpointMessage.text,  // Commit hash stored in text
                    ts: checkpointMessage.ts,
                    mode: "preview",
                })
            } catch (error) {
                vscode.window.showErrorMessage(t("common:errors.checkpoint_failed"))
            }
            
            // Wait for restore to settle
            await new Promise((resolve) => setTimeout(resolve, 500))
        } else {
            provider.log(`[Edit Message] No checkpoint found before timestamp ${timestamp}`)
            vscode.window.showErrorMessage(t("kilocode:userFeedback.no_checkpoint_found"))
        }
    }
    
    // Update message text in UI
    const updatedMessages = [...currentCline.clineMessages]
    updatedMessages[messageIndex] = {
        ...updatedMessages[messageIndex],
        text: newText,
    }
    await currentCline.overwriteClineMessages(updatedMessages)
    
    // Resend message
    const success = await resendMessageSequence(
        provider,
        currentCline.taskId,
        messageIndex,
        timestamp,
        newText,
        images,
    )
    
    if (success) {
        vscode.window.showInformationMessage(t("kilocode:userFeedback.message_updated"))
    }
}
```

**Checkpoint Revert Flow**:
```
User clicks "Revert & Edit" on message #8
↓
Find most recent checkpoint before #8
↓
Cancel current task execution
↓
Restore git shadow to checkpoint commit
↓
Edit message #8 with new text
↓
Delete messages 8-onwards
↓
Resend edited message
```

**Key Points**:
- **Checkpoint search**: Filters `say === "checkpoint_saved"` messages before edit timestamp
- **Commit hash storage**: Stored in `checkpointMessage.text` field
- **Preview mode**: Restores to checkpoint without committing changes yet
- **Task cancellation**: Required to safely restore git state
- **Timeout handling**: 3-second wait for cancellation, shows error if timeout

---

### 3. Cloud Notifications

#### fetchKilocodeNotificationsHandler()

```typescript
export const fetchKilocodeNotificationsHandler = async (
    provider: ClineProvider
) => {
    try {
        const { apiConfiguration } = await provider.getState()
        const kilocodeToken = apiConfiguration?.kilocodeToken
        
        // Only fetch if using Kilocode provider
        if (!kilocodeToken || apiConfiguration?.apiProvider !== "kilocode") {
            provider.postMessageToWebview({
                type: "kilocodeNotificationsResponse",
                notifications: [],
            })
            return
        }
        
        // Fetch from Kilocode API
        const response = await axios.get(
            `${getKiloBaseUriFromToken(kilocodeToken)}/api/users/notifications`,
            {
                headers: {
                    Authorization: `Bearer ${kilocodeToken}`,
                    "Content-Type": "application/json",
                },
                timeout: 5000,
            }
        )
        
        provider.postMessageToWebview({
            type: "kilocodeNotificationsResponse",
            notifications: response.data?.notifications || [],
        })
    } catch (error: any) {
        provider.log(`Error fetching Kilocode notifications: ${error.message}`)
        provider.postMessageToWebview({
            type: "kilocodeNotificationsResponse",
            notifications: [],
        })
    }
}
```

**API Endpoint**: `{kiloBaseUri}/api/users/notifications`
- **Auth**: Bearer token from `apiConfiguration.kilocodeToken`
- **Timeout**: 5 seconds
- **Error handling**: Silent failure, returns empty array

**Base URI resolution**:
```typescript
// From shared/kilocode/token.ts
getKiloBaseUriFromToken(token: string): string {
    // Extract environment from token claims
    // Returns https://api.kilocode.ai or https://api.dev.kilocode.ai
}
```

**Notification Schema** (inferred):
```typescript
type KilocodeNotification = {
    id: string
    title: string
    message: string
    type: "info" | "warning" | "error" | "success"
    timestamp: number
    read: boolean
    action?: {
        label: string
        url: string
    }
}
```

---

## 🔄 MESSAGE FLOW DIAGRAMS

### Edit Message Flow

```
┌─────────────────────────────────────────────────────────────┐
│ USER: Clicks "Edit" on message #5                          │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. Find message by timestamp                                │
│    messageIndex = clineMessages.findIndex(m => m.ts == ts)  │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Delete subsequent messages                               │
│    UI: clineMessages.slice(0, messageIndex)                 │
│    API: apiHistory.slice(0, apiHistoryIndex)                │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Re-initialize task with truncated history               │
│    newCline = createTaskWithHistoryItem(historyItem)        │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Send edited message                                      │
│    newCline.handleWebviewAskResponse(editedText)            │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. AI processes and generates new response                 │
└─────────────────────────────────────────────────────────────┘
```

### Revert & Edit Flow

```
┌─────────────────────────────────────────────────────────────┐
│ USER: Clicks "Revert & Edit" on message #8                 │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. Find most recent checkpoint before #8                   │
│    checkpoint = clineMessages                               │
│      .filter(m => m.say === "checkpoint_saved")             │
│      .filter(m => m.ts <= timestamp)                        │
│      .sort((a,b) => b.ts - a.ts)[0]                         │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Cancel current task execution                           │
│    provider.cancelTask()                                    │
│    await pWaitFor(() => cline.isInitialized, 3000ms)        │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Restore git shadow to checkpoint                        │
│    cline.checkpointRestore({                                │
│      commitHash: checkpoint.text,                           │
│      mode: "preview"                                        │
│    })                                                       │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Update message text in UI                               │
│    clineMessages[messageIndex].text = editedText            │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Continue with standard edit flow (delete + resend)      │
└─────────────────────────────────────────────────────────────┘
```

---

## 🦀 RUST TRANSLATION

```rust
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KilocodeNotification {
    pub id: String,
    pub title: String,
    pub message: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub timestamp: u64,
    pub read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub url: String,
}

/// Delete messages from edit point onwards
async fn delete_messages_for_resend(
    task: &mut Task,
    original_message_index: usize,
    original_message_ts: u64,
) -> Result<()> {
    // Truncate UI messages
    let new_cline_messages = task.cline_messages[..original_message_index].to_vec();
    task.overwrite_cline_messages(new_cline_messages).await?;
    
    // Truncate API messages
    let time_cutoff = original_message_ts.saturating_sub(1000);
    let api_history_index = task.api_conversation_history
        .iter()
        .position(|entry| entry.ts.unwrap_or(0) >= time_cutoff);
    
    if let Some(index) = api_history_index {
        let new_api_history = task.api_conversation_history[..index].to_vec();
        task.overwrite_api_conversation_history(new_api_history).await?;
    }
    
    Ok(())
}

/// Resend message after editing
pub async fn resend_message_sequence(
    provider: &mut ClineProvider,
    task_id: &str,
    original_message_index: usize,
    original_message_timestamp: u64,
    edited_text: String,
    images: Option<Vec<String>>,
) -> Result<bool> {
    // 1. Get current task instance
    let current_task = provider.get_current_task()
        .ok_or_else(|| anyhow::anyhow!("No current task instance"))?;
    
    if current_task.task_id != task_id {
        log::error!("Task ID mismatch: {} != {}", current_task.task_id, task_id);
        return Ok(false);
    }
    
    // 2. Delete messages
    delete_messages_for_resend(
        current_task,
        original_message_index,
        original_message_timestamp,
    ).await?;
    
    provider.post_state_to_webview().await?;
    
    // 3. Re-initialize task
    let history_item = provider.get_task_with_id(task_id).await?
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve history item"))?;
    
    let new_task = provider.create_task_with_history_item(history_item).await?
        .ok_or_else(|| anyhow::anyhow!("Failed to re-initialize task"))?;
    
    // 4. Send edited message
    tokio::time::sleep(Duration::from_millis(100)).await;  // Race condition mitigation
    new_task.handle_webview_ask_response("messageResponse", edited_text, images).await?;
    
    Ok(true)
}

/// Handle message editing with optional checkpoint revert
pub async fn edit_message_handler(
    provider: &mut ClineProvider,
    message: &WebviewMessage,
) -> Result<()> {
    let timestamp = message.values.get("ts")
        .ok_or_else(|| anyhow::anyhow!("Missing timestamp"))?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;
    
    let new_text = message.values.get("text")
        .ok_or_else(|| anyhow::anyhow!("Missing text"))?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid text"))?
        .to_string();
    
    let revert = message.values.get("revert")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let images = message.values.get("images")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
    
    let current_task = provider.get_current_task()
        .ok_or_else(|| anyhow::anyhow!("No active task instance"))?;
    
    // Find message by timestamp
    let message_index = current_task.cline_messages
        .iter()
        .position(|msg| msg.ts == Some(timestamp))
        .ok_or_else(|| anyhow::anyhow!("Message not found"))?;
    
    if revert {
        // Find most recent checkpoint
        let checkpoint = current_task.cline_messages
            .iter()
            .filter(|msg| msg.say == Some("checkpoint_saved".to_string()))
            .filter(|msg| msg.ts.unwrap_or(0) <= timestamp)
            .max_by_key(|msg| msg.ts.unwrap_or(0));
        
        if let Some(checkpoint_msg) = checkpoint {
            if let Some(commit_hash) = &checkpoint_msg.text {
                // Cancel current task
                provider.cancel_task().await?;
                
                // Wait for cancellation
                let result = timeout(
                    Duration::from_secs(3),
                    wait_for_initialization(current_task),
                ).await;
                
                if result.is_err() {
                    return Err(anyhow::anyhow!("Checkpoint timeout"));
                }
                
                // Restore checkpoint
                current_task.checkpoint_restore(CheckpointRestoreOptions {
                    commit_hash: commit_hash.clone(),
                    ts: checkpoint_msg.ts,
                    mode: "preview".to_string(),
                }).await?;
                
                // Wait for restore to settle
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } else {
            return Err(anyhow::anyhow!("No checkpoint found"));
        }
    }
    
    // Update message text
    let mut updated_messages = current_task.cline_messages.clone();
    updated_messages[message_index].text = Some(new_text.clone());
    current_task.overwrite_cline_messages(updated_messages).await?;
    
    // Resend message
    let success = resend_message_sequence(
        provider,
        &current_task.task_id,
        message_index,
        timestamp,
        new_text,
        images,
    ).await?;
    
    if success {
        show_notification("Message updated successfully");
    }
    
    Ok(())
}

/// Fetch notifications from Kilocode API
pub async fn fetch_kilocode_notifications(
    provider: &ClineProvider,
) -> Result<Vec<KilocodeNotification>> {
    let state = provider.get_state().await?;
    let api_config = state.api_configuration
        .ok_or_else(|| anyhow::anyhow!("No API configuration"))?;
    
    let kilocode_token = api_config.kilocode_token
        .ok_or_else(|| anyhow::anyhow!("No Kilocode token"))?;
    
    if api_config.api_provider != "kilocode" {
        return Ok(Vec::new());
    }
    
    let base_uri = get_kilo_base_uri_from_token(&kilocode_token)?;
    let url = format!("{}/api/users/notifications", base_uri);
    
    let client = Client::new();
    let response = timeout(
        Duration::from_secs(5),
        client.get(&url)
            .header("Authorization", format!("Bearer {}", kilocode_token))
            .header("Content-Type", "application/json")
            .send()
    ).await
        .context("Request timeout")?
        .context("Request failed")?;
    
    let notifications: Vec<KilocodeNotification> = response.json().await
        .context("Failed to parse notifications")?;
    
    Ok(notifications)
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Timestamp-Based Message Matching

**Why timestamps instead of indices?**
- Indices change when messages are deleted
- Timestamps are immutable identifiers
- Allows matching across UI and API message arrays

**Edge case**: -1000ms buffer for API message lookup
- Accounts for timing differences
- Prevents missing the corresponding API message

### 2. Task Re-initialization on Edit

**Why not edit in-place?**
- Clean execution state
- Prevents stale references
- Ensures checkpoint tracking consistency
- Avoids complex state synchronization

**Trade-off**: Slightly slower but much safer

### 3. Race Condition Mitigation

**100ms delay before sending edited message**:
```typescript
await new Promise((resolve) => setTimeout(resolve, 100))
```

**Why needed?**
- Task re-initialization is async
- Message handling might start before ready
- 100ms is imperceptible to users

**500ms delay after checkpoint restore**:
```typescript
await new Promise((resolve) => setTimeout(resolve, 500))
```

**Why longer?**
- Git operations are slower
- File system needs time to settle
- VSCode watchers need to update

### 4. Silent Notification Failures

**Why not show errors?**
```typescript
catch (error) {
    provider.log(`Error fetching notifications: ${error.message}`)
    provider.postMessageToWebview({
        type: "kilocodeNotificationsResponse",
        notifications: [],
    })
}
```

**Reasoning**:
- Notifications are non-critical
- Network failures are common
- Don't interrupt user workflow
- Log for debugging

### 5. Checkpoint Search Strategy

**Filters + Sort + First**:
```typescript
const checkpointMessage = clineMessages
    .filter(msg => msg.say === "checkpoint_saved")
    .filter(msg => msg.ts && msg.ts <= timestamp)
    .sort((a, b) => (b.ts || 0) - (a.ts || 0))[0]
```

**Why not findLast?**
- More explicit about ordering
- Handles missing timestamps
- Clear intent in code

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `vscode` - VSCode API
- `p-wait-for` (^4.1.0) - Polling with timeout
- `axios` (^1.6.0) - HTTP client

**Internal Modules**:
- `../../webview/ClineProvider` - Main provider
- `../../task/Task` - Task execution
- `../../../i18n` - Internationalization
- `../../../shared/WebviewMessage` - Message types
- `../../../shared/kilocode/token` - Token utilities

**Rust Crates**:
- `reqwest` (0.11) - HTTP client
- `tokio` (1.35) - Async runtime
- `serde` (1.0) - Serialization
- `anyhow` (1.0) - Error handling
- `log` (0.4) - Logging

---

## 📊 PERFORMANCE CHARACTERISTICS

### Message Editing
- **Delete operation**: O(n) array slicing
- **Re-initialization**: ~100-500ms (includes file I/O)
- **Total latency**: ~500-1000ms perceived by user

### Checkpoint Revert
- **Checkpoint search**: O(n) message scan
- **Git restore**: ~500-2000ms (depends on repo size)
- **Total latency**: ~1-3 seconds

### Notification Fetch
- **Network latency**: 50-500ms (depends on connection)
- **Timeout**: 5 seconds max
- **Cache**: No caching (always fresh)

---

## 🎓 KEY TAKEAWAYS

✅ **Message Editing**: Truncate + resend pattern for clean state

✅ **Checkpoint Integration**: Restore git state before continuing

✅ **Race Condition Handling**: Strategic delays prevent timing issues

✅ **Error Recovery**: Graceful fallback for all operations

✅ **Cloud Integration**: Optional Kilocode-specific features

✅ **Timestamp-Based**: Immutable identifiers for message matching

✅ **Small Module**: Only 194 lines, focused functionality

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Medium
**Estimated Effort**: 3-4 hours
**Lines of Rust**: ~250 lines
**Dependencies**: `reqwest`, `tokio`, async traits
**Key Challenge**: Async task re-initialization, git operations
**Risk**: Medium - timing-dependent operations

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-20 (i18n/)
