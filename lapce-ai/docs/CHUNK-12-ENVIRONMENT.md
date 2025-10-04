# CHUNK-12: ENVIRONMENT (SYSTEM CONTEXT & WORKSPACE DETECTION)

## 📁 Complete System Analysis

```
Environment Context System:
├── Codex/src/core/environment/
│   ├── getEnvironmentDetails.ts    (302 lines) - Main context builder
│   └── reminder.ts                 (39 lines)  - Todo list formatter
└── Related:
    ├── services/glob/list-files.ts  - Recursive file listing
    └── integrations/terminal/       - Terminal state tracking

TOTAL: 341+ lines environment introspection
```

---

## 🎯 PURPOSE

**Dynamic Context Injection**: Build real-time snapshot of workspace state to inject into every AI message.

**Critical for**:
- AI awareness of file changes
- Terminal command results tracking
- Workspace structure understanding
- Time-aware operations
- Cost tracking visibility
- Mode-specific constraints

---

## 📊 ARCHITECTURE OVERVIEW

```
getEnvironmentDetails() Flow:

1. VSCode Visible Files (what user is looking at)
2. VSCode Open Tabs (what user has open)
3. Actively Running Terminals (with output)
4. Inactive Terminals with Completed Processes
5. Recently Modified Files (stale context warning)
6. Current Time (with timezone)
7. Current Cost (API spending)
8. Current Mode (with role definition)
9. Workspace Files (recursive tree, max 200)
10. Todo List (reminders section)

Output: XML-wrapped context string (~5,000-20,000 chars)
```

---

## 🔧 FILE 1: getEnvironmentDetails.ts (302 lines)

### Function Signature - Line 31

```typescript
export async function getEnvironmentDetails(
    cline: Task,
    includeFileDetails: boolean = false
): Promise<string>
```

**Called**:
- Before every AI API request
- After tool execution completes
- When resuming from idle state

**Parameters**:
- `cline`: Task instance (provides context)
- `includeFileDetails`: Include full workspace tree (only on first message)

---

### Section 1: Visible Files - Lines 44-61

**Purpose**: Show AI what user is actively viewing in editor.

```typescript
details += "\n\n# VSCode Visible Files"

const visibleFilePaths = vscode.window.visibleTextEditors
    ?.map(editor => editor.document?.uri?.fsPath)
    .filter(Boolean)
    .map(absolutePath => path.relative(cline.cwd, absolutePath))
    .slice(0, maxWorkspaceFiles)

// Filter through .rooignore
const allowedVisibleFiles = cline.rooIgnoreController
    ? cline.rooIgnoreController.filterPaths(visibleFilePaths)
    : visibleFilePaths.map(p => p.toPosix()).join("\n")

if (allowedVisibleFiles) {
    details += `\n${allowedVisibleFiles}`
} else {
    details += "\n(No visible files)"
}
```

**Output example**:
```
# VSCode Visible Files
src/main.rs
src/lib.rs
```

**Why relative paths?** Shorter, workspace-centric context.

**Why filter through rooignore?** Respect user privacy (don't expose `.env`, secrets).

---

### Section 2: Open Tabs - Lines 63-83

**Purpose**: Show AI broader context of what user is working on.

```typescript
details += "\n\n# VSCode Open Tabs"

const { maxOpenTabsContext } = state ?? {}
const maxTabs = maxOpenTabsContext ?? 20

const openTabPaths = vscode.window.tabGroups.all
    .flatMap(group => group.tabs)
    .filter(tab => tab.input instanceof vscode.TabInputText)
    .map(tab => (tab.input as vscode.TabInputText).uri.fsPath)
    .filter(Boolean)
    .map(absolutePath => path.relative(cline.cwd, absolutePath).toPosix())
    .slice(0, maxTabs)

const allowedOpenTabs = cline.rooIgnoreController
    ? cline.rooIgnoreController.filterPaths(openTabPaths)
    : openTabPaths.map(p => p.toPosix()).join("\n")

if (allowedOpenTabs) {
    details += `\n${allowedOpenTabs}`
} else {
    details += "\n(No open tabs)"
}
```

**Limit**: 20 tabs max (configurable) to prevent context bloat.

**Why tabs AND visible?** Visible = split editors (immediate focus), Tabs = broader context.

---

### Section 3: Terminal State - Lines 85-181

**Most complex section** - Tracks command execution results.

#### 3A: Classify Terminals - Lines 86-94

```typescript
// Get task-specific and background terminals
const busyTerminals = [
    ...TerminalRegistry.getTerminals(true, cline.taskId),  // Busy in this task
    ...TerminalRegistry.getBackgroundTerminals(true),      // Busy globally
]

const inactiveTerminals = [
    ...TerminalRegistry.getTerminals(false, cline.taskId),  // Idle in this task
    ...TerminalRegistry.getBackgroundTerminals(false),      // Idle globally
]
```

**Terminal types**:
- **Task-specific**: Created by `execute_command` in current task
- **Background**: User's pre-existing terminals or long-running servers

**Busy vs Inactive**:
- **Busy**: Process currently running
- **Inactive**: No active process, but may have completed process output

#### 3B: Wait for Output - Lines 96-109

```typescript
if (busyTerminals.length > 0) {
    if (cline.didEditFile) {
        await delay(300)  // Let watchers trigger (e.g., `npm run build` after file save)
    }
    
    // Wait for terminals to "cool down" (process completes)
    await pWaitFor(() => busyTerminals.every(t => !TerminalRegistry.isProcessHot(t.id)), {
        interval: 100,
        timeout: 5_000,
    }).catch(() => {})
}

cline.didEditFile = false
```

**"Hot" process**: Still producing output rapidly.

**Why wait?** Capture full command output instead of partial.

**Example timeline**:
```
T+0ms:   AI writes file via write_to_file
T+10ms:  File watcher triggers `cargo check`
T+300ms: Delay complete
T+400ms: cargo check still running (hot)
T+1200ms: cargo check completes (cool)
T+1200ms: Retrieve output → AI sees compile errors
```

#### 3C: Active Terminal Output - Lines 115-135

```typescript
if (busyTerminals.length > 0) {
    terminalDetails += "\n\n# Actively Running Terminals"
    
    for (const busyTerminal of busyTerminals) {
        const cwd = busyTerminal.getCurrentWorkingDirectory()
        terminalDetails += `\n## Terminal ${busyTerminal.id} (Active)`
        terminalDetails += `\n### Working Directory: \`${cwd}\``
        terminalDetails += `\n### Original command: \`${busyTerminal.getLastCommand()}\``
        
        let newOutput = TerminalRegistry.getUnretrievedOutput(busyTerminal.id)
        
        if (newOutput) {
            newOutput = Terminal.compressTerminalOutput(
                newOutput,
                terminalOutputLineLimit,      // Default: 500 lines
                terminalOutputCharacterLimit,  // Default: 50,000 chars
            )
            terminalDetails += `\n### New Output\n${newOutput}`
        }
    }
}
```

**"Unretrieved output"**: New output since last call to `getEnvironmentDetails()`.

**Compression**: Truncates extremely long outputs (e.g., verbose logs).

**Example output**:
```
# Actively Running Terminals
## Terminal 1 (Active)
### Working Directory: `/home/user/project`
### Original command: `npm run dev`
### New Output
[vite] server started at http://localhost:5173
```

#### 3D: Inactive Terminal Completed Processes - Lines 138-181

```typescript
const terminalsWithOutput = inactiveTerminals.filter(terminal => {
    const completedProcesses = terminal.getProcessesWithOutput()
    return completedProcesses.length > 0
})

if (terminalsWithOutput.length > 0) {
    terminalDetails += "\n\n# Inactive Terminals with Completed Process Output"
    
    for (const inactiveTerminal of terminalsWithOutput) {
        let terminalOutputs: string[] = []
        
        const completedProcesses = inactiveTerminal.getProcessesWithOutput()
        
        for (const process of completedProcesses) {
            let output = process.getUnretrievedOutput()
            
            if (output) {
                output = Terminal.compressTerminalOutput(
                    output,
                    terminalOutputLineLimit,
                    terminalOutputCharacterLimit,
                )
                terminalOutputs.push(`Command: \`${process.command}\`\n${output}`)
            }
        }
        
        // Clean the queue after retrieving
        inactiveTerminal.cleanCompletedProcessQueue()
        
        if (terminalOutputs.length > 0) {
            const cwd = inactiveTerminal.getCurrentWorkingDirectory()
            terminalDetails += `\n## Terminal ${inactiveTerminal.id} (Inactive)`
            terminalDetails += `\n### Working Directory: \`${cwd}\``
            terminalOutputs.forEach(output => {
                terminalDetails += `\n### New Output\n${output}`
            })
        }
    }
}
```

**Queue system**: Terminal tracks completed processes, shows output once, then clears queue.

**Why separate active/inactive?** Active = current, Inactive = historical context.

**Example**:
```
# Inactive Terminals with Completed Process Output
## Terminal 2 (Inactive)
### Working Directory: `/home/user/project`
### New Output
Command: `cargo test`
running 5 tests
test test_parse ... ok
test test_format ... FAILED
```

---

### Section 4: Recently Modified Files - Lines 186-194

**Purpose**: Warn AI about stale context.

```typescript
const recentlyModifiedFiles = cline.fileContextTracker.getAndClearRecentlyModifiedFiles()

if (recentlyModifiedFiles.length > 0) {
    details += "\n\n# Recently Modified Files\n"
    details += "These files have been modified since you last accessed them "
    details += "(file was just edited so you may need to re-read it before editing):"
    for (const filePath of recentlyModifiedFiles) {
        details += `\n${filePath}`
    }
}
```

**Integration with FileContextTracker** (CHUNK-10):
- User edits file externally → Watcher fires → Adds to set
- `getAndClearRecentlyModifiedFiles()` → Returns and clears set

**Output**:
```
# Recently Modified Files
These files have been modified since you last accessed them (file was just edited so you may need to re-read it before editing):
src/main.rs
src/helper.rs
```

**AI behavior**: Sees this → Re-reads file before applying diff.

---

### Section 5: Current Time - Lines 201-208

**Purpose**: Enable time-aware operations.

```typescript
const now = new Date()

const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
const timeZoneOffset = -now.getTimezoneOffset() / 60
const timeZoneOffsetHours = Math.floor(Math.abs(timeZoneOffset))
const timeZoneOffsetMinutes = Math.abs(Math.round((Math.abs(timeZoneOffset) - timeZoneOffsetHours) * 60))
const timeZoneOffsetStr = `${timeZoneOffset >= 0 ? "+" : "-"}${timeZoneOffsetHours}:${timeZoneOffsetMinutes.toString().padStart(2, "0")}`

details += `\n\n# Current Time\n`
details += `Current time in ISO 8601 UTC format: ${now.toISOString()}\n`
details += `User time zone: ${timeZone}, UTC${timeZoneOffsetStr}`
```

**Output**:
```
# Current Time
Current time in ISO 8601 UTC format: 2025-10-01T03:20:40.123Z
User time zone: Asia/Kolkata, UTC+5:30
```

**Use cases**:
- "Schedule this task for tomorrow" → AI knows current date
- "Log timestamp in Eastern time" → AI can convert
- Debug log analysis with timestamps

---

### Section 6: Model & Cost Info - Lines 211-231

```typescript
const { contextTokens, totalCost } = getApiMetrics(cline.clineMessages)

// Fetch model info (for OpenRouter, Ollama)
if (cline.api instanceof OpenRouterHandler || cline.api instanceof NativeOllamaHandler) {
    try {
        await cline.api.fetchModel()
    } catch (e) {
        TelemetryService.instance.captureException(e, { context: "getEnvironmentDetails" })
        await cline.say("error", t("kilocode:task.notLoggedInError", { error: e.message }))
        return `<environment_details>\n${details.trim()}\n</environment_details>`
    }
}

const { id: modelId, info: modelInfo } = cline.api.getModel()

details += `\n\n# Current Cost\n`
details += `${totalCost !== null ? `$${totalCost.toFixed(2)}` : "(Not available)"}`
```

**Why fetch model?** OpenRouter/Ollama require API call to get model capabilities.

**Cost calculation**: Sum of all API calls in current task (input + output tokens).

**Output**:
```
# Current Cost
$0.47
```

---

### Section 7: Current Mode - Lines 234-262

```typescript
const { mode, customModes, customModePrompts, experiments, globalCustomInstructions, language } = state ?? {}

const modeDetails = await getFullModeDetails(mode ?? defaultModeSlug, customModes, customModePrompts, {
    cwd: cline.cwd,
    globalCustomInstructions,
    language: language ?? formatLanguage(vscode.env.language),
})

const currentMode = modeDetails.slug ?? mode

details += `\n\n# Current Mode\n`
details += `<slug>${currentMode}</slug>\n`
details += `<name>${modeDetails.name}</name>\n`
details += `<model>${modelId}</model>\n`

if (Experiments.isEnabled(experiments ?? {}, EXPERIMENT_IDS.POWER_STEERING)) {
    details += `<role>${modeDetails.roleDefinition}</role>\n`
    
    if (modeDetails.customInstructions) {
        details += `<custom_instructions>${modeDetails.customInstructions}</custom_instructions>\n`
    }
}
```

**POWER_STEERING experiment**: Injects mode details into environment (experimental feature).

**Output**:
```
# Current Mode
<slug>code</slug>
<name>Code</name>
<model>claude-3-5-sonnet-20241022</model>
<role>You are a senior software engineer...</role>
```

---

### Section 8: Workspace Files - Lines 264-293

**Only included when `includeFileDetails = true`** (first message only).

```typescript
if (includeFileDetails) {
    details += `\n\n# Current Workspace Directory (${cline.cwd.toPosix()}) Files\n`
    
    const isDesktop = arePathsEqual(cline.cwd, path.join(os.homedir(), "Desktop"))
    
    if (isDesktop) {
        details += "(Desktop files not shown automatically. Use list_files to explore if needed.)"
    } else {
        const maxFiles = maxWorkspaceFiles ?? 200
        
        if (maxFiles === 0) {
            details += "(Workspace files context disabled. Use list_files to explore if needed.)"
        } else {
            const [files, didHitLimit] = await listFiles(cline.cwd, true, maxFiles)
            const { showRooIgnoredFiles = true } = state ?? {}
            
            const result = formatResponse.formatFilesList(
                cline.cwd,
                files,
                didHitLimit,
                cline.rooIgnoreController,
                showRooIgnoredFiles,
            )
            
            details += result
        }
    }
}
```

**Desktop protection**: Requires explicit permission to access Desktop (privacy).

**File limit**: 200 files max to prevent context explosion.

**Output** (tree format):
```
# Current Workspace Directory (/home/user/project) Files
src/
  main.rs
  lib.rs
  utils/
    helper.rs
tests/
  integration_test.rs
Cargo.toml
README.md
```

**`.rooignore` filtering**: Excludes `node_modules/`, `.git/`, etc.

---

### Section 9: Todo List - Lines 295-300

```typescript
const todoListEnabled = state?.apiConfiguration?.todoListEnabled ?? true
const reminderSection = todoListEnabled ? formatReminderSection(cline.todoList) : ""

return `<environment_details>\n${details.trim()}\n${reminderSection}\n</environment_details>`
```

**Final output wrapping**: All sections wrapped in `<environment_details>` XML tag.

---

## 🔧 FILE 2: reminder.ts (39 lines)

### Purpose: Format Todo List as Reminder

```typescript
export function formatReminderSection(todoList?: TodoItem[]): string {
    if (!todoList || todoList.length === 0) {
        return "You have not created a todo list yet. Create one with `update_todo_list` if your task is complicated or involves multiple steps."
    }
    
    const statusMap: Record<TodoStatus, string> = {
        pending: "Pending",
        in_progress: "In Progress",
        completed: "Completed",
    }
    
    const lines: string[] = [
        "====",
        "",
        "REMINDERS",
        "",
        "Below is your current list of reminders for this task. Keep them updated as you progress.",
        "",
    ]
    
    lines.push("| # | Content | Status |")
    lines.push("|---|---------|--------|")
    
    todoList.forEach((item, idx) => {
        const escapedContent = item.content.replace(/\\/g, "\\\\").replace(/\|/g, "\\|")
        lines.push(`| ${idx + 1} | ${escapedContent} | ${statusMap[item.status] || item.status} |`)
    })
    
    lines.push("")
    lines.push("IMPORTANT: When task status changes, remember to call the `update_todo_list` tool to update your progress.")
    lines.push("")
    
    return lines.join("\n")
}
```

**Output**:
```
====

REMINDERS

Below is your current list of reminders for this task. Keep them updated as you progress.

| # | Content | Status |
|---|---------|--------|
| 1 | Fix authentication bug | Completed |
| 2 | Add unit tests | In Progress |
| 3 | Update documentation | Pending |

IMPORTANT: When task status changes, remember to call the `update_todo_list` tool to update your progress.
```

**Escaping**: Prevents markdown table breaking on pipe characters in content.

---

## 🎯 COMPLETE OUTPUT EXAMPLE

```xml
<environment_details>

# VSCode Visible Files
src/main.rs

# VSCode Open Tabs
src/main.rs
src/lib.rs
Cargo.toml

# Actively Running Terminals
## Terminal 1 (Active)
### Working Directory: `/home/user/project`
### Original command: `cargo run`
### New Output
   Compiling my-project v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.34s
     Running `target/debug/my-project`
Hello, world!

# Recently Modified Files
These files have been modified since you last accessed them (file was just edited so you may need to re-read it before editing):
src/lib.rs

# Current Time
Current time in ISO 8601 UTC format: 2025-10-01T03:20:40.123Z
User time zone: Asia/Kolkata, UTC+5:30

# Current Cost
$0.47

# Current Mode
<slug>code</slug>
<name>Code</name>
<model>claude-3-5-sonnet-20241022</model>

====

REMINDERS

Below is your current list of reminders for this task. Keep them updated as you progress.

| # | Content | Status |
|---|---------|--------|
| 1 | Implement login feature | Completed |
| 2 | Write tests for auth | In Progress |
| 3 | Deploy to staging | Pending |

IMPORTANT: When task status changes, remember to call the `update_todo_list` tool to update your progress.

</environment_details>
```

---

## 🎯 TOKEN BUDGET ANALYSIS

| Section | Typical Tokens | Frequency |
|---------|---------------|-----------|
| Visible Files | 50-100 | Every message |
| Open Tabs | 100-200 | Every message |
| Terminal Output | 500-2,000 | When commands run |
| Modified Files | 0-50 | When user edits |
| Time/Cost/Mode | 100-150 | Every message |
| Workspace Files | 1,000-5,000 | First message only |
| Todo List | 100-500 | Every message |
| **TOTAL** | **2,000-8,000** | **Every API call** |

**Optimization strategies**:
1. Disable workspace files after first message
2. Limit terminal output to 500 lines
3. Cap open tabs at 20
4. Compress terminal output aggressively

---

## 🎯 RUST TRANSLATION PATTERNS

```rust
use chrono::{DateTime, Utc, Local};
use std::path::PathBuf;

pub struct EnvironmentDetails {
    visible_files: Vec<PathBuf>,
    open_tabs: Vec<PathBuf>,
    terminal_outputs: Vec<TerminalOutput>,
    recently_modified: Vec<PathBuf>,
    current_time: DateTime<Utc>,
    current_cost: Option<f64>,
    mode_info: ModeInfo,
    workspace_files: Option<Vec<PathBuf>>,
    todo_list: Option<Vec<TodoItem>>,
}

impl EnvironmentDetails {
    pub async fn build(task: &Task, include_files: bool) -> Result<String, Error> {
        let mut sections = Vec::new();
        
        // Section 1: Visible files
        sections.push(Self::format_visible_files(&task).await?);
        
        // Section 2: Open tabs
        sections.push(Self::format_open_tabs(&task).await?);
        
        // Section 3: Terminal state
        sections.push(Self::format_terminals(&task).await?);
        
        // Section 4: Recently modified
        if let Some(modified) = Self::get_recently_modified(&task) {
            sections.push(modified);
        }
        
        // Section 5: Time
        sections.push(Self::format_time_info());
        
        // Section 6: Cost
        sections.push(Self::format_cost_info(&task));
        
        // Section 7: Mode
        sections.push(Self::format_mode_info(&task).await?);
        
        // Section 8: Workspace files (optional)
        if include_files {
            sections.push(Self::format_workspace_files(&task).await?);
        }
        
        // Section 9: Todo list
        if let Some(todos) = &task.todo_list {
            sections.push(Self::format_reminder_section(todos));
        }
        
        Ok(format!("<environment_details>\n{}\n</environment_details>", sections.join("\n")))
    }
    
    async fn format_terminals(task: &Task) -> Result<String, Error> {
        let busy = TerminalRegistry::get_busy_terminals(task.task_id);
        let inactive = TerminalRegistry::get_inactive_terminals(task.task_id);
        
        let mut output = String::new();
        
        if !busy.is_empty() {
            if task.did_edit_file {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            
            // Wait for terminals to cool down
            let timeout = Duration::from_secs(5);
            tokio::time::timeout(timeout, async {
                while busy.iter().any(|t| TerminalRegistry::is_process_hot(t.id)) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await.ok();
            
            output.push_str("\n\n# Actively Running Terminals");
            for terminal in busy {
                output.push_str(&format!("\n## Terminal {} (Active)", terminal.id));
                output.push_str(&format!("\n### Working Directory: `{}`", terminal.cwd()));
                output.push_str(&format!("\n### Original command: `{}`", terminal.last_command()));
                
                if let Some(new_output) = TerminalRegistry::get_unretrieved_output(terminal.id) {
                    let compressed = Terminal::compress_output(&new_output, 500, 50_000);
                    output.push_str(&format!("\n### New Output\n{}", compressed));
                }
            }
        }
        
        // Handle inactive terminals with completed processes
        let with_output: Vec<_> = inactive.into_iter()
            .filter(|t| !t.get_processes_with_output().is_empty())
            .collect();
        
        if !with_output.is_empty() {
            output.push_str("\n\n# Inactive Terminals with Completed Process Output");
            for terminal in with_output {
                for process in terminal.get_processes_with_output() {
                    if let Some(out) = process.get_unretrieved_output() {
                        let compressed = Terminal::compress_output(&out, 500, 50_000);
                        output.push_str(&format!("\n## Terminal {} (Inactive)", terminal.id));
                        output.push_str(&format!("\n### Working Directory: `{}`", terminal.cwd()));
                        output.push_str(&format!("\n### New Output\nCommand: `{}`\n{}", process.command, compressed));
                    }
                }
                terminal.clean_completed_process_queue();
            }
        }
        
        Ok(output)
    }
    
    fn format_time_info() -> String {
        let now = Utc::now();
        let local = Local::now();
        let offset = local.offset();
        
        format!(
            "\n\n# Current Time\nCurrent time in ISO 8601 UTC format: {}\nUser time zone: {}, UTC{}",
            now.to_rfc3339(),
            offset.timezone().name(),
            offset
        )
    }
    
    fn format_reminder_section(todos: &[TodoItem]) -> String {
        if todos.is_empty() {
            return "You have not created a todo list yet. Create one with `update_todo_list` if your task is complicated or involves multiple steps.".to_string();
        }
        
        let mut lines = vec![
            "====".to_string(),
            "".to_string(),
            "REMINDERS".to_string(),
            "".to_string(),
            "Below is your current list of reminders for this task. Keep them updated as you progress.".to_string(),
            "".to_string(),
            "| # | Content | Status |".to_string(),
            "|---|---------|--------|".to_string(),
        ];
        
        for (idx, item) in todos.iter().enumerate() {
            let escaped = item.content.replace('\\', "\\\\").replace('|', "\\|");
            let status = match item.status {
                TodoStatus::Pending => "Pending",
                TodoStatus::InProgress => "In Progress",
                TodoStatus::Completed => "Completed",
            };
            lines.push(format!("| {} | {} | {} |", idx + 1, escaped, status));
        }
        
        lines.push("".to_string());
        lines.push("IMPORTANT: When task status changes, remember to call the `update_todo_list` tool to update your progress.".to_string());
        
        lines.join("\n")
    }
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] Environment details flow documented
- [x] All 9 sections explained
- [x] Terminal state tracking detailed
- [x] File modification detection covered
- [x] Time/cost/mode formatting shown
- [x] Todo list reminder system analyzed
- [x] Token budget breakdown provided
- [x] Rust translation patterns defined

**STATUS**: CHUNK-12 COMPLETE (deep environment context analysis)
