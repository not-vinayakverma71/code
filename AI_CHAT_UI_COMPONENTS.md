# AI Chat UI/UX Components Architecture

## Overview
Phase C UI translation: All visual components for AI agent interaction in Lapce/Floem.

Reference: Codex has 54+ chat components in `webview-ui/src/components/chat/`

---

## Component Categories

### 1. MESSAGE DISPLAY (Core)
**ChatRow** - Main message container
- User messages (with attachments, images)
- Assistant messages (with streaming support)
- System messages (errors, warnings, info)
- Timestamps (optional via config)
- Expand/collapse for tool blocks
- Highlighted messages (search, navigation)

**MessageBubble** - Individual message styling
- User bubble (right-aligned, different color)
- Assistant bubble (left-aligned)
- Avatar/icon display
- Copy button
- Edit button (for user messages)

---

### 2. TOOL EXECUTION DISPLAY

**ToolUseBlock** - Generic tool execution
- Tool name + icon
- Status indicator (pending, running, success, error)
- Collapsible header
- Content area (input/output)

**ReadFileDisplay** - File read operations
- 📄 File path (clickable to open)
- Line range indicator (if specified)
- File preview (first N lines)
- "Open in editor" button
- File size, encoding metadata

**WriteFileDisplay** - File write operations
- 📝 File path (clickable to open)
- New/Modified badge
- Diff preview (before/after)
- "View diff" button
- Backup indicator

**SearchReplaceDisplay** - Search and replace
- 🔍 Search pattern
- Replace pattern
- Files affected count
- Matches count
- Expandable list of changes
- Undo button

**CommandExecution** - Terminal commands
- 💻 Command text (syntax highlighted)
- Status (running, completed, failed)
- Output (collapsible, scrollable)
- Exit code
- Duration timer
- Command pattern selector (allow/deny)
- "Open in terminal" button

**McpExecution** - MCP tool calls
- 🔌 MCP server name
- Tool name + parameters
- Result display
- Error handling

**BrowserSession** - Browser automation
- 🌐 URL displayed
- Screenshot preview
- Actions taken (click, type, scroll)
- Console logs

---

### 3. DIFF SYSTEM

**DiffView** - Inline diff display
- Side-by-side or unified view
- Syntax highlighting
- Line numbers
- Accept/Reject buttons per hunk
- "Accept All" / "Reject All" buttons
- Expand context lines

**BatchDiffApproval** - Multiple file diffs
- List of files with changes
- Summary (lines added/removed)
- Approve All / Reject All
- Individual file approval
- Preview each diff

**DiffStats** - Change summary
- Files changed count
- Lines added (green)
- Lines removed (red)
- Visual bar graph

---

### 4. APPROVAL SYSTEM

**ApprovalRequest** - Generic approval UI
- ⚠️ Warning icon
- Action description
- Risk level indicator (low, medium, high)
- Approve / Reject buttons
- "Always allow" checkbox
- Timeout countdown (if auto-approve enabled)

**BatchFilePermission** - Multiple file operations
- List of files to read/write
- Workspace boundary warnings
- Read-only file warnings
- Bulk approve/reject

**CommandApproval** - Command execution approval
- Command preview
- Safety warnings (destructive, network, sudo)
- Pattern matching (allowed/denied lists)
- Alternative safer commands suggestion

---

### 5. PROGRESS & STATUS

**ProgressIndicator** - Streaming animation
- Spinning indicator
- Dots animation (...)
- Pulse effect

**StreamingText** - Text streaming
- Character-by-character reveal
- Cursor animation
- Smooth scroll to bottom

**ThinkingIndicator** - AI thinking state
- "Thinking..." text
- Animated dots
- Elapsed time counter

**StatusBadge** - Operation status
- ✅ Success (green)
- ⏳ Pending (yellow)
- ▶️ Running (blue)
- ❌ Error (red)
- 🚫 Rejected (gray)

---

### 6. REASONING & PLANNING

**ReasoningBlock** - Thinking toggle
- 💡 Lightbulb icon
- "Thinking" header
- Elapsed timer (seconds)
- Collapsible content (markdown)
- User config: `reasoning-block-collapsed`

**TodoListDisplay** - Task breakdown
- ☑️ Todo items
- Status indicators:
  - ⚪ Pending
  - 🔵 In progress
  - ✅ Completed
- Progress bar (X/Y completed)
- Auto-scroll to current item
- Collapsible when all done

**TaskTimeline** - Action history
- Chronological list of all actions
- Timestamps
- Icons per action type
- Expandable details
- Filter by action type

---

### 7. CONTEXT & WORKSPACE

**ContextWindowProgress** - Token usage
- Progress bar (used/total tokens)
- Percentage indicator
- Color coding (green → yellow → red)
- Warning when near limit

**CondenseContextRow** - Context compression
- "Context condensed" message
- Before/after token counts
- Condensing strategy used
- Manual condense button

**CodebaseSearchResults** - Semantic search
- 🔍 Search query
- Results count
- Expandable file list
- Relevance scores
- "View in editor" links

---

### 8. MEDIA & ATTACHMENTS

**ImageBlock** - Image display
- Thumbnail preview
- Full-size lightbox on click
- Image metadata (size, dimensions)
- Remove button (before send)

**ImageWarningBanner** - Upload limits
- ⚠️ Warning when exceeding limits
- File size, total size, count
- Which limit exceeded
- Remove images suggestion

**FileAttachmentChip** - Attached files
- 📎 File icon
- File name
- File size
- Remove button

---

### 9. INPUT & INTERACTION

**ChatTextArea** - Main input
- Multiline text input
- Placeholder text
- @ mentions support
- / slash commands menu
- Drag & drop files
- Paste image support
- Character/token count
- Submit on Enter (configurable)

**SlashCommandMenu** - Command autocomplete
- Dropdown on "/"
- Command list with descriptions
- Icons per command
- Keyboard navigation
- Filter as you type

**MentionDropdown** - @file autocomplete
- File/folder suggestions
- Recent files
- Open files priority
- Fuzzy search
- Preview on hover

**FollowUpSuggest** - AI suggestions
- Quick action buttons
- Context-aware suggestions
- One-click prompts

---

### 10. HISTORY & NAVIGATION

**HistoryPanel** - Past conversations
- List of tasks
- Timestamps
- Preview snippet
- Search/filter
- Delete/archive
- Resume task button

**CheckpointIndicator** - Saved checkpoints
- 💾 Checkpoint saved message
- Restore button
- Checkpoint name/timestamp

**TaskActions** - Task controls
- ▶️ Resume
- ⏸️ Pause
- 🔄 Restart
- 🗑️ Delete
- 📤 Share (cloud)

---

### 11. ERRORS & WARNINGS

**ErrorRow** - Error display
- ❌ Error icon
- Error message
- Stack trace (collapsible)
- Retry button
- Report bug link

**CommandExecutionError** - Terminal errors
- Exit code
- stderr output (red text)
- Retry with sudo suggestion
- Pattern suggestions

**ValidationWarning** - Input validation
- ⚠️ Warning icon
- Validation message
- Fix suggestion

**ProfileViolationWarning** - Budget exceeded
- 💰 Cost/request limit reached
- Current usage vs limit
- Upgrade/adjust settings link

---

### 12. SPECIAL FEATURES

**AutoApproveMenu** - Auto-approval controls
- Master toggle
- Individual permission toggles
- Timeout slider
- Current status display

**ModeSelector** - AI mode switcher
- Dropdown with mode icons
- Mode descriptions
- Keyboard shortcut indicator
- Current mode highlighted

**ModelSelector** - Model chooser
- Dropdown with model list
- Model capabilities icons
- Tier/cost indicator
- Provider logos

**CostDisplay** - Token cost tracking
- Cost per message
- Running total
- Cost breakdown (input/output/cache)
- Budget warning

---

## Implementation Strategy

### Phase 1: Core Message Display (Priority 1)
1. ✅ ChatRow container
2. ✅ MessageBubble (user/assistant)
3. ✅ StreamingText with animation
4. ✅ ProgressIndicator

### Phase 2: Tool Execution (Priority 1)
5. ToolUseBlock generic container
6. ReadFileDisplay
7. WriteFileDisplay
8. SearchReplaceDisplay
9. CommandExecution
10. McpExecution

### Phase 3: Approvals (Priority 1)
11. ApprovalRequest generic
12. CommandApproval
13. BatchFilePermission
14. BatchDiffApproval

### Phase 4: Diff System (Priority 2)
15. DiffView inline
16. DiffStats summary
17. Accept/Reject controls

### Phase 5: Context & Planning (Priority 2)
18. ReasoningBlock with timer
19. TodoListDisplay
20. TaskTimeline
21. ContextWindowProgress

### Phase 6: Input & Interaction (Priority 2)
22. ChatTextArea enhanced
23. SlashCommandMenu
24. MentionDropdown
25. FollowUpSuggest

### Phase 7: Enhancements (Priority 3)
26. History panel
27. Error displays
28. Media handling
29. Auto-approve menu
30. Cost tracking

---

## Floem-Specific Considerations

### Styling
- Use `LapceConfig` theme colors throughout
- Reactive signals for all state
- Smooth animations via Floem's style system

### Performance
- Virtual scrolling for long chat histories
- Lazy loading for diffs and file previews
- Debounced text input
- Memoized expensive computations

### Accessibility
- Keyboard navigation for all controls
- Screen reader labels
- Focus management
- Color contrast compliance

---

## Next Steps

1. **Create component modules**:
   - `/home/verma/lapce/lapce-app/src/panel/ai_chat/components/messages/`
   - `/home/verma/lapce/lapce-app/src/panel/ai_chat/components/tools/`
   - `/home/verma/lapce/lapce-app/src/panel/ai_chat/components/approvals/`
   - `/home/verma/lapce/lapce-app/src/panel/ai_chat/components/diff/`

2. **Implement Phase 1** (Core Message Display) first
3. **Test each component** individually
4. **Integrate** into main chat view
5. **Iterate** based on UX feedback

All components will be **IPC-agnostic** - they receive data via props, emit events via callbacks. Backend wiring happens later.
