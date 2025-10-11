# Codex UI → Lapce AI Panel Porting Plan

**Status**: Phase 0 in progress  
**Rule**: Read full source file before converting any component  
**No mocks**: Use NoTransport (disconnected state) and real transcripts only

---

## Phased Execution Strategy

### Phase 0: Foundation (Week 1)
**Goal**: Establish core infrastructure before any UI porting

- [ ] 0.1 Bridge contracts (`ai_bridge/bridge.rs`)
  - `BridgeClient` trait with send/receive
  - `Transport` trait (NoTransport impl)
  - Message envelopes matching `lapce-ai/docs/CHUNK-02-TOOLS-EXECUTION.md`
  - Connection states: Disconnected/Connecting/Connected
  
- [ ] 0.2 Panel registration (`panel/ai_chat/mod.rs`)
  - Add `PanelKind::AIChat` 
  - Basic panel shell (empty container)
  - Panel lifecycle hooks
  
- [ ] 0.3 UI state store (`ai_state.rs`)
  - Port ExtensionStateContext essentials
  - Auto-approval toggles
  - Message queue
  - Connection state
  
- [ ] 0.4 i18n foundation (`i18n/`)
  - Translation provider trait
  - Load minimal keys from Codex locales
  - Fallback handling

**Exit criteria**: Can open AIChat panel, see connection status, bridge ready for messages

---

### Phase 1: Primitives & Utilities (Week 1-2)
**Goal**: Port shared building blocks needed by all components

- [ ] 1.1 UI primitives (`panel/ai_chat/ui/`)
  - Button, Dropdown, Dialog, Select, Checkbox
  - Tooltip, Badge, Progress, Separator
  - Textarea with autosize
  - ~25 components from `components/ui/`
  
- [ ] 1.2 Core utils (`panel/ai_chat/utils/`)
  - `message_colors.rs` (VSCode → Lapce theme tokens)
  - `language_detection.rs` (getLanguageFromPath)
  - `path_utils.rs` (removeLeadingNonAlphanumeric)
  - `image_utils.rs`
  - `command_validation.rs`
  - `highlighter.rs` (syntax highlighting setup)
  
- [ ] 1.3 Hooks equivalents
  - Keybindings handler
  - Auto-approval state
  - Escape key handler

**Exit criteria**: All primitives render, utils available, no compilation errors

---

### Phase 2: Shared Blocks (Week 2)
**Goal**: Build reusable chat components

- [ ] 2.1 Code viewer (`shared/code_accordion.rs`)
  - Read full `components/common/CodeAccordian.tsx`
  - Collapsible header with path
  - Lapce editor integration (read-only)
  - Diff syntax highlighting
  - Open file action → envelope
  - Tests: expand/collapse, large content
  
- [ ] 2.2 Markdown renderer (`shared/markdown_block.rs`)
  - Read full `components/common/MarkdownBlock.tsx`
  - Basic markdown → styled text
  - Code blocks with syntax
  - Tests: headings, links, code
  
- [ ] 2.3 Tool containers (`shared/tool_use_block.rs`)
  - Read full `components/common/ToolUseBlock.tsx`
  - Header with icon and status
  - Content area styling
  
- [ ] 2.4 Images (`shared/image_block.rs`)
  - Read full `components/common/ImageBlock.tsx`
  - Thumbnail + viewer
  - Size limits

**Exit criteria**: Can render code/markdown/tools in isolation with tests

---

### Phase 3: Chat Row Renderers (Week 3)
**Goal**: Build all tool-specific row renderers

- [ ] 3.1 File operations
  - Read full relevant sections of `ChatRow.tsx`
  - `readFile` renderer (single + batch via `BatchFilePermission`)
  - `listFilesTopLevel` / `listFilesRecursive`
  - `searchFiles` with results
  - `fetchInstructions`
  - Tests: snapshots per tool payload
  
- [ ] 3.2 Diff operations
  - `appliedDiff` / `editedExistingFile`
  - `insertContent`
  - `searchAndReplace`
  - `BatchDiffApproval` with per-file toggles
  - Tests: diff viewer integration
  
- [ ] 3.3 Tasking
  - `newTask` / `finishTask`
  - Read full `UpdateTodoListToolBlock.tsx`
  - Editable todo list (emit updates)
  - Tests: edit behavior
  
- [ ] 3.4 Command & MCP
  - Read full `CommandExecution.tsx`, `CommandExecutionError.tsx`
  - Continue/abort buttons → terminal envelopes
  - Read full `McpExecution.tsx`
  - MCP tool/resource display
  - Tests: button emissions
  
- [ ] 3.5 Other tools
  - `codebaseSearch` results display
  - `switchMode`
  - Browser session rows
  - Reasoning block, error row, progress indicator

**Exit criteria**: All tool renderers work in isolation with real payloads

---

### Phase 4: Chat Container (Week 4)
**Goal**: Assemble chat UI

- [ ] 4.1 ChatRow assembly (`chat/chat_row.rs`)
  - Read full `ChatRow.tsx` (50k)
  - Router: message type → renderer
  - Height tracking for virtualization
  - Expand/collapse state
  - Tests: all message types render
  
- [ ] 4.2 ChatTextArea (`chat/chat_text_area.rs`)
  - Read full `ChatTextArea.tsx` (53k)
  - Multiline input with autosize
  - Image selection (file picker)
  - Keybindings (Ctrl/Cmd+Enter → send)
  - Disabled states during asks
  - Emit: `newTask`, `askResponse`, `selectImages`
  - Tests: keyboard, image limits, disabled
  
- [ ] 4.3 ChatView (`chat/chat_view.rs`)
  - Read full `ChatView.tsx` (72k) systematically in chunks
  - Virtualized list (react-virtuoso equivalent)
  - Scroll-to-bottom logic
  - Partial streaming UI states
  - Ask flows: tool/command/followup/resume/completion
  - Approval bar (Yes/No/message response)
  - Queued messages display
  - Sound toggle + playback hooks
  - Tests: scroll, approvals, queue

**Exit criteria**: Can send messages, see tool approvals, approve/reject, view streaming

---

### Phase 5: History & Timeline (Week 5)
**Goal**: Task history browsing

- [ ] 5.1 History view
  - Read full `HistoryView.tsx`
  - Task list with search
  - Delete/export dialogs
  - Tests: list rendering, search
  
- [ ] 5.2 Timeline
  - Read full `TaskTimeline.tsx`, `TaskHeader.tsx`
  - Checkpoint UI
  - Tests: timeline display

**Exit criteria**: Can browse past tasks, delete, export

---

### Phase 6: Settings (Week 6-7)
**Goal**: Full settings UI

- [ ] 6.1 Settings shell
  - Read full `SettingsView.tsx` (37k)
  - Tab navigation
  - Persist to Lapce settings store
  
- [ ] 6.2 Core panels (high priority)
  - `ApiOptions` (provider configs)
  - `ModelPicker`
  - `TerminalSettings`
  - `AutoApproveSettings`
  - `ContextManagementSettings`
  - `PromptsSettings`
  - `NotificationSettings`
  - Each: read full file, port form, tests
  
- [ ] 6.3 Provider panels (43 files)
  - Systematic: one provider per commit
  - OpenAI, Anthropic, Bedrock, Ollama, etc.
  - UI-only forms, no network
  
- [ ] 6.4 Remaining panels
  - Display, Browser, Language, Image gen, etc.

**Exit criteria**: All settings editable, persisted, validated

---

### Phase 7: Modes & MCP (Week 7)
**Goal**: Custom modes and MCP management

- [ ] 7.1 Modes
  - Read full `ModesView.tsx` (58k)
  - Mode list, edit, delete
  - Tests: CRUD operations
  
- [ ] 7.2 MCP view
  - Read full `McpView.tsx`
  - Server list, enable/disable
  - Tool/resource browsing
  - Tests: list rendering

**Exit criteria**: Can create/edit modes, enable MCP servers

---

### Phase 8: Polish & Optional (Week 8)
**Goal**: Finish remaining surfaces

- [ ] 8.1 Cloud/Marketplace (low priority)
  - CloudView, MarketplaceView (UI-only)
  
- [ ] 8.2 Welcome screen
  - WelcomeView, tips
  
- [ ] 8.3 Connection status
  - Disconnected banner
  - Retry button
  - Degraded states throughout UI
  
- [ ] 8.4 Notifications & sounds
  - In-UI toasts
  - Audio playback hooks

**Exit criteria**: Full UI parity with Codex

---

### Phase 9: Testing & Fixtures (Ongoing)
**Goal**: Production-grade test coverage

- [ ] 9.1 Envelope schemas
  - JSON-schema definitions
  - Validation tests
  
- [ ] 9.2 UI snapshots
  - ChatView, ChatRow, Settings
  - Golden test framework
  
- [ ] 9.3 Event emission
  - Assert exact envelopes on actions
  - Approval flows, send, terminal ops
  
- [ ] 9.4 Recorded transcripts
  - Capture from `lapce-ai-cli`
  - Store fixtures for rendering tests
  - No mocks: real backend data only

**Exit criteria**: >80% coverage, all critical paths tested

---

### Phase 10: Optional Pre-IPC Enhancement
**Goal**: Manual QA with real backend

- [ ] 10.1 CLITransport
  - Spawn `lapce-ai-cli` as subprocess
  - Stream envelopes to UI
  - Full manual testing before IPC ready

**Exit criteria**: Can use full AI features via CLI before IPC lands

---

## Current Status
- **Phase**: 0 (Foundation)
- **Next task**: Create `ai_bridge/bridge.rs` with BridgeClient trait

## Principles
1. **One file at a time**: Read full source, understand, port, test, commit
2. **No mocks**: Use NoTransport or real CLI streams only
3. **Tests before next**: Each component needs snapshot + emission tests
4. **Dependencies first**: Primitives → Shared → Chat → Settings
5. **Production-grade**: No TODOs, no panics, full error handling

## File Tracking
See `UI_PORTING_TRACKER.md` for per-file checklist (345 files total)
