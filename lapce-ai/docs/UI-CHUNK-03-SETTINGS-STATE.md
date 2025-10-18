# UI CHUNK 03: SETTINGS & STATE MANAGEMENT

## Settings UI Architecture (43 files, ~11,000 lines)

### Main Components

| Component | Lines | Purpose |
|-----------|-------|---------|
| **SettingsView.tsx** | 899 | Main settings container with tab navigation |
| **ApiOptions.tsx** | 902 | Provider-specific configuration UI |
| **ApiConfigManager.tsx** | 10,294 | Multi-profile management system |
| **AutoApproveSettings.tsx** | 14,835 | Permission and auto-approval UI |
| **ContextManagementSettings.tsx** | 16,800 | Context window and file limits |
| **TerminalSettings.tsx** | 15,494 | Shell integration configuration |
| **PromptsSettings.tsx** | 9,943 | Custom mode and prompt editor |

### Settings Tab Structure

```typescript
const sectionNames = [
  "providers",        // API provider configuration
  "autoApprove",      // Permission toggles
  "browser",          // Browser automation
  "checkpoints",      // Checkpoint/restore settings
  "display",          // UI preferences
  "notifications",    // Notification settings
  "contextManagement",// Context limits
  "terminal",         // Terminal integration
  "prompts",          // Custom modes/prompts
  "experimental",     // Feature flags
  "language",         // i18n settings
  "mcp",              // MCP server management
  "about",            // Version info
] as const
```

### API Provider System (40+ Providers)

```typescript
export const PROVIDERS = [
  { value: "kilocode", label: "Kilo Code" },
  { value: "openrouter", label: "OpenRouter" },
  { value: "anthropic", label: "Anthropic" },
  { value: "openai-native", label: "OpenAI" },
  { value: "gemini", label: "Google Gemini" },
  { value: "deepseek", label: "DeepSeek" },
  { value: "cerebras", label: "Cerebras" },
  { value: "groq", label: "Groq" },
  { value: "ollama", label: "Ollama" },
  { value: "vertex", label: "GCP Vertex AI" },
  { value: "bedrock", label: "Amazon Bedrock" },
  // ... 30+ more providers
]
```

### Settings State Management

```typescript
// Local cache pattern
const [cachedState, setCachedState] = useState(extensionState)
const [isChangeDetected, setChangeDetected] = useState(false)

// Save handler sends 60+ messages
const handleSubmit = () => {
  vscode.postMessage({ type: "language", text: language })
  vscode.postMessage({ type: "alwaysAllowReadOnly", bool: alwaysAllowReadOnly })
  vscode.postMessage({ type: "allowedCommands", commands: allowedCommands })
  // ... 55+ more settings
  
  setChangeDetected(false)
}
```

---

## Global State Management (ExtensionStateContext.tsx)

**625 lines** managing **179 state properties** + **60+ setter functions**

### State Structure

```typescript
interface ExtensionStateContextType {
  // Core
  clineMessages: ClineMessage[]
  taskHistory: TaskHistoryItem[]
  apiConfiguration?: ProviderSettings
  
  // Permissions (12 toggles)
  alwaysAllowReadOnly: boolean
  alwaysAllowWrite: boolean
  alwaysAllowExecute: boolean
  alwaysAllowBrowser: boolean
  alwaysAllowMcp: boolean
  
  // Limits (10+ settings)
  maxOpenTabsContext: number
  maxWorkspaceFiles: number
  maxImageFileSize: number
  terminalOutputLineLimit: number
  
  // UI Preferences
  soundEnabled: boolean
  diffEnabled: boolean
  showTaskTimeline: boolean
  
  // 179 total properties
  // 60+ setter functions
}
```

### Message Handling

```typescript
const handleMessage = useCallback((event: MessageEvent) => {
  const message: ExtensionMessage = event.data
  
  switch (message.type) {
    case "state":
      setState((prev) => mergeExtensionState(prev, message.state))
      break
    case "workspaceUpdated":
      setFilePaths(message.filePaths)
      break
    case "messageUpdated":
      // Update single message (streaming)
      break
  }
}, [])
```

---

## Translation Strategy for Rust Backend

### State Persistence

```rust
#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub messages: Vec<ClineMessage>,
    pub permissions: Permissions,
    pub terminal_settings: TerminalSettings,
    // ... all 179 fields
}

pub struct StateStore {
    db: Arc<RocksDB>,
}

impl StateStore {
    pub async fn save_state(&self, state: &AppState) -> Result<()> {
        let value = bincode::serialize(state)?;
        self.db.put(b"app_state", value)?;
        Ok(())
    }
}
```

### WebSocket Sync

```rust
// Broadcast state updates
async fn broadcast_state(
    state: Arc<RwLock<AppState>>,
    clients: Arc<RwLock<HashMap<Uuid, WebSocket>>>
) {
    let message = json!({ "type": "state", "state": *state.read().await });
    for (_, ws) in clients.read().await.iter() {
        ws.send(Message::Text(message.to_string())).await;
    }
}
```

---

**NEXT CHUNK:** Message Communication Layer & Styling System
