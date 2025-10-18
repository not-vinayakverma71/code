# CHUNK 04: src/core/webview/ - CLINE PROVIDER (2831 LINES!)

## Overview
ClineProvider is the **VS Code Webview Provider** that manages:
- UI lifecycle (sidebar/panel)
- Task stack management
- Message routing (webview ↔ extension)
- State synchronization
- Global settings
- MCP server management
- Code index coordination

## Critical Statistics
- **2,831 lines** in ClineProvider.ts
- **3,946 lines** in webviewMessageHandler.ts
- **80+ message types** from webview
- **Task stack architecture** for nested tasks

## Architecture: VS Code Webview Provider

```typescript
export class ClineProvider 
    extends EventEmitter<TaskProviderEvents>
    implements vscode.WebviewViewProvider, TelemetryPropertiesProvider, TaskProviderLike {
    
    // Webview instances
    private view?: vscode.WebviewView | vscode.WebviewPanel
    
    // Task management
    private clineStack: Task[] = []
    
    // Services
    protected mcpHub?: McpHub
    private marketplaceManager: MarketplaceManager
    private mdmService?: MdmService
    public readonly providerSettingsManager: ProviderSettingsManager
    public readonly customModesManager: CustomModesManager
    private currentWorkspaceManager?: CodeIndexManager
    private _workspaceTracker?: WorkspaceTracker
    
    // State
    public isViewLaunched = false
    private recentTasksCache?: string[]
}
```

**CRITICAL VS CODE DEPENDENCY:** This entire class is VS Code-specific. Must be completely redesigned for Lapce.

## Task Stack Architecture

**Nested Task Support:**
```typescript
private clineStack: Task[] = []

// Push new subtask
async startNewTask(task: string, images?: string[]) {
    const newTask = new Task({
        context: this.context,
        provider: this,
        task,
        images,
        parentTask: this.getCurrentTask(),
        rootTask: this.clineStack[0] || undefined,
    })
    
    this.clineStack.push(newTask)
    
    // Pause parent task
    const parentTask = this.clineStack[this.clineStack.length - 2]
    if (parentTask) {
        parentTask.isPaused = true
        parentTask.emit(RooCodeEventName.TaskPaused)
    }
}

// Pop finished subtask
async finishSubTask(lastMessage: string) {
    await this.removeClineFromStack()
    
    // Resume parent
    await this.getCurrentTask()?.resumePausedTask(lastMessage)
}

getCurrentTask(): Task | undefined {
    return this.clineStack[this.clineStack.length - 1]
}
```

**RUST TRANSLATION:**
```rust
pub struct TaskProvider {
    task_stack: Arc<RwLock<Vec<Arc<RwLock<Task>>>>>,
    current_workspace_manager: Option<Arc<CodeIndexManager>>,
    mcp_hub: Option<Arc<McpHub>>,
    settings_manager: Arc<ProviderSettingsManager>,
    modes_manager: Arc<CustomModesManager>,
}

impl TaskProvider {
    pub async fn start_new_task(&self, task: String, images: Option<Vec<String>>) -> Result<(), Error> {
        let parent_task = self.get_current_task().await?;
        
        let new_task = Task::new(TaskOptions {
            task,
            images,
            parent_task: parent_task.clone(),
            root_task: self.get_root_task().await?,
            ..Default::default()
        })?;
        
        // Pause parent
        if let Some(parent) = parent_task {
            parent.write().await.is_paused = true;
            parent.write().await.emit(TaskEvent::TaskPaused);
        }
        
        self.task_stack.write().await.push(Arc::new(RwLock::new(new_task)));
        
        Ok(())
    }
    
    pub async fn get_current_task(&self) -> Option<Arc<RwLock<Task>>> {
        let stack = self.task_stack.read().await;
        stack.last().cloned()
    }
}
```

## Message Routing: webviewMessageHandler.ts (3946 lines!)

This file handles **ALL** communication from webview to extension.

**Message Types:**
```typescript
export type WebviewMessage =
    | { type: "webviewDidLaunch" }
    | { type: "newTask"; text?: string; images?: string[] }
    | { type: "apiConfiguration"; apiConfiguration: ProviderSettings }
    | { type: "askResponse"; askResponse: ClineAskResponse; text?: string; images?: string[] }
    | { type: "clearTask" }
    | { type: "didShowAnnouncement" }
    | { type: "selectImages" }
    | { type: "exportCurrentTask" }
    | { type: "showTaskWithId"; text: string }
    | { type: "deleteTaskWithId"; text: string }
    | { type: "exportTaskWithId"; text: string }
    | { type: "resetState" }
    | { type: "requestOllamaModels"; apiConfiguration: ProviderSettings }
    | { type: "openImage"; text: string }
    | { type: "openFile"; text: string }
    | { type: "openMention"; text: string }
    | { type: "toggleExperiment"; experimentId: string }
    | { type: "cancelTask" }
    | { type: "editMessage"; messageTs: number; text: string }
    | { type: "deleteMessage"; messageTs: number }
    // ... 60+ more message types
```

**Handler Pattern:**
```typescript
export const webviewMessageHandler = async (
    provider: ClineProvider,
    message: WebviewMessage,
    marketplaceManager?: MarketplaceManager
) => {
    switch (message.type) {
        case "newTask":
            await provider.startNewTask(message.text, message.images)
            break
            
        case "askResponse":
            const currentTask = provider.getCurrentTask()
            if (currentTask) {
                currentTask.askResponse = message.askResponse
                currentTask.askResponseText = message.text
                currentTask.askResponseImages = message.images
            }
            break
            
        case "apiConfiguration":
            await provider.updateGlobalState("apiConfiguration", message.apiConfiguration)
            await provider.postStateToWebview()
            break
            
        case "clearTask":
            await provider.clearTask()
            break
            
        // ... 60+ cases
    }
}
```

**RUST TRANSLATION:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    WebviewDidLaunch,
    NewTask { text: Option<String>, images: Option<Vec<String>> },
    ApiConfiguration { api_configuration: ProviderSettings },
    AskResponse { 
        ask_response: ClineAskResponse, 
        text: Option<String>, 
        images: Option<Vec<String>> 
    },
    ClearTask,
    SelectImages,
    ExportCurrentTask,
    ShowTaskWithId { text: String },
    DeleteTaskWithId { text: String },
    CancelTask,
    EditMessage { message_ts: u64, text: String },
    DeleteMessage { message_ts: u64 },
    // ... 60+ variants
}

pub async fn handle_webview_message(
    provider: &TaskProvider,
    message: WebviewMessage,
) -> Result<(), Error> {
    match message {
        WebviewMessage::NewTask { text, images } => {
            provider.start_new_task(text.unwrap_or_default(), images).await?;
        }
        
        WebviewMessage::AskResponse { ask_response, text, images } => {
            if let Some(task) = provider.get_current_task().await {
                let mut task = task.write().await;
                task.ask_response = Some(ask_response);
                task.ask_response_text = text;
                task.ask_response_images = images;
            }
        }
        
        WebviewMessage::ClearTask => {
            provider.clear_task().await?;
        }
        
        // ... 60+ match arms
        _ => {}
    }
    
    Ok(())
}
```

## State Synchronization: postStateToWebview()

The provider constantly syncs state to the webview:

```typescript
async postStateToWebview() {
    const state = await this.getState()
    await this.postMessageToWebview({ type: "state", state })
}

async getState(): Promise<GlobalState> {
    const currentTask = this.getCurrentTask()
    
    return {
        version: this.context.extension.packageJSON.version,
        apiConfiguration: this.getGlobalState("apiConfiguration"),
        customInstructions: this.getGlobalState("customInstructions"),
        clineMessages: currentTask?.clineMessages ?? [],
        taskHistory: this.getGlobalState("taskHistory") ?? [],
        shouldShowAnnouncement: this.shouldShowAnnouncement(),
        mode: this.getGlobalState("mode") || defaultModeSlug,
        customModes: await this.customModesManager.getCustomModes(),
        mcpServers: this.mcpHub?.getServers() ?? [],
        experiments: this.getGlobalState("experiments") ?? experimentDefault,
        // ... 30+ more fields
    }
}
```

**RUST:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub version: String,
    pub api_configuration: Option<ProviderSettings>,
    pub custom_instructions: Option<String>,
    pub cline_messages: Vec<ClineMessage>,
    pub task_history: Vec<HistoryItem>,
    pub should_show_announcement: bool,
    pub mode: String,
    pub custom_modes: Vec<ModeConfig>,
    pub mcp_servers: Vec<McpServerInfo>,
    pub experiments: HashMap<String, bool>,
    // ... 30+ fields
}

pub async fn post_state_to_webview(&self) -> Result<(), Error> {
    let state = self.get_state().await?;
    self.post_message_to_webview(ExtensionMessage::State { state }).await?;
    Ok(())
}
```

## Singleton Pattern for Global Access

```typescript
private static activeInstances: Set<ClineProvider> = new Set()

constructor() {
    ClineProvider.activeInstances.add(this)
}

public static getVisibleInstance(): ClineProvider | undefined {
    return findLast(
        Array.from(this.activeInstances), 
        (instance) => instance.view?.visible === true
    )
}

public static async getInstance(): Promise<ClineProvider | undefined> {
    let visibleProvider = ClineProvider.getVisibleInstance()
    
    if (!visibleProvider) {
        await vscode.commands.executeCommand(`${Package.name}.SidebarProvider.focus`)
        await delay(100)
        visibleProvider = ClineProvider.getVisibleInstance()
    }
    
    return visibleProvider
}
```

## CRITICAL: VS Code → Lapce Translation

### Webview Architecture
**VS Code:**
- `vscode.WebviewView` - Sidebar panel
- `vscode.WebviewPanel` - Editor tab
- HTML/CSS/JS in webview
- `postMessage()` API for communication

**Lapce:**
- **DIFFERENT UI SYSTEM** - Floem-based native UI
- **NO HTML WEBVIEWS** - Pure Rust UI
- Must use Lapce plugin API for UI

**TRANSLATION STRATEGY:**
```rust
// Instead of webview, use Lapce UI primitives
use lapce_plugin_api::{
    ui::{Container, Text, Button, List},
    Context,
};

pub struct LapceTaskUI {
    context: Context,
    state: Arc<RwLock<GlobalState>>,
}

impl LapceTaskUI {
    pub fn render(&self) -> Container {
        Container::column()
            .child(self.render_task_input())
            .child(self.render_messages())
            .child(self.render_controls())
    }
    
    fn render_messages(&self) -> List {
        let messages = self.state.read().unwrap().cline_messages.clone();
        List::new(messages, |msg| {
            Text::new(msg.text.clone())
        })
    }
}
```

### Event Bus Replacement

**VS Code:** Webview messaging
**Lapce:** Direct function calls via plugin API

```rust
// No postMessage(), direct state updates
pub async fn update_ui_state(&self, state: GlobalState) {
    self.context.update_ui(serde_json::to_value(state)?);
}
```

## Code Index Integration

```typescript
private codeIndexStatusSubscription?: vscode.Disposable

async initializeCodeIndex() {
    const manager = CodeIndexManager.getInstance(this.context, this.cwd)
    
    if (manager.isFeatureEnabled) {
        this.codeIndexStatusSubscription = manager.subscribeToProgress(
            (update: IndexProgressUpdate) => {
                this.postMessageToWebview({
                    type: "indexProgress",
                    progress: update
                })
            }
        )
    }
}
```

## MCP Server Lifecycle

```typescript
McpServerManager.getInstance(this.context, this)
    .then((hub) => {
        this.mcpHub = hub
        this.mcpHub.registerClient()
    })
    .catch((error) => {
        this.log(`Failed to initialize MCP Hub: ${error}`)
    })
```

## Dispose Pattern

```typescript
async dispose() {
    // Clear all tasks
    while (this.clineStack.length > 0) {
        await this.removeClineFromStack()
    }
    
    // Dispose webview
    if (this.view && "dispose" in this.view) {
        this.view.dispose()
    }
    
    // Clear resources
    this.clearWebviewResources()
    
    // Cleanup services
    this._workspaceTracker?.dispose()
    await this.mcpHub?.unregisterClient()
    this.marketplaceManager?.cleanup()
    this.customModesManager?.dispose()
    
    // Remove from active instances
    ClineProvider.activeInstances.delete(this)
    
    // Clean up event listeners
    this.removeAllListeners()
}
```

**RUST:**
```rust
impl Drop for TaskProvider {
    fn drop(&mut self) {
        // Cleanup happens automatically via RAII
        // But can add explicit cleanup if needed
    }
}
```

## Next: CHUNK 05 - API Providers (145 files!)
40+ LLM provider implementations with streaming support.
