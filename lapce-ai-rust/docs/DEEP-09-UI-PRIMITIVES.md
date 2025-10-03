# DEEP ANALYSIS 09: UI PRIMITIVES - SHADCN COMPONENTS

## 📁 Analyzed Files

```
Codex/webview-ui/src/components/ui/ (25 primitives)
├── toggle-switch.tsx                 (Toggle switches)
├── input.tsx                         (Text inputs)
├── textarea.tsx                      (Multi-line inputs)
├── autosize-textarea.tsx             (Dynamic height)
├── tooltip.tsx                       (Tooltips)
├── standard-tooltip.tsx              (Standardized tooltips)
├── badge.tsx                         (Status badges)
├── progress.tsx                      (Progress bars)
├── labeled-progress.tsx              (Progress with labels)
├── alert-dialog.tsx                  (Confirmation dialogs)
├── popover.tsx                       (Popovers)
├── dropdown-menu.tsx                 (Context menus)
├── command.tsx                       (Command palette)
├── collapsible.tsx                   (Collapsible sections)
├── separator.tsx                     (Dividers)
├── slider.tsx                        (Range sliders)
└── searchable-select.tsx             (Searchable dropdowns)

Total: 25 UI primitives → Frontend-only (no Rust translation)

Backend Role: Provide data structures only
- Button states (enabled/disabled/loading)
- Dialog content & visibility flags
- Select options arrays
- Input values & validation errors
- Toast/notification messages
```

---

## Overview
UI components are **frontend-only** React components built with shadcn/ui and VSCode Webview Toolkit. Backend doesn't render UI - it only provides data via WebSocket messages.

---

## Translation Strategy

**Frontend (React):** Full UI component library
**Backend (Rust):** Data structures only - no UI rendering

```rust
// Backend provides data structures
#[derive(Serialize)]
pub struct ButtonData {
    pub label: String,
    pub disabled: bool,
    pub variant: ButtonVariant,
}

// Frontend renders UI
<Button variant={data.variant} disabled={data.disabled}>
    {data.label}
</Button>
```

---

## Core UI Components

### 1. Button (16 variants)
- Primary, Secondary, Ghost, Link, Destructive, Outline
- Backend: No translation needed (UI only)

### 2. Dialog/Modal
- Backend: Sends state indicating which dialog to show
- Frontend: Renders dialog based on state

### 3. Select/Dropdown
- Backend: Provides options array
- Frontend: Renders dropdown UI

### 4. Checkbox/Toggle
- Backend: Provides boolean state
- Frontend: Renders interactive control

### 5. Input/TextField
- Backend: Validates and stores value
- Frontend: Renders input with validation feedback

### 6. Tooltip
- Frontend only - no backend interaction

### 7. Toast/Notification
- Backend: Sends notification message
- Frontend: Displays toast UI

---

## Key Pattern

```typescript
// Frontend component
const HistoryView = () => {
    const { tasks } = useExtensionState()  // Data from backend
    return (
        <div>
            {tasks.map(task => (
                <TaskCard key={task.id} data={task} />  // UI rendering
            ))}
        </div>
    )
}
```

```rust
// Backend data provider
pub async fn get_tasks(state: Arc<RwLock<AppState>>) -> Vec<HistoryItem> {
    state.read().await.task_history.clone()
}

// Broadcast to frontend
broadcast_message(ExtensionMessage::TaskHistoryUpdated { 
    tasks: get_tasks(state).await 
}).await;
```

---

**STATUS:** UI primitives are frontend-only, backend provides data structures
**NEXT:** DEEP-10-TRANSLATION-MAP.md - Complete React → Rust patterns
