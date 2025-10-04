# UI CHUNK 02: CHAT INTERFACE DEEP DIVE

## Overview

The chat interface is the **heart of the application** - ~47 files, 15,000+ lines of React code handling real-time streaming AI conversations with sophisticated interaction patterns.

## Core Architecture

### Main Components

| Component | Lines | Purpose |
|-----------|-------|---------|
| **ChatView.tsx** | 2,237 | Main chat container, state orchestration, virtualization |
| **ChatTextArea.tsx** | 1,662 | Input area with @mentions, slash commands, images |
| **ChatRow.tsx** | 1,442 | Single message renderer with tool displays |
| **CodeIndexPopover.tsx** | 1,288 | Codebase indexing configuration UI |
| **BrowserSessionRow.tsx** | 580 | Browser automation session display |
| **UpdateTodoListToolBlock.tsx** | 455 | Todo list management interface |
| **TaskHeader.tsx** | 330 | Task metadata and controls |
| **ChatTextArea/** | Multiple | Input features (context menu, slash commands) |

### Component Hierarchy

```
ChatView (2237 lines)
├── KiloTaskHeader (task metadata)
├── HistoryPreview (task history sidebar)
├── Announcement (update notifications)
├── SystemPromptWarning (custom prompt indicator)
├── CheckpointWarning (checkpoint restore prompt)
├── QueuedMessages (pending message display)
│
├── Virtuoso (react-virtuoso)
│   └── ChatRow[] (virtualized message list)
│       ├── KiloChatRowGutterBar (timeline indicator)
│       ├── ToolUseBlock (file operations)
│       ├── CommandExecution (terminal commands)
│       ├── McpExecution (MCP tool calls)
│       ├── CodeBlock (syntax highlighted code)
│       ├── MarkdownBlock (rich text)
│       ├── ReasoningBlock (thinking process)
│       ├── UpdateTodoListToolBlock (todos)
│       ├── BatchFilePermission (multi-file approval)
│       ├── BatchDiffApproval (multi-diff review)
│       └── FollowUpSuggest (suggestion chips)
│
├── ChatTextArea (input)
│   ├── Thumbnails (image previews)
│   ├── ContextMenu (@mention autocomplete)
│   ├── SlashCommandMenu (/command autocomplete)
│   ├── KiloModeSelector (AI mode dropdown)
│   ├── ApiConfigSelector (provider selector)
│   ├── IndexingStatusBadge (codebase index status)
│   ├── ImageWarningBanner (image limit warnings)
│   └── EditModeControls (message editing)
│
├── AutoApproveMenu (permission quick toggles)
├── IdeaSuggestionsBox (starter prompts)
└── BottomControls (status bar)
```

## ChatView.tsx - Main Container (2237 lines)

### State Management (50+ state variables)

#### Message State
```typescript
// Core message data
const { clineMessages: messages } = useExtensionState()
const [clineAsk, setClineAsk] = useState<ClineAsk>()
const [enableButtons, setEnableButtons] = useState(false)
const [primaryButtonText, setPrimaryButtonText] = useState<string>()
const [secondaryButtonText, setSecondaryButtonText] = useState<string>()

// Computed message states
const modifiedMessages = useMemo(() => 
  combineApiRequests(combineCommandSequences(messages.slice(1)))
, [messages])
const visibleMessages = useMemo(() => /* filter logic */, [modifiedMessages])
const apiMetrics = useMemo(() => getApiMetrics(modifiedMessages), [modifiedMessages])
```

#### UI State
```typescript
// Input state
const [inputValue, setInputValue] = useState("")
const [selectedImages, setSelectedImages] = useState<string[]>([])
const [sendingDisabled, setSendingDisabled] = useState(false)

// Scroll state
const virtuosoRef = useRef<VirtuosoHandle>(null)
const [showScrollToBottom, setShowScrollToBottom] = useState(false)
const [isAtBottom, setIsAtBottom] = useState(false)
const disableAutoScrollRef = useRef(false)

// Row expansion
const [expandedRows, setExpandedRows] = useState<Record<number, boolean>>({})
```

#### Message Queue (Background Processing)
```typescript
const [messageQueue, setMessageQueue] = useState<QueuedMessage[]>([])
const isProcessingQueueRef = useRef(false)
const retryCountRef = useRef<Map<string, number>>(new Map())
const MAX_RETRY_ATTEMPTS = 3
```

### Message Processing Pipeline

#### 1. Message Type Detection (Lines 275-467)
```typescript
useDeepCompareEffect(() => {
  if (lastMessage?.type === "ask") {
    switch (lastMessage.ask) {
      case "tool":
        // File operation approval needed
        setSendingDisabled(true)
        setClineAsk("tool")
        setEnableButtons(true)
        setPrimaryButtonText(t("chat:save.title"))
        break
        
      case "command":
        // Terminal command approval
        setPrimaryButtonText(t("chat:runCommand.title"))
        break
        
      case "followup":
        // Follow-up question
        setSendingDisabled(false)
        setClineAsk("followup")
        break
        
      case "completion_result":
        // Task complete
        playSound("celebration")
        setPrimaryButtonText(t("chat:startNewTask.title"))
        break
    }
  }
}, [lastMessage, secondLastMessage])
```

#### 2. Auto-Approval Logic (Lines 1190-1350)
```typescript
const isAutoApproved = useCallback((message: ClineMessage) => {
  if (!autoApprovalEnabled || !hasEnabledOptions) return false
  
  switch (message.ask) {
    case "tool":
      const tool = JSON.parse(message.text).tool
      if (["readFile", "listFiles", "searchFiles"].includes(tool)) {
        return alwaysAllowReadOnly
      }
      if (["editedExistingFile", "newFileCreated"].includes(tool)) {
        return alwaysAllowWrite
      }
      break
      
    case "command":
      return isAllowedCommand(message)
      
    case "browser_action_launch":
      return alwaysAllowBrowser
      
    case "use_mcp_server":
      return alwaysAllowMcp && isMcpToolAlwaysAllowed(message)
  }
  
  return false
}, [autoApprovalEnabled, hasEnabledOptions, /* ... */])
```

#### 3. Follow-up Auto-Approval (Lines 1375-1490)
```typescript
// Auto-approve follow-up questions after timeout
useEffect(() => {
  if (clineAsk === "followup" && alwaysAllowFollowupQuestions) {
    const timeout = followupAutoApproveTimeoutMs || 5000
    
    autoApproveTimeoutRef.current = setTimeout(() => {
      if (!userRespondedRef.current && isMountedRef.current) {
        handlePrimaryButtonClick() // Auto-approve
      }
    }, timeout)
    
    return () => clearTimeout(autoApproveTimeoutRef.current)
  }
}, [clineAsk, alwaysAllowFollowupQuestions, followupAutoApproveTimeoutMs])
```

### Virtualized Rendering (react-virtuoso)

```typescript
// Virtuoso handles 1000+ messages efficiently
<Virtuoso
  ref={virtuosoRef}
  data={visibleMessages}
  itemContent={(index, message) => (
    <ChatRow
      message={message}
      isExpanded={expandedRows[message.ts] ?? false}
      isLast={index === visibleMessages.length - 1}
      isStreaming={isStreaming && index === visibleMessages.length - 1}
      onToggleExpand={(ts) => setExpandedRows(prev => ({
        ...prev,
        [ts]: !prev[ts]
      }))}
      onHeightChange={(isTaller) => {
        // Auto-scroll on height change
        if (isAtBottom && !disableAutoScrollRef.current) {
          virtuosoRef.current?.scrollToIndex({
            index: visibleMessages.length - 1,
            behavior: "smooth"
          })
        }
      }}
    />
  )}
  followOutput="smooth"
  atBottomStateChange={setIsAtBottom}
  components={{
    Footer: () => showScrollToBottom ? <ScrollToBottom /> : null
  }}
/>
```

### Message Queue System (Lines 697-754)

Handles messages sent while AI is busy:

```typescript
useEffect(() => {
  // Only process queue when task is complete
  if (sendingDisabled || messageQueue.length === 0 || 
      clineAsk !== "completion_result") {
    return
  }
  
  isProcessingQueueRef.current = true
  const [nextMessage, ...remaining] = messageQueue
  setMessageQueue(remaining)
  
  Promise.resolve()
    .then(() => handleSendMessage(nextMessage.text, nextMessage.images, true))
    .catch((error) => {
      const retryCount = retryCountRef.current.get(nextMessage.id) || 0
      
      if (retryCount < MAX_RETRY_ATTEMPTS) {
        retryCountRef.current.set(nextMessage.id, retryCount + 1)
        setMessageQueue(current => [...current, nextMessage]) // Retry
      } else {
        console.error(`Message ${nextMessage.id} failed after 3 attempts`)
      }
    })
    .finally(() => {
      isProcessingQueueRef.current = false
    })
}, [sendingDisabled, messageQueue, clineAsk])
```

### Sound Effects (use-sound)

```typescript
const [playNotification] = useSound(getAudioUrl("notification.wav"), {
  volume: soundVolume || 0.5,
  soundEnabled
})
const [playCelebration] = useSound(getAudioUrl("celebration.wav"), soundConfig)
const [playProgressLoop] = useSound(getAudioUrl("progress_loop.wav"), soundConfig)

function playSound(audioType: AudioType) {
  switch (audioType) {
    case "notification": playNotification(); break
    case "celebration": playCelebration(); break
    case "progress_loop": playProgressLoop(); break
  }
}

// Usage in message handling
case "completion_result":
  playSound("celebration")
  break
  
case "tool":
  if (!isAutoApproved(lastMessage)) {
    playSound("notification")
  }
  break
```

## ChatTextArea.tsx - Input Component (1662 lines)

### Features Implemented

#### 1. @Mention Autocomplete
```typescript
// Detect @ character
const shouldShowMenu = shouldShowContextMenu(
  inputValue,
  cursorPosition,
  mentionRegex
)

// Context menu options
const queryItems = useMemo(() => [
  { type: ContextMenuOptionType.Problems, value: "problems" },
  { type: ContextMenuOptionType.Terminal, value: "terminal" },
  ...gitCommits.map(commit => ({
    type: ContextMenuOptionType.Git,
    value: commit.hash,
    label: commit.subject
  })),
  ...openedTabs.map(tab => ({
    type: ContextMenuOptionType.OpenedFile,
    value: "/" + tab.path
  })),
  ...filePaths.map(file => ({
    type: ContextMenuOptionType.File,
    value: "/" + file
  }))
], [filePaths, gitCommits, openedTabs])

// Fuzzy search with fzf
const filteredOptions = useFzf(queryItems, searchQuery, {
  selector: (item) => item.label || item.value
})
```

#### 2. Slash Command Menu
```typescript
const showSlashCommandsMenu = shouldShowSlashCommandsMenu(inputValue, cursorPosition)

const availableCommands = useMemo(() => [
  { name: "architect", description: "Plan approach before coding" },
  { name: "code", description: "Direct implementation" },
  { name: "ask", description: "Answer questions" },
  { name: "search", description: "Search codebase" },
  // ... custom modes from settings
  ...customModes.map(mode => ({
    name: mode.slug,
    description: mode.name
  }))
], [customModes])

const matchingCommands = getMatchingSlashCommands(
  slashCommandsQuery,
  availableCommands
)
```

#### 3. Image Upload & Preview
```typescript
const MAX_IMAGES_PER_MESSAGE = 20

// Drag & drop support
const handleDrop = useCallback((e: DragEvent) => {
  e.preventDefault()
  const files = Array.from(e.dataTransfer.files)
  
  const imageFiles = files.filter(file => 
    file.type.startsWith("image/")
  )
  
  Promise.all(imageFiles.map(file => {
    return new Promise((resolve) => {
      const reader = new FileReader()
      reader.onload = () => resolve(reader.result)
      reader.readAsDataURL(file)
    })
  })).then(dataUrls => {
    setSelectedImages(prev => 
      appendImages(prev, dataUrls, MAX_IMAGES_PER_MESSAGE)
    )
  })
}, [])

// Warning banner
{imageWarning && (
  <ImageWarningBanner
    messageKey={imageWarning}
    onDismiss={dismissImageWarning}
  />
)}
```

#### 4. Mode Selector Integration
```typescript
<KiloModeSelector
  mode={mode}
  onChange={(newMode) => {
    setMode(newMode)
    vscode.postMessage({ type: "mode", text: newMode })
  }}
  modes={getAllModes(customModes)}
  disabled={selectApiConfigDisabled}
/>
```

#### 5. Prompt Enhancement
```typescript
const handleEnhancePrompt = useCallback(() => {
  const trimmedInput = inputValue.trim()
  
  if (trimmedInput) {
    setIsEnhancingPrompt(true)
    vscode.postMessage({ 
      type: "enhancePrompt",
      text: trimmedInput 
    })
  }
}, [inputValue])

// Response handler
useEffect(() => {
  const messageHandler = (event: MessageEvent) => {
    if (event.data.type === "enhancedPrompt") {
      // Preserve undo history with execCommand
      if (document.execCommand) {
        textAreaRef.current?.select()
        document.execCommand("insertText", false, event.data.text)
      } else {
        setInputValue(event.data.text)
      }
      setIsEnhancingPrompt(false)
    }
  }
  
  window.addEventListener("message", messageHandler)
  return () => window.removeEventListener("message", messageHandler)
}, [])
```

#### 6. Prompt History Navigation
```typescript
const { handleHistoryNavigation } = usePromptHistory({
  clineMessages,
  taskHistory,
  cwd,
  inputValue,
  setInputValue
})

// Keyboard shortcuts
const handleKeyDown = (e: KeyboardEvent) => {
  // Arrow up/down for history
  if (e.key === "ArrowUp" && e.altKey) {
    handleHistoryNavigation("up")
  } else if (e.key === "ArrowDown" && e.altKey) {
    handleHistoryNavigation("down")
  }
  
  // Ctrl+Enter to send
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    onSend()
  }
}
```

## ChatRow.tsx - Message Renderer (1442 lines)

### Message Types Rendered

| Type | Component | Purpose |
|------|-----------|---------|
| **task** | TaskHeader | Task initialization message |
| **say:text** | MarkdownBlock | AI text response with markdown |
| **say:tool** | ToolUseBlock | File operation details |
| **say:command_output** | CommandExecution | Terminal output display |
| **say:browser_action** | BrowserSessionRow | Browser automation steps |
| **say:mcp_server_response** | McpExecution | MCP tool call results |
| **say:api_req_started** | API metrics | Token count, cost, model |
| **say:reasoning** | ReasoningBlock | AI thinking process (Claude) |
| **ask:tool** | ToolUseBlock + Buttons | File operation approval |
| **ask:command** | CommandExecution + Buttons | Command approval |
| **ask:followup** | Input field | Follow-up question |
| **ask:completion_result** | Summary + Button | Task complete |

### Tool Rendering Examples

#### File Operations
```typescript
case "editedExistingFile":
case "appliedDiff":
case "newFileCreated":
  return (
    <ToolUseBlock
      header={
        <ToolUseBlockHeader
          icon="file-edit"
          title={`${tool.tool === "newFileCreated" ? "Created" : "Edited"} ${tool.path}`}
        />
      }
    >
      <CodeAccordian
        diff={tool.diff}
        language={getLanguageFromPath(tool.path)}
        isExpanded={isExpanded}
        onToggle={() => onToggleExpand(message.ts)}
      />
      
      {!isAutoApproved && (
        <div className="button-group">
          <Button onClick={() => handleApprove()}>
            {t("chat:save.title")}
          </Button>
          <Button variant="secondary" onClick={() => handleReject()}>
            {t("chat:reject.title")}
          </Button>
        </div>
      )}
    </ToolUseBlock>
  )
```

#### Terminal Commands
```typescript
case "command":
  const { command, commandPrefix } = JSON.parse(message.text)
  
  return (
    <CommandExecution
      command={command}
      output={commandOutput}
      isRunning={isStreaming}
      exitCode={exitCode}
      isAutoApproved={isAutoApproved(message)}
      isDenied={isDeniedCommand(message)}
      deniedPrefix={getDeniedPrefix(command)}
    >
      {!isAutoApproved && !isDenied && (
        <Button onClick={handleRunCommand}>
          {t("chat:runCommand.title")}
        </Button>
      )}
    </CommandExecution>
  )
```

#### Batch File Operations
```typescript
case "readFile":
  if (tool.batchFiles && Array.isArray(tool.batchFiles)) {
    return (
      <BatchFilePermission
        files={tool.batchFiles}
        onResponse={(response) => {
          // response: { [filePath: string]: boolean }
          onBatchFileResponse?.(response)
        }}
      />
    )
  }
```

### Message Height Tracking

```typescript
const [chatrow, { height }] = useSize(
  <div className="chat-row">
    <ChatRowContent {...props} />
  </div>
)

useEffect(() => {
  if (isLast && height !== prevHeightRef.current) {
    // Notify parent of height change for scroll adjustment
    onHeightChange(height > prevHeightRef.current)
    prevHeightRef.current = height
  }
}, [height, isLast])
```

## Performance Optimizations

### 1. Memo & Deep Comparison
```typescript
const ChatRow = memo((props: ChatRowProps) => {
  // Component implementation
}, deepEqual) // Deep comparison for arrays/objects
```

### 2. LRU Cache for Visibility
```typescript
const everVisibleMessagesTsRef = useRef<LRUCache<number, boolean>>(
  new LRUCache({
    max: 100,      // Keep last 100 messages
    ttl: 1000 * 60 * 5  // 5 minute TTL
  })
)

// Only render messages that have been visible
const visibleMessages = useMemo(() => {
  return modifiedMessages.filter((message) => {
    if (everVisibleMessagesTsRef.current.has(message.ts)) {
      return true
    }
    // ... visibility logic
  })
}, [modifiedMessages])
```

### 3. Debounced Focus
```typescript
useDebounceEffect(() => {
  if (!isHidden && !sendingDisabled && !enableButtons) {
    textAreaRef.current?.focus()
  }
}, 50, [isHidden, sendingDisabled, enableButtons])
```

### 4. Virtualized List (Virtuoso)
- Renders only visible messages
- Handles 1000+ messages smoothly
- Automatic scroll tracking
- Dynamic height support

## Key Interaction Patterns

### 1. Streaming Updates
```typescript
const isStreaming = useMemo(() => {
  const isLastMessagePartial = modifiedMessages.at(-1)?.partial === true
  if (isLastMessagePartial) return true
  
  // Check if API request is still pending
  const lastApiReq = findLast(modifiedMessages, 
    m => m.say === "api_req_started"
  )
  return lastApiReq && !lastApiReq.cost // No cost = not finished
}, [modifiedMessages])
```

### 2. Context Window Progress
```typescript
<ContextWindowProgress
  inputTokens={apiMetrics.totalInputTokens}
  outputTokens={apiMetrics.totalOutputTokens}
  contextLimit={model?.info.maxTokens || 128000}
  onCondense={() => {
    vscode.postMessage({ type: "condense" })
  }}
/>
```

### 3. Todo List Updates
```typescript
const latestTodos = useMemo(() => {
  // Initial todos from task state
  if (currentTaskTodos?.length > 0) {
    const messageBasedTodos = getLatestTodo(messages)
    // Message-based takes precedence
    return messageBasedTodos?.length > 0 
      ? messageBasedTodos 
      : currentTaskTodos
  }
  return getLatestTodo(messages)
}, [messages, currentTaskTodos])
```

## Translation Notes for Rust Port

### Keep React (Client-Side)
- ✅ All React components stay
- ✅ Virtuoso for performance
- ✅ Markdown/code rendering
- ✅ Sound effects (Web Audio API)

### Replace with Rust Backend
```typescript
// Current: VS Code API
vscode.postMessage({ type: "newTask", text, images })
vscode.postMessage({ type: "askResponse", askResponse: "yesButtonClicked" })

// New: WebSocket API
ws.send(JSON.stringify({ type: "newTask", text, images }))
ws.send(JSON.stringify({ type: "askResponse", askResponse: "yesButtonClicked" }))
```

### State Synchronization
```typescript
// Current: Extension broadcasts state
case "state":
  setState(message)
  break

// New: Server-sent events
eventSource.addEventListener("state", (event) => {
  setState(JSON.parse(event.data))
})

// WebSocket alternative
ws.onmessage = (event) => {
  const message = JSON.parse(event.data)
  if (message.type === "state") {
    setState(message)
  }
}
```

---

**NEXT CHUNK:** Settings UI Architecture (43 files, 11,000 lines)
