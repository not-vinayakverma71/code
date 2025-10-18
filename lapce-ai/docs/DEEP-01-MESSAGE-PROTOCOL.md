# DEEP ANALYSIS 01: MESSAGE PROTOCOL - COMPLETE SPECIFICATION **IT WILL BE IMPLEMENTED IN PHASE C**

## 📁 Analyzed Files

```
Codex/src/shared/
├── WebviewMessage.ts (436 lines, 267 types)
└── ExtensionMessage.ts (502 lines, 139 types)
│   └── Kilocode Extensions          (177 types: modes, rules, profiles, etc.)
│
└── ExtensionMessage.ts               (502 lines, 139 message types)
    ├── State Updates                 (1 type: state with 179 fields)
    ├── Model/Provider Updates        (18 types: modelInfo, routerModels, etc.)
    ├── Task/History Updates          (8 types: taskHistory, currentTask, etc.)
    ├── Permission Requests           (6 types: approval prompts)
    ├── MCP Messages                  (12 types: server status, tools, etc.)
    └── UI Notifications              (94 types: errors, warnings, info)

Total: 406 message types mapped to Rust enums
```

---

## Overview
Total message types: **267 WebviewMessage types** + **139 ExtensionMessage types** = **406 total message types**

---

## WEBVIEW → BACKEND (267 Message Types)

### Core Task Management (10 types)

#### newTask
```typescript
// Frontend sends
vscode.postMessage({ 
  type: "newTask", 
  text: "Build a todo app",
  images: ["data:image/png;base64,..."]
})
```
```rust
// Rust backend receives
#[derive(Deserialize)]
struct NewTaskMessage {
    text: String,
    images: Option<Vec<String>>, // Base64 data URLs
}

// Handler
async fn handle_new_task(msg: NewTaskMessage, state: Arc<RwLock<AppState>>) {
    let task_id = Uuid::new_v4();
    let task = Task {
        id: task_id,
        ts: SystemTime::now(),
        prompt: msg.text,
        images: msg.images.unwrap_or_default(),
        messages: vec![],
    };
    
    state.write().await.current_task = Some(task);
    state.write().await.save_to_db().await?;
    
    // Start AI processing
    spawn_task_processor(task_id, state).await;
}
```

#### askResponse
```typescript
// User approves/rejects AI action
vscode.postMessage({
  type: "askResponse",
  askResponse: "yesButtonClicked" | "noButtonClicked" | "messageResponse",
  text?: "optional feedback",
  images?: ["data:..."]
})
```
```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum AskResponse {
    YesButtonClicked,
    NoButtonClicked,
    MessageResponse,
    RetryClicked,
}

#[derive(Deserialize)]
struct AskResponseMessage {
    ask_response: AskResponse,
    text: Option<String>,
    images: Option<Vec<String>>,
}

async fn handle_ask_response(msg: AskResponseMessage, state: Arc<RwLock<AppState>>) {
    let task = state.read().await.current_task.as_ref().unwrap();
    
    match msg.ask_response {
        AskResponse::YesButtonClicked => {
            // Execute pending action
            execute_pending_action(task.id, msg.text, msg.images).await?;
        }
        AskResponse::NoButtonClicked => {
            // Reject action, ask AI to try different approach
            send_rejection_to_ai(task.id, msg.text).await?;
        }
        AskResponse::MessageResponse => {
            // User provided custom feedback
            send_user_feedback_to_ai(task.id, msg.text.unwrap()).await?;
        }
        AskResponse::RetryClicked => {
            // Retry failed API request
            retry_last_request(task.id).await?;
        }
    }
}
```

#### clearTask
```typescript
vscode.postMessage({ type: "clearTask" })
```
```rust
async fn handle_clear_task(state: Arc<RwLock<AppState>>) {
    let mut state = state.write().await;
    
    // Save current task to history
    if let Some(task) = state.current_task.take() {
        state.task_history.push(HistoryItem {
            id: task.id,
            ts: task.ts,
            task: task.prompt,
            messages: task.messages,
            cost: calculate_cost(&task),
        });
    }
    
    state.current_task = None;
    state.save_to_db().await?;
    
    // Broadcast to all connected clients
    broadcast_state_update(&state).await;
}
```

#### cancelTask
```typescript
vscode.postMessage({ type: "cancelTask" })
```
```rust
async fn handle_cancel_task(state: Arc<RwLock<AppState>>) {
    let task_id = state.read().await.current_task.as_ref().map(|t| t.id);
    
    if let Some(id) = task_id {
        // Signal cancellation to task processor
        TASK_CANCELLATION_TOKENS.write().await
            .get(&id)
            .map(|token| token.cancel());
        
        // Update task state
        state.write().await.current_task.as_mut().unwrap()
            .status = TaskStatus::Cancelled;
    }
}
```

#### terminalOperation
```typescript
vscode.postMessage({
  type: "terminalOperation",
  terminalOperation: "continue" | "abort"
})
```
```rust
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TerminalOperation {
    Continue,
    Abort,
}

#[derive(Deserialize)]
struct TerminalOperationMessage {
    terminal_operation: TerminalOperation,
}

async fn handle_terminal_operation(msg: TerminalOperationMessage) {
    match msg.terminal_operation {
        TerminalOperation::Continue => {
            // Let command continue running in background
            RUNNING_COMMANDS.write().await
                .get_mut(&current_command_id)
                .map(|cmd| cmd.detach = true);
        }
        TerminalOperation::Abort => {
            // Kill running command
            if let Some(process) = RUNNING_COMMANDS.write().await.remove(&current_command_id) {
                process.kill().await?;
            }
        }
    }
}
```

---

### Settings Messages (120 types)

#### API Configuration (15 types)

**upsertApiConfiguration**
```typescript
vscode.postMessage({
  type: "upsertApiConfiguration",
  text: "my-profile-name",
  apiConfiguration: {
    apiProvider: "anthropic",
    anthropicApiKey: "sk-ant-...",
    apiModelId: "claude-3-5-sonnet-20241022",
    temperature: 0.7,
    // ... 50+ more fields
  }
})
```
```rust
#[derive(Serialize, Deserialize, Clone)]
struct ProviderSettings {
    // Provider selection
    api_provider: Option<String>, // "anthropic" | "openai" | etc (40+ providers)
    
    // API Keys (40+ providers)
    anthropic_api_key: Option<String>,
    openai_api_key: Option<String>,
    open_router_api_key: Option<String>,
    gemini_api_key: Option<String>,
    deepseek_api_key: Option<String>,
    // ... 35+ more API keys
    
    // Model selection
    api_model_id: Option<String>,
    
    // Parameters
    temperature: Option<f32>,
    thinking_budget: Option<i32>,
    verbosity: Option<i32>,
    max_tokens: Option<i32>,
    
    // OpenAI Native specific
    openai_native_api_key: Option<String>,
    openai_stream_options: Option<bool>,
    
    // OpenRouter specific
    open_router_use_middle_out_transform: Option<bool>,
    open_router_model_preferences: Option<Vec<String>>,
    
    // Vertex specific
    vertex_project_id: Option<String>,
    vertex_region: Option<String>,
    
    // Bedrock specific
    aws_access_key_id: Option<String>,
    aws_secret_access_key: Option<String>,
    aws_session_token: Option<String>,
    aws_region: Option<String>,
    aws_profile: Option<String>,
    aws_use_cross_region_inference: Option<bool>,
    aws_use_prompt_cache: Option<bool>,
    bedrock_custom_model_arn: Option<String>,
    
    // OpenAI Compatible
    openai_base_url: Option<String>,
    openai_model_id: Option<String>,
    openai_headers: Option<HashMap<String, String>>,
    
    // Advanced
    consecutive_mistake_limit: Option<i32>,
    rate_limit_seconds: Option<i32>,
}

async fn handle_upsert_api_config(
    config_name: String,
    config: ProviderSettings,
    state: Arc<RwLock<AppState>>
) {
    let mut state = state.write().await;
    
    // Save to profiles map
    state.api_profiles.insert(config_name.clone(), config.clone());
    state.current_api_config_name = config_name.clone();
    state.api_configuration = Some(config);
    
    // Persist to database
    state.save_to_db().await?;
    
    // Broadcast update
    broadcast_setting_update("apiConfiguration", &state.api_configuration).await;
}
```

**currentApiConfigName**
```typescript
vscode.postMessage({
  type: "currentApiConfigName",
  text: "profile-name"
})
```
```rust
async fn handle_current_api_config_name(name: String, state: Arc<RwLock<AppState>>) {
    let mut state = state.write().await;
    
    // Switch to different profile
    if let Some(config) = state.api_profiles.get(&name) {
        state.current_api_config_name = name;
        state.api_configuration = Some(config.clone());
        state.save_to_db().await?;
        
        broadcast_state_update(&state).await;
    }
}
```

#### Permission Toggles (12 types)

```rust
// Each permission maps to identical handler pattern
#[derive(Serialize, Deserialize)]
struct PermissionToggleMessage {
    bool: bool,
}

macro_rules! impl_permission_handler {
    ($fn_name:ident, $field:ident) => {
        async fn $fn_name(value: bool, state: Arc<RwLock<AppState>>) {
            state.write().await.$field = value;
            state.write().await.save_to_db().await?;
            broadcast_setting_update(stringify!($field), &value).await;
        }
    };
}

impl_permission_handler!(handle_always_allow_read_only, always_allow_read_only);
impl_permission_handler!(handle_always_allow_write, always_allow_write);
impl_permission_handler!(handle_always_allow_execute, always_allow_execute);
impl_permission_handler!(handle_always_allow_browser, always_allow_browser);
impl_permission_handler!(handle_always_allow_mcp, always_allow_mcp);
impl_permission_handler!(handle_always_allow_mode_switch, always_allow_mode_switch);
impl_permission_handler!(handle_always_allow_subtasks, always_allow_subtasks);
impl_permission_handler!(handle_always_allow_followup_questions, always_allow_followup_questions);
impl_permission_handler!(handle_always_allow_update_todo_list, always_allow_update_todo_list);
```

#### Command Filtering (2 types)

```typescript
vscode.postMessage({
  type: "allowedCommands",
  commands: ["npm install", "git status", "ls -la"]
})

vscode.postMessage({
  type: "deniedCommands",
  commands: ["rm -rf", "sudo", "chmod 777"]
})
```
```rust
#[derive(Deserialize)]
struct CommandsMessage {
    commands: Vec<String>,
}

async fn handle_allowed_commands(cmds: Vec<String>, state: Arc<RwLock<AppState>>) {
    state.write().await.allowed_commands = cmds.clone();
    state.write().await.save_to_db().await?;
    broadcast_setting_update("allowedCommands", &cmds).await;
}

async fn handle_denied_commands(cmds: Vec<String>, state: Arc<RwLock<AppState>>) {
    state.write().await.denied_commands = cmds.clone();
    state.write().await.save_to_db().await?;
    broadcast_setting_update("deniedCommands", &cmds).await;
}

// Command validation logic
fn is_command_allowed(command: &str, settings: &Settings) -> CommandDecision {
    // Check denied list first (takes precedence)
    for denied in &settings.denied_commands {
        if command_matches_pattern(command, denied) {
            return CommandDecision::AutoDeny;
        }
    }
    
    // Check allowed list
    for allowed in &settings.allowed_commands {
        if command_matches_pattern(command, allowed) {
            return CommandDecision::AutoApprove;
        }
    }
    
    // Default: ask user
    CommandDecision::AskUser
}

enum CommandDecision {
    AutoApprove,
    AutoDeny,
    AskUser,
}
```

#### Terminal Settings (15 types)

```typescript
vscode.postMessage({ type: "terminalOutputLineLimit", value: 500 })
vscode.postMessage({ type: "terminalOutputCharacterLimit", value: 50000 })
vscode.postMessage({ type: "terminalShellIntegrationTimeout", value: 4000 })
vscode.postMessage({ type: "terminalShellIntegrationDisabled", bool: false })
vscode.postMessage({ type: "terminalCommandDelay", value: 100 })
vscode.postMessage({ type: "terminalPowershellCounter", bool: true })
vscode.postMessage({ type: "terminalZshOhMy", bool: true })
vscode.postMessage({ type: "terminalZshP10k", bool: true })
vscode.postMessage({ type: "terminalZdotdir", bool: false })
vscode.postMessage({ type: "terminalCompressProgressBar", bool: true })
```
```rust
#[derive(Serialize, Deserialize, Clone)]
struct TerminalSettings {
    output_line_limit: i32,           // Default: 500
    output_character_limit: i32,      // Default: 50000
    shell_integration_timeout: i32,   // Default: 4000ms
    shell_integration_disabled: bool, // Default: false
    command_delay: i32,               // Default: 100ms
    powershell_counter: bool,         // Default: false
    zsh_oh_my: bool,                  // Default: false
    zsh_p10k: bool,                   // Default: false
    zsh_dotdir: bool,                 // Default: false
    compress_progress_bar: bool,      // Default: true
}

// Execute command with settings applied
async fn execute_terminal_command(
    command: String,
    settings: &TerminalSettings
) -> Result<CommandOutput> {
    // Apply command delay
    tokio::time::sleep(Duration::from_millis(settings.command_delay as u64)).await;
    
    // Build shell command with integration
    let shell_cmd = build_shell_command(&command, settings)?;
    
    // Execute with timeout
    let output = tokio::time::timeout(
        Duration::from_millis(settings.shell_integration_timeout as u64),
        Command::new("sh").arg("-c").arg(&shell_cmd).output()
    ).await??;
    
    // Process output with limits
    let stdout = truncate_output(
        String::from_utf8_lossy(&output.stdout).to_string(),
        settings.output_line_limit,
        settings.output_character_limit
    );
    
    Ok(CommandOutput {
        stdout,
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}
```

---

### Image Operations (5 types)

```typescript
// Select images via file picker
vscode.postMessage({ type: "selectImages" })

// Open image in viewer
vscode.postMessage({ 
  type: "openImage",
  dataUri: "data:image/png;base64,..."
})

// Save image to disk
vscode.postMessage({
  type: "saveImage",
  dataUri: "data:image/png;base64,..."
})

// Handle dragged images
vscode.postMessage({
  type: "draggedImages",
  dataUrls: ["data:image/png;base64,..."]
})
```
```rust
async fn handle_select_images() -> Result<Vec<String>> {
    // Open native file picker
    let files = FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
        .set_max_files(20)
        .pick_files()?;
    
    // Convert to base64 data URLs
    let mut data_urls = Vec::new();
    for file in files {
        let bytes = fs::read(&file).await?;
        let mime = get_mime_type(&file);
        let base64 = base64::encode(&bytes);
        data_urls.push(format!("data:{};base64,{}", mime, base64));
    }
    
    Ok(data_urls)
}

async fn handle_open_image(data_uri: String) {
    // Decode base64
    let (mime, base64_data) = parse_data_uri(&data_uri)?;
    let bytes = base64::decode(base64_data)?;
    
    // Create temp file
    let temp_path = temp_dir().join(format!("image_{}.{}", Uuid::new_v4(), get_extension(&mime)));
    fs::write(&temp_path, bytes).await?;
    
    // Open with default viewer
    open::that(&temp_path)?;
}

async fn handle_save_image(data_uri: String) {
    let (_, base64_data) = parse_data_uri(&data_uri)?;
    let bytes = base64::decode(base64_data)?;
    
    // Save dialog
    if let Some(path) = FileDialog::new()
        .add_filter("Images", &["png", "jpg"])
        .save_file() 
    {
        fs::write(path, bytes).await?;
    }
}
```

---

### MCP (Model Context Protocol) Messages (25 types)

```typescript
// Toggle MCP enabled globally
vscode.postMessage({ type: "mcpEnabled", bool: true })

// Restart specific server
vscode.postMessage({
  type: "restartMcpServer",
  serverName: "filesystem"
})

// Refresh all servers
vscode.postMessage({ type: "refreshAllMcpServers" })

// Toggle tool auto-approve
vscode.postMessage({
  type: "toggleToolAlwaysAllow",
  serverName: "filesystem",
  toolName: "read_file",
  alwaysAllow: true
})

// Update timeout
vscode.postMessage({
  type: "updateMcpTimeout",
  serverName: "filesystem",
  timeout: 30000
})

// Delete server
vscode.postMessage({
  type: "deleteMcpServer",
  mcpId: "server-uuid"
})

// Download from marketplace
vscode.postMessage({
  type: "downloadMcp",
  mcpId: "github.com/author/server"
})
```
```rust
#[derive(Serialize, Deserialize, Clone)]
struct McpServer {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    enabled: bool,
    timeout_ms: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct McpTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    always_allow: bool,
    enabled_for_prompt: bool,
}

async fn handle_restart_mcp_server(server_name: String, state: Arc<RwLock<AppState>>) {
    let mut state = state.write().await;
    
    if let Some(server) = state.mcp_servers.iter_mut().find(|s| s.name == server_name) {
        // Kill existing process
        if let Some(process) = MCP_PROCESSES.write().await.remove(&server.id) {
            process.kill().await?;
        }
        
        // Start new process
        let child = Command::new(&server.command)
            .args(&server.args)
            .envs(&server.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        
        MCP_PROCESSES.write().await.insert(server.id.clone(), child);
        
        // Initialize MCP connection
        initialize_mcp_server(&server).await?;
    }
}

async fn initialize_mcp_server(server: &McpServer) -> Result<()> {
    let process = MCP_PROCESSES.read().await.get(&server.id).unwrap();
    
    // Send initialize request
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            }
        }
    });
    
    write_jsonrpc(&process.stdin, &init_req).await?;
    
    // Read response
    let response: serde_json::Value = read_jsonrpc(&process.stdout).await?;
    
    // List tools
    let list_tools_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    
    write_jsonrpc(&process.stdin, &list_tools_req).await?;
    let tools_response = read_jsonrpc(&process.stdout).await?;
    
    // Update server with discovered tools
    // ...
    
    Ok(())
}
```

---

## BACKEND → FRONTEND (139 Message Types)

### State Broadcast (1 type)

```rust
// Rust sends complete state
#[derive(Serialize)]
struct StateMessage {
    r#type: "state",
    state: ExtensionState,
}

async fn broadcast_state(state: &AppState, clients: &HashMap<Uuid, WebSocket>) {
    let msg = StateMessage {
        r#type: "state",
        state: state.to_extension_state(),
    };
    
    let json = serde_json::to_string(&msg)?;
    
    for (_, ws) in clients.iter() {
        ws.send(Message::Text(json.clone())).await?;
    }
}
```
```typescript
// Frontend receives
window.addEventListener("message", (event) => {
  const message: ExtensionMessage = event.data
  
  if (message.type === "state") {
    setState((prev) => mergeExtensionState(prev, message.state))
  }
})
```

### Message Streaming (1 type)

```rust
// Rust streams individual message updates
#[derive(Serialize)]
struct MessageUpdatedMessage {
    r#type: "messageUpdated",
    cline_message: ClineMessage,
}

// During AI streaming
async fn stream_ai_response(task_id: Uuid, clients: &HashMap<Uuid, WebSocket>) {
    let mut stream = anthropic_client.stream_message(request).await?;
    
    while let Some(chunk) = stream.next().await {
        let delta = chunk?;
        
        // Update message with new content
        let msg = MessageUpdatedMessage {
            r#type: "messageUpdated",
            cline_message: ClineMessage {
                ts: current_message.ts,
                r#type: "say",
                say: Some("text"),
                text: Some(accumulated_text + &delta.text),
                partial: Some(true), // Still streaming
            },
        };
        
        broadcast_to_clients(&msg, clients).await;
    }
    
    // Send final message with partial: false
    let final_msg = MessageUpdatedMessage {
        r#type: "messageUpdated",
        cline_message: ClineMessage {
            ts: current_message.ts,
            r#type: "say",
            say: Some("text"),
            text: Some(final_text),
            partial: Some(false), // Done streaming
        },
    };
    
    broadcast_to_clients(&final_msg, clients).await;
}
```

### Workspace Updates (1 type)

```rust
#[derive(Serialize)]
struct WorkspaceUpdatedMessage {
    r#type: "workspaceUpdated",
    file_paths: Vec<String>,
    opened_tabs: Vec<Tab>,
}

// Watch workspace for changes
async fn watch_workspace(workspace_path: PathBuf, clients: Arc<RwLock<HashMap<Uuid, WebSocket>>>) {
    let (tx, rx) = channel();
    let mut watcher = notify::watcher(tx, Duration::from_secs(2))?;
    watcher.watch(&workspace_path, RecursiveMode::Recursive)?;
    
    loop {
        match rx.recv() {
            Ok(event) => {
                // Scan all files
                let files = scan_workspace_files(&workspace_path).await?;
                
                let msg = WorkspaceUpdatedMessage {
                    r#type: "workspaceUpdated",
                    file_paths: files,
                    opened_tabs: get_opened_tabs().await?,
                };
                
                broadcast_to_all(&msg, &clients).await;
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        }
    }
}
```

---

## Complete Rust Message Router

```rust
pub async fn handle_webview_message(
    msg: WebviewMessage,
    state: Arc<RwLock<AppState>>,
    clients: Arc<RwLock<HashMap<Uuid, WebSocket>>>,
    client_id: Uuid,
) -> Result<()> {
    match msg.r#type.as_str() {
        // Task management
        "newTask" => handle_new_task(msg.text.unwrap(), msg.images, state).await?,
        "askResponse" => handle_ask_response(msg.ask_response.unwrap(), msg.text, msg.images, state).await?,
        "clearTask" => handle_clear_task(state).await?,
        "cancelTask" => handle_cancel_task(state).await?,
        "terminalOperation" => handle_terminal_operation(msg.terminal_operation.unwrap(), state).await?,
        
        // Settings - API Config
        "upsertApiConfiguration" => handle_upsert_api_config(msg.text.unwrap(), msg.api_configuration.unwrap(), state).await?,
        "currentApiConfigName" => handle_current_api_config_name(msg.text.unwrap(), state).await?,
        "deleteApiConfiguration" => handle_delete_api_config(msg.text.unwrap(), state).await?,
        
        // Settings - Permissions (12 handlers)
        "alwaysAllowReadOnly" => handle_always_allow_read_only(msg.bool.unwrap(), state).await?,
        "alwaysAllowWrite" => handle_always_allow_write(msg.bool.unwrap(), state).await?,
        "alwaysAllowExecute" => handle_always_allow_execute(msg.bool.unwrap(), state).await?,
        // ... 9 more permission handlers
        
        // Settings - Commands
        "allowedCommands" => handle_allowed_commands(msg.commands.unwrap(), state).await?,
        "deniedCommands" => handle_denied_commands(msg.commands.unwrap(), state).await?,
        
        // Settings - Terminal (15 handlers)
        "terminalOutputLineLimit" => handle_terminal_output_line_limit(msg.value.unwrap(), state).await?,
        // ... 14 more terminal handlers
        
        // Images
        "selectImages" => {
            let images = handle_select_images().await?;
            send_to_client(client_id, ExtensionMessage {
                r#type: "selectedImages",
                images: Some(images),
            }, &clients).await?;
        }
        "openImage" => handle_open_image(msg.data_uri.unwrap()).await?,
        "saveImage" => handle_save_image(msg.data_uri.unwrap()).await?,
        
        // MCP (25 handlers)
        "mcpEnabled" => handle_mcp_enabled(msg.bool.unwrap(), state).await?,
        "restartMcpServer" => handle_restart_mcp_server(msg.server_name.unwrap(), state).await?,
        // ... 23 more MCP handlers
        
        // History
        "showTaskWithId" => handle_show_task(msg.text.unwrap(), state).await?,
        "deleteTaskWithId" => handle_delete_task(msg.text.unwrap(), state).await?,
        "exportTaskWithId" => handle_export_task(msg.text.unwrap()).await?,
        
        // ... 200+ more handlers
        
        _ => {
            eprintln!("Unknown message type: {}", msg.r#type);
        }
    }
    
    Ok(())
}
```

---

**STATUS:** This is 1 of 10 deep analysis files covering exact protocol translation.
**NEXT:** DEEP-02-STATE-MANAGEMENT.md - Every state field with exact Rust types
