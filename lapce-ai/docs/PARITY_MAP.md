# Codex ↔ Lapce‑AI Tools Parity Map (pre‑IPC)

**Last Updated:** 2025-10-09

**Scope:** Tools runnable without the IPC bridge. 
- Codex source: `Codex/src/core/tools/` (27 TypeScript tools)
- Rust tools: `lapce-ai/src/core/tools/` (20 Rust tools)

**Legend:** 
- **Parity Status**: Full | Partial | Different | None
- **Full**: Complete functional equivalence
- **Partial**: Core functionality present but missing some features
- **Different**: Different approach to similar problem
- **None**: No equivalent tool on the other side

---

## Core File System Tools

### **read_file**
- **Codex**: `readFileTool.ts` - multi-file XML args, approvals, RooIgnore, images/binary handling, optional line ranges
- **Rust**: `fs/read_file.rs` (`readFile`) - single file JSON-based reader
- **Params**: path, optional lineStart/lineEnd
- **Output**: JSON { path, content, lineStart, lineEnd }
- **Parity**: **Partial** - Codex supports multi-file batch + rich XML output; Rust is single-file JSON

### **simple_read_file** 
- **Codex**: `simpleReadFileTool.ts` - simplified single-file reader for models with limited XML support
- **Rust**: None (use standard `read_file`)
- **Params**: path only
- **Output**: file content with line numbers
- **Parity**: **None** - Codex-only simplified variant

### **write_to_file**
- **Codex**: `writeToFileTool.ts` - diff view streaming, requires `line_count`, approvals
- **Rust**: `fs/write_file.rs` (`writeFile`) - file writer with directory creation
- **Params**: path, content, createDirs?
- **Output**: JSON { path, operation(create|overwrite), bytesWritten }
- **Parity**: **Partial** - Rust doesn't require `line_count`; pre-processes code fences/line numbers

### **edit_file**
- **Codex**: `editFileTool.ts` - Fast Apply via Morph API, diff UI, approvals
- **Rust**: `fs/edit_file.rs` (`editFile`) - find/replace based editor
- **Params**: path, oldContent, newContent, replaceAll?
- **Output**: JSON { replacements, bytesWritten }
- **Parity**: **Partial** - No Fast Apply API in Rust; functional replacement semantics provided

### **insert_content**
- **Codex**: `insertContentTool.ts` - content insertion at specific positions
- **Rust**: `fs/insert_content.rs` (`insertContent`) - equivalent insertion tool
- **Params**: path, content, position(start|end|line:N|byte:N)
- **Output**: JSON { inserted_at_byte, bytes_inserted, new_file_size }
- **Parity**: **Full** - Complete functional equivalence

### **search_and_replace**
- **Codex**: `searchAndReplaceTool.ts` - optional line ranges, diff view, approvals
- **Rust**: `fs/search_and_replace.rs` (`searchAndReplace`) - global search/replace
- **Params**: path, search, replace, mode(literal|regex), multiline?, preview?
- **Output**: JSON { replacements_made | preview }
- **Parity**: **Partial** - Rust lacks line-range targeted replace

### **list_files**
- **Codex**: `listFilesTool.ts` - directory listing with patterns
- **Rust**: `fs/list_files.rs` (`listFiles`) - equivalent listing functionality
- **Params**: path, pattern?, recursive?
- **Output**: JSON { path, files[], count }
- **Parity**: **Full** - Complete functional equivalence

### **search_files**
- **Codex**: `searchFilesTool.ts` - ripgrep service integration
- **Rust**: `fs/search_files.rs` (`searchFiles`) - globset+regex based search
- **Params**: path, query, filePattern?, isRegex?, caseSensitive?
- **Output**: JSON { query, path, matches[] }
- **Parity**: **Partial** - Rust uses globset+regex; ripgrep integration planned

---

## Diff & Patch Tools

### **apply_diff / multi_apply_diff**
- **Codex**: `applyDiffTool.ts`, `multiApplyDiffTool.ts` - SEARCH/REPLACE blocks, batching
- **Rust**: `diff_tool.rs` (`diff` tool) - unified diff handler with multiple operations
- **Operations**: preview | apply | multiApply | search_replace | edit_range
- **Output**: JSON op-specific; multiApply returns per-file successes/failures
- **Parity**: **Different** - Rust uses unified tool with operation modes; Codex has separate tools

---

## Command Execution

### **execute_command**
- **Codex**: `executeCommandTool.ts` - terminal integration, streaming output
- **Rust**: `execute_command.rs` (`executeCommand`) - command runner with streaming
- **Params**: command, cwd?, timeout?
- **Output**: JSON { exitCode, stdout, stderr, duration_ms, correlation_id }
- **Parity**: **Partial→Full** - Rust streams via emitter; dangerous commands blocked

---

## Code Intelligence Tools (Codex-only)

### **codebase_search**
- **Codex**: `codebaseSearchTool.ts` - vector store/embedding-based semantic search
- **Rust**: None
- **Parity**: **None** - P2 backlog for Rust (tree-sitter based)

### **list_code_definition_names**
- **Codex**: `listCodeDefinitionNamesTool.ts` - tree-sitter based symbol extraction
- **Rust**: None
- **Parity**: **None** - P2 backlog for Rust

---

## UI/Interaction Tools (Codex-only)

### **ask_followup_question**
- **Codex**: `askFollowupQuestionTool.ts` - interactive Q&A with user
- **Rust**: None (IPC-dependent)
- **Parity**: **None**

### **attempt_completion**
- **Codex**: `attemptCompletionTool.ts` - task completion flow
- **Rust**: None (task management via IPC)
- **Parity**: **None**

### **browser_action**
- **Codex**: `browserActionTool.ts` - browser automation (launch, click, type, etc.)
- **Rust**: None
- **Parity**: **None**

### **generate_image**
- **Codex**: `generateImageTool.ts` - AI image generation via API
- **Rust**: None
- **Parity**: **None**

### **update_todo_list**
- **Codex**: `updateTodoListTool.ts` - todo list management
- **Rust**: None (state management via IPC)
- **Parity**: **None**

### **condense**
- **Codex**: `condenseTool.ts` - conversation summarization
- **Rust**: None (conversation management via IPC)
- **Parity**: **None**

### **report_bug**
- **Codex**: `reportBugTool.ts` - GitHub issue creation
- **Rust**: None
- **Parity**: **None**

---

## MCP & Integration Tools (Codex-only)

### **use_mcp_tool**
- **Codex**: `useMcpToolTool.ts` - MCP server tool execution
- **Rust**: None (MCP bridge via IPC)
- **Parity**: **None**

### **access_mcp_resource**
- **Codex**: `accessMcpResourceTool.ts` - MCP resource access
- **Rust**: None (MCP bridge via IPC)
- **Parity**: **None**

### **fetch_instructions**
- **Codex**: `fetchInstructionsTool.ts` - MCP-based instruction fetching
- **Rust**: None
- **Parity**: **None**

---

## Workflow Tools (Codex-only)

### **new_rule**
- **Codex**: `newRuleTool.ts` - rule/instruction management
- **Rust**: None
- **Parity**: **None**

### **new_task**
- **Codex**: `newTaskTool.ts` - task creation and management
- **Rust**: None
- **Parity**: **None**

### **run_slash_command**
- **Codex**: `runSlashCommandTool.ts` - slash command execution
- **Rust**: None
- **Parity**: **None**

### **switch_mode**
- **Codex**: `switchModeTool.ts` - mode switching (architect/code/ask)
- **Rust**: None
- **Parity**: **None**

---

## Expanded Tools (Rust-only)

### **git_status**
- **Rust**: `expanded_tools.rs` - Git repository status
- **Codex**: None
- **Parity**: **None**

### **git_diff**
- **Rust**: `expanded_tools.rs` - Git diff for files
- **Codex**: None
- **Parity**: **None**

### **count_lines**
- **Rust**: `expanded_tools.rs` - Line counting utility
- **Codex**: None (integrated in readFile)
- **Parity**: **None**

### **file_size**
- **Rust**: `expanded_tools.rs` - File/directory size calculation
- **Codex**: None
- **Parity**: **None**

### **process_list**
- **Rust**: `expanded_tools.rs` - Running process listing (requires approval)
- **Codex**: None
- **Parity**: **None**

### **environment**
- **Rust**: `expanded_tools.rs` - Environment variable access
- **Codex**: None
- **Parity**: **None**

### **base64**
- **Rust**: `expanded_tools.rs` - Base64 encode/decode
- **Codex**: None
- **Parity**: **None**

### **json_format**
- **Rust**: `expanded_tools.rs` - JSON formatting/validation
- **Codex**: None
- **Parity**: **None**

### **zip**
- **Rust**: `expanded_tools.rs` - ZIP archive create/extract (requires approval)
- **Codex**: None
- **Parity**: **None**

### **curl**
- **Rust**: `expanded_tools.rs` - HTTP request tool (requires approval)
- **Codex**: None
- **Parity**: **None**

---

## Cross-cutting Concerns

### **RooIgnore Enforcement**
- **Rust**: Enforced in all tools via `ToolContext.is_path_allowed()` and `ensure_workspace_path()`
- **Codex**: Enforced via RooIgnore controller checks
- **Status**: Both sides implement, different mechanisms

### **Approval System**
- **Rust**: `requires_approval()` trait method + `context.require_approval` flag
- **Codex**: Approval flow via UI callbacks
- **Status**: Both sides implement, awaiting IPC bridge

### **Streaming Events**
- **Rust**: `streaming.rs` - `ToolExecutionProgress`, `CommandExecutionStatus`, `DiffStreamUpdate`
- **Codex**: Direct UI updates via callbacks
- **Status**: Rust has types defined, IPC wiring pending

### **Error Handling**
- **Rust**: `ToolError` enum - RooIgnoreBlocked, ApprovalRequired, PermissionDenied, InvalidArguments, Io, Timeout, Other
- **Codex**: Mixed error handling with UI formatting
- **Status**: Rust normalized, UI mapping pending

---

## Summary Statistics

- **Total Codex Tools**: 27
- **Total Rust Tools**: 20 (10 core FS/diff/cmd + 10 expanded)
- **Tools with Full Parity**: 2 (insert_content, list_files)
- **Tools with Partial Parity**: 6 (read_file, write_to_file, edit_file, search_and_replace, search_files, execute_command)
- **Tools with Different Implementation**: 1 (diff/apply_diff/multi_apply_diff)
- **Codex-only Tools**: 18 (mostly UI/MCP/workflow related)
- **Rust-only Tools**: 10 (expanded utilities)

---

## Pre-IPC Gaps to Address

### High Priority
1. ✅ **Complete this parity mapping** (DONE)
2. Ripgrep-backed `searchFiles` for performance parity
3. Error shape documentation for UI mapping
4. Streaming event emitter completion

### Medium Priority
1. Line-range support in `search_and_replace`
2. Multi-file batch API for `readFile`
3. Terminal tool implementation with OSC markers

### Low Priority (P2 Backlog)
1. Code intelligence tools (tree-sitter based)
2. Git tool suite expansion
3. Project operation tools (format/lint/test runners)
