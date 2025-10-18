# CHUNK 07: VS CODE → LAPCE API MAPPING

## Critical Challenge
The entire Codex codebase is built on **143 files** importing VS Code APIs. This is the BIGGEST translation challenge - replacing an entire IDE's API surface.

## VS Code API Categories Used

### 1. Extension Context
```typescript
// VS Code
import * as vscode from "vscode"
const context: vscode.ExtensionContext

context.globalStorageUri.fsPath
context.extension.packageJSON
context.secrets.get(key)
context.secrets.store(key, value)
context.globalState.get(key)
context.globalState.update(key, value)
context.subscriptions.push(disposable)
```

**Lapce Equivalent:**
```rust
use lapce_plugin::{
    psp_types::Request,
    Http, LapcePlugin, VoltEnvironment,
    PLUGIN_RPC,
};

pub struct Plugin {
    volt: VoltEnvironment,
}

// Storage
let storage_path = std::env::var("LAPCE_PLUGIN_DATA")?;
let config_dir = format!("{}/config", storage_path);

// No built-in secrets management - use system keyring
use keyring::Entry;
let entry = Entry::new("lapce-ai", "api-key")?;
entry.set_password(&api_key)?;

// Global state - use JSON file
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize)]
struct GlobalState {
    // fields
}
let state_path = format!("{}/state.json", storage_path);
let state: GlobalState = serde_json::from_str(&std::fs::read_to_string(&state_path)?)?;
```

### 2. Webview (CRITICAL - NO DIRECT EQUIVALENT)
```typescript
// VS Code
vscode.window.createWebviewPanel(viewType, title, showOptions, {
    enableScripts: true,
    retainContextWhenHidden: true
})

webview.postMessage({ type: "update", data })
webview.onDidReceiveMessage(message => { /* handle */ })
webview.html = htmlContent
```

**Lapce Reality:**
- **NO HTML WEBVIEWS** - Lapce uses native Floem UI
- **Complete architectural redesign required**

**Strategy : Native Lapce UI with Floem**
```rust
use floem::views::{container, label, button, text_input, scroll};

pub fn build_ui() -> impl View {
    container((
        label("Task Input:"),
        text_input()
            .on_change(|text| {
                // Handle input
            }),
        button("Start Task")
            .on_click(|_| {
                // Start task
            }),
        scroll(
            // Message list
        )
    ))
}



### 3. Commands
```typescript
// VS Code
vscode.commands.registerCommand('extension.command', callback)
vscode.commands.executeCommand('vscode.open', uri)
```

**Lapce:**
```rust
use lapce_plugin::{register_plugin, VoltEnvironment};

impl LapcePlugin for Plugin {
    fn handle_request(&mut self, _id: u64, method: String, params: Value) {
        match method.as_str() {
            "custom_command" => {
                // Handle command
            }
            _ => {}
        }
    }
}

// Execute built-in Lapce command
PLUGIN_RPC.host_request(
    Request::new("lapce.command")
        .with_params(json!({
            "command": "open_file",
            "path": "/path/to/file"
        }))
);
```

### 4. File System
```typescript
// VS Code
vscode.workspace.fs.readFile(uri)
vscode.workspace.fs.writeFile(uri, bytes)
vscode.workspace.fs.readDirectory(uri)
vscode.workspace.fs.createDirectory(uri)
vscode.workspace.fs.delete(uri)
```

**Lapce:**
```rust
// Use Tokio FS directly
use tokio::fs;

let content = fs::read_to_string(path).await?;
fs::write(path, content).await?;
fs::create_dir_all(path).await?;
fs::remove_file(path).await?;

let mut dir = fs::read_dir(path).await?;
while let Some(entry) = dir.next_entry().await? {
    let path = entry.path();
    // Process entry
}
```

### 5. Workspace
```typescript
// VS Code
vscode.workspace.workspaceFolders
vscode.workspace.getConfiguration('section')
vscode.workspace.onDidChangeConfiguration(listener)
vscode.workspace.findFiles(pattern, exclude)
```

**Lapce:**
```rust
// Get workspace root
let workspace_root = std::env::var("LAPCE_WORKSPACE")?;

// Configuration - read from config file
let config_path = format!("{}/.lapce/config.toml", workspace_root);
use toml;
#[derive(Deserialize)]
struct Config {
    // fields
}
let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;

// File search - use walkdir or ignore crate
use ignore::WalkBuilder;
let walker = WalkBuilder::new(workspace_root)
    .hidden(false)
    .git_ignore(true)
    .build();

for entry in walker {
    let entry = entry?;
    if entry.file_type().unwrap().is_file() {
        // Process file
    }
}
```

### 6. Window/UI
```typescript
// VS Code
vscode.window.showInformationMessage(message)
vscode.window.showErrorMessage(message)
vscode.window.showWarningMessage(message)
vscode.window.showInputBox({ prompt, placeholder })
vscode.window.showQuickPick(items)
vscode.window.showOpenDialog(options)
vscode.window.createStatusBarItem()
vscode.window.createOutputChannel(name)
```

**Lapce:**
```rust
// Notifications
PLUGIN_RPC.notification(
    "show_message",
    json!({
        "type": "info", // "error", "warning"
        "message": "Task completed successfully"
    })
);

// Input dialog - no built-in, need custom UI or HTTP endpoint
// For now, use stdin/stdout or HTTP API

// Status bar - not available in plugin API yet
// Output channel - write to log file
use tracing::{info, error, warn};
info!("Task started");
```

### 7. Terminal
```typescript
// VS Code
vscode.window.createTerminal({ name, cwd })
terminal.sendText(command)
terminal.show()
terminal.dispose()

// Shell integration
terminal.shellIntegration?.executeCommand(command)
```

**Lapce:**
```rust
// Execute commands via process
use tokio::process::Command;

let output = Command::new("sh")
    .arg("-c")
    .arg(command)
    .current_dir(cwd)
    .output()
    .await?;

let stdout = String::from_utf8_lossy(&output.stdout);
let stderr = String::from_utf8_lossy(&output.stderr);
let exit_code = output.status.code().unwrap_or(-1);

// Stream output
use tokio::io::{AsyncBufReadExt, BufReader};

let mut child = Command::new("sh")
    .arg("-c")
    .arg(command)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

let stdout = child.stdout.take().unwrap();
let mut reader = BufReader::new(stdout).lines();

while let Some(line) = reader.next_line().await? {
    println!("{}", line);
    // Send to UI
}
```

### 8. Text Editor
```typescript
// VS Code
vscode.window.activeTextEditor
vscode.window.showTextDocument(uri)
editor.edit(editBuilder => {
    editBuilder.replace(range, text)
    editBuilder.insert(position, text)
    editBuilder.delete(range)
})
```

**Lapce:**
```rust
// Open file in editor
PLUGIN_RPC.host_request(
    Request::new("lapce.open_file")
        .with_params(json!({
            "path": file_path
        }))
);

// Edit file - direct file system operations
let content = fs::read_to_string(&file_path).await?;
let new_content = apply_edits(content);
fs::write(&file_path, new_content).await?;

// Notify Lapce to reload
PLUGIN_RPC.notification(
    "file_changed",
    json!({ "path": file_path })
);
```

### 9. Diff View
```typescript
// VS Code
vscode.commands.executeCommand('vscode.diff', 
    leftUri, rightUri, title)
```

**Lapce:**
```rust
// No built-in diff view in plugin API
// Options:
// 1. Write temp files and let user open them
// 2. Use external diff tool
// 3. Implement custom UI with diff library

use similar::{ChangeTag, TextDiff};

let diff = TextDiff::from_lines(&old_content, &new_content);
for change in diff.iter_all_changes() {
    match change.tag() {
        ChangeTag::Delete => println!("- {}", change),
        ChangeTag::Insert => println!("+ {}", change),
        ChangeTag::Equal => println!("  {}", change),
    }
}
```

### 10. Source Control (Git)
```typescript
// VS Code
vscode.extensions.getExtension('vscode.git')
const gitAPI = extension.exports.getAPI(1)
const repo = gitAPI.repositories[0]
repo.commit(message)
repo.push()
```

**Lapce:**
```rust
// Use git2 crate directly
use git2::{Repository, Signature};

let repo = Repository::open(workspace_root)?;
let mut index = repo.index()?;

// Stage files
index.add_path(Path::new("file.txt"))?;
index.write()?;

// Commit
let signature = Signature::now("Author", "email@example.com")?;
let tree_id = index.write_tree()?;
let tree = repo.find_tree(tree_id)?;
let parent_commit = repo.head()?.peel_to_commit()?;

repo.commit(
    Some("HEAD"),
    &signature,
    &signature,
    "Commit message",
    &tree,
    &[&parent_commit],
)?;
```

### 11. Language Server Protocol
```typescript
// VS Code
vscode.languages.registerCodeActionsProvider(selector, provider)
vscode.languages.registerCompletionItemProvider(selector, provider)
vscode.languages.registerHoverProvider(selector, provider)
```

**Lapce:**
```rust
// Lapce has built-in LSP support
// Plugin provides LSP server via stdio

use tower_lsp::{LspService, Server};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                // ... other capabilities
                ..Default::default()
            },
            ..Default::default()
        })
    }
    
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Provide completions
        Ok(None)
    }
}
```

### 12. Configuration
```typescript
// VS Code
const config = vscode.workspace.getConfiguration('extension')
config.get<string>('setting')
config.update('setting', value, ConfigurationTarget.Global)
```

**Lapce:**
```rust
// Read from config file
#[derive(Deserialize)]
struct ExtensionConfig {
    api_key: Option<String>,
    model: String,
    temperature: f64,
}

let config_path = format!("{}/.lapce/ai-config.toml", workspace_root);
let config: ExtensionConfig = toml::from_str(&fs::read_to_string(config_path)?)?;

// Write config
let updated_config = ExtensionConfig {
    model: "gpt-4".to_string(),
    ..config
};
fs::write(config_path, toml::to_string(&updated_config)?).await?;
```

## Architecture Decision: Hybrid Approach

Given Lapce's limited plugin API, **recommend hybrid architecture**:

### Core Backend (Rust)
- Task execution
- API provider management
- Tool execution
- State management
- IPC with Lapce

### Web UI (TypeScript/React)
- Port existing Codex UI
- Communicate via HTTP/WebSocket
- Run as separate process
- Accessible via browser at `localhost:PORT`

### Lapce Plugin (Minimal)
- Command registration
- File system bridge
- Terminal integration
- Launch web UI

```rust
// main.rs - Launch both server and register plugin
#[tokio::main]
async fn main() {
    // Start HTTP server in background
    tokio::spawn(async {
        start_web_server().await;
    });
    
    // Register Lapce plugin
    register_plugin(Plugin::new());
}

struct Plugin {
    server_url: String,
}

impl LapcePlugin for Plugin {
    fn handle_request(&mut self, id: u64, method: String, params: Value) {
        match method.as_str() {
            "start_task" => {
                // Forward to HTTP server
                let response = reqwest::get(&format!("{}/api/task", self.server_url))
                    .await?
                    .json::<Value>()
                    .await?;
                    
                PLUGIN_RPC.host_notification("open_browser", 
                    json!({ "url": format!("{}/ui", self.server_url) }));
            }
            _ => {}
        }
    }
}
```

## Translation Priority Matrix

| VS Code API | Usage Frequency | Lapce Equivalent | Difficulty |
|------------|----------------|------------------|-----------|
| Webview | Very High (UI) | **None** - Need web server | Critical |
| File System | Very High | Tokio FS | Easy |
| Commands | High | Plugin RPC | Medium |
| Terminal | High | tokio::process | Medium |
| Configuration | High | TOML files | Easy |
| Window/Messages | Medium | Notifications | Medium |
| Text Editor | Medium | File ops + notify | Hard |
| Workspace | Medium | Env vars + search | Medium |
| Git Integration | Low | git2 crate | Easy |
| LSP | Low | tower-lsp | Medium |

## Next: CHUNK 08 - Critical Services Analysis
Services directory contains code indexing, MCP, browser automation, etc.
