# Phase C: Complete UI Translation TODO List

**Goal:** Port 100% of Codex webview-ui to Lapce Floem components

---

## 📊 Overview

- **Current Progress:** 2,606 lines (~20% complete)
- **Estimated Total:** ~13,000 lines needed
- **Components to Port:** ~85 components
- **Estimated Time:** 3-4 weeks full-time

---

## 🔴 CRITICAL PRIORITY (Must Have for MVP)

### Chat Core Components
- [ ] **1. ChatView Header Bar**
  - Source: `ChatView.tsx` lines 2000-2100
  - Components: Model selector button, History toggle, Settings button
  - Effort: 2-3 hours
  
- [ ] **2. Model Selector Dropdown**
  - Source: `ModelPicker.tsx` (9197 bytes)
  - Features: Model list, search, model info display
  - Dependencies: Popover component
  - Effort: 4-5 hours

- [ ] **3. History Preview Panel**
  - Source: `HistoryPreview.tsx`
  - Features: Task list, expand/collapse, task selection
  - Effort: 3-4 hours

- [ ] **4. File Upload Button + Dialog**
  - Source: `ChatTextArea.tsx` lines 400-600
  - Features: File picker, multi-select, file type validation
  - Effort: 3-4 hours

- [ ] **5. Image Upload + Preview**
  - Source: `ChatTextArea.tsx` + `Thumbnails.tsx`
  - Features: Image picker, preview thumbnails, remove button
  - Effort: 4-5 hours

- [ ] **6. Task Header Component**
  - Source: `TaskHeader.tsx` (336 lines)
  - Features: Token count, cost display, context progress, actions
  - Effort: 5-6 hours

- [ ] **7. Settings Panel (Basic)**
  - Source: `SettingsView.tsx` (37901 bytes!)
  - Must have: API key input, provider selector
  - Effort: 8-10 hours (simplified version)

- [ ] **8. Welcome View (Full)**
  - Source: `WelcomeView.tsx` (212 lines)
  - Features: Provider cards, API config, start button
  - Effort: 4-5 hours

### Chat Input Enhancement
- [ ] **9. Multiline Input Support**
  - Source: `ChatTextArea.tsx` lines 200-300
  - Features: Auto-expand, Shift+Enter
  - Effort: 2-3 hours

- [ ] **10. Mention Support (@file, @folder)**
  - Source: `ChatTextArea.tsx` mention handling
  - Features: @ trigger, autocomplete, file selection
  - Effort: 4-5 hours

---

## 🟠 HIGH PRIORITY (Important Features)

### Mode & Context Management
- [ ] **11. Mode Selector Dropdown**
  - Source: `ModeSelector.tsx` (332 lines)
  - Features: Mode list, search, descriptions, shortcuts
  - Effort: 4-5 hours

- [ ] **12. Edit Mode Controls**
  - Source: `EditModeControls.tsx`
  - Features: Custom mode editing
  - Effort: 3-4 hours

- [ ] **13. Context Window Progress**
  - Source: `ContextWindowProgress.tsx`
  - Features: Progress bar, token usage display
  - Effort: 2-3 hours

- [ ] **14. Condense Context Button**
  - Source: `TaskHeader.tsx` condense action
  - Features: Context compression trigger
  - Effort: 1-2 hours

### Auto-Approval System
- [ ] **15. Auto-Approve Menu**
  - Source: `AutoApproveMenu.tsx`
  - Features: Quick toggles dropdown
  - Effort: 3-4 hours

- [ ] **16. Auto-Approve Settings Panel**
  - Source: `AutoApproveSettings.tsx` (14857 bytes)
  - Features: Granular approval rules
  - Effort: 6-8 hours

- [ ] **17. Auto-Approve Toggle Components**
  - Source: `AutoApproveToggle.tsx`
  - Features: Individual toggle switches
  - Effort: 2-3 hours

### Task Management
- [ ] **18. Task Timeline**
  - Source: `TaskTimeline.tsx`
  - Features: Visual task progress timeline
  - Effort: 4-5 hours

- [ ] **19. Task Actions Menu**
  - Source: `TaskActions.tsx`
  - Features: Stop, resume, checkpoint actions
  - Effort: 3-4 hours

- [ ] **20. Todo List Display**
  - Source: `TodoListDisplay.tsx`
  - Features: Todo items, check/uncheck, editing
  - Effort: 3-4 hours

- [ ] **21. Update Todo List Tool Block**
  - Source: `UpdateTodoListToolBlock.tsx`
  - Features: Editable todo block, emit updates
  - Effort: 4-5 hours

### Message Components
- [ ] **22. Browser Session Row**
  - Source: `BrowserSessionRow.tsx`
  - Features: Browser action display
  - Effort: 2-3 hours

- [ ] **23. Command Execution Display**
  - Source: `CommandExecution.tsx`
  - Features: Terminal output, ANSI colors
  - Effort: 3-4 hours

- [ ] **24. Command Execution Error**
  - Source: `CommandExecutionError.tsx`
  - Features: Error display with retry
  - Effort: 2-3 hours

- [ ] **25. Queued Messages Display**
  - Source: `QueuedMessages.tsx`
  - Features: Show queued user messages
  - Effort: 2-3 hours

---

## 🟡 MEDIUM PRIORITY (Nice to Have)

### Settings Panels (Detailed)
- [ ] **26. API Options (Full)**
  - Source: `ApiOptions.tsx` (33131 bytes)
  - Features: All provider configs
  - Effort: 10-12 hours

- [ ] **27. Api Config Manager**
  - Source: `ApiConfigManager.tsx`
  - Features: Manage multiple API configs
  - Effort: 5-6 hours

- [ ] **28. Terminal Settings**
  - Source: `TerminalSettings.tsx` (15494 bytes)
  - Features: Shell selection, command validation
  - Effort: 6-8 hours

- [ ] **29. Context Management Settings**
  - Source: `ContextManagementSettings.tsx` (16800 bytes)
  - Features: Context window controls
  - Effort: 6-8 hours

- [ ] **30. Prompts Settings**
  - Source: `PromptsSettings.tsx` (10726 bytes)
  - Features: System prompt customization
  - Effort: 5-6 hours

- [ ] **31. Notification Settings**
  - Source: `NotificationSettings.tsx`
  - Features: Sound, desktop notifications
  - Effort: 3-4 hours

- [ ] **32. Thinking Budget**
  - Source: `ThinkingBudget.tsx`
  - Features: Thinking time limits
  - Effort: 3-4 hours

- [ ] **33. Browser Settings**
  - Source: `BrowserSettings.tsx`
  - Features: Browser integration settings
  - Effort: 4-5 hours

- [ ] **34. Display Settings**
  - Source: `DisplaySettings.tsx`
  - Features: UI preferences (timestamps, sounds)
  - Effort: 3-4 hours

- [ ] **35. Slash Commands Settings**
  - Source: `SlashCommandsSettings.tsx`
  - Features: Custom slash commands
  - Effort: 4-5 hours

- [ ] **36. Experimental Settings**
  - Source: `ExperimentalSettings.tsx`
  - Features: Experimental feature flags
  - Effort: 3-4 hours

- [ ] **37. Language Settings**
  - Source: `LanguageSettings.tsx`
  - Features: i18n language selection
  - Effort: 2-3 hours

- [ ] **38. Image Generation Settings**
  - Source: `ImageGenerationSettings.tsx`
  - Features: Image AI settings
  - Effort: 4-5 hours

- [ ] **39. Checkpoint Settings**
  - Source: `CheckpointSettings.tsx`
  - Features: Checkpoint configuration
  - Effort: 2-3 hours

- [ ] **40. Fast Apply Settings**
  - Source: `FastApplySettings.tsx`
  - Features: Fast apply preferences
  - Effort: 2-3 hours

### MCP Integration
- [ ] **41. MCP View Panel**
  - Source: `McpView.tsx`
  - Features: MCP server list
  - Effort: 4-5 hours

- [ ] **42. MCP Resource Row**
  - Source: `McpResourceRow.tsx`
  - Features: Resource display, actions
  - Effort: 3-4 hours

- [ ] **43. MCP Tool Row**
  - Source: `McpToolRow.tsx`
  - Features: Tool display, invoke button
  - Effort: 3-4 hours

- [ ] **44. MCP Error Row**
  - Source: `McpErrorRow.tsx`
  - Features: Error display, retry
  - Effort: 2-3 hours

### History & Modes
- [ ] **45. History View (Full)**
  - Source: `HistoryView.tsx`
  - Features: Full history panel with search
  - Effort: 5-6 hours

- [ ] **46. Task Item Component**
  - Source: Task item components
  - Features: Individual history items
  - Effort: 3-4 hours

- [ ] **47. Delete Mode Dialog**
  - Source: `DeleteModeDialog.tsx`
  - Features: Confirmation dialog
  - Effort: 2-3 hours

- [ ] **48. Modes View**
  - Source: `ModesView.tsx`
  - Features: Modes management panel
  - Effort: 4-5 hours

### Notifications & Warnings
- [ ] **49. System Prompt Warning**
  - Source: `SystemPromptWarning.tsx`
  - Features: Override warning banner
  - Effort: 2-3 hours

- [ ] **50. Checkpoint Warning**
  - Source: `CheckpointWarning.tsx`
  - Features: Checkpoint notification
  - Effort: 2-3 hours

- [ ] **51. Profile Violation Warning**
  - Source: `ProfileViolationWarning.tsx`
  - Features: Org profile warning
  - Effort: 2-3 hours

- [ ] **52. Announcement Banner**
  - Source: `Announcement.tsx`
  - Features: Dismissible announcements
  - Effort: 2-3 hours

- [ ] **53. Telemetry Banner**
  - Source: `TelemetryBanner.tsx`
  - Features: Telemetry consent
  - Effort: 2-3 hours

- [ ] **54. Kilocode Notifications**
  - Source: `KilocodeNotifications.tsx`
  - Features: Notification system
  - Effort: 3-4 hours

### Search & Results
- [ ] **55. Codebase Search Results Display**
  - Source: `CodebaseSearchResultsDisplay.tsx`
  - Features: Search results list
  - Effort: 4-5 hours

- [ ] **56. Codebase Search Result Item**
  - Source: `CodebaseSearchResult.tsx`
  - Features: Individual result display
  - Effort: 2-3 hours

- [ ] **57. Code Index Popover**
  - Source: `CodeIndexPopover.tsx`
  - Features: Indexing status UI
  - Effort: 2-3 hours

- [ ] **58. Indexing Status Badge**
  - Source: `IndexingStatusBadge.tsx`
  - Features: Status indicator
  - Effort: 1-2 hours

---

## 🟢 LOW PRIORITY (Future Enhancements)

### Cloud Features
- [ ] **59. Cloud View**
  - Source: `CloudView.tsx`
  - Features: Cloud features panel
  - Effort: 5-6 hours

- [ ] **60. Cloud Upsell Dialog**
  - Source: `CloudUpsellDialog.tsx`
  - Features: Upgrade prompts
  - Effort: 3-4 hours

- [ ] **61. Account Switcher**
  - Source: Account components
  - Features: Switch accounts
  - Effort: 3-4 hours

- [ ] **62. Organization Selector**
  - Source: `OrganizationSelector.tsx`
  - Features: Org switcher
  - Effort: 3-4 hours

### Marketplace
- [ ] **63. Marketplace View**
  - Source: `MarketplaceView.tsx`
  - Features: Extensions marketplace
  - Effort: 6-8 hours

- [ ] **64. Marketplace List View**
  - Source: `MarketplaceListView.tsx`
  - Features: Extension list
  - Effort: 4-5 hours

- [ ] **65. Marketplace State Manager**
  - Source: `MarketplaceViewStateManager.ts`
  - Features: State management
  - Effort: 3-4 hours

### Additional UI Components
- [ ] **66. Version Indicator**
  - Source: `VersionIndicator.tsx`
  - Features: Show version info
  - Effort: 1-2 hours

- [ ] **67. Dismissible Upsell**
  - Source: `DismissibleUpsell.tsx`
  - Features: Dismissible upsell cards
  - Effort: 2-3 hours

- [ ] **68. Idea Suggestions Box**
  - Source: `IdeaSuggestionsBox.tsx`
  - Features: Suggestion chips
  - Effort: 3-4 hours

- [ ] **69. Bottom Controls**
  - Source: `BottomControls.tsx`
  - Features: Bottom action bar
  - Effort: 3-4 hours

---

## 🎨 UI PRIMITIVES & LIBRARY (Foundation)

### Core Primitives (Must build first)
- [ ] **70. Popover Component**
  - Source: `components/ui/Popover.tsx`
  - Used by: Model selector, menus, etc.
  - Effort: 4-5 hours

- [ ] **71. Dialog/Modal Component**
  - Source: `components/ui/Dialog.tsx`
  - Used by: Settings, confirmations
  - Effort: 3-4 hours

- [ ] **72. Dropdown Menu**
  - Source: `components/ui/Dropdown.tsx`
  - Used by: Settings, actions
  - Effort: 3-4 hours

- [ ] **73. Tabs Component**
  - Source: `components/common/Tab.tsx`
  - Used by: Settings, welcome
  - Effort: 2-3 hours

- [ ] **74. Tooltip Component**
  - Source: `components/ui/Tooltip.tsx`
  - Used by: Everywhere
  - Effort: 2-3 hours

- [ ] **75. Input Component**
  - Source: `components/ui/Input.tsx`
  - Used by: Forms, settings
  - Effort: 2-3 hours

- [ ] **76. Checkbox Component**
  - Source: `components/ui/Checkbox.tsx`
  - Used by: Settings
  - Effort: 2-3 hours

- [ ] **77. Switch/Toggle Component**
  - Source: `components/ui/Switch.tsx`
  - Used by: Settings
  - Effort: 2-3 hours

- [ ] **78. Select Component**
  - Source: `components/ui/Select.tsx`
  - Used by: Dropdowns
  - Effort: 3-4 hours

- [ ] **79. Textarea Component**
  - Source: `components/ui/Textarea.tsx`
  - Used by: Forms
  - Effort: 2-3 hours

- [ ] **80. Badge Component (Enhanced)**
  - Source: `components/ui/Badge.tsx`
  - Current: Basic version exists
  - Effort: 1-2 hours

- [ ] **81. Button Component (Enhanced)**
  - Source: `components/ui/Button.tsx`
  - Current: Basic version exists
  - Effort: 2-3 hours

- [ ] **82. Icon Library Integration**
  - Source: Lucide icons usage
  - Need: Icon component system
  - Effort: 4-5 hours

- [ ] **83. Thumbnails Component**
  - Source: `Thumbnails.tsx`
  - Used by: Image attachments
  - Effort: 2-3 hours

- [ ] **84. Mention Component**
  - Source: `Mention.tsx`
  - Used by: @file mentions
  - Effort: 3-4 hours

- [ ] **85. Icon Button**
  - Source: `IconButton.tsx`
  - Used by: Toolbars
  - Effort: 1-2 hours

---

## 🔧 SUPPORTING SYSTEMS

### State Management
- [ ] **86. Extension State Context**
  - Source: `ExtensionStateContext.tsx`
  - Port to: Lapce state store
  - Effort: 6-8 hours

### Hooks
- [ ] **87. useAutoApprovalState**
  - Source: `hooks/useAutoApprovalState.ts`
  - Effort: 2-3 hours

- [ ] **88. useAutoApprovalToggles**
  - Source: `hooks/useAutoApprovalToggles.ts`
  - Effort: 2-3 hours

- [ ] **89. useSelectedModel**
  - Source: `hooks/useSelectedModel.ts`
  - Effort: 2-3 hours

- [ ] **90. useKeybindings**
  - Source: `hooks/useKeybindings.ts`
  - Effort: 3-4 hours

- [ ] **91. useEscapeKey**
  - Source: `hooks/useEscapeKey.ts`
  - Effort: 1-2 hours

- [ ] **92. useCloudUpsell**
  - Source: `hooks/useCloudUpsell.ts`
  - Effort: 2-3 hours

### Utilities
- [ ] **93. Command Validation**
  - Source: `utils/command-validation.ts`
  - Effort: 3-4 hours

- [ ] **94. Highlighter (Syntax)**
  - Source: `utils/highlighter.ts`
  - Effort: 4-5 hours

- [ ] **95. TextMate to Highlight.js**
  - Source: `utils/textMateToHljs.ts`
  - Effort: 3-4 hours

- [ ] **96. Image Utils**
  - Source: `utils/imageUtils.ts`
  - Current: Partial
  - Effort: 2-3 hours

- [ ] **97. Clipboard Utils**
  - Source: `utils/clipboard.ts`
  - Effort: 2-3 hours

- [ ] **98. Path Mentions**
  - Source: `utils/pathMentions.ts`
  - Effort: 2-3 hours

- [ ] **99. Timeline Utils**
  - Source: `utils/timeline.ts`
  - Effort: 2-3 hours

### i18n (Internationalization)
- [ ] **100. Translation Context**
  - Source: `i18n/TranslationContext.tsx`
  - Port to: Lapce i18n system
  - Effort: 4-5 hours

- [ ] **101. Translation Files**
  - Source: `i18n/locales/`
  - Port all languages
  - Effort: 2-3 hours

---

## 📝 TESTING & VALIDATION

- [ ] **102. Component Snapshot Tests**
  - Create visual regression tests
  - Effort: 8-10 hours

- [ ] **103. Event Emission Tests**
  - Test UI → IPC message flow
  - Effort: 6-8 hours

- [ ] **104. Recorded Transcript Tests**
  - Capture lapce-ai-cli fixtures
  - Effort: 4-5 hours

- [ ] **105. Manual QA Checklist**
  - Document all flows to test
  - Effort: 3-4 hours

---

## 📊 SUMMARY

### Total Components: 105
- **Critical:** 10 components (~40 hours)
- **High Priority:** 15 components (~60 hours)
- **Medium Priority:** 33 components (~130 hours)
- **Low Priority:** 15 components (~60 hours)
- **UI Primitives:** 16 components (~45 hours)
- **Supporting:** 16 systems (~50 hours)
- **Testing:** 4 tasks (~25 hours)

### Total Estimated Effort: **410 hours** (~10 weeks at 40hrs/week)

### Recommended Approach:
1. **Week 1-2:** Build UI primitives (popover, dialog, tabs, etc.)
2. **Week 3-4:** Critical components (model selector, settings, uploads)
3. **Week 5-6:** High priority (modes, auto-approve, task management)
4. **Week 7-8:** Medium priority (detailed settings, MCP, history)
5. **Week 9-10:** Low priority + testing

### MVP (Minimum Viable Product) - 2 weeks:
- Focus on Critical + UI Primitives only
- Estimated: ~85 hours
- Result: Functional AI chat with basic features
