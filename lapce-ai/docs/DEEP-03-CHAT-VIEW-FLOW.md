# DEEP ANALYSIS 03: CHAT VIEW - COMPLETE MESSAGE FLOW

## 📁 Analyzed Files

```
Codex/webview-ui/src/components/chat/
├── ChatView.tsx                      (899 lines, main message UI)
│   ├── Message Processing Pipeline
│   ├── Auto-Approval Logic
│   ├── Streaming Detection
│   ├── Queue Management
│   └── Button State Handling
│
├── ChatTextArea.tsx                  (200 lines, input UI)
│   ├── Mentions System
│   ├── Slash Commands
│   └── Dynamic Resizing
│
├── ChatRow.tsx                       (450 lines, message display)
│   ├── 15 Ask Types (require response)
│   ├── 24 Say Types (informational)
│   ├── Image Display
│   └── Reasoning Toggle
│
└── Supporting Components
    ├── McpExecution.tsx              (MCP tool execution UI)
    ├── Announcement.tsx              (Notices)
    └── Task*.tsx                     (Task-related displays)

Total: ~2237 lines of chat UI logic → Rust message handlers
```

---

## Overview
ChatView is the core UI component handling AI streaming, user interactions, and message display. **2237 lines** of complex React state management.

---

## Message Flow Architecture

### 1. Message Types & State Machine

```typescript
// ClineMessage union type
type ClineMessage = {
    ts: number                    // Unique timestamp ID
    type: "ask" | "say"          // Ask = needs user input, Say = info only
    ask?: ClineAsk               // 15 possible ask types
    say?: ClineSay               // 24 possible say types
    text?: string                // JSON or markdown content
    images?: string[]            // Base64 data URLs
    partial?: boolean            // Still streaming if true
    reasoning?: string           // Hidden reasoning content
}

// Ask types (require user response)
type ClineAsk = 
    | "followup"                 // AI asks clarification question
    | "command"                  // Execute shell command
    | "command_output"           // Continue/abort running command
    | "completion_result"        // Task finished
    | "tool"                     // File operation (read/write/etc)
    | "api_req_failed"          // Retry failed API request
    | "resume_task"             // Resume paused task
    | "resume_completed_task"   // Resume finished task
    | "mistake_limit_reached"   // Too many errors
    | "browser_action_launch"   // Browser automation
    | "use_mcp_server"          // MCP tool/resource access
    | "auto_approval_max_req_reached"
    | "payment_required_prompt" // Low credits (Kilocode)
    | "report_bug"              // Report bug (Kilocode)
    | "condense"                // Condense context (Kilocode)

// Say types (informational)
type ClineSay =
    | "error"
    | "api_req_started"         // API request initiated
    | "api_req_finished"        // API request completed
    | "api_req_retried"
    | "api_req_deleted"
    | "text"                    // AI response text
    | "reasoning"               // AI reasoning (hidden by default)
    | "completion_result"       // Final result
    | "user_feedback"           // User's message
    | "command_output"          // Terminal output
    | "browser_action"
    | "browser_action_result"
    | "mcp_server_request_started"
    | "mcp_server_response"
    | "checkpoint_saved"
    | "diff_error"
    | "condense_context"
    // ... 24 total
```

---

## 2. WebSocket Message Reception

```rust
// Rust backend sends messages via WebSocket
#[derive(Serialize)]
struct MessageUpdateEvent {
    r#type: "messageUpdated",
    cline_message: ClineMessage,
}

// React frontend receives
window.addEventListener("message", (event) => {
    const message: ExtensionMessage = event.data
    
    if (message.type === "messageUpdated") {
        setState((prev) => {
            // Find existing message by timestamp
            const lastIndex = findLastIndex(
                prev.clineMessages,
                (msg) => msg.ts === message.clineMessage.ts
            )
            
            if (lastIndex !== -1) {
                // Update existing message (streaming)
                const newMessages = [...prev.clineMessages]
                newMessages[lastIndex] = message.clineMessage
                return { ...prev, clineMessages: newMessages }
            } else {
                // Append new message
                return {
                    ...prev,
                    clineMessages: [...prev.clineMessages, message.clineMessage]
                }
            }
        })
    }
})
```

---

## 3. UI State Management (ChatView.tsx lines 177-230)

```typescript
const ChatView = () => {
    // Core state
    const [inputValue, setInputValue] = useState("")
    const [sendingDisabled, setSendingDisabled] = useState(false)
    const [selectedImages, setSelectedImages] = useState<string[]>([])
    const [clineAsk, setClineAsk] = useState<ClineAsk | undefined>()
    
    // Button state
    const [enableButtons, setEnableButtons] = useState(false)
    const [primaryButtonText, setPrimaryButtonText] = useState<string>()
    const [secondaryButtonText, setSecondaryButtonText] = useState<string>()
    
    // Message queue (when sending disabled)
    const [messageQueue, setMessageQueue] = useState<QueuedMessage[]>([])
    const isProcessingQueueRef = useRef(false)
    const retryCountRef = useRef<Map<string, number>>(new Map())
    
    // Auto-approval
    const autoApproveTimeoutRef = useRef<NodeJS.Timeout | null>(null)
    const userRespondedRef = useRef<boolean>(false)
    const [currentFollowUpTs, setCurrentFollowUpTs] = useState<number | null>(null)
    
    // UI behavior
    const [expandedRows, setExpandedRows] = useState<Record<number, boolean>>({})
    const [showScrollToBottom, setShowScrollToBottom] = useState(false)
    const [isAtBottom, setIsAtBottom] = useState(false)
    const disableAutoScrollRef = useRef(false)
    
    // Virtualization
    const virtuosoRef = useRef<VirtuosoHandle>(null)
    const everVisibleMessagesTsRef = useRef<LRUCache<number, boolean>>(
        new LRUCache({ max: 100, ttl: 1000 * 60 * 5 })
    )
}
```

---

## 4. Message Processing Effect (lines 275-467)

```typescript
// Core effect: processes last message to update UI state
useDeepCompareEffect(() => {
    if (lastMessage) {
        switch (lastMessage.type) {
            case "ask":
                userRespondedRef.current = false  // Reset for new ask
                const isPartial = lastMessage.partial === true
                
                switch (lastMessage.ask) {
                    case "api_req_failed":
                        playSound("progress_loop")
                        setSendingDisabled(true)
                        setClineAsk("api_req_failed")
                        setEnableButtons(true)
                        setPrimaryButtonText("Retry")
                        setSecondaryButtonText("Start New Task")
                        break
                        
                    case "tool":
                        if (!isAutoApproved(lastMessage) && !isPartial) {
                            playSound("notification")
                            showSystemNotification("Tool Request")
                        }
                        setSendingDisabled(isPartial)
                        setClineAsk("tool")
                        setEnableButtons(!isPartial)
                        
                        const tool = JSON.parse(lastMessage.text || "{}") as ClineSayTool
                        switch (tool.tool) {
                            case "editedExistingFile":
                            case "newFileCreated":
                                setPrimaryButtonText("Save")
                                setSecondaryButtonText("Reject")
                                break
                            case "readFile":
                                if (tool.batchFiles) {
                                    setPrimaryButtonText("Approve All")
                                    setSecondaryButtonText("Deny All")
                                } else {
                                    setPrimaryButtonText("Approve")
                                    setSecondaryButtonText("Reject")
                                }
                                break
                        }
                        break
                        
                    case "command":
                        if (!isAutoApproved(lastMessage) && !isPartial) {
                            playSound("notification")
                            showSystemNotification("Command")
                        }
                        setSendingDisabled(isPartial)
                        setClineAsk("command")
                        setEnableButtons(!isPartial)
                        setPrimaryButtonText("Run Command")
                        setSecondaryButtonText("Reject")
                        break
                        
                    case "followup":
                        if (!isPartial) playSound("notification")
                        setSendingDisabled(isPartial)
                        setClineAsk("followup")
                        setEnableButtons(true)
                        // No buttons for followup, just text input
                        break
                        
                    case "completion_result":
                        if (!isPartial) playSound("celebration")
                        setSendingDisabled(isPartial)
                        setClineAsk("completion_result")
                        setEnableButtons(!isPartial)
                        setPrimaryButtonText("Start New Task")
                        break
                        
                    // ... 10 more ask types
                }
                break
                
            case "say":
                // Don't reset state, could be "say" after "ask"
                switch (lastMessage.say) {
                    case "api_req_retry_delayed":
                        setSendingDisabled(true)
                        break
                    case "api_req_started":
                        if (secondLastMessage?.ask === "command_output") {
                            // Command finished, AI processing output
                            setSendingDisabled(true)
                            setSelectedImages([])
                            setClineAsk(undefined)
                            setEnableButtons(false)
                        }
                        break
                }
                break
        }
    }
}, [lastMessage, secondLastMessage])
```

---

## 5. Auto-Approval Logic

```typescript
// Auto-approval decision (useAutoApprovalState hook)
function isAutoApproved(message: ClineMessage): boolean {
    if (!autoApprovalEnabled) return false
    if (message.ask !== "tool" && message.ask !== "command" && 
        message.ask !== "browser_action_launch" && message.ask !== "use_mcp_server") {
        return false
    }
    
    const tool = JSON.parse(message.text || "{}") as ClineSayTool
    
    // Check permission settings
    switch (message.ask) {
        case "tool":
            switch (tool.tool) {
                case "readFile":
                    if (tool.isOutsideWorkspace) {
                        return alwaysAllowReadOnlyOutsideWorkspace
                    }
                    return alwaysAllowReadOnly
                    
                case "editedExistingFile":
                case "newFileCreated":
                    if (tool.isProtected) {
                        return alwaysAllowWriteProtected
                    }
                    if (tool.isOutsideWorkspace) {
                        return alwaysAllowWriteOutsideWorkspace
                    }
                    return alwaysAllowWrite
                    
                case "execute_command":
                    return alwaysAllowExecute
            }
            break
            
        case "command":
            const command = parseCommand(message.text)
            const decision = getCommandDecision(
                command,
                allowedCommands,
                deniedCommands
            )
            return decision === CommandDecision.AutoApprove
            
        case "browser_action_launch":
            return alwaysAllowBrowser
            
        case "use_mcp_server":
            const mcpUse = JSON.parse(message.text)
            const server = mcpServers.find(s => s.name === mcpUse.serverName)
            if (server && mcpUse.type === "use_mcp_tool") {
                const tool = server.tools.find(t => t.name === mcpUse.toolName)
                return tool?.always_allow || false
            }
            return alwaysAllowMcp
    }
    
    return false
}

// Auto-approval with timeout (for followup questions)
useEffect(() => {
    if (clineAsk === "followup" && 
        alwaysAllowFollowupQuestions &&
        followupAutoApproveTimeoutMs > 0 &&
        !userRespondedRef.current) {
        
        autoApproveTimeoutRef.current = setTimeout(() => {
            if (!userRespondedRef.current) {
                // Auto-approve with empty response after timeout
                vscode.postMessage({
                    type: "askResponse",
                    askResponse: "yesButtonClicked"
                })
            }
        }, followupAutoApproveTimeoutMs)
    }
    
    return () => {
        if (autoApproveTimeoutRef.current) {
            clearTimeout(autoApproveTimeoutRef.current)
        }
    }
}, [clineAsk, alwaysAllowFollowupQuestions, followupAutoApproveTimeoutMs])
```

---

## 6. User Response Handling (lines 612-895)

```typescript
// Send message (new task or response to ask)
const handleSendMessage = useCallback((text: string, images: string[]) => {
    text = text.trim()
    
    if (text || images.length > 0) {
        // Check if sending disabled (queue message)
        if (sendingDisabled) {
            const messageId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
            setMessageQueue(prev => [...prev, { id: messageId, text, images }])
            setInputValue("")
            setSelectedImages([])
            return
        }
        
        // Mark user responded (prevents auto-approval)
        userRespondedRef.current = true
        
        if (messages.length === 0) {
            // Start new task
            vscode.postMessage({ type: "newTask", text, images })
        } else if (clineAsk) {
            // Response to ask
            if (clineAsk === "followup") {
                markFollowUpAsAnswered()
            }
            
            vscode.postMessage({
                type: "askResponse",
                askResponse: "messageResponse",
                text,
                images
            })
        } else {
            // Additional feedback in ongoing task
            vscode.postMessage({
                type: "askResponse",
                askResponse: "messageResponse",
                text,
                images
            })
        }
        
        handleChatReset()  // Clear input, disable sending
    }
}, [sendingDisabled, clineAsk, messages.length])

// Primary button (Approve/Run/Save)
const handlePrimaryButtonClick = useCallback(() => {
    userRespondedRef.current = true
    
    switch (clineAsk) {
        case "tool":
        case "command":
        case "browser_action_launch":
        case "use_mcp_server":
            vscode.postMessage({
                type: "askResponse",
                askResponse: "yesButtonClicked",
                text: inputValue.trim() || undefined,
                images: selectedImages.length > 0 ? selectedImages : undefined
            })
            break
            
        case "completion_result":
            // Start new task
            vscode.postMessage({ type: "clearTask" })
            break
            
        case "command_output":
            // Let command continue in background
            vscode.postMessage({
                type: "terminalOperation",
                terminalOperation: "continue"
            })
            break
    }
    
    setSendingDisabled(true)
    setClineAsk(undefined)
    setEnableButtons(false)
}, [clineAsk, inputValue, selectedImages])

// Secondary button (Reject/Cancel/Deny)
const handleSecondaryButtonClick = useCallback(() => {
    userRespondedRef.current = true
    
    if (isStreaming) {
        // Cancel task
        vscode.postMessage({ type: "cancelTask" })
        return
    }
    
    switch (clineAsk) {
        case "tool":
        case "command":
        case "browser_action_launch":
        case "use_mcp_server":
            vscode.postMessage({
                type: "askResponse",
                askResponse: "noButtonClicked",
                text: inputValue.trim() || undefined
            })
            break
            
        case "command_output":
            // Kill running command
            vscode.postMessage({
                type: "terminalOperation",
                terminalOperation: "abort"
            })
            break
    }
    
    setSendingDisabled(true)
    setClineAsk(undefined)
    setEnableButtons(false)
}, [clineAsk, isStreaming, inputValue])
```

---

## 7. Message Queue Processing

```typescript
// Process queued messages when task completes
useEffect(() => {
    if (sendingDisabled || 
        messageQueue.length === 0 ||
        isProcessingQueueRef.current ||
        clineAsk !== "completion_result") {
        return
    }
    
    isProcessingQueueRef.current = true
    
    const [nextMessage, ...remaining] = messageQueue
    setMessageQueue(remaining)
    
    Promise.resolve()
        .then(() => {
            handleSendMessage(nextMessage.text, nextMessage.images, true)
            retryCountRef.current.delete(nextMessage.id)
        })
        .catch((error) => {
            const retryCount = retryCountRef.current.get(nextMessage.id) || 0
            
            if (retryCount < MAX_RETRY_ATTEMPTS) {
                retryCountRef.current.set(nextMessage.id, retryCount + 1)
                setMessageQueue(current => [...current, nextMessage])
            } else {
                console.error(`Message ${nextMessage.id} failed after 3 attempts`)
                retryCountRef.current.delete(nextMessage.id)
            }
        })
        .finally(() => {
            isProcessingQueueRef.current = false
        })
}, [sendingDisabled, messageQueue, clineAsk])
```

---

## 8. Streaming Detection

```typescript
const isStreaming = useMemo(() => {
    // Check if last message is partial
    if (modifiedMessages.at(-1)?.partial === true) {
        return true
    }
    
    // Check if API request hasn't finished
    const lastApiReqStarted = findLast(
        modifiedMessages,
        (msg) => msg.say === "api_req_started"
    )
    
    if (lastApiReqStarted?.text) {
        const info = JSON.parse(lastApiReqStarted.text)
        if (info.cost === undefined) {
            return true  // API request ongoing
        }
    }
    
    // Check if waiting for tool approval
    const isLastAsk = !!modifiedMessages.at(-1)?.ask
    if (isLastAsk && clineAsk && enableButtons && primaryButtonText) {
        return false  // Waiting for user input, not streaming
    }
    
    return false
}, [modifiedMessages, clineAsk, enableButtons, primaryButtonText])
```

---

## 9. Virtualized Rendering (react-virtuoso)

```typescript
<Virtuoso
    ref={virtuosoRef}
    data={visibleMessages}
    itemContent={(index, message) => (
        <ChatRow
            message={message}
            isExpanded={expandedRows[message.ts] ?? false}
            isLast={index === visibleMessages.length - 1}
            isStreaming={isStreaming}
            onToggleExpand={(ts) => {
                setExpandedRows(prev => ({
                    ...prev,
                    [ts]: !prev[ts]
                }))
            }}
            onHeightChange={(isTaller) => {
                if (isTaller && !disableAutoScrollRef.current) {
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
/>
```

---

## Rust Backend Translation

```rust
// WebSocket message handler
pub async fn handle_chat_interaction(
    message: ClineMessage,
    state: Arc<RwLock<AppState>>,
    clients: Arc<RwLock<HashMap<Uuid, WebSocket>>>
) {
    // 1. Process message through AI
    if message.r#type == "ask" {
        // Check auto-approval
        if is_auto_approved(&message, &state.read().await) {
            // Execute immediately
            execute_action(&message).await?;
            
            // Send completion message
            send_message_update(create_say_message("completion_result"), &clients).await;
        } else {
            // Send ask message, wait for user response
            send_message_update(message, &clients).await;
        }
    }
}

// Streaming AI response
pub async fn stream_ai_response(
    request: AnthropicRequest,
    clients: Arc<RwLock<HashMap<Uuid, WebSocket>>>
) {
    let mut stream = anthropic_client.stream(request).await?;
    let ts = SystemTime::now().timestamp_millis();
    
    while let Some(chunk) = stream.next().await {
        let delta = chunk?;
        accumulated_text += &delta.text;
        
        // Send partial update
        send_message_update(ClineMessage {
            ts,
            r#type: "say",
            say: Some("text"),
            text: Some(accumulated_text.clone()),
            partial: Some(true),
        }, &clients).await;
    }
    
    // Send final complete message
    send_message_update(ClineMessage {
        ts,
        r#type: "say",
        say: Some("text"),
        text: Some(accumulated_text),
        partial: Some(false),
    }, &clients).await;
}
```

---

**STATUS:** Complete ChatView message flow analysis (2237 lines → key patterns extracted)
**NEXT:** DEEP-04-HOOKS.md - All custom React hooks analysis
