# UI CHUNK 01: WEBVIEW FRONTEND - COMPLETE OVERVIEW

## Executive Summary

**Total Files:** ~350 TypeScript/TSX files  
**Total Lines:** ~58,000 lines of React code  
**Framework:** React 18.3 + TypeScript 5.8  
**Build Tool:** Vite 6.3  
**UI Library:** Radix UI + shadcn/ui components  
**Styling:** Tailwind CSS v4 + styled-components  
**State Management:** React Context + TanStack Query  
**Testing:** Vitest + Testing Library  

## Directory Structure

```
webview-ui/src/
├── App.tsx (350 lines) - Main application shell
├── index.tsx (19 lines) - React entry point
├── index.css (487 lines) - Global styles with Tailwind v4
│
├── components/ (~336 files) - React components
│   ├── chat/ (47 files, ~15,000 lines) - Main chat interface
│   ├── settings/ (43 files, ~11,000 lines) - Settings UI
│   ├── kilocode/ (45+ files, ~8,000 lines) - Kilo Code branded UI
│   ├── ui/ (24 files, ~2,500 lines) - shadcn/ui primitives
│   ├── history/ (10 files, ~2,000 lines) - Task history
│   ├── mcp/ (8 files, ~1,500 lines) - MCP server management
│   ├── modes/ (6 files, ~1,200 lines) - AI mode selector
│   ├── marketplace/ (15 files, ~3,000 lines) - Extension marketplace
│   ├── common/ (12 files, ~2,000 lines) - Shared components
│   └── ... (welcome, account, human-relay, etc.)
│
├── context/ (2 files, ~29,000 lines) - Global state
│   └── ExtensionStateContext.tsx (625 lines) - Main state container
│
├── hooks/ (6 files, ~800 lines) - Custom React hooks
│   ├── useAutoApprovalState.ts
│   ├── useAutoApprovalToggles.ts
│   ├── useEscapeKey.ts
│   └── useTooltip.ts
│
├── utils/ (30+ files, ~6,000 lines) - Utility functions
│   ├── vscode.ts (90 lines) - VS Code API wrapper
│   ├── context-mentions.ts (12,510 lines) - @mention handling
│   ├── command-validation.ts (26,237 lines) - Command parsing
│   ├── messageColors.ts - UI theming
│   ├── highlighter.ts - Shiki code highlighting
│   └── ... (clipboard, format, path-mentions, etc.)
│
├── i18n/ (246 files) - Internationalization
│   ├── TranslationContext.tsx
│   ├── setup.ts
│   └── locales/ (242 files) - Translation files
│
├── services/ (3 files) - Business logic
│   ├── MemoryService.ts
│   └── mermaidSyntaxFixer.ts
│
└── vite-plugins/ (1 file) - Custom Vite plugins
    └── sourcemapPlugin.ts
```

## Component Architecture Summary

### By Module (Production Files Only)

| Module | Files | Lines | Purpose |
|--------|-------|-------|---------|
| **chat/** | 47 | ~15,000 | Core chat interface, message rendering, input |
| **settings/** | 43 | ~11,000 | Provider config, permissions, terminal settings |
| **kilocode/** | 45+ | ~8,000 | Kilo Code branding and custom features |
| **ui/** | 24 | ~2,500 | Reusable UI primitives (shadcn/ui) |
| **history/** | 10 | ~2,000 | Task history browser and search |
| **marketplace/** | 15 | ~3,000 | Extension/MCP server marketplace |
| **mcp/** | 8 | ~1,500 | MCP server configuration |
| **modes/** | 6 | ~1,200 | AI mode selector and custom modes |
| **common/** | 12 | ~2,000 | Shared components (CodeBlock, Thumbnails) |
| **welcome/** | 5 | ~1,000 | Onboarding and welcome screens |

**Total Component Lines:** ~58,000+ lines of React/TypeScript

## Largest Files (Critical Components)

| File | Lines | Purpose |
|------|-------|---------|
| **ChatView.tsx** | 2,237 | Main chat container with virtualized scrolling |
| **ChatTextArea.tsx** | 1,662 | Chat input with @mentions, slash commands |
| **ChatRow.tsx** | 1,442 | Single message row renderer |
| **CodeIndexPopover.tsx** | 1,288 | Codebase indexing configuration |
| **SettingsView.tsx** | 899 | Main settings screen |
| **ExtensionStateContext.tsx** | 625 | Global state management |
| **BrowserSessionRow.tsx** | 580 | Browser automation display |
| **ApiOptions.tsx** | 950 | API provider configuration |
| **App.tsx** | 350 | Root component and routing |

## Technology Stack

### Core Framework
- **React 18.3.1** - UI library with concurrent features
- **TypeScript 5.8.3** - Type safety
- **Vite 6.3.5** - Build tool (ESM, HMR)
- **React DOM 18.3.1** - DOM rendering

### UI Component Libraries
- **Radix UI** (12+ packages) - Headless accessible components
  - Dialog, Dropdown, Select, Checkbox, Tooltip, etc.
- **shadcn/ui** - Pre-styled Radix components
- **@vscode/webview-ui-toolkit** - VS Code themed components
- **Lucide React** - Icon library (518 icons)

### Styling
- **Tailwind CSS v4** - Utility-first CSS
- **@tailwindcss/vite** - Vite integration
- **styled-components 6.1** - CSS-in-JS (legacy)
- **class-variance-authority** - Component variants
- **tailwind-merge** - Class name merging
- **tailwindcss-animate** - Animation utilities

### State Management
- **React Context API** - Global state
- **TanStack Query 5.68** - Server state management
- **react-use** - Hook utilities

### Markdown & Code Rendering
- **react-markdown 9.0** - Markdown rendering
- **remark-gfm** - GitHub Flavored Markdown
- **remark-math** - Math support
- **rehype-katex** - LaTeX rendering
- **rehype-highlight** - Code highlighting
- **Shiki 3.2** - Syntax highlighting
- **mermaid 11.4** - Diagram rendering

### Utilities
- **axios 1.7** - HTTP client
- **zod 3.25** - Schema validation
- **date-fns 4.1** - Date utilities
- **fzf 0.5** - Fuzzy search
- **dompurify 3.2** - XSS sanitization
- **lru-cache 11.1** - Caching

### Testing
- **Vitest 3.2** - Test runner
- **@testing-library/react 16.2** - Component testing
- **@testing-library/user-event 14.6** - User interactions
- **jsdom 26.0** - DOM environment

### Internationalization
- **i18next 25.0** - i18n framework
- **react-i18next 15.4** - React bindings
- **i18next-http-backend 3.0** - Translation loading

### Performance
- **react-virtuoso 4.7** - Virtualized lists
- **debounce 2.1** - Function debouncing
- **@use-gesture/react 10.3** - Gesture handling

### Audio
- **use-sound 5.0** - Sound effects
- **Audio files** - WAV format notifications

### Special Features
- **posthog-js** - Analytics (opt-in)
- **stacktrace-js** - Error reporting
- **source-map 0.7** - Source map support
- **shell-quote** - Command escaping

## Critical Architecture Patterns

### 1. Message-Driven Architecture
```typescript
// VS Code API Wrapper
vscode.postMessage(message: WebviewMessage)
window.addEventListener('message', handleExtensionMessage)
```

### 2. Global State Container
```typescript
ExtensionStateContext (625 lines):
- 179 state properties
- Provider settings, permissions, UI state
- Task messages, history, MCP servers
- Synchronized with backend via messages
```

### 3. Component Composition
```
App.tsx
  └─ ExtensionStateContextProvider
       ├─ ChatView (main view)
       ├─ SettingsView
       ├─ HistoryView
       ├─ McpView
       ├─ MarketplaceView
       └─ ModesView
```

### 4. Virtualized Rendering
```typescript
// Chat uses react-virtuoso for performance
<Virtuoso
  data={messages}
  itemContent={(index, message) => <ChatRow message={message} />}
/>
```

### 5. Tailwind v4 Theme Integration
```css
@theme {
  --font-display: var(--vscode-font-family);
  --color-background: var(--vscode-editor-background);
  // ... 50+ VS Code variable mappings
}
```

## Key Features Implemented

### Chat Interface
- ✅ Virtualized message rendering (2000+ messages)
- ✅ Streaming text updates with partial rendering
- ✅ @mention autocomplete (files, folders, URLs, problems)
- ✅ Slash command menu (/search, /architect, etc.)
- ✅ Image upload (20 max per message)
- ✅ Code blocks with syntax highlighting (Shiki)
- ✅ Markdown rendering with LaTeX and diagrams
- ✅ Tool execution display (file operations, terminal)
- ✅ Reasoning block collapse/expand
- ✅ Context window progress indicator
- ✅ Follow-up suggestions
- ✅ Message editing and deletion
- ✅ Task timeline visualization

### Settings UI
- ✅ 40+ API provider configurations
- ✅ Model picker with pricing and context limits
- ✅ Permission toggles (read, write, execute, browser, MCP)
- ✅ Auto-approve rules for common operations
- ✅ Terminal shell integration settings
- ✅ Browser automation configuration
- ✅ Context management (auto-condense, limits)
- ✅ Checkpoint/restore settings
- ✅ Experimental features toggles
- ✅ Internationalization (language picker)

### History & Search
- ✅ Task history browser with search
- ✅ Fuzzy search (fzf)
- ✅ Task filtering and sorting
- ✅ Export to markdown
- ✅ Batch delete
- ✅ Resume from checkpoint

### MCP Integration
- ✅ MCP server configuration UI
- ✅ Marketplace browser
- ✅ Tool/resource/prompt discovery
- ✅ Connection status indicators
- ✅ Server logs viewer

### Marketplace
- ✅ Extension/MCP browse and install
- ✅ Category filtering
- ✅ Search functionality
- ✅ Version management
- ✅ Installation tracking

## Critical Dependencies for Rust Port

### Must Port (Core Functionality)
1. **Message Communication** (vscode.ts) - WebSocket replacement
2. **ExtensionStateContext** - Server-side state store
3. **ChatView** - React app (port entire component tree)
4. **Message Types** - Already in `@clean-code/types`

### Can Reuse (Keep React)
1. **All React components** - Port to standalone React app
2. **Tailwind CSS** - Works with any backend
3. **i18n files** - JSON files, language-agnostic
4. **Markdown/code rendering** - Client-side libraries

### Backend Replaces (Rust Implementation)
1. **vscode.postMessage** → WebSocket/HTTP API
2. **Extension state sync** → Server-sent events
3. **File operations** → Rust file system APIs
4. **Command execution** → Rust process spawning

## Translation Strategy

### Phase 1: Separate React App
```
Web UI (React) - localhost:3000
  ↕ WebSocket + HTTP
Rust Backend (Axum) - localhost:8080
  ↕ File System Bridge
Lapce Plugin (Minimal)
```

### Phase 2: Message Protocol
```typescript
// Current: VS Code API
vscode.postMessage({ type: "askResponse", response: "..." })

// New: WebSocket API
ws.send(JSON.stringify({ type: "askResponse", response: "..." }))
```

### Phase 3: State Synchronization
```typescript
// Current: Extension broadcasts state
message.type === "state" → setState(message)

// New: Server-sent events
eventSource.addEventListener("state", (event) => {
  setState(JSON.parse(event.data))
})
```

## File Count Summary

| Category | Count |
|----------|-------|
| **TSX Components** | ~280 files |
| **TypeScript Utils** | ~70 files |
| **CSS Files** | 3 files |
| **i18n Locales** | 242 files |
| **Test Files** | ~80 files |
| **Config Files** | 6 files |
| **TOTAL** | ~681 files |

## Lines of Code Breakdown

| Category | Lines |
|----------|-------|
| **Components** | ~58,000 |
| **Context** | ~625 |
| **Utils** | ~6,000 |
| **Hooks** | ~800 |
| **CSS** | ~15,000 |
| **i18n** | ~50,000 (JSON) |
| **Tests** | ~15,000 |
| **TOTAL** | ~145,000 |

---

**NEXT CHUNKS:**
- CHUNK 02: Chat Interface Deep Dive (ChatView, ChatRow, ChatTextArea)
- CHUNK 03: Settings UI Architecture
- CHUNK 04: State Management & Message Flow
- CHUNK 05: Styling System (Tailwind v4 + VS Code Theme)
- CHUNK 06: Component Library (shadcn/ui + Custom)
- CHUNK 07: Translation Strategy for Rust Port
