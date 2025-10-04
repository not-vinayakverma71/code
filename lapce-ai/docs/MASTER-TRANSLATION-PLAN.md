# MASTER TRANSLATION PLAN: CODEX → LAPCE AI

## Executive Summary
**Total Files:** 843 (711 TS + 132 JSON)  
**Total Lines:** ~66,400 lines of TypeScript code  
**Total Tasks:** 150 granular tasks (expanded from 95)  
**Estimated Timeline:** 10-14 weeks (8-10 with parallel work)  
**Critical Path:** Webview → Lapce UI architecture redesign  
**Critical Blockers:** 5 major architectural decisions identified

## Architecture Decision: HYBRID APPROACH

Given Lapce's limited plugin API, we adopt a **three-tier architecture**:

```
┌─────────────────────────────────────────┐
│  WEB UI (TypeScript/React)              │
│  - Port existing Codex UI               │
│  - Runs on localhost:3000               │
│  - WebSocket to backend                 │
└─────────────────────────────────────────┘
                  ↕ HTTP/WS
┌─────────────────────────────────────────┐
│  CORE BACKEND (Rust)                    │
│  - Task execution engine                │
│  - API provider management              │
│  - Tool execution                       │
│  - State persistence                    │
└─────────────────────────────────────────┘
                  ↕ IPC
┌─────────────────────────────────────────┐
│  LAPCE PLUGIN (Minimal Rust)            │
│  - Command registration                 │
│  - File system bridge                   │
│  - Launch UI/backend                    │
└─────────────────────────────────────────┘
```

---

## PHASE 1: FOUNDATION (Weeks 1-2) - 25 Tasks

### Core Types & Serialization (5 tasks)
- [ ] **T001:** Port `@clean-code/types` package → `src/types/`
  - All message types (ClineMessage, ExtensionMessage, WebviewMessage)
  - All configuration types (ProviderSettings, ModelInfo)
  - All task types (TaskMetadata, HistoryItem, TokenUsage)
  - Ensure exact field name matching for JSON compatibility

- [ ] **T002:** Create Rust serialization layer with serde
  - Derive Serialize/Deserialize for all types
  - Custom serializers for compatibility (camelCase ↔ snake_case)
  - Validation with JSON schema generation

- [ ] **T003:** Port shared utilities
  - String manipulation (kebab-case, camelCase, PascalCase)
  - Path utilities (workspace detection, relative paths)
  - Array helpers (findLast, chunk)
  - Error serialization

- [ ] **T004:** Create error type hierarchy
  - TaskError, ApiError, ToolError, ConfigError
  - Error context propagation
  - User-friendly error messages

- [ ] **T005:** Set up logging & telemetry stubs
  - tracing setup with multiple levels
  - Metrics collection points
  - Telemetry event definitions

### XML Parser & Core Utilities (5 tasks)
- [ ] **T021:** Implement XmlMatcher for streaming XML parsing
  - State machine: TEXT, TAG_OPEN, TAG_CLOSE
  - Character-by-character parsing
  - Depth tracking for nested tags
  - Test with <thinking> and tool tags

- [ ] **T022:** Implement path utilities
  - Cross-platform path normalization
  - toPosixPath for display
  - arePathsEqual (case-insensitive on Windows)
  - getReadablePath for user-friendly paths

- [ ] **T023:** Implement git utilities with git2
  - Repository info extraction
  - Commit search
  - URL sanitization
  - Branch detection

- [ ] **T024:** Implement shell detection
  - Windows: PowerShell 7 vs legacy
  - macOS: zsh detection
  - Linux: bash detection
  - Environment variable support

- [ ] **T025:** Implement file system utilities
  - createDirectoriesForFile
  - readDirectory with exclusions
  - fileExistsAtPath
  - isDirectory checks

### Storage & State (5 tasks)
- [ ] **T006:** Implement global state management
  - JSON file-based storage
  - Arc<RwLock<GlobalState>> pattern
  - Atomic updates with file locking

- [ ] **T007:** Implement task persistence
  - Save/load API conversation history
  - Save/load UI messages
  - Task metadata indexing

- [ ] **T008:** Implement configuration management
  - TOML config file parsing
  - Environment variable overrides
  - Hot reload support

- [ ] **T009:** Implement secrets management
  - System keyring integration (keyring crate)
  - Encrypted storage fallback
  - API key validation

- [ ] **T010:** Create history management
  - Task history with search
  - Favorites/bookmarks
  - Export/import functionality

### API Foundation (5 tasks)
- [ ] **T011:** Create ApiHandler trait
  - createMessage() signature
  - getModel() implementation
  - countTokens() with tiktoken

- [ ] **T012:** Implement ApiStream type
  - Pin<Box<dyn Stream<Item = Result<ApiStreamChunk>>>>
  - Chunk types: Text, Usage, ToolUse, Error
  - Backpressure handling

- [ ] **T013:** Create BaseProvider
  - Common token counting
  - Common error handling
  - Common timeout logic

- [ ] **T014:** Implement streaming adapters
  - AnthropicStreamAdapter
  - OpenAiStreamAdapter
  - GeminiStreamAdapter

- [ ] **T015:** Create provider factory
  - buildApiHandler() function
  - Provider registration system
  - Dynamic provider loading

### Message Format Conversion (5 tasks)
- [ ] **T016:** Implement Anthropic → OpenAI converter
  - Message role mapping
  - Content block conversion
  - Tool call transformation

- [ ] **T017:** Implement Anthropic → Gemini converter
  - "assistant" → "model" role
  - Image format conversion
  - Function call mapping

- [ ] **T018:** Implement Anthropic → Bedrock converter
  - AWS SDK message format
  - Converse API support

- [ ] **T019:** Implement R1 format converter (DeepSeek)
  - Reasoning block handling
  - <think> tag wrapping

- [ ] **T020:** Create format converter tests
  - Golden test files for each converter
  - Round-trip conversion tests
  - Edge case handling

---

## PHASE 2: API PROVIDERS (Weeks 3-4) - 20 Tasks

### Priority Providers (8 tasks)
- [ ] **T021:** Implement AnthropicHandler
  - Official SDK integration (anthropic crate)
  - Prompt caching support
  - Streaming with usage tracking

- [ ] **T022:** Implement OpenAiHandler
  - async-openai crate
  - Azure OpenAI support
  - O1/O3 special handling

- [ ] **T023:** Implement OpenRouterHandler
  - Multi-provider routing
  - Model fallback logic
  - Cost optimization

- [ ] **T024:** Implement BedrockHandler
  - AWS SDK integration
  - Cross-region inference
  - IAM authentication

- [ ] **T025:** Implement GeminiHandler
  - Google GenAI SDK
  - Multimodal support
  - Safety settings

- [ ] **T026:** Implement OllamaHandler
  - Local model support
  - Model discovery
  - Streaming optimization

- [ ] **T027:** Implement GroqHandler
  - Ultra-fast inference
  - Token usage tracking

- [ ] **T028:** Implement LmStudioHandler
  - Local server discovery
  - Model switching

### Provider Infrastructure (7 tasks)
- [ ] **T029:** Implement timeout configuration
  - Per-provider timeouts
  - Request retry logic
  - Exponential backoff

- [ ] **T030:** Implement rate limiting
  - Token bucket algorithm
  - Per-provider limits
  - Queue management

- [ ] **T031:** Implement token counting
  - tiktoken integration
  - Provider-specific counters
  - Cache-aware counting

- [ ] **T032:** Implement prompt caching
  - Cache strategy system
  - Multi-point caching
  - Cache analytics

- [ ] **T033:** Implement cost calculation
  - Per-provider pricing
  - Cache cost calculation
  - Usage analytics

- [ ] **T034:** Create provider tests
  - Mock server tests
  - Integration tests
  - Streaming tests

- [ ] **T035:** Create model registry with pricing
  - ModelInfo struct with all fields
  - get_cerebras_models, get_anthropic_models, etc.
  - ModelRegistry builder pattern
  - Model lookup by provider + ID

- [ ] **T036:** Implement Cerebras provider
  - HTTP client with Cerebras API
  - Model configuration (llama3.1-8b, etc.)

- [ ] **T037:** Implement DeepSeek provider
  - R1 reasoning format support
  - <think> tag wrapping

- [ ] **T038:** Implement Gemini Flash/Pro variants
  - Different model tiers
  - Multimodal support

- [ ] **T039:** Implement remaining 28 providers (batched)
  - Use BaseProvider + HTTP client template
  - Provider-specific quirks documented
  - Minimal viable implementations

- [ ] **T040:** Create provider integration tests
  - Mock server for each provider
  - Streaming response tests
  - Error handling tests
  - Token counting validation

---

## PHASE 3: TOOL SYSTEM (Weeks 5-6) - 25 Tasks

### Tool Infrastructure (5 tasks)
- [ ] **T036:** Create Tool trait system
  - execute() signature
  - validate() implementation
  - Permission checking

- [ ] **T037:** Implement ToolRegistry
  - Dynamic tool registration
  - Tool discovery
  - Capability negotiation

- [ ] **T038:** Create tool parameter parsing
  - XML parsing from AI output
  - Parameter validation
  - Type conversion

- [ ] **T039:** Implement tool result formatting
  - Success/error responses
  - Metadata inclusion
  - Image embedding

- [ ] **T040:** Create tool permission system
  - Ask approval flow
  - Auto-approval rules
  - Permission caching

### Core Tools (10 tasks)
- [ ] **T041:** Implement read_file tool
  - Multi-file reading
  - Line range support
  - Binary file handling (PDF, DOCX, images)

- [ ] **T042:** Implement write_to_file tool
  - File creation/modification
  - Line number stripping
  - Code omission detection

- [ ] **T043:** Implement execute_command tool
  - Command execution via tokio::process
  - Output streaming
  - Timeout handling

- [ ] **T044:** Implement apply_diff tool
  - Unified diff parsing
  - Search/replace strategy
  - Fuzzy matching

- [ ] **T045:** Implement search_files tool
  - Ripgrep integration
  - Glob pattern support
  - Result limiting

- [ ] **T046:** Implement list_files tool
  - Directory traversal
  - .gitignore support
  - Metadata collection

- [ ] **T047:** Implement codebase_search tool
  - Semantic search (if indexing available)
  - Fallback to grep
  - Result ranking

- [ ] **T048:** Implement ask_followup_question tool
  - Question formatting
  - Response handling
  - History tracking

- [ ] **T049:** Implement attempt_completion tool
  - Task completion logic
  - Result summarization
  - Success criteria

- [ ] **T050:** Implement update_todo_list tool
  - TODO list management
  - Status tracking
  - Persistence

### Advanced Tools (5 tasks)
- [ ] **T051:** Implement browser_action tool
  - headless_chrome integration
  - Screenshot capture
  - Content extraction

- [ ] **T052:** Implement mcp_tool execution
  - MCP protocol implementation
  - Dynamic tool discovery
  - Stdio communication

- [ ] **T053:** Implement new_task tool (subtasks)
  - Subtask spawning
  - Parent task pausing
  - Result propagation

- [ ] **T054:** Implement checkpoint tools
  - Git checkpoint creation
  - Checkpoint restoration
  - Diff viewing

- [ ] **T055:** Implement read_lines utility
  - Line range reading
  - Efficient for large files
  - Line counting

### Binary File Extraction (5 tasks)
- [ ] **T056:** Implement PDF text extraction
  - pdf-extract crate
  - Line numbering
  - Error handling

- [ ] **T057:** Implement DOCX text extraction
  - docx-rs crate
  - Paragraph extraction
  - Line numbering

- [ ] **T058:** Implement XLSX text extraction
  - calamine crate
  - Sheet iteration
  - Cell value formatting

- [ ] **T059:** Implement IPYNB text extraction
  - JSON parsing
  - Code cell extraction
  - Markdown cell extraction

- [ ] **T060:** Implement image processing
  - Base64 encoding/decoding
  - URL download
  - Media type detection
  - Image resizing (optional)

### Tool Tests (5 tasks)
- [ ] **T061:** Create tool unit tests
  - Test each tool in isolation
  - Mock file system
  - Mock process execution

- [ ] **T062:** Create tool integration tests
  - Full tool execution
  - Real file operations
  - Permission flow

- [ ] **T063:** Create tool permission tests
  - Ask approval flow
  - Auto-approval rules
  - Permission caching

- [ ] **T064:** Create tool error handling tests
  - Invalid parameters
  - File not found
  - Permission denied

- [ ] **T065:** Create tool performance tests
  - Large file handling
  - Concurrent tool execution
  - Memory usage

---

## PHASE 4: TASK ENGINE (Week 7) - 20 Tasks

### Task State Machine (5 tasks)
- [ ] **T056:** Implement Task struct
  - EventEmitter pattern with broadcast channel
  - State flags (abort, paused, streaming, etc.)
  - Conversation history

- [ ] **T057:** Implement task lifecycle
  - startTask()
  - resumeTaskFromHistory()
  - abortTask()
  - dispose()

- [ ] **T058:** Implement recursivelyMakeClineRequests()
  - Iterative loop with stack
  - Mistake limit checking
  - Tool execution orchestration

- [ ] **T059:** Implement streaming message handling
  - Real-time chunk processing
  - presentAssistantMessage()
  - Partial message updates

- [ ] **T060:** Implement ask() system
  - Permission requests
  - User interaction blocking
  - Response handling

### Task Management (5 tasks)
- [ ] **T061:** Implement TaskProvider
  - Task stack management
  - Subtask support
  - Task switching

- [ ] **T062:** Implement context window management
  - Message truncation
  - Sliding window
  - Forced reduction on errors

- [ ] **T063:** Implement error recovery
  - Consecutive mistake tracking
  - Auto-retry logic
  - User guidance

- [ ] **T089:** Implement task history
  - History storage
  - Search/filter
  - Favorites

- [ ] **T090:** Implement task switching
  - Task stack management
  - Subtask support
  - Task switching logic

- [ ] **T091:** Implement task tests
  - State machine tests
  - Streaming tests
  - Error recovery tests

- [ ] **T067:** Implement file context tracking
  - Open file tracking
  - Modified file tracking
  - Context window optimization

- [ ] **T068:** Implement diff view provider
  - Diff generation
  - Side-by-side display (via external tool)
  - Accept/reject changes

- [ ] **T069:** Implement terminal integration
  - Shell command execution
  - Output capturing
  - Shell integration fallback

- [ ] **T070:** Implement checkpoint service
  - Git-based versioning
  - Shadow workspace
  - Diff computation

---

## PHASE 5: UI LAYER (Weeks 8-9) - 20 Tasks

### Web UI (8 tasks)
- [ ] **T071:** Set up Vite + React project
  - TypeScript configuration
  - Component structure
  - Routing

- [ ] **T072:** Port message list component
  - Message rendering
  - Streaming updates
  - Virtualization

- [ ] **T073:** Port task input component
  - Text input with mentions
  - Image upload
  - Command palette

- [ ] **T074:** Port settings panel
  - API configuration
  - Model selection
  - Experiment toggles

- [ ] **T075:** Port task history sidebar
  - History list
  - Search/filter
  - Quick actions

- [ ] **T076:** Implement WebSocket client
  - Real-time updates
  - Reconnection logic
  - Message queuing

- [ ] **T077:** Implement state synchronization
  - Redux/Zustand store
  - Optimistic updates
  - Conflict resolution

- [ ] **T078:** Create UI tests
  - Component tests
  - Integration tests
  - E2E tests

### Backend HTTP API (7 tasks)
- [ ] **T079:** Set up Axum web server
  - Route definitions
  - Middleware (CORS, logging)
  - Error handling

- [ ] **T080:** Implement REST endpoints
  - POST /api/tasks - Create task
  - GET /api/tasks - List tasks
  - GET /api/tasks/:id - Get task
  - POST /api/tasks/:id/ask - Respond to ask

- [ ] **T081:** Implement WebSocket handler
  - Connection management
  - Message broadcasting
  - Authentication

- [ ] **T082:** Implement state endpoints
  - GET /api/state - Global state
  - PATCH /api/state - Update state
  - POST /api/config - Update config

- [ ] **T083:** Implement history endpoints
  - GET /api/history - Task history
  - GET /api/history/:id - Get task messages
  - DELETE /api/history/:id - Delete task

- [ ] **T084:** Implement export endpoints
  - GET /api/export/:id - Export markdown
  - POST /api/import - Import task

- [ ] **T085:** Create API tests
  - Integration tests
  - Load tests
  - Security tests

### WebSocket & Real-time Sync (5 tasks)
- [ ] **T086:** Implement WebSocket handler
  - Connection management
  - Per-client state tracking
  - Heartbeat/keepalive
  - Reconnection handling

- [ ] **T087:** Implement message broadcasting
  - Broadcast to all clients
  - Broadcast to specific client
  - Message queuing
  - Delivery confirmation

- [ ] **T088:** Implement state synchronization
  - Full state sync on connect
  - Incremental updates
  - Conflict resolution
  - Optimistic updates

- [ ] **T089:** Implement real-time streaming
  - Stream API responses to UI
  - Partial message updates
  - Token usage updates
  - Progress indicators

- [ ] **T090:** Create WebSocket tests
  - Connection lifecycle
  - Message delivery
  - Reconnection scenarios
  - Concurrent clients

---

## PHASE 6: LAPCE INTEGRATION (Week 10) - 10 Tasks

### Minimal Plugin (5 tasks)
- [ ] **T086:** Create Lapce plugin scaffold
  - Plugin manifest
  - Basic initialization
  - Command registration

- [ ] **T087:** Implement command handlers
  - "Start Task" command
  - "Open UI" command
  - "Show History" command

- [ ] **T088:** Implement file system bridge
  - Read file command
  - Write file command
  - List files command

- [ ] **T089:** Implement backend launcher
  - Start HTTP server on activation
  - Port detection
  - Health checks

- [ ] **T095:** Create plugin package
  - Build script
  - Distribution package
  - Installation instructions

### File Bridge & IPC (5 tasks)
- [ ] **T096:** Implement file system bridge
  - Lapce → Backend file operations
  - Read file requests
  - Write file requests
  - List directory requests

- [ ] **T097:** Implement backend launcher
  - Start HTTP server on plugin activation
  - Port detection and management
  - Health checks
  - Automatic restart on crash

- [ ] **T098:** Implement IPC protocol
  - Message format definition
  - Request/response handling
  - Event notifications
  - Error propagation

- [ ] **T099:** Implement configuration sync
  - Read Lapce config
  - Sync with backend
  - Hot reload support
  - Config validation

- [ ] **T100:** Create plugin tests
  - Command execution
  - File bridge operations
  - Backend communication
  - Error scenarios

---

## PHASE 7: SERVICES & UTILITIES (Week 11) - 15 Tasks

### Git Operations (5 tasks)
- [ ] **T101:** Implement repository info extraction
  - Parse .git/config
  - Extract remote URL
  - Detect default branch
  - Repository name extraction

- [ ] **T102:** Implement commit search
  - git2 revwalk
  - Message search
  - Author filtering
  - Date range filtering

- [ ] **T103:** Implement git diff generation
  - Diff between commits
  - Staged changes
  - Working directory changes
  - Unified diff format

- [ ] **T104:** Implement git operations
  - Stage files
  - Create commits
  - Push/pull (basic)
  - Branch operations

- [ ] **T105:** Create git tests
  - Repository detection
  - Commit creation
  - Search functionality
  - Error handling

### Terminal Integration (5 tasks)
- [ ] **T106:** Implement terminal command execution
  - tokio::process::Command
  - Shell detection and selection
  - Working directory support
  - Environment variables

- [ ] **T107:** Implement output streaming
  - Real-time stdout capture
  - Real-time stderr capture
  - Line-by-line processing
  - ANSI code stripping

- [ ] **T108:** Implement exit code handling
  - Process status detection
  - Success/failure determination
  - Signal handling
  - Timeout support

- [ ] **T109:** Implement cross-platform shell support
  - PowerShell on Windows
  - bash/zsh on macOS/Linux
  - Shell detection
  - Command escaping

- [ ] **T110:** Create terminal tests
  - Command execution
  - Output capture
  - Error handling
  - Cross-platform compatibility

### File Extraction Utilities (5 tasks)
- [ ] **T111:** Implement binary file detection
  - Magic number checking
  - Extension-based detection
  - Content analysis
  - Error handling

- [ ] **T112:** Implement line counter
  - Efficient line counting
  - Large file support
  - Memory-efficient streaming
  - Progress reporting

- [ ] **T113:** Implement line range reader
  - Start/end line support
  - Efficient seeking
  - Memory-efficient
  - Line number tracking

- [ ] **T114:** Implement truncation utilities
  - Character limit truncation
  - Line limit truncation
  - Smart truncation (preserve structure)
  - Truncation indicators

- [ ] **T115:** Create extraction tests
  - Each file format
  - Large file handling
  - Error cases
  - Performance benchmarks

---

## PHASE 8: TESTING & REFINEMENT (Week 12) - 15 Tasks

### Integration Testing (5 tasks)
- [ ] **T116:** Create end-to-end test suite
  - Full task execution (create file → modify → complete)
  - Multi-tool task scenarios
  - Error recovery flows
  - Permission approval flows

- [ ] **T117:** Create multi-provider tests
  - Test each of 8 priority providers
  - Streaming correctness
  - Token counting accuracy
  - Cost calculation validation

- [ ] **T118:** Create error scenario tests
  - API timeout handling
  - Network failures
  - Invalid responses
  - Quota exceeded

- [ ] **T119:** Create compatibility tests
  - Compare outputs with original Codex
  - Message format validation (JSON schemas)
  - API compatibility checks
  - Tool behavior parity

- [ ] **T120:** Create concurrency tests
  - Multiple simultaneous tasks
  - Concurrent tool execution
  - Race condition detection
  - Resource contention

### Performance & Optimization (5 tasks)
- [ ] **T121:** Create performance benchmarks
  - Streaming latency measurement
  - Token throughput (tokens/sec)
  - Memory usage profiling
  - Tool execution time

- [ ] **T122:** Optimize streaming path
  - Minimize allocations
  - Efficient chunk processing
  - Backpressure handling
  - Buffer sizing

- [ ] **T123:** Optimize file operations
  - Async I/O everywhere
  - Concurrent file reads
  - Efficient large file handling
  - Memory-mapped files (where appropriate)

- [ ] **T124:** Profile and optimize hot paths
  - Flamegraph analysis
  - Identify bottlenecks
  - Cache improvements
  - Algorithm optimization

- [ ] **T125:** Create load tests
  - Sustained task execution
  - Peak load handling
  - Resource cleanup validation
  - Memory leak detection

### Documentation & Polish (5 tasks)
- [ ] **T126:** Create user documentation
  - Installation guide (Lapce plugin + backend)
  - Configuration guide (API keys, settings)
  - Usage examples (common tasks)
  - Troubleshooting guide

- [ ] **T127:** Create developer documentation
  - Architecture overview (hybrid approach)
  - API reference (HTTP endpoints, WebSocket)
  - Type reference (all message types)
  - Contributing guide

- [ ] **T128:** Create deployment guide
  - Building from source
  - Packaging for distribution
  - Platform-specific notes
  - Version management

- [ ] **T129:** Polish UI/UX
  - Error messages clarity
  - Loading states
  - Progress indicators
  - Keyboard shortcuts

- [ ] **T130:** Final integration polish
  - Code cleanup
  - Remove debug logging
  - Version bumps
  - Release notes

---

## CRITICAL PATH ANALYSIS

**Longest dependency chain:**
```
T001-T005 → T011-T015 → T021-T028 → T036-T050 → T056-T070 → T071-T085 → T086-T090
Foundation → API → Providers → Tools → Task Engine → UI → Lapce Plugin
```

**Estimated timeline: 10 weeks**

## RISK MITIGATION

### Risk 1: Webview Replacement Complexity
**Mitigation:** Adopt hybrid architecture with separate web UI

### Risk 2: Provider SDK Availability
**Mitigation:** Implement HTTP client fallback for all providers

### Risk 3: Lapce Plugin API Limitations
**Mitigation:** Keep plugin minimal, move logic to backend

### Risk 4: Performance Degradation
**Mitigation:** Benchmark early, optimize streaming path

### Risk 5: Type System Mismatch
**Mitigation:** JSON schema validation, integration tests

## SUCCESS CRITERIA

1. ✅ **Functional Parity:** All 20+ core tools working
2. ✅ **Provider Support:** Top 8 providers (Anthropic, OpenAI, etc.)
3. ✅ **Performance:** Streaming latency < 100ms
4. ✅ **Reliability:** Error recovery without data loss
5. ✅ **Usability:** UI launches and connects automatically

## DEFERRED FEATURES (v2.0)

- Browser automation (Puppeteer)
- Advanced code indexing (embeddings)
- MCP server implementations (beyond basic)
- VS Code LM provider
- Legacy provider support (< 5% usage)

---

## SUMMARY

**TOTAL: 150 Tasks across 8 phases (12 weeks)**

### Phase Breakdown
- Phase 1 (Foundation): 25 tasks, Weeks 1-2
- Phase 2 (API Providers): 20 tasks, Weeks 3-4
- Phase 3 (Tool System): 25 tasks, Weeks 5-6
- Phase 4 (Task Engine): 20 tasks, Week 7
- Phase 5 (UI Layer): 20 tasks, Weeks 8-9
- Phase 6 (Lapce Integration): 10 tasks, Week 10
- Phase 7 (Services): 15 tasks, Week 11
- Phase 8 (Testing & Polish): 15 tasks, Week 12

### Parallel Work Possible
- Stream A: Types → API → Task Engine (4 weeks)
- Stream B: Tools → File Ops → Terminal (4 weeks)
- Stream C: Web UI → HTTP API → WebSocket (4 weeks)
- Stream D: Services (Git, extraction) (2 weeks)

**With 2-3 developers: 8-10 weeks**

**Ready for execution with clear dependencies and acceptance criteria**
