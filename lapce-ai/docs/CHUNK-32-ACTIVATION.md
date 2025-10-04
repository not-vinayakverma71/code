# CHUNK 12: src/activate/ & extension.ts - LIFECYCLE (10 FILES)

## Overview
The activation directory handles VS Code extension lifecycle:
- **registerCommands.ts** (341 lines) - Register 50+ commands
- **handleUri.ts** (73 lines) - OAuth callbacks, deep links
- **handleTask.ts** - Task creation from context
- **registerCodeActions.ts** - Code action provider
- **registerTerminalActions.ts** - Terminal integration
- **extension.ts** - Main entry point

## registerCommands.ts (341 LINES)

**Purpose:** Register ALL extension commands with VS Code

### Command Structure
```typescript
export type RegisterCommandOptions = {
    context: vscode.ExtensionContext
    outputChannel: vscode.OutputChannel
    provider: ClineProvider
}

export const registerCommands = (options: RegisterCommandOptions) => {
    const { context } = options
    
    for (const [id, callback] of Object.entries(getCommandsMap(options))) {
        const command = getCommand(id as CommandId)
        context.subscriptions.push(
            vscode.commands.registerCommand(command, callback)
        )
    }
}
```

### All Commands (50+)
```typescript
const getCommandsMap = ({ context, outputChannel }: RegisterCommandOptions) => ({
    // UI Commands
    plusButtonClicked: async () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        await provider.removeClineFromStack()
        await provider.postStateToWebview()
        await provider.postMessageToWebview({ 
            type: "action", 
            action: "chatButtonClicked" 
        })
    },
    
    mcpButtonClicked: () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        provider.postMessageToWebview({ 
            type: "action", 
            action: "mcpButtonClicked" 
        })
    },
    
    settingsButtonClicked: () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        provider.postMessageToWebview({ 
            type: "action", 
            action: "settingsButtonClicked" 
        })
    },
    
    historyButtonClicked: () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        provider.postMessageToWebview({ 
            type: "action", 
            action: "historyButtonClicked" 
        })
    },
    
    promptsButtonClicked: () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        provider.postMessageToWebview({ 
            type: "action", 
            action: "promptsButtonClicked" 
        })
    },
    
    // Task Commands
    openInNewTab: () => openClineInNewTab({ context, outputChannel }),
    
    popoutButtonClicked: () => openClineInNewTab({ context, outputChannel }),
    
    // File Operations
    executeCommandAndChat: async (command: string) => {
        const provider = getVisibleProviderOrLog(outputChannel)
        await handleNewTask(provider, { command })
    },
    
    // Settings
    exportSettings: async () => {
        const provider = getVisibleProviderOrLog(outputChannel)
        const settings = await exportSettings(context)
        // ... export logic
    },
    
    importSettings: async () => {
        await importSettingsWithFeedback(context, outputChannel)
    },
    
    // Code Index
    triggerCodebaseIndexing: async () => {
        const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
        const indexManager = CodeIndexManager.getInstance(context, workspacePath)
        await indexManager?.startIndexing()
    },
    
    // MDM (Mobile Device Management)
    getMdmStatus: async () => {
        return MdmService.instance.getStatus()
    },
    
    // Human Relay
    registerHumanRelayCallback: () => {
        registerHumanRelayCallback(context, outputChannel)
    },
    
    // ... 30+ more commands
})
```

**RUST TRANSLATION:**
```rust
use lapce_plugin::{register_plugin, VoltEnvironment, PLUGIN_RPC};

pub struct LapceAiPlugin {
    provider: Option<Arc<Mutex<Provider>>>,
}

impl LapcePlugin for LapceAiPlugin {
    fn handle_request(&mut self, _id: u64, method: String, params: Value) {
        match method.as_str() {
            // UI Commands
            "plus_button_clicked" => {
                if let Some(provider) = &self.provider {
                    let provider = provider.lock().unwrap();
                    provider.remove_task_from_stack();
                    provider.post_state_to_webview();
                    provider.post_message_to_webview(json!({
                        "type": "action",
                        "action": "chatButtonClicked"
                    }));
                }
            }
            
            "settings_button_clicked" => {
                if let Some(provider) = &self.provider {
                    let provider = provider.lock().unwrap();
                    provider.post_message_to_webview(json!({
                        "type": "action",
                        "action": "settingsButtonClicked"
                    }));
                }
            }
            
            // Task Commands
            "execute_command_and_chat" => {
                let command = params["command"].as_str().unwrap();
                if let Some(provider) = &self.provider {
                    let provider = provider.lock().unwrap();
                    provider.handle_new_task(TaskInput {
                        command: Some(command.to_string()),
                        ..Default::default()
                    });
                }
            }
            
            // Settings
            "export_settings" => {
                let settings = self.export_settings();
                PLUGIN_RPC.host_notification(
                    "save_file",
                    json!({
                        "path": "settings.json",
                        "content": serde_json::to_string(&settings).unwrap()
                    })
                );
            }
            
            "import_settings" => {
                // Request file from host
                PLUGIN_RPC.host_request(
                    Request::new("open_file_dialog")
                );
            }
            
            _ => {
                eprintln!("Unknown command: {}", method);
            }
        }
    }
}
```

## handleUri.ts (73 LINES)

**Purpose:** Handle OAuth callbacks and deep links

```typescript
export const handleUri = async (uri: vscode.Uri) => {
    const path = uri.path
    const query = new URLSearchParams(uri.query.replace(/\+/g, "%2B"))
    const visibleProvider = ClineProvider.getVisibleInstance()
    
    if (!visibleProvider) {
        return
    }
    
    switch (path) {
        case "/glama": {
            const code = query.get("code")
            if (code) {
                await visibleProvider.handleGlamaCallback(code)
            }
            break
        }
        
        case "/openrouter": {
            const code = query.get("code")
            if (code) {
                await visibleProvider.handleOpenRouterCallback(code)
            }
            break
        }
        
        case "/kilocode": {
            const token = query.get("token")
            if (token) {
                await visibleProvider.handleKiloCodeCallback(token)
            }
            break
        }
        
        case "/auth/clerk/callback": {
            const code = query.get("code")
            const state = query.get("state")
            const organizationId = query.get("organizationId")
            
            await CloudService.instance.handleAuthCallback(
                code,
                state,
                organizationId === "null" ? null : organizationId
            )
            break
        }
        
        default:
            break
    }
}
```

**RUST TRANSLATION:**
```rust
pub async fn handle_uri(uri: &str, provider: &Provider) -> Result<()> {
    let parsed_uri = url::Url::parse(uri)?;
    let path = parsed_uri.path();
    let query: HashMap<_, _> = parsed_uri.query_pairs().collect();
    
    match path {
        "/glama" => {
            if let Some(code) = query.get("code") {
                provider.handle_glama_callback(code.to_string()).await?;
            }
        }
        
        "/openrouter" => {
            if let Some(code) = query.get("code") {
                provider.handle_openrouter_callback(code.to_string()).await?;
            }
        }
        
        "/kilocode" => {
            if let Some(token) = query.get("token") {
                provider.handle_kilocode_callback(token.to_string()).await?;
            }
        }
        
        "/auth/clerk/callback" => {
            let code = query.get("code").map(|s| s.to_string());
            let state = query.get("state").map(|s| s.to_string());
            let org_id = query.get("organizationId")
                .filter(|&s| s != "null")
                .map(|s| s.to_string());
            
            CloudService::instance()
                .handle_auth_callback(code, state, org_id)
                .await?;
        }
        
        _ => {}
    }
    
    Ok(())
}
```

## extension.ts - MAIN ENTRY POINT

**Purpose:** Extension activation and initialization

```typescript
export async function activate(context: vscode.ExtensionContext) {
    // Initialize telemetry
    TelemetryService.initialize(context)
    
    // Create output channel
    const outputChannel = vscode.window.createOutputChannel("Kilo Code")
    
    // Initialize MDM service
    MdmService.initialize(context)
    
    // Register URI handler
    context.subscriptions.push(
        vscode.window.registerUriHandler({
            handleUri: (uri) => handleUri(uri)
        })
    )
    
    // Create webview provider
    const provider = new ClineProvider(context, outputChannel)
    
    // Register webview
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            ClineProvider.sidebarId,
            provider,
            {
                webviewOptions: {
                    retainContextWhenHidden: true
                }
            }
        )
    )
    
    // Register commands
    registerCommands({ context, outputChannel, provider })
    
    // Register code actions
    registerCodeActions(context, provider)
    
    // Register terminal actions
    registerTerminalActions(context)
    
    // Initialize code index manager
    const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
    if (workspacePath) {
        const indexManager = CodeIndexManager.getInstance(context, workspacePath)
        await indexManager?.initialize()
    }
    
    // Auto-launch task if requested
    const autoLaunchTask = context.globalState.get("autoLaunchTask")
    if (autoLaunchTask) {
        context.globalState.update("autoLaunchTask", undefined)
        await handleNewTask(provider, autoLaunchTask)
    }
    
    // Mark activation complete
    await vscode.commands.executeCommand(
        "setContext",
        "kilocode.activationCompleted",
        true
    )
}

export function deactivate() {
    // Clean up code index
    CodeIndexManager.disposeAll()
    
    // Clean up telemetry
    TelemetryService.instance.dispose()
}
```

**RUST TRANSLATION:**
```rust
use lapce_plugin::*;

#[derive(Default)]
struct LapceAiPlugin {
    provider: Option<Arc<Mutex<Provider>>>,
    output_channel: Option<OutputChannel>,
}

impl LapcePlugin for LapceAiPlugin {
    fn initialize(&mut self, _env: VoltEnvironment) {
        // Initialize telemetry
        TelemetryService::initialize();
        
        // Create provider
        let provider = Arc::new(Mutex::new(Provider::new()));
        self.provider = Some(provider.clone());
        
        // Start background services
        tokio::spawn(async move {
            // Initialize code index manager
            let workspace_path = std::env::var("LAPCE_WORKSPACE").ok();
            if let Some(path) = workspace_path {
                let index_manager = CodeIndexManager::get_instance(&path);
                index_manager.initialize().await.ok();
            }
        });
        
        // Mark activation complete
        PLUGIN_RPC.host_notification(
            "plugin_activated",
            json!({ "name": "lapce-ai" })
        );
    }
    
    fn handle_request(&mut self, id: u64, method: String, params: Value) {
        // Handle commands (as shown above)
        match method.as_str() {
            // ... command handlers
            _ => {}
        }
    }
}

#[no_mangle]
pub fn lapce_plugin_init(env: VoltEnvironment) -> Box<dyn LapcePlugin> {
    Box::new(LapceAiPlugin::default())
}
```

## Code Actions

```typescript
export function registerCodeActions(
    context: vscode.ExtensionContext,
    provider: ClineProvider
) {
    const codeActionProvider = new CodeActionProvider(provider)
    
    context.subscriptions.push(
        vscode.languages.registerCodeActionsProvider(
            { scheme: "file" },
            codeActionProvider,
            {
                providedCodeActionKinds: [
                    vscode.CodeActionKind.QuickFix,
                    vscode.CodeActionKind.Refactor
                ]
            }
        )
    )
}

export class CodeActionProvider implements vscode.CodeActionProvider {
    constructor(private provider: ClineProvider) {}
    
    provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range,
        context: vscode.CodeActionContext
    ): vscode.CodeAction[] {
        const actions: vscode.CodeAction[] = []
        
        // "Ask Kilo Code to fix" action for diagnostics
        for (const diagnostic of context.diagnostics) {
            const action = new vscode.CodeAction(
                "Ask Kilo Code to fix this",
                vscode.CodeActionKind.QuickFix
            )
            action.command = {
                command: "kilocode.askToFix",
                title: "Ask Kilo Code",
                arguments: [document, diagnostic]
            }
            actions.push(action)
        }
        
        // "Refactor with Kilo Code" action
        if (!range.isEmpty) {
            const action = new vscode.CodeAction(
                "Refactor with Kilo Code",
                vscode.CodeActionKind.Refactor
            )
            action.command = {
                command: "kilocode.refactor",
                title: "Refactor",
                arguments: [document, range]
            }
            actions.push(action)
        }
        
        return actions
    }
}
```

**RUST:** Lapce doesn't have code actions API yet - defer to future.

## Summary: Activation Flow

```
extension.ts (activate)
    ├─ Initialize telemetry
    ├─ Create output channel
    ├─ Register URI handler (handleUri.ts)
    ├─ Create ClineProvider (webview)
    ├─ Register webview provider
    ├─ Register commands (registerCommands.ts)
    │   └─ 50+ command handlers
    ├─ Register code actions
    ├─ Register terminal actions
    ├─ Initialize code index
    └─ Auto-launch task (if requested)
```

## Rust Equivalent Architecture

```
main.rs (plugin init)
    ├─ Initialize telemetry
    ├─ Create Provider (backend service)
    ├─ Register command handlers
    │   └─ 50+ commands via RPC
    ├─ Start HTTP server (for web UI)
    ├─ Initialize code index
    └─ Listen for Lapce RPC messages
```

**Key Differences:**
- No webview → Separate web UI with HTTP server
- No VS Code commands → Lapce plugin RPC
- No URI handler → HTTP endpoints for OAuth
- No code actions → Future feature

## Translation Priority

| Component | Lines | Priority | Strategy |
|-----------|-------|----------|----------|
| Commands | 341 | High | Map to Lapce RPC |
| URI Handler | 73 | Medium | HTTP endpoints |
| Extension Entry | ~200 | High | Plugin init |
| Code Actions | ~100 | Low | Future feature |

## Next: CHUNK 13 - Complete File Count & Statistics
Final overview, dependency graph, complete task breakdown.
