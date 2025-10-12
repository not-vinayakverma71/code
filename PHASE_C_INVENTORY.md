# Phase C UI Translation Inventory

## ✅ What We Have (2606 lines)

### Components Translated:
1. **chat_view.rs** (140 lines) - Basic message list + input
2. **chat_row.rs** (450 lines) - Message rendering with tool routing
3. **chat_text_area.rs** (151 lines) - Input bar with send button
4. **welcome_screen.rs** (79 lines) - Basic welcome with shortcuts
5. **approval_dialog.rs** (2830 bytes) - TODO: Check if implemented

### Shared Components:
6. **code_accordion.rs** (57 lines) - Code block display
7. **markdown_block.rs** (68 lines) - Basic text rendering
8. **tool_use_block.rs** - Tool execution display
9. **image_block.rs** - Image display

### Tool Renderers (Working):
10. **file_ops.rs** - read_file, list_files, search_files
11. **diff_ops.rs** - apply_diff, search_and_replace, insert_content
12. **command_ops.rs** - Terminal command execution
13. **task_ops.rs** - Todo list updates
14. **mcp_ops.rs** - MCP tool/resource display

### UI Primitives:
15. **button.rs** - Button component
16. **badge.rs** - Badge component

### Utils:
17. **language_detection.rs** - File language detection
18. **message_colors.rs** - Message styling
19. **path_utils.rs** - Path handling

---

## ❌ What's MISSING (Compare with Codex)

### Missing Core UI Components:

#### From ChatView.tsx:
- [ ] **TaskHeader** (task info, tokens, cost, actions)
- [ ] **ModeSelector** (mode dropdown with search)
- [ ] **HistoryPreview** (task history list with expand/collapse)
- [ ] **OrganizationSelector** (org switcher)
- [ ] **AutoApproveMenu** (approval settings dropdown)
- [ ] **SystemPromptWarning** (override warning)
- [ ] **CheckpointWarning** (checkpoint notifications)
- [ ] **QueuedMessages** (queued message display)
- [ ] **Announcement** (banner notifications)
- [ ] **TelemetryBanner** (telemetry consent)
- [ ] **IdeaSuggestionsBox** (suggestion chips)
- [ ] **KilocodeNotifications** (notifications panel)
- [ ] **BottomControls** (bottom action bar)

#### From ChatTextArea.tsx:
- [ ] **File upload button** (attach files)
- [ ] **Image upload button** (attach images)
- [ ] **Image preview** (show attached images)
- [ ] **Multiline input** (expand/collapse)
- [ ] **Mention support** (@file, @folder)
- [ ] **Keybinding display** (Cmd+Enter hint)
- [ ] **Character/token counter** (input length)
- [ ] **Submit button states** (loading, disabled)

#### From Settings:
- [ ] **SettingsView** (full settings panel)
- [ ] **ModelPicker** (model selection with info)
- [ ] **ApiOptions** (provider configuration)
- [ ] **ApiConfigManager** (manage multiple configs)
- [ ] **TerminalSettings** (terminal preferences)
- [ ] **AutoApproveSettings** (auto-approval rules)
- [ ] **ContextManagementSettings** (context window)
- [ ] **PromptsSettings** (system prompts)
- [ ] **NotificationSettings** (notification prefs)
- [ ] **ThinkingBudget** (thinking time limits)
- [ ] **BrowserSettings** (browser integration)
- [ ] **DisplaySettings** (UI preferences)
- [ ] **SlashCommandsSettings** (slash commands)
- [ ] **ExperimentalSettings** (experimental features)

#### From History:
- [ ] **HistoryView** (task history panel)
- [ ] **TaskItem** (history item component)
- [ ] **DeleteModeDialog** (delete confirmation)

#### From MCP:
- [ ] **McpView** (MCP servers panel)
- [ ] **McpResourceRow** (MCP resource display)
- [ ] **McpToolRow** (MCP tool display)
- [ ] **McpErrorRow** (MCP error display)

#### From Modes:
- [ ] **ModesView** (modes management panel)
- [ ] **EditModeControls** (mode editing UI)

#### From Cloud:
- [ ] **CloudView** (cloud features panel)
- [ ] **CloudUpsellDialog** (upgrade prompts)
- [ ] **Account/Org switchers** (account management)

#### From Marketplace:
- [ ] **MarketplaceView** (extensions marketplace)
- [ ] **MarketplaceListView** (extension list)

#### Missing Shared Components:
- [ ] **Tab/TabContent** (tab navigation)
- [ ] **Popover** (popover menus)
- [ ] **Tooltip** (tooltips)
- [ ] **Dialog** (modal dialogs)
- [ ] **Dropdown** (dropdown menus)
- [ ] **Input** (form inputs)
- [ ] **Checkbox** (checkboxes)
- [ ] **Switch** (toggle switches)
- [ ] **Select** (select dropdowns)
- [ ] **Textarea** (multiline text)
- [ ] **Badge** (enhanced badges)
- [ ] **Icons** (icon library - Lucide)
- [ ] **Thumbnails** (image thumbnails)

---

## 🎯 Critical Missing Features for Basic Functionality

### Top Priority (Must Have):
1. **Model Selector** - Can't select model
2. **History Button** - Can't view past tasks
3. **File/Image Upload** - Can't attach context
4. **Settings Panel** - Can't configure API
5. **Task Header** - Can't see cost/tokens

### Medium Priority (Important):
6. **MCP Integration UI** - MCP tools not accessible
7. **Auto-Approve Menu** - Can't configure approvals
8. **Mode Selector** - Can't switch modes
9. **Context Management** - Can't control context
10. **Multiline Input** - Limited input

### Lower Priority (Nice to Have):
11. Cloud features
12. Marketplace
13. Organization management
14. Telemetry banners

---

## 📊 Progress Summary

- **Lines written:** 2606
- **Core functionality:** ~20%
- **UI components:** ~15%
- **Settings panels:** 0%
- **MCP UI:** 0%
- **History UI:** 0%

**Estimated remaining work:**
- ~8000-10000 more lines needed
- ~40-50 more components
- ~15-20 settings panels
