# Tier 1 Implementation TODO (Ultra Comprehensive)

Scope: Complete Tier 1 backend + UI wiring for Lapce AI using the Step 29 architecture (Native Lapce UI + SharedMemory IPC + Backend).

Source of Truth Docs:
- `lapce-ai/docs/CHUNK-03-TASK-ORCHESTRATOR.md`
- `lapce-ai/docs/CHUNK-01-PROMPTS-SYSTEM.md`
- `lapce-ai/docs/CHUNK-02-TOOLS-EXECUTION.md`
- `lapce-ai/docs/30-MESSAGE-PROTOCOL.md`
- `lapce-ai/docs/29-VSCODE-LAPCE-BRIDGE-FINAL-REVISED.md`

Success Criteria for Tier 1 (all must pass):
- End-to-end prompt request: UI → BuildPrompt → PromptReady → UI rendered
- Terminal command streaming: ExecuteCommand → TerminalOutput chunks → CommandComplete
- Diff view streaming: RequestDiff → StreamDiffLine chunks → DiffComplete
- Task lifecycle: StartTask → streaming TaskEvent → AbortTask
- Performance: IPC roundtrip < 10μs; no panics; stable reconnect

Notes:
- Use the consolidated v1 IpcMessage set specified in `30-MESSAGE-PROTOCOL.md` (defer full 309-variant translation until Tier 2+)
- Map to ACTUAL Lapce components only; avoid hypothetical files
- No mocks or placeholders; production-grade implementation

---

## 0) Pre-flight Checklist

- [ ] Verify Rust toolchain (as per repository `rust-toolchain.toml`)
- [ ] Ensure `lapce-ai-rust/src/shared_memory_complete.rs` is present and compiled
- [ ] Baseline build (expect errors before Tier 1 changes):
  - [ ] `cargo build --release -q` at `lapce-ai-rust/`
  - [ ] `cargo build --release -q` at `lapce/`
- [ ] Read the following files to confirm current state:
  - [ ] `lapce-app/src/ai_panel/message_handler.rs`
  - [ ] `lapce-app/src/window_tab.rs`
  - [ ] `lapce-app/src/terminal/panel.rs`
  - [ ] `lapce-app/src/editor/diff.rs`
  - [ ] `lapce-ai-rust/src/ipc_server.rs`

Acceptance:
- Builds may fail at this stage, but all files exist where expected.

---

## 1) Protocol: Lock Minimal v1 IpcMessage Set

Files:
- `lapce-ai/docs/30-MESSAGE-PROTOCOL.md`
- `lapce-rpc/src/ai_messages.rs` (to be created/updated if not present)

Tasks:
- [ ] Create/confirm `IpcMessage` enum includes at least:
  - [ ] StartTask { task, mode }, TaskEvent(TaskEvent), AbortTask { task_id }
  - [ ] ExecuteTool { tool, params }, ToolResult { tool, output }
  - [ ] ExecuteCommand { cmd, cwd }, TerminalOutput { data, markers }, CommandComplete { exit_code, duration_ms }
  - [ ] RequestDiff { file_path, original, modified }, StreamDiffLine { line_num, content, change_type }, DiffComplete { total_lines }
  - [ ] BuildPrompt { mode, workspace }, PromptReady { prompt, token_count }
  - [ ] Error { message, recoverable }
- [ ] Derive rkyv for zero-copy and ensure `#[archive(check_bytes)]`
- [ ] Align shared type enums: `ShellMarker`, `DiffChangeType`, `FileChangeType`

Acceptance:
- [ ] `lapce-rpc` crate compiles standalone with `IpcMessage` and all shared enums.

---

## 2) UI: Add SharedMemory IPC Client to CommonData

Files:
- `lapce-app/src/window_tab.rs`

Tasks:
- [ ] Add field to `CommonData`:
  - [ ] `pub ai_ipc: Arc<LapceAiIpcClient>`
- [ ] Implement `LapceAiIpcClient` with SharedMemory connect/send/send_stream/reconnect
  - [ ] `pub async fn send(&self, msg: IpcMessage) -> Result<IpcMessage>`
  - [ ] `pub async fn send_stream(&self, msg: IpcMessage) -> Result<mpsc::Receiver<IpcMessage>>`
  - [ ] Reconnect logic with backoff
- [ ] Wire initialization at window start-up

Acceptance:
- [ ] Lapce builds with the new `ai_ipc` field
- [ ] No UI regressions at startup

---

## 3) UI: MessageHandler Integration (ACTUAL component)

Files:
- `lapce-app/src/ai_panel/message_handler.rs`

Tasks:
- [ ] Replace `bridge: Arc<LapceAiInterface>` with `ipc_client: Arc<LapceAiIpcClient>` or obtain from `CommonData`
- [ ] Modify `handle_ipc()` to route via SharedMemory IPC
- [ ] Add prompt APIs:
  - [ ] `request_prompt(mode: String, workspace: PathBuf) -> Result<String>` → `IpcMessage::BuildPrompt`
  - [ ] `switch_mode(new_mode: String) -> Result<()>` → `IpcMessage::SwitchMode`
  - [ ] `update_custom_instructions(instructions: String) -> Result<()>`
- [ ] Add task APIs:
  - [ ] `start_task(task_str: String, mode: String) -> Result<String>` → returns task_id
  - [ ] `subscribe_to_task(task_id: String) -> Result<()>` → stream TaskEvent and update UI
  - [ ] `abort_task(task_id: String) -> Result<()>`
- [ ] Add tool APIs:
  - [ ] `execute_tool(tool_name: &str, params: serde_json::Value) -> Result<serde_json::Value>`
  - [ ] `read_file(path: String, line_ranges: Option<Vec<(usize, usize)>>) -> Result<String>`
  - [ ] `write_file(path: String, content: String) -> Result<()>`

Acceptance:
- [ ] `message_handler.rs` compiles
- [ ] Can invoke all IPC calls (no runtime yet)

---

## 4) UI: Terminal Panel Streaming

Files:
- `lapce-app/src/terminal/panel.rs`

Tasks:
- [ ] Extend `TerminalPanelData` with:
  - [ ] `async fn execute_ai_command(&self, cmd: String) -> Result<()>`
  - [ ] Use `self.common.ai_ipc.send_stream(IpcMessage::ExecuteCommand { ... })`
  - [ ] On `TerminalOutput`, route bytes to existing terminal buffer
  - [ ] On `CommandComplete`, log/notify and stop stream

Acceptance:
- [ ] Terminal UI updates live when fed with fake echo commands from backend (after backend is wired)

---

## 5) UI: Diff Editor Streaming

Files:
- `lapce-app/src/editor/diff.rs`

Tasks:
- [ ] Extend `DiffEditorData` with:
  - [ ] `async fn apply_ai_diff(&self, changes: String) -> Result<()>`
  - [ ] Request diff via `IpcMessage::RequestDiff`
  - [ ] On `StreamDiffLine`, update right-side rope and changes via existing `rope_diff()` infrastructure
  - [ ] On `DiffComplete`, finalize UI

Acceptance:
- [ ] Diff view updates line-by-line when receiving stream from backend handler

---

## 6) UI: Config additions (AI section)

Files:
- `lapce-app/src/config.rs`

Tasks:
- [ ] Add `AiConfig` sub-struct with fields:
  - [ ] `custom_instructions: Option<String>`
  - [ ] `use_agent_rules: bool`
  - [ ] `default_mode: String`
  - [ ] `max_concurrent_file_reads: u32`
  - [ ] `enable_mcp: bool`
  - [ ] `browser_viewport_size: String`
- [ ] Add `ai: Option<AiConfig>` to `LapceConfig`

Acceptance:
- [ ] Config compiles and is loadable; defaults sensible

---

## 7) Backend: Handler Wiring in ipc_server.rs

Files:
- `lapce-ai-rust/src/ipc_server.rs`

Tasks:
- [ ] Register handlers for:
  - [ ] StartTask, TaskEvent streaming, AbortTask → `TaskOrchestrator`
  - [ ] ExecuteCommand, TerminalOutput, CommandComplete → `TerminalHandler`
  - [ ] RequestDiff, StreamDiffLine, DiffComplete → `DiffHandler`
  - [ ] BuildPrompt, PromptReady → `PromptHandler`
  - [ ] ExecuteTool, ToolResult → `ToolHandler`
- [ ] Ensure SharedMemory server uses `shared_memory_complete.rs`

Acceptance:
- [ ] `lapce-ai-rust` builds with handler registrations

---

## 8) Backend: PromptHandler

Files:
- `lapce-ai-rust/src/handlers/prompts.rs` (NEW)

Tasks:
- [ ] Implement `handle_build_prompt(mode, workspace) -> PromptReady`
- [ ] Implement `handle_update_instructions(instructions) -> PromptReady`
- [ ] Implement `handle_switch_mode(old, new) -> ModeChanged`
- [ ] Use `PromptBuilder`, `CustomInstructionsLoader`, `ToolRegistry`

Acceptance:
- [ ] `BuildPrompt` → `PromptReady` roundtrip works in isolation

---

## 9) Backend: Tool Handlers

Files:
- `lapce-ai-rust/src/handlers/tools/mod.rs` (NEW)
- `lapce-ai-rust/src/handlers/tools/terminal.rs` (NEW)
- `lapce-ai-rust/src/handlers/tools/diff.rs` (NEW)
- `lapce-ai-rust/src/handlers/tools/file.rs` (NEW)
- `lapce-ai-rust/src/handlers/tools/search.rs` (NEW)

Tasks:
- [ ] Define `Tool` trait and `ToolRegistry`
- [ ] Implement Terminal tool: PTY exec + OSC 633/133 parsing + streaming
- [ ] Implement Diff tool: compute diffs and stream line-by-line
- [ ] Implement File tools: read/write with `.rooignore` validation
- [ ] Implement Search tool: ripgrep or equivalent

Acceptance:
- [ ] Each handler returns correct `IpcMessage` sequences

---

## 10) Backend: TaskOrchestrator

Files:
- `lapce-ai-rust/src/handlers/task_orchestrator.rs` (NEW)

Tasks:
- [ ] Implement `TaskEvent` enum and `TaskOrchestrator` struct
- [ ] Implement `handle_start_task()`, `handle_abort_task()`
- [ ] Streaming of `TaskEvent::StreamToken`, `TaskEvent::TaskCompleted`, etc.
- [ ] Maintain state with atomic flags + RwLock-backed structures

Acceptance:
- [ ] Task start/stream/abort behaves deterministically and produces proper events

---

## 11) Build & Smoke Tests

Commands (do not auto-run here):
- `cd lapce-ai-rust && cargo build --release`
- `cd ../lapce && cargo build --release`

UI Smoke (manual):
- [ ] Launch Lapce with backend running; open AI panel; trigger prompt build
- [ ] Run a short terminal command via AI (e.g., `echo hello`)
- [ ] Request a small diff; confirm streaming applies updates

Acceptance:
- [ ] No panics; visible UI updates; basic end-to-end works

---

## 12) Integration Tests (Backend)

Files:
- `lapce-ai-rust/tests/`

Tests to add:
- [ ] `test_terminal_command_execution` (ExecuteCommand → CommandComplete)
- [ ] `test_diff_streaming` (RequestDiff → StreamDiffLine → DiffComplete)
- [ ] `test_prompt_build_roundtrip` (BuildPrompt → PromptReady)
- [ ] `test_task_lifecycle` (StartTask → events → AbortTask)

Acceptance:
- [ ] All tests pass in CI

---

## 13) Performance Validation

Targets:
- IPC Roundtrip < 10μs (already achieved per docs)
- Streaming stability (no drops) under small load

Tasks:
- [ ] Criterion micro-bench for `IpcMessage` serialize/deserialize path
- [ ] Load test: 1K messages/sec for 60s with zero drops
- [ ] Validate auto-reconnect < 100ms by restarting backend while UI is open

Acceptance:
- [ ] Benchmarks confirm targets; reconnect validated manually

---

## 14) Logging & Error Handling

Tasks:
- [ ] Add structured logging in handlers with request IDs / task IDs
- [ ] Ensure `Error { message, recoverable }` used consistently across handlers
- [ ] Timeouts and retries for tool execution as specified in docs

Acceptance:
- [ ] Logs enable tracing of message flows end-to-end

---

## 15) Documentation Updates

Tasks:
- [ ] Update `ACTUAL-LAPCE-COMPONENT-MAPPING.md` if any paths change
- [ ] Add a short README section in `lapce-app/src/ai_panel/` describing IPC
- [ ] Cross-link `TIER-1-IMPLEMENTATION-TODO.md` from `CHUNK-TRANSFORMATION-SEQUENCE.md`

Acceptance:
- [ ] Docs point to final implementation

---

## 16) Done-of-Done Checklist

- [ ] All UI methods compile and are reachable via the AI panel
- [ ] All backend handlers registered and return correct `IpcMessage` sequences
- [ ] Prompt, Terminal, Diff, and Task flows verified end-to-end
- [ ] IPC reconnection verified; no deadlocks or leaks observed
- [ ] Performance targets met or exceeded
- [ ] CI green for added tests

---

## Appendix A: Risk & Mitigation

- __Handler Signature Drift__: Keep `IpcMessage` v1 minimal; avoid adding variants mid-stream. If necessary, gate new variants behind feature flags.
- __UI State Desync__: Ensure event ordering (FIFO) for messages related to the same task. Use queues and avoid blocking UI thread.
- __PTY/OSC Edge Cases__: Add parser tests for OSC 633/133 sequences including truncated frames.
- __Diff Large Files__: Consider chunking and backpressure; verify memory use under large diffs.

---

## Appendix B: Traceability to Tier 1 Docs

- Prompts → `CHUNK-01-PROMPTS-SYSTEM.md`
- Tools (Terminal/Diff/File/Search) → `CHUNK-02-TOOLS-EXECUTION.md`
- Task Engine → `CHUNK-03-TASK-ORCHESTRATOR.md`
- Protocol → `30-MESSAGE-PROTOCOL.md`
- UI/IPC/Backend split → `29-VSCODE-LAPCE-BRIDGE-FINAL-REVISED.md`
