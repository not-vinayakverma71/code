# Critical Architecture Integration Plan

## Component Connection Architecture

**Everything connects THROUGH IPC**

```
┌────────────────────────────────────────────────────────┐
│                    LAPCE EDITOR                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  UI Layer (Floem)                                │  │
│  │  - Editor tabs                                   │  │
│  │  - AI Chat Panel                                 │  │
│  │  - File explorer                                 │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                  │
│  ┌──────────────────▼───────────────────────────────┐  │
│  │  AI Bridge Module (NEW)                          │  │
│  │  lapce-app/src/ai_bridge.rs                      │  │
│  │  - Manages IPC connection                        │  │
│  │  - Handles message routing                       │  │
│  └──────────────────┬───────────────────────────────┘  │
└─────────────────────┼──────────────────────────────────┘
                      │
                 IPC BOUNDARY
                 (Shared Memory)
                      │
┌─────────────────────▼───────────────────────────────────┐
│              LAPCE-AI-RUST ENGINE                       │
│  ┌──────────────────────────────────────────────────┐   │
│  │  IPC Server (Step 1)                             │   │
│  │  - Binary Protocol (Step 2)                      │   │
│  │  - Message dispatcher                            │   │
│  └──────────────────┬───────────────────────────────┘   │
│                     │                                   │
│         Routes to all components:                       │
│              ┌──────┴──────┐                            │
│       ┌──────▼────┐  ┌─────▼─────┐  ┌─────────────┐     │
│       │Semantic   │  │Tree-sitter│  │AI Providers │     │
│       │Search     │  │Integration│  │(OpenAI etc) │     │
│       │(Step 6)   │  │(Step 5)   │  │(Steps 03 )  │     │
│       └───────────┘  └───────────┘  └─────────────┘     │
│       ┌───────────┐  ┌───────────┐  ┌─────────────┐     │
│       │MCP Tools  │  │Cache      │  │Streaming    │     │
│       │           │  │           |  │ (Step 08)   │     │
│       └───────────┘  └───────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
```

**Key Points:**
- Lapce NEVER directly calls AI components
- ALL communication goes through IPC
- This enables hot-reload, process isolation, language agnostic

### 3. UI Integration Strategy

**Integrated as Lapce Panel (NOT standalone)**

```rust
// lapce-app/src/panel/mod.rs
pub enum PanelKind {
    Terminal,
    FileExplorer,
    Search,
    SourceControl,
    AIChat,  // NEW - Added by Step 22
}

// The AI panel is just another panel in Lapce
// Toggle with Cmd+Shift+L
// Docked to right side by default
// Can be moved/resized like other panels
```

**Why integrated:**
- Users want AI in their editor, not separate app
- Shares themes, keybindings, settings with Lapce
- Can access current file context directly
- Matches Codex/Cursor/GitHub Copilot UX

## Implementation Phases

### Phase A: Core IPC Infrastructure (DONE)
```
Week 1-2:
[✓] Step 1: IPC Server 
[→] Step 2: Binary Protocol
[→] Create ai_bridge.rs in lapce-app
[→] Test IPC connection Lapce ↔ AI Engine

```


### Phase B: IMPLEMENT THE OPTIMIZED COMPONENT (DONE)
Week 3-4:
[→]  Step 3&4 (AI Providers & Connection Pooling) 
[→]  Step 5 (Tree-sitter) 
[→]  Step 6&7 (Semantic Search) 
[→]  Step 8 (Streaming) 

```


### Phase C: Full Backend translation in RUST  ( As Describe above, UI as full Native & Backend through IPC -- We don`t have to make it  Lapce Plugin, make it like Cursor AI but, A New Architecture -  Fully Native - only backend component communicate through IPC  - Everything else is Pure Native)  --

CURRENTLY WORKING ON ---------------


Week 5: 
[→] Translate 100%  ALl component - Make sure it fits with Our Architectural Plan  - nor VS Code extension  neither Lapce Plugin - Pure made for Lapce IDE with exact AI   // Make Sure you don`t translate that already Implemented in Phase B
[→] Connect components to IPC dispatcher
[→] Test end-to-end: Component → IPC → Lapce IDE → IPC → Component
```

### Phase D: UI  Translation 
```
Week 6:
[→]  Port full Codex UI/UX to Floem
[→] Add to Lapce panel system
[→] Connect UI events to IPC messages
[→] Test end-to-end: UI→IPC→Component→IPC→UI
```
 

## Critical File Structure

```
lapce/
├── lapce-app/
│   ├── src/
│   │   ├── ai_bridge.rs          # NEW - IPC client
│   │   ├── panel/
│   │   │   ├── ai_chat.rs       # FULL - Chat UI/UX
│   │   │   └── mod.rs            # Register AI panel
│   │   └── main.rs               # Start IPC connection
│   └── Cargo.toml                # Add lapce-ai-rust dep
│
└── lapce-ai-rust/               # Separate process
    ├── src/
    │   ├── ipc_server.rs        # Receives all requests
    │   ├── binary_protocol.rs   # Fast serialization
    │   ├── dispatcher.rs        # Routes to components
    │   ├── semantic_search/     # All AI logic here
    │   ├── providers/           # Not in Lapce
    │   └── every single AI componenet   # Full translated backend
    |   └── main.rs              # Standalone binary 
    └── Cargo.toml

```

#

## Message Flow Example

```
User types in AI chat → 
Lapce UI → 
ai_bridge.send_message() → 
IPC (binary protocol) → 
lapce-ai-rust receives → 
dispatcher routes to provider →           // Full  Native feels, just componenets via IPC
provider generates response → 
streams back through IPC → 
ai_bridge receives chunks → 
UI updates in real-time
```

## Benefits of This Architecture

1. **Process Isolation**: AI crash doesn't kill editor
2. **Hot Reload**: Can update AI without restarting Lapce
3. **Memory Control**: AI runs in separate process with own limits
4. **Language Agnostic**: Could rewrite AI in Python/Go later
5. **Debugging**: Can test AI engine standalone
6. **Performance**: Binary protocol + shared memory = fast
7. **Give Feels like Cursor**: Full Native while full backend & high system resource usage component connect via IPC

## DO NOT:

- Embed AI directly in Lapce binary
- Make it Lapce Plugin
- Make UI standalone app
- Let components bypass IPC
