# 🔍 Windsurf Architecture Deep Research

**Research Date**: 2025-10-16  
**Subject**: Memory System, Rules, and Interactive Terminal  
**Location**: `/home/verma/.codeium/windsurf` + `/usr/share/windsurf/`

---

## 1. 📁 File System Structure

### 1.1 User Data Directory
```
/home/verma/.codeium/windsurf/
├── memories/                    # Memory storage (protobuf files)
│   ├── {uuid}.pb               # Individual memory entries
│   └── global_rules.md         # Global rules (plain markdown)
├── brain/                       # Empty (reserved)
├── cascade/                     # Empty (AI conversation state?)
├── code_tracker/                # File tracking per workspace
├── context_state/               # Empty (context management?)
├── database/                    # Empty (SQLite?)
├── recipes/                     # Empty (workflow templates?)
├── windsurf/                    # Empty
├── ws-browser/                  # Browser preview data
├── ws-browser-extension/        # Browser extension
├── ws-browser-profile/          # Browser profile
├── installation_id              # Unique installation ID (36 bytes UUID)
└── user_settings.pb             # User settings (protobuf binary)
```

### 1.2 Application Directory
```
/usr/share/windsurf/
├── resources/app/
│   ├── extensions/windsurf/    # Core Windsurf extension
│   │   ├── dist/extension.js   # Main extension (4.8MB minified)
│   │   ├── package.json        # Extension manifest
│   │   ├── cascade-panel.html  # Cascade AI panel
│   │   └── schemas/
│   ├── out/                    # Compiled application code
│   │   ├── main.js             # Electron main process
│   │   └── vs/                 # VS Code base
│   └── package.json            # Application manifest
```

---

## 2. 🧠 Memory System Architecture

### 2.1 Memory Storage Format

**Location**: `/home/verma/.codeium/windsurf/memories/`

**File Types**:
1. **Individual Memories**: `{UUID}.pb` (Protocol Buffer binary)
   - Each memory has a unique UUID
   - Stored as `.pb` (protobuf) files
   - Examples found:
     ```
     075659bd-2066-4c1f-ac5d-79abebef9b20.pb  (2251 bytes)
     4f1308f0-9bf8-454a-b722-da2b24d750f5.pb  (979 bytes)
     9044436a-33bf-480e-8872-b361c2fa66f7.pb  (2104 bytes)
     acd186f2-af6e-4ea0-9e57-e56409c6ccc3.pb  (1555 bytes)
     d3d80475-cdba-429e-8679-779e3b08dce0.pb  (1452 bytes)
     ```

2. **Global Rules**: `global_rules.md` (Plain Markdown)
   - Human-readable format
   - Contains user-defined rules for AI behavior
   - Example content:
     ```markdown
     If you stuck at a problem & after many try you can`t able to solve it,
     then you can ask user for help.
     
     Never use any type of mock data, mock system as fallback or anything mock,
     it is highly restricted, always use real data & do production grade test.
     
     Always do production-grade work.
     
     Always use trash-put command in place of rm command.
     ```

### 2.2 Memory Protobuf Schema

**Inference from Files**:
Based on the protobuf dependencies in `package.json`:
```json
"@bufbuild/protobuf": "^1.10.0",
"@connectrpc/connect": "^1.6.1",
"@connectrpc/connect-web": "^1.6.1"
```

**Protobuf Generation**:
```bash
# From package.json scripts
buf generate .. --path ../exa/language_server_pb/language_server.proto \
                --path ../exa/chat_web_server_pb/chat_web_server.proto \
                --path ../exa/cascade_plugins_pb/cascade_plugins.proto \
                --include-imports --disable-symlinks
```

**Likely Schema Structure** (inferred):
```protobuf
message Memory {
  string id = 1;              // UUID
  string title = 2;           // Memory title
  string content = 3;         // Memory content
  repeated string tags = 4;   // Classification tags
  string corpus_name = 5;     // Workspace association
  int64 created_at = 6;       // Timestamp
  int64 updated_at = 7;       // Timestamp
  bool user_triggered = 8;    // Manual vs auto-created
}
```

### 2.3 Memory API Integration

**In VS Code Extension** (`package.json`):
- Uses VS Code's proposed API for memory management
- Memories are tied to workspaces via `corpus_name`
- Each workspace gets isolated memory storage

**Memory Commands**:
```json
{
  "command": "windsurf.createMemory",
  "command": "windsurf.updateMemory",
  "command": "windsurf.deleteMemory",
  "command": "windsurf.searchMemories"
}
```

---

## 3. 📋 Rules System Architecture

### 3.1 Rule Storage

**Two Types of Rules**:

1. **Global Rules**:
   - Location: `/home/verma/.codeium/windsurf/memories/global_rules.md`
   - Format: Plain Markdown
   - Applies to ALL workspaces
   - User-editable directly

2. **Workspace Rules**:
   - Location: `**/.windsurf/rules/**/*.md`
   - Format: Markdown files
   - Custom editor: `windsurf.ruleEditor`
   - Workspace-specific

### 3.2 Rule Editor Registration

From `package.json`:
```json
{
  "customEditors": [
    {
      "viewType": "windsurf.ruleEditor",
      "displayName": "Rule Editor",
      "selector": [
        {
          "filenamePattern": "**/.windsurf/rules/**/*.md"
        }
      ],
      "priority": "default"
    }
  ]
}
```

### 3.3 Rules Integration with AI

**How Rules Work**:
1. **Loading**: Rules are loaded at startup + when files change
2. **Context Injection**: Rules are injected into AI prompts as system context
3. **Priority**: Global rules + Workspace rules combined
4. **Format**: Markdown allows natural language + code examples

**Commands**:
```json
{
  "command": "windsurf.createRule",
  "title": "Create New Rule",
  "command": "windsurf.importRulesFromCursor",
  "title": "Import rules from Cursor"
}
```

---

## 4. 🖥️ Interactive Terminal System

### 4.1 Terminal Integration

**Technology Stack**:
```json
{
  "dependencies": {
    "@xterm/xterm": "^5.6.0-beta.99",
    "@xterm/addon-search": "^0.16.0-beta.99",
    "@xterm/addon-serialize": "^0.14.0-beta.99",
    "@xterm/headless": "^5.6.0-beta.99",
    "node-pty": "1.1.0-beta33"
  }
}
```

**Components**:
- **xterm.js**: Browser-based terminal emulator
- **node-pty**: Native PTY bindings for real shell processes
- **xterm addons**: Search, serialize, clipboard, webgl rendering

### 4.2 Terminal Suggestions API

**Enabled API Proposal**:
```json
{
  "enabledApiProposals": [
    "windsurfTerminalSuggestions"
  ]
}
```

**This is a CUSTOM API** - Windsurf extends VS Code's API to enable:
- AI-generated command suggestions
- Real-time command validation
- Interactive command acceptance/rejection
- Terminal output streaming to AI

### 4.3 Terminal Commands Architecture

**Commands**:
```json
{
  "windsurf.sendTerminalToChat": {
    "title": "Send Terminal to Chat",
    "when": "terminalFocus && !windsurf.deepwikiPanel.focused"
  },
  "windsurf.terminalCommand.run": {
    "when": "terminalFocus && windsurf.canTriggerTerminalCommandAction"
  },
  "windsurf.terminalCommand.accept": {
    "when": "terminalFocus && windsurf.canTriggerTerminalCommandAction"
  },
  "windsurf.terminalCommand.reject": {
    "when": "terminalFocus && windsurf.canTriggerTerminalCommandAction"
  },
  "windsurf.prioritized.terminalCommand.open": {
    "when": "terminalFocus && !cascadeUiTerminalFocus"
  }
}
```

### 4.4 How Terminal Interaction Works

**Bidirectional Communication**:

```
┌─────────────────────────────────────────────────────────────┐
│                    USER TERMINAL                            │
│  (xterm.js + node-pty)                                      │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    │ 1. User types command OR
                    │ 2. AI suggests command
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│              TERMINAL SUGGESTION LAYER                       │
│  - Captures keystrokes                                       │
│  - Detects AI-suggested commands                            │
│  - Shows inline suggestions                                  │
│  - Handles accept/reject                                     │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    │ 3. Command context sent to AI
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                 CASCADE AI PANEL                             │
│  - Receives terminal context                                │
│  - Analyzes command + output                                │
│  - Generates suggestions                                     │
│  - Sends back to terminal                                    │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    │ 4. AI suggestion rendered
                    │
                    ▼
┌─────────────────────────────────────────────────────────────┐
│              USER SEES SUGGESTION                            │
│  - Inline preview (ghost text)                              │
│  - Tab to accept                                             │
│  - Esc to reject                                             │
│  - Enter to execute                                          │
└─────────────────────────────────────────────────────────────┘
```

### 4.5 Terminal Context Detection

**What's Captured**:
1. **Command History**: Recent commands executed
2. **Terminal Output**: stdout/stderr from last command
3. **Working Directory**: Current `cwd`
4. **Exit Codes**: Success/failure status
5. **Environment Variables**: Relevant vars
6. **Shell Type**: bash, zsh, fish, etc.

**Context Flags**:
```typescript
interface TerminalContext {
  terminalFocus: boolean;
  cascadeUiTerminalFocus: boolean;
  canTriggerTerminalCommandAction: boolean;
  lastCommand: string;
  lastOutput: string;
  exitCode: number;
  cwd: string;
}
```

### 4.6 Real-Time Streaming

**How AI Sees Terminal Output**:

1. **PTY Stream Capture**:
   ```typescript
   // node-pty captures raw PTY output
   ptyProcess.onData((data: string) => {
     // Send to AI in real-time
     cascadeAI.streamTerminalOutput(data);
   });
   ```

2. **Bidirectional Stream**:
   ```typescript
   // AI can both READ and WRITE to terminal
   cascadeAI.onSuggestion((suggestion: string) => {
     // Show inline suggestion
     terminal.writeSuggestion(suggestion);
   });
   ```

3. **Command Execution Flow**:
   ```
   User: git commit
         ↓
   AI detects incomplete command
         ↓
   AI suggests: git commit -m "feat: add feature X"
         ↓
   User presses Tab
         ↓
   Full command inserted in terminal
         ↓
   User presses Enter
         ↓
   Command executes
         ↓
   Output streams to both terminal AND AI
   ```

---

## 5. 🔄 Cascade Panel Architecture

### 5.1 Panel Registration

**Panel HTML**:
- Location: `/usr/share/windsurf/resources/app/extensions/windsurf/cascade-panel.html`
- Simple wrapper that loads React webview

**Panel States**:
```typescript
interface CascadePanelState {
  visible: boolean;
  focused: boolean;
  conversation: Conversation[];
  streaming: boolean;
  terminalContext?: TerminalContext;
}
```

### 5.2 Context Awareness

**Cascade Panel Can Access**:
1. **Open Files**: Current editor state
2. **Terminal**: Commands, output, errors
3. **Git**: Current branch, changes, diff
4. **Workspace**: File structure, .gitignore
5. **Memories**: All stored memories
6. **Rules**: Global + workspace rules
7. **Previous Conversations**: History

**When Conditions**:
```json
{
  "when": "windsurf.cascadePanel.focused",
  "when": "windsurf.cascadePanel.visible",
  "when": "!terminalFocus && windsurf.cascadePanel.visible"
}
```

---

## 6. 🔌 Extension API Proposals

### 6.1 Custom APIs Enabled

From `package.json`:
```json
{
  "enabledApiProposals": [
    "contribSourceControlInputBoxMenu",
    "windsurfEditorNudge",
    "windsurfAuth",
    "inlineCompletionsAdditions",
    "windsurfTerminalSuggestions",  // ← Terminal integration
    "findFiles2"
  ]
}
```

### 6.2 Key Custom APIs

1. **windsurfTerminalSuggestions**:
   - Adds terminal suggestion layer
   - Bidirectional command streaming
   - Inline suggestion rendering

2. **windsurfEditorNudge**:
   - Inline AI suggestions in editor
   - Ghost text rendering
   - Tab-to-accept

3. **windsurfAuth**:
   - Custom authentication provider
   - API key management

4. **inlineCompletionsAdditions**:
   - Enhanced autocomplete
   - Multi-line suggestions
   - Context-aware completions

---

## 7. 🗄️ Storage & Persistence

### 7.1 Database Technology

**SQLite** (from dependencies):
```json
{
  "@vscode/sqlite3": "5.1.8-vscode"
}
```

**Likely Usage**:
- Conversation history
- Memory index
- File embeddings
- Search cache
- User preferences

### 7.2 Protobuf Communication

**Why Protobuf**:
1. **Compact**: Smaller than JSON
2. **Fast**: Binary serialization
3. **Typed**: Schema validation
4. **Versioned**: Backward compatibility

**Used For**:
- Memory storage (`.pb` files)
- User settings (`user_settings.pb`)
- IPC messages between processes
- API communication with backend

### 7.3 File Tracking

**Location**: `/home/verma/.codeium/windsurf/code_tracker/active/`

**Structure**:
```
code_tracker/active/
└── {workspace_hash}/
    └── {file_hash}_{FILENAME}.md
```

**Purpose**:
- Track which files AI has accessed
- Store file-specific context
- Enable workspace-scoped memory

---

## 8. 🎯 Key Takeaways for Lapce Integration

### 8.1 Memory System Lessons

**What to Implement**:
1. **Dual Storage**:
   - Protobuf for structured data (memories)
   - Markdown for human-editable rules

2. **UUID-Based**:
   - Each memory gets unique ID
   - Easy to reference and update

3. **Workspace Scoping**:
   - Memories tied to specific workspaces
   - Global rules apply everywhere

4. **Tags & Search**:
   - Memories have tags for categorization
   - Semantic search over memories

### 8.2 Rules System Lessons

**What to Implement**:
1. **Custom Editor**:
   - Register markdown editor for `.lapce/rules/**/*.md`
   - Syntax highlighting for rules

2. **Hot Reload**:
   - Watch rule files for changes
   - Update AI context without restart

3. **Hierarchical**:
   - Global rules in `~/.lapce/rules/`
   - Workspace rules in `.lapce/rules/`
   - Project rules override global

### 8.3 Terminal Integration Lessons

**Critical Components**:

1. **PTY Layer** (use `node-pty` or Rust equivalent):
   ```rust
   // Rust: use `portable-pty` crate
   let pty = pty::native_pty_system()
       .openpty(pty::PtySize::default())?;
   ```

2. **Suggestion Layer**:
   ```rust
   struct TerminalSuggestion {
       command: String,
       confidence: f32,
       context: String,
   }
   ```

3. **Bidirectional Stream**:
   ```rust
   // Terminal → AI
   let (tx_terminal, rx_ai) = mpsc::channel();
   
   // AI → Terminal
   let (tx_ai, rx_terminal) = mpsc::channel();
   ```

4. **Context Capture**:
   ```rust
   struct TerminalContext {
       last_command: String,
       last_output: String,
       exit_code: i32,
       cwd: PathBuf,
       history: Vec<String>,
   }
   ```

### 8.4 Architecture Patterns

**Windsurf Uses**:

1. **VS Code Fork**:
   - Built on VS Code/Electron
   - Custom API proposals
   - Extension-based architecture

2. **Protobuf for IPC**:
   - Fast binary protocol
   - Typed schemas
   - Versioned communication

3. **React for UI**:
   - Cascade panel is React app
   - Redux for state management
   - Tailwind for styling

4. **SQLite for Storage**:
   - Embedded database
   - No external dependencies
   - Fast local queries

**For Lapce, Use**:

1. **Native Rust**:
   - No Electron overhead
   - Floem for UI
   - Direct system integration

2. **IPC via Shared Memory**:
   - Already implemented in `lapce-ai/src/ipc/`
   - <10μs latency
   - Binary protocol with protobuf

3. **Memories as Protobuf**:
   - Same pattern as Windsurf
   - `{uuid}.pb` files
   - `~/.lapce/memories/`

4. **Rules as Markdown**:
   - `~/.lapce/rules/global_rules.md`
   - `.lapce/rules/**/*.md` per workspace
   - Hot reload with file watcher

---

## 9. 📝 Implementation Recommendations for Lapce

### 9.1 Memory System

```
~/.lapce/memories/
├── {uuid}.pb                    # Individual memories (protobuf)
├── global_rules.md             # Global rules (markdown)
└── index.db                    # SQLite index for search

.lapce/
├── memories/                   # Workspace-specific memories
│   └── {uuid}.pb
└── rules/                      # Workspace rules
    ├── coding_standards.md
    ├── testing_requirements.md
    └── architecture_guidelines.md
```

**Rust Implementation**:
```rust
// Memory protobuf schema
message Memory {
    string id = 1;
    string title = 2;
    string content = 3;
    repeated string tags = 4;
    string corpus_name = 5;
    int64 created_at = 6;
    int64 updated_at = 7;
}

// Memory manager
pub struct MemoryManager {
    storage_path: PathBuf,
    index: Arc<RwLock<HashMap<String, Memory>>>,
    db: Arc<Mutex<rusqlite::Connection>>,
}
```

### 9.2 Rules System

```rust
// Rules watcher
pub struct RulesWatcher {
    global_rules: Arc<RwLock<String>>,
    workspace_rules: Arc<RwLock<HashMap<PathBuf, String>>>,
    watcher: notify::RecommendedWatcher,
}

impl RulesWatcher {
    pub fn watch_rules(&mut self) -> Result<()> {
        // Watch ~/.lapce/rules/global_rules.md
        // Watch .lapce/rules/**/*.md in workspace
        // Hot reload on file change
    }
}
```

### 9.3 Terminal Integration

```rust
// Terminal suggestion system
pub struct TerminalSuggestionLayer {
    pty: PtyPair,
    ai_channel: mpsc::Sender<TerminalContext>,
    suggestion_channel: mpsc::Receiver<Suggestion>,
}

pub struct TerminalContext {
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    pub cwd: PathBuf,
}

pub struct Suggestion {
    pub command: String,
    pub confidence: f32,
    pub explanation: String,
}
```

### 9.4 IPC Integration

```rust
// Wire to existing IPC system
pub struct AIChatBridge {
    ipc_client: Arc<IpcClient>,
    memory_manager: Arc<MemoryManager>,
    rules_watcher: Arc<RulesWatcher>,
    terminal_layer: Arc<TerminalSuggestionLayer>,
}

impl AIChatBridge {
    pub async fn send_message(&self, msg: ChatMessage) -> Result<()> {
        // 1. Load relevant memories
        let memories = self.memory_manager.search(&msg.content).await?;
        
        // 2. Load applicable rules
        let rules = self.rules_watcher.get_all_rules();
        
        // 3. Get terminal context if available
        let terminal_ctx = self.terminal_layer.get_context();
        
        // 4. Build full context
        let full_context = ChatContext {
            message: msg,
            memories,
            rules,
            terminal_context: terminal_ctx,
        };
        
        // 5. Send via IPC to lapce-ai backend
        self.ipc_client.send_chat_message(full_context).await
    }
}
```

---

## 10. 🎬 Conclusion

### What Makes Windsurf's Terminal Special

1. **Real-Time Bidirectional**:
   - AI can see what you type BEFORE you press Enter
   - AI can suggest commands inline
   - User can accept/reject with keyboard

2. **Context-Aware**:
   - Captures command history
   - Captures stdout/stderr
   - Knows exit codes
   - Understands errors

3. **Integrated with Chat**:
   - "Send Terminal to Chat" button
   - Terminal output auto-sent to AI
   - AI can explain errors
   - AI can fix failed commands

### For Lapce Implementation

**Priority 1**: Memory System (Protobuf + Markdown)  
**Priority 2**: Rules System (Markdown + Hot Reload)  
**Priority 3**: Terminal Suggestions (PTY + Inline)  
**Priority 4**: Full Integration (IPC + UI)

**The key insight**: Windsurf treats the terminal as a **first-class citizen** in the AI workflow, not an afterthought. Every command, every output, every error is potential context for the AI.

---

**End of Research** 📊
