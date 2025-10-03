# DEEP ANALYSIS 06: HISTORY COMPONENTS - TASK MANAGEMENT UI

## 📁 Analyzed Files

```
Codex/webview-ui/src/components/history/
├── HistoryView.tsx                   (315 lines, main history browser)
│   ├── Search/Filter/Sort UI
│   ├── Selection Mode (batch ops)
│   ├── Workspace Filtering
│   └── Virtualized List (react-virtuoso)
│
├── TaskItem.tsx                      (107 lines, task card)
│   ├── Compact/Full Variants
│   ├── Selection Checkbox
│   ├── Click Handlers
│   └── TaskItemFooter Integration
│
├── TaskItemFooter.tsx                (48 lines, metadata + actions)
│   ├── Time Ago Formatting
│   ├── Cost Display
│   ├── Action Buttons
│   └── CopyButton/ExportButton/DeleteButton/FavoriteButton
│
├── HistoryPreview.tsx                (37 lines, top 3 tasks)
│   └── Welcome Screen Integration
│
├── DeleteButton.tsx                  (40 lines)
├── ExportButton.tsx                  (30 lines)
├── CopyButton.tsx
├── DeleteTaskDialog.tsx
├── BatchDeleteTaskDialog.tsx
└── hooks/
    └── useTaskSearch.ts              (Fuzzy search + filtering)

Total: 9 components + 1 hook → Rust API endpoints + RocksDB queries
```

---

## Overview
The history system provides task browsing, search, filtering, and management. **9 components + 1 hook** handle the complete task history UI.

---

## Component Architecture

```
HistoryView (315 lines)
├── useTaskSearch hook ──────────► Fuzzy search + filtering
├── TaskItem (107 lines) ────────► Individual task display
│   └── TaskItemFooter (48 lines)
│       ├── CopyButton
│       ├── ExportButton
│       ├── DeleteButton
│       └── FavoriteButton
├── DeleteTaskDialog
├── BatchDeleteTaskDialog
└── HistoryPreview (37 lines) ───► Compact view (top 3 tasks)
```

---

## 1. HistoryItem Data Type

```typescript
// Core history record stored in database
interface HistoryItem {
    id: string                      // UUID
    ts: number                      // Unix timestamp (milliseconds)
    task: string                    // User's prompt/task description
    tokensIn: number               // Input tokens used
    tokensOut: number              // Output tokens generated
    cacheWrites?: number           // Prompt cache writes
    cacheReads?: number            // Prompt cache reads
    totalCost: number              // Total API cost in USD
    workspace?: string             // Workspace path
    isFavorited?: boolean          // User favorite flag (Kilocode)
    fileNotfound?: boolean         // Task file deleted flag (Kilocode)
}

// Display version with fuzzy search highlighting
interface DisplayHistoryItem extends HistoryItem {
    highlight?: string             // HTML with <mark> tags for search matches
}
```

**Rust Translation:**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryItem {
    pub id: String,
    pub ts: i64,
    pub task: String,
    pub tokens_in: i32,
    pub tokens_out: i32,
    pub cache_writes: Option<i32>,
    pub cache_reads: Option<i32>,
    pub total_cost: f64,
    pub workspace: Option<String>,
    pub is_favorited: Option<bool>,
    pub file_notfound: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DisplayHistoryItem {
    #[serde(flatten)]
    pub item: HistoryItem,
    pub highlight: Option<String>,
}
```

---

## 2. HistoryView Component (315 lines)

**Purpose:** Main history browser with search, sort, filter, and batch operations.

### Key Features

```typescript
// State management
const [isSelectionMode, setIsSelectionMode] = useState(false)
const [selectedTaskIds, setSelectedTaskIds] = useState<string[]>([])
const [deleteTaskId, setDeleteTaskId] = useState<string | null>(null)
const [showBatchDeleteDialog, setShowBatchDeleteDialog] = useState(false)

// Search and filter from hook
const {
    tasks,                    // Filtered + sorted results
    searchQuery,
    setSearchQuery,
    sortOption,               // "newest" | "oldest" | "mostExpensive" | "mostTokens" | "mostRelevant"
    setSortOption,
    showAllWorkspaces,        // Current workspace vs all
    setShowAllWorkspaces,
    showFavoritesOnly,        // Kilocode feature
    setShowFavoritesOnly,
} = useTaskSearch()

// Selection mode operations
const toggleSelectionMode = () => {
    setIsSelectionMode(!isSelectionMode)
    if (isSelectionMode) {
        setSelectedTaskIds([])  // Clear on exit
    }
}

const toggleTaskSelection = (taskId: string, isSelected: boolean) => {
    if (isSelected) {
        setSelectedTaskIds(prev => [...prev, taskId])
    } else {
        setSelectedTaskIds(prev => prev.filter(id => id !== taskId))
    }
}

const toggleSelectAll = (selectAll: boolean) => {
    if (selectAll) {
        setSelectedTaskIds(tasks.map(task => task.id))
    } else {
        setSelectedTaskIds([])
    }
}

const handleBatchDelete = () => {
    if (selectedTaskIds.length > 0) {
        setShowBatchDeleteDialog(true)
    }
}
```

### UI Sections

```typescript
// 1. Search bar with clear button
<VSCodeTextField
    placeholder="Search history..."
    value={searchQuery}
    onInput={(e) => {
        const newValue = (e.target as HTMLInputElement).value
        setSearchQuery(newValue)
        
        // Auto-switch to "most relevant" when searching
        if (newValue && !searchQuery && sortOption !== "mostRelevant") {
            setLastNonRelevantSort(sortOption)
            setSortOption("mostRelevant")
        }
    }}>
    <div slot="start" className="codicon codicon-search" />
    {searchQuery && (
        <div 
            className="codicon codicon-close"
            onClick={() => setSearchQuery("")}
            slot="end"
        />
    )}
</VSCodeTextField>

// 2. Workspace filter dropdown
<Select
    value={showAllWorkspaces ? "all" : "current"}
    onValueChange={(value) => setShowAllWorkspaces(value === "all")}>
    <SelectTrigger>
        <SelectValue>
            Workspace: {showAllWorkspaces ? "All" : "Current"}
        </SelectValue>
    </SelectTrigger>
    <SelectContent>
        <SelectItem value="current">
            <span className="codicon codicon-folder" />
            Current Workspace
        </SelectItem>
        <SelectItem value="all">
            <span className="codicon codicon-folder-opened" />
            All Workspaces
        </SelectItem>
    </SelectContent>
</Select>

// 3. Sort dropdown
<Select value={sortOption} onValueChange={setSortOption}>
    <SelectContent>
        <SelectItem value="newest">
            <span className="codicon codicon-arrow-down" />
            Newest First
        </SelectItem>
        <SelectItem value="oldest">
            <span className="codicon codicon-arrow-up" />
            Oldest First
        </SelectItem>
        <SelectItem value="mostExpensive">
            <span className="codicon codicon-credit-card" />
            Most Expensive
        </SelectItem>
        <SelectItem value="mostTokens">
            <span className="codicon codicon-symbol-numeric" />
            Most Tokens
        </SelectItem>
        <SelectItem value="mostRelevant" disabled={!searchQuery}>
            <span className="codicon codicon-search" />
            Most Relevant
        </SelectItem>
    </SelectContent>
</Select>

// 4. Favorites checkbox (Kilocode)
<Checkbox
    id="show-favorites-only"
    checked={showFavoritesOnly}
    onCheckedChange={(checked) => setShowFavoritesOnly(checked)}
/>
<label htmlFor="show-favorites-only">
    Show Favorites Only
</label>

// 5. Select all (in selection mode)
{isSelectionMode && tasks.length > 0 && (
    <Checkbox
        checked={selectedTaskIds.length === tasks.length}
        onCheckedChange={toggleSelectAll}
    />
    <span>
        {selectedTaskIds.length === tasks.length ? "Deselect All" : "Select All"}
    </span>
    <span className="ml-auto">
        {selectedTaskIds.length} / {tasks.length} selected
    </span>
)}

// 6. Virtualized task list (react-virtuoso)
<Virtuoso
    data={tasks}
    itemContent={(_index, item) => (
        <TaskItem
            key={item.id}
            item={item}
            variant="full"
            showWorkspace={showAllWorkspaces}
            isSelectionMode={isSelectionMode}
            isSelected={selectedTaskIds.includes(item.id)}
            onToggleSelection={toggleTaskSelection}
            onDelete={setDeleteTaskId}
        />
    )}
/>

// 7. Fixed action bar (bottom, selection mode only)
{isSelectionMode && selectedTaskIds.length > 0 && (
    <div className="fixed bottom-0 left-0 right-0 bg-vscode-editor-background border-t p-2">
        <div>{selectedTaskIds.length} / {tasks.length} selected</div>
        <Button variant="secondary" onClick={() => setSelectedTaskIds([])}>
            Clear Selection
        </Button>
        <Button variant="default" onClick={handleBatchDelete}>
            Delete Selected
        </Button>
    </div>
)}
```

**Rust Translation:**

```rust
// Backend provides search/filter API, frontend handles UI state
pub async fn search_history(
    request: HistorySearchRequest,
    state: Arc<RwLock<AppState>>,
) -> Result<Vec<DisplayHistoryItem>> {
    let state = state.read().await;
    let mut tasks = state.task_history.clone();
    
    // Filter by workspace
    if !request.show_all_workspaces {
        let current_workspace = &state.cwd;
        tasks.retain(|t| t.workspace.as_ref() == Some(current_workspace));
    }
    
    // Filter by favorites
    if request.show_favorites_only {
        tasks.retain(|t| t.is_favorited.unwrap_or(false));
    }
    
    // Fuzzy search
    let results = if !request.search_query.is_empty() {
        fuzzy_search_tasks(&tasks, &request.search_query)
    } else {
        tasks.into_iter()
            .map(|item| DisplayHistoryItem { item, highlight: None })
            .collect()
    };
    
    // Sort
    let mut results = results;
    sort_tasks(&mut results, &request.sort_option);
    
    Ok(results)
}

#[derive(Deserialize)]
pub struct HistorySearchRequest {
    pub search_query: String,
    pub sort_option: SortOption,
    pub show_all_workspaces: bool,
    pub show_favorites_only: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SortOption {
    Newest,
    Oldest,
    MostExpensive,
    MostTokens,
    MostRelevant,
}

fn sort_tasks(tasks: &mut Vec<DisplayHistoryItem>, sort: &SortOption) {
    tasks.sort_by(|a, b| {
        match sort {
            SortOption::Newest => b.item.ts.cmp(&a.item.ts),
            SortOption::Oldest => a.item.ts.cmp(&b.item.ts),
            SortOption::MostExpensive => {
                b.item.total_cost.partial_cmp(&a.item.total_cost)
                    .unwrap_or(Ordering::Equal)
            }
            SortOption::MostTokens => {
                let a_tokens = a.item.tokens_in + a.item.tokens_out;
                let b_tokens = b.item.tokens_in + b.item.tokens_out;
                b_tokens.cmp(&a_tokens)
            }
            SortOption::MostRelevant => {
                // Already sorted by fuzzy score
                Ordering::Equal
            }
        }
    });
}
```

---

## 3. TaskItem Component (107 lines)

**Purpose:** Display individual task with actions.

```typescript
interface TaskItemProps {
    item: DisplayHistoryItem
    variant: "compact" | "full"           // Compact = no actions, Full = with actions
    showWorkspace?: boolean               // Show workspace path
    isSelectionMode?: boolean             // Show checkbox
    isSelected?: boolean                  // Checkbox state
    onToggleSelection?: (id: string, selected: boolean) => void
    onDelete?: (id: string) => void       // Trigger delete dialog
}

const TaskItem = ({ item, variant, ... }: TaskItemProps) => {
    const handleClick = () => {
        if (isSelectionMode && onToggleSelection) {
            onToggleSelection(item.id, !isSelected)
        } else {
            // Open task
            vscode.postMessage({ type: "showTaskWithId", text: item.id })
        }
    }
    
    return (
        <div
            onClick={handleClick}
            className={cn(
                "cursor-pointer group bg-vscode-editor-background rounded border hover:bg-vscode-list-hoverBackground",
                {
                    "bg-red-900": item.fileNotfound,  // Visual indicator
                }
            )}>
            
            {/* Checkbox (selection mode, full variant only) */}
            {variant === "full" && isSelectionMode && (
                <Checkbox
                    checked={isSelected}
                    onCheckedChange={(checked) => onToggleSelection?.(item.id, checked)}
                    onClick={(e) => e.stopPropagation()}
                />
            )}
            
            {/* Task content */}
            <div className="flex-1 min-w-0">
                {/* Task text with fuzzy search highlighting */}
                <div
                    className="overflow-hidden whitespace-pre-wrap text-ellipsis line-clamp-2"
                    dangerouslySetInnerHTML={
                        item.highlight 
                            ? { __html: item.highlight }
                            : undefined
                    }>
                    {item.highlight ? undefined : item.task}
                </div>
                
                {/* Footer with actions */}
                <TaskItemFooter
                    item={item}
                    variant={variant}
                    isSelectionMode={isSelectionMode}
                    onDelete={onDelete}
                />
                
                {/* Workspace path (if enabled) */}
                {showWorkspace && item.workspace && (
                    <div className="flex gap-1 text-xs mt-1">
                        <span className="codicon codicon-folder" />
                        <span>{item.workspace}</span>
                    </div>
                )}
            </div>
        </div>
    )
}
```

**Rust Backend Messages:**

```rust
// Open task
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    #[serde(rename = "showTaskWithId")]
    ShowTaskWithId { text: String },  // Task ID
}

pub async fn handle_show_task(
    task_id: &str,
    state: Arc<RwLock<AppState>>,
    clients: &HashMap<Uuid, WebSocket>,
) -> Result<()> {
    let state = state.read().await;
    
    // Load task from database
    let task = load_task_from_db(task_id).await?;
    
    // Update current task state
    drop(state);
    let mut state = state.write().await;
    state.cline_messages = task.messages;
    state.current_task_item = Some(task.metadata);
    
    // Broadcast state update
    broadcast_state_update(&*state, clients).await;
    
    Ok(())
}
```

---

## 4. TaskItemFooter Component (48 lines)

**Purpose:** Display metadata and action buttons.

```typescript
const TaskItemFooter = ({ item, variant, isSelectionMode, onDelete }) => {
    return (
        <div className="flex justify-between items-center text-xs">
            {/* Metadata */}
            <div className="flex gap-2 text-vscode-descriptionForeground/60">
                {/* Time ago with hover tooltip */}
                <StandardTooltip content={new Date(item.ts).toLocaleString()}>
                    <span>{formatTimeAgo(item.ts)}</span>
                </StandardTooltip>
                
                <span>·</span>
                
                {/* Cost */}
                {item.totalCost > 0 && (
                    <span>${item.totalCost.toFixed(2)}</span>
                )}
            </div>
            
            {/* Action buttons (not in selection mode) */}
            {!isSelectionMode && (
                <div className="flex gap-0">
                    <CopyButton itemTask={item.task} />
                    <FavoriteButton isFavorited={item.isFavorited ?? false} id={item.id} />
                    {variant === "full" && <ExportButton itemId={item.id} />}
                    {onDelete && <DeleteButton itemId={item.id} onDelete={onDelete} />}
                </div>
            )}
        </div>
    )
}

// Time ago formatter
function formatTimeAgo(ts: number): string {
    const seconds = Math.floor((Date.now() - ts) / 1000)
    
    if (seconds < 60) return "just now"
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    if (seconds < 2592000) return `${Math.floor(seconds / 86400)}d ago`
    return `${Math.floor(seconds / 2592000)}mo ago`
}
```

**Rust Translation:**

```rust
use chrono::{DateTime, Utc, Duration};

pub fn format_time_ago(ts: i64) -> String {
    let now = Utc::now().timestamp_millis();
    let seconds = (now - ts) / 1000;
    
    match seconds {
        s if s < 60 => "just now".to_string(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86400 => format!("{}h ago", s / 3600),
        s if s < 2592000 => format!("{}d ago", s / 86400),
        s => format!("{}mo ago", s / 2592000),
    }
}

pub fn format_timestamp(ts: i64) -> String {
    DateTime::from_timestamp(ts / 1000, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Invalid date".to_string())
}
```

---

## 5. Action Buttons

### CopyButton
```typescript
export const CopyButton = ({ itemTask }: { itemTask: string }) => {
    const { showCopyFeedback, copyWithFeedback } = useCopyToClipboard()
    
    return (
        <StandardTooltip content={showCopyFeedback ? "Copied!" : "Copy task"}>
            <Button
                variant="ghost"
                size="icon"
                onClick={(e) => {
                    e.stopPropagation()
                    copyWithFeedback(itemTask)
                }}>
                <span className={showCopyFeedback ? "codicon-check" : "codicon-copy"} />
            </Button>
        </StandardTooltip>
    )
}
```

### DeleteButton
```typescript
export const DeleteButton = ({ itemId, onDelete }) => {
    const handleDeleteClick = (e: React.MouseEvent) => {
        e.stopPropagation()
        
        if (e.shiftKey) {
            // Shift+click = immediate delete (no confirmation)
            vscode.postMessage({ type: "deleteTaskWithId", text: itemId })
        } else {
            // Normal click = show confirmation dialog
            onDelete?.(itemId)
        }
    }
    
    return (
        <StandardTooltip content="Delete task">
            <Button variant="ghost" size="icon" onClick={handleDeleteClick}>
                <span className="codicon codicon-trash" />
            </Button>
        </StandardTooltip>
    )
}
```

### ExportButton
```typescript
export const ExportButton = ({ itemId }) => {
    const handleExportClick = (e: React.MouseEvent) => {
        e.stopPropagation()
        vscode.postMessage({ type: "exportTaskWithId", text: itemId })
    }
    
    return (
        <StandardTooltip content="Export task">
            <Button variant="ghost" size="icon" onClick={handleExportClick}>
                <span className="codicon codicon-desktop-download" />
            </Button>
        </StandardTooltip>
    )
}
```

**Rust Backend Handlers:**

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    #[serde(rename = "deleteTaskWithId")]
    DeleteTaskWithId { text: String },  // Task ID
    
    #[serde(rename = "exportTaskWithId")]
    ExportTaskWithId { text: String },  // Task ID
}

pub async fn handle_delete_task(
    task_id: &str,
    state: Arc<RwLock<AppState>>,
) -> Result<()> {
    // Remove from database
    delete_task_from_db(task_id).await?;
    
    // Update state
    let mut state = state.write().await;
    state.task_history.retain(|t| t.id != task_id);
    
    // Broadcast update
    broadcast_history_update(&state.task_history, clients).await;
    
    Ok(())
}

pub async fn handle_export_task(
    task_id: &str,
    state: Arc<RwLock<AppState>>,
) -> Result<()> {
    let task = load_task_from_db(task_id).await?;
    
    // Format as markdown
    let markdown = format_task_as_markdown(&task);
    
    // Save to file (use native file dialog)
    let default_filename = format!("task_{}.md", task_id);
    
    // Return path or content to frontend
    Ok(())
}

fn format_task_as_markdown(task: &Task) -> String {
    format!(
        "# {}\n\n**Date:** {}\n**Cost:** ${:.2}\n**Tokens:** {} in / {} out\n\n## Messages\n\n{}",
        task.metadata.task,
        format_timestamp(task.metadata.ts),
        task.metadata.total_cost,
        task.metadata.tokens_in,
        task.metadata.tokens_out,
        task.messages.iter()
            .map(|msg| format!("### {}\n\n{}", msg.r#type, msg.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n\n")
    )
}
```

---

## 6. HistoryPreview Component (37 lines)

**Purpose:** Compact history view for welcome screen (top 3 tasks).

```typescript
const HistoryPreview = () => {
    const { tasks } = useTaskSearch()
    
    const handleViewAllHistory = () => {
        vscode.postMessage({ type: "switchTab", tab: "history" })
    }
    
    return (
        <div className="flex flex-col gap-3">
            {tasks.length !== 0 && (
                <>
                    {/* Show top 3 tasks */}
                    {tasks.slice(0, 3).map((item) => (
                        <TaskItem 
                            key={item.id} 
                            item={item} 
                            variant="compact"  // No actions
                        />
                    ))}
                    
                    {/* View all button */}
                    <button
                        onClick={handleViewAllHistory}
                        className="text-base text-vscode-descriptionForeground hover:text-vscode-textLink-foreground">
                        View All History
                    </button>
                </>
            )}
        </div>
    )
}
```

---

## Database Schema

```rust
// RocksDB column family: cf_task_history
// Key format: "{timestamp}_{uuid}" for chronological ordering
// Value: HistoryItem (JSON)

pub async fn save_task_to_history(
    task: &Task,
    db: &Arc<DB>,
) -> Result<()> {
    let cf = db.cf_handle("task_history").unwrap();
    
    let history_item = HistoryItem {
        id: task.id.clone(),
        ts: task.created_at,
        task: task.prompt.clone(),
        tokens_in: task.usage.input_tokens,
        tokens_out: task.usage.output_tokens,
        cache_writes: Some(task.usage.cache_creation_input_tokens),
        cache_reads: Some(task.usage.cache_read_input_tokens),
        total_cost: task.usage.total_cost,
        workspace: Some(task.workspace.clone()),
        is_favorited: Some(false),
        file_notfound: Some(false),
    };
    
    let key = format!("{}_{}", task.created_at, task.id);
    let value = serde_json::to_vec(&history_item)?;
    
    db.put_cf(cf, key.as_bytes(), &value)?;
    
    Ok(())
}

pub async fn load_history(
    db: &Arc<DB>,
    limit: Option<usize>,
) -> Result<Vec<HistoryItem>> {
    let cf = db.cf_handle("task_history").unwrap();
    
    let mut items = Vec::new();
    let iter = db.iterator_cf(cf, IteratorMode::End);
    
    for (i, (key, value)) in iter.enumerate() {
        if let Some(limit) = limit {
            if i >= limit {
                break;
            }
        }
        
        let item: HistoryItem = serde_json::from_slice(&value)?;
        items.push(item);
    }
    
    Ok(items)
}
```

---

**STATUS:** Complete history components analysis (9 components + database schema)
**NEXT:** DEEP-07-KILOCODE-COMPONENTS.md - Modes, rules, workflows
