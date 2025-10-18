// AI Bridge Message Envelopes
// Aligned with lapce-ai/docs/CHUNK-02-TOOLS-EXECUTION.md and Codex ExtensionMessage types
//
// Design principle: These are the contract between UI and backend.
// All envelopes are JSON-serializable for protocol flexibility.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

// ============================================================================
// OUTBOUND: UI → Backend
// ============================================================================

/// Messages sent from Lapce UI to lapce-ai backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OutboundMessage {
    /// User sends a new task/message
    NewTask {
        text: String,
        images: Vec<String>, // Base64 or file paths
        model: Option<String>, // e.g. "Claude Sonnet 4.5 Thinking"
        mode: Option<String>,  // "Code" or "Chat"
    },

    /// User responds to an ask (approval/rejection/input)
    AskResponse {
        ask_ts: u64, // Timestamp of the ask message
        response: AskResponseType,
    },

    /// User selects images for current context
    SelectImages { images: Vec<String> },

    /// Terminal operation (continue/abort command)
    TerminalOperation {
        terminal_id: String,
        operation: TerminalOp,
    },

    /// User cancels current API request
    CancelTask,

    /// User requests task history
    GetHistory {
        offset: usize,
        limit: usize,
    },

    /// User deletes a task from history
    DeleteTask { task_id: String },

    /// User exports task
    ExportTask { task_id: String },

    /// Settings update
    UpdateSettings { settings: HashMap<String, JsonValue> },

    /// MCP server enable/disable
    McpServerToggle { server_name: String, enabled: bool },

    // ============================================================================
    // Context System Operations
    // ============================================================================
    
    /// Request conversation truncation with sliding window
    TruncateConversation {
        messages: Vec<JsonValue>,
        model_id: String,
        context_window: usize,
        max_tokens: Option<usize>,
    },

    /// Request conversation condense/summarization
    CondenseConversation {
        messages: Vec<JsonValue>,
        model_id: String,
    },

    /// Track file context (read/write/edit)
    TrackFileContext {
        file_path: String,
        source: FileContextSource,
    },

    /// Get list of stale files
    GetStaleFiles { task_id: String },
    
    // ============================================================================
    // Provider Chat Operations (Phase C - UI Streaming)
    // ============================================================================
    
    /// Send chat message to AI provider (streaming)
    ProviderChatStream {
        model: String,
        messages: Vec<ProviderChatMessage>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    },
    
    /// Send chat message to AI provider (non-streaming)
    ProviderChat {
        model: String,
        messages: Vec<ProviderChatMessage>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    },
    
    // ============================================================================
    // LSP Gateway Operations (Native Tree-Sitter Based)
    // ============================================================================
    
    /// Send LSP request to native gateway
    LspRequest {
        id: String,
        method: String,
        uri: String,
        language_id: String,
        params: JsonValue,
    },
    
    /// Cancel an LSP request
    LspCancel { request_id: String },
    
    // ============================================================================
    // Tool Execution Operations
    // ============================================================================
    
    /// Send approval response for tool execution
    ToolApprovalResponse {
        execution_id: String,
        approved: bool,
        reason: Option<String>,
    },
}

/// Response types for ask interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AskResponseType {
    /// User approves (Yes)
    Approve,

    /// User rejects (No)
    Reject,

    /// User provides text input
    MessageResponse { text: String },

    /// Batch file permission response
    BatchFileResponse {
        responses: HashMap<String, bool>, // path → allowed
    },

    /// Batch diff approval response
    BatchDiffResponse {
        responses: HashMap<String, bool>, // path → allowed
    },
}

/// Terminal operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalOp {
    Continue,
    Abort,
    /// Inject a command (from AI)
    InjectCommand {
        command: String,
        source: CommandSource,
    },
    /// Send interrupt signal (Ctrl+C)
    SendInterrupt,
    /// Send control signal
    SendControlSignal { signal: String },
}

/// Command source (matches lapce-app terminal types)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CommandSource {
    User,
    Cascade,
}

/// File context source (how file entered context)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileContextSource {
    Read,
    Write,
    DiffApply,
    Mention,
    UserEdit,
    RooEdit,
}

// ============================================================================
// INBOUND: Backend → UI
// ============================================================================

/// Messages received from lapce-ai backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InboundMessage {
    /// Chat message (say or ask)
    ChatMessage {
        ts: u64,
        message: ClineMessage,
    },

    /// Partial update to last message (streaming)
    PartialMessage {
        ts: u64,
        partial: ClineMessage,
    },

    /// Task completed
    TaskCompleted { task_id: String, success: bool },

    /// API request started (with cost info)
    ApiRequestStarted {
        ts: u64,
        cost: Option<f64>,
        model: String,
    },

    /// API request failed
    ApiRequestFailed { ts: u64, error: String },

    /// Terminal output chunk
    TerminalOutput {
        terminal_id: String,
        data: String,
        markers: Vec<OscMarker>,
    },

    /// Terminal command started
    TerminalCommandStarted {
        terminal_id: String,
        command: String,
        source: CommandSource,
        cwd: String,
    },

    /// Terminal command completed
    TerminalCommandCompleted {
        terminal_id: String,
        command: String,
        exit_code: i32,
        duration_ms: u64,
        forced_exit: bool,
    },

    /// Terminal command injection result
    TerminalCommandInjected {
        terminal_id: String,
        command: String,
        success: bool,
        error: Option<String>,
    },

    /// History results
    HistoryResults {
        tasks: Vec<TaskSummary>,
        total: usize,
    },

    /// Error notification
    Error { message: String, recoverable: bool },

    /// Connection status change
    ConnectionStatus { status: ConnectionStatusType },

    // ============================================================================
    // Context System Responses
    // ============================================================================
    
    /// Truncate conversation response
    TruncateConversationResponse {
        messages: Vec<JsonValue>,
        summary: String,
        cost: f64,
        new_context_tokens: Option<usize>,
        prev_context_tokens: usize,
    },

    /// Condense conversation response
    CondenseConversationResponse {
        summary: String,
        messages_condensed: usize,
        cost: f64,
    },

    /// Track file context response
    TrackFileContextResponse {
        success: bool,
        error: Option<String>,
    },

    /// Stale files response
    StaleFilesResponse { stale_files: Vec<String> },

    /// Context operation error
    ContextError { operation: String, message: String },
    
    // ============================================================================
    // Provider Streaming Responses (Phase C - UI Streaming)
    // ============================================================================
    
    /// Provider streaming chunk
    ProviderStreamChunk {
        content: String,
        tool_call: Option<ToolCallChunk>,
    },
    
    /// Provider streaming complete
    ProviderStreamDone {
        usage: Option<ProviderUsage>,
    },
    
    /// Provider chat response (non-streaming)
    ProviderChatResponse {
        id: String,
        content: String,
        usage: Option<ProviderUsage>,
        tool_calls: Vec<ToolCall>,
    },
    
    /// Provider error
    ProviderError { message: String },
    
    // ============================================================================
    // LSP Gateway Responses (Native Tree-Sitter Based)
    // ============================================================================
    
    /// LSP request response (success or error)
    LspResponse {
        id: String,
        ok: bool,
        result: Option<JsonValue>,
        error: Option<String>,
        error_code: Option<i32>,
    },
    
    /// LSP diagnostics notification
    LspDiagnostics {
        uri: String,
        version: Option<i32>,
        diagnostics: Vec<LspDiagnostic>,
    },
    
    /// LSP progress notification
    LspProgress {
        token: String,
        kind: LspProgressKind,
        title: Option<String>,
        message: Option<String>,
        percentage: Option<u32>,
    },
    
    // ============================================================================
    // Tool Execution Lifecycle Events
    // ============================================================================
    
    /// Tool execution started
    ToolExecutionStarted {
        execution_id: String,
        tool_name: String,
        timestamp: u64,
    },
    
    /// Tool execution progress update
    ToolExecutionProgress {
        execution_id: String,
        message: String,
        percentage: Option<u8>,
    },
    
    /// Tool execution completed successfully
    ToolExecutionCompleted {
        execution_id: String,
        output: ToolExecutionOutput,
        duration_ms: u64,
    },
    
    /// Tool execution failed
    ToolExecutionFailed {
        execution_id: String,
        error: String,
        duration_ms: u64,
    },
    
    /// Approval required    /// Tool approval request
    ToolApprovalRequest {
        execution_id: String,
        tool_name: String,
        operation: String,
        target: String,
        risk_level: ApprovalRiskLevel,
        approval_id: String,
    },
    
    // ============================================================================
    // Command Execution Streaming
    // ============================================================================
    
    /// Command execution started
    CommandExecutionStarted {
        command: String,
        args: Vec<String>,
        correlation_id: String,
    },
    
    /// Command output chunk received
    CommandExecutionOutput {
        correlation_id: String,
        chunk: String,
        stream_type: CommandStreamType,
    },
    
    /// Command execution completed
    CommandExecutionExit {
        correlation_id: String,
        exit_code: i32,
        duration_ms: u64,
    },
    
    // ============================================================================
    // Diff Streaming
    // ============================================================================
    
    /// Diff operation progress update
    DiffStreamUpdate {
        correlation_id: String,
        file_path: String,
        hunk_index: usize,
        total_hunks: usize,
        lines_added: usize,
        lines_removed: usize,
        status: DiffStreamStatus,
        preview: Option<String>,
    },

/// Chat message structure (mirrors Codex ClineMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
{{ ... }}
    pub ts: u64,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub text: Option<String>,
    pub partial: bool,
    pub say: Option<SayType>,
    pub ask: Option<AskType>,
}

/// Message type (say or ask)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Say,
    Ask,
}

/// Say message types (backend → user info)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SayType {
    Text,
    Tool,
    #[serde(rename = "api_req_started")]
    ApiReqStarted,
    #[serde(rename = "api_req_finished")]
    ApiReqFinished,
    #[serde(rename = "completion_result")]
    CompletionResult,
    #[serde(rename = "mcp_server_request_started")]
    McpServerRequestStarted,
    Error,
}

/// Ask message types (backend requests user interaction)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskType {
    Tool,
    Command,
    #[serde(rename = "api_req_failed")]
    ApiReqFailed,
    Followup,
    #[serde(rename = "completion_result")]
    CompletionResult,
    #[serde(rename = "resume_task")]
    ResumeTask,
    #[serde(rename = "resume_completed_task")]
    ResumeCompletedTask,
    #[serde(rename = "use_mcp_server")]
    UseMcpServer,
    #[serde(rename = "browser_action_launch")]
    BrowserActionLaunch,
}

/// OSC marker (for terminal command output parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscMarker {
    pub marker_type: String, // "command_start", "command_end", etc.
    pub position: usize,
    pub data: Option<String>,
}

/// Task summary for history list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub timestamp: u64,
    pub completed: bool,
    pub cost: Option<f64>,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatusType {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

// ============================================================================
// Tool-specific payloads (parsed from ClineMessage.text as JSON)
// ============================================================================
/// Tool payload for "say: tool" messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "camelCase")]
pub enum ToolPayload {
    #[serde(rename = "readFile")]
    ReadFile {
        path: String,
        content: Option<String>,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
        #[serde(rename = "additionalFileCount")]
        additional_file_count: Option<usize>,
        #[serde(rename = "batchFiles")]
        batch_files: Option<Vec<String>>,
    },

    #[serde(rename = "listFilesTopLevel")]
    ListFilesTopLevel {
        path: String,
        content: String,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },

    #[serde(rename = "listFilesRecursive")]
    ListFilesRecursive {
        path: String,
        content: String,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },

    #[serde(rename = "searchFiles")]
    SearchFiles {
        path: String,
        regex: String,
        #[serde(rename = "filePattern")]
        file_pattern: Option<String>,
        content: String,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },

    #[serde(rename = "appliedDiff")]
    AppliedDiff {
        path: String,
        diff: Option<String>,
        content: Option<String>,
        #[serde(rename = "isProtected")]
        is_protected: bool,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
        #[serde(rename = "batchDiffs")]
        batch_diffs: Option<Vec<BatchDiffItem>>,
    },

    #[serde(rename = "editedExistingFile")]
    EditedExistingFile {
        path: String,
        diff: Option<String>,
        #[serde(rename = "isProtected")]
        is_protected: bool,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },

    #[serde(rename = "insertContent")]
    InsertContent {
        path: String,
        diff: String,
        #[serde(rename = "lineNumber")]
        line_number: usize,
        #[serde(rename = "isProtected")]
        is_protected: bool,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },

    #[serde(rename = "searchAndReplace")]
    SearchAndReplace {
        path: String,
        diff: String,
        #[serde(rename = "isProtected")]
        is_protected: bool,
    },

    #[serde(rename = "newFileCreated")]
    NewFileCreated {
        path: String,
        content: String,
        #[serde(rename = "isProtected")]
        is_protected: bool,
    },

    #[serde(rename = "codebaseSearch")]
    CodebaseSearch {
        query: String,
        path: Option<String>,
    },

    #[serde(rename = "updateTodoList")]
    UpdateTodoList {
        todos: Vec<TodoItem>,
        content: Option<String>,
    },

    #[serde(rename = "switchMode")]
    SwitchMode {
        mode: String,
        reason: Option<String>,
    },

    #[serde(rename = "newTask")]
    NewTask { mode: String, content: String },

    #[serde(rename = "finishTask")]
    FinishTask,

    #[serde(rename = "fetchInstructions")]
    FetchInstructions { content: String },

    #[serde(rename = "listCodeDefinitionNames")]
    ListCodeDefinitionNames {
        path: String,
        content: String,
        #[serde(rename = "isOutsideWorkspace")]
        is_outside_workspace: bool,
    },
}

/// Batch diff item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDiffItem {
    pub path: String,
    pub diff: String,
}

/// Todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

// ============================================================================
// Provider Types (Phase C - UI Streaming)
// ============================================================================

/// Provider chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderChatMessage {
    pub role: String,  // "user" or "assistant" or "system"
    pub content: String,
}

/// Provider usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Tool call chunk (streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallChunk {
    pub id: String,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// Tool call (complete)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

// ============================================================================
// Tool Execution Types
// ============================================================================

/// Tool execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionOutput {
    pub success: bool,
    pub result: JsonValue,
    pub error: Option<String>,
    pub metadata: HashMap<String, JsonValue>,
}

/// Approval risk level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Command output stream type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandStreamType {
    Stdout,
    Stderr,
}

/// Diff operation status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffStreamStatus {
    Analyzing,
    ApplyingHunk,
    HunkApplied,
    HunkFailed,
    Complete,
    RolledBack,
}

// ============================================================================
// LSP Types (matching lsp-types spec)
// ============================================================================

/// LSP diagnostic (error, warning, info, hint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<LspDiagnosticSeverity>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub related_information: Option<Vec<LspDiagnosticRelatedInformation>>,
}

/// LSP range (start/end positions)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP position (line, character)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP diagnostic severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum LspDiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// LSP diagnostic related information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnosticRelatedInformation {
    pub location: LspLocation,
    pub message: String,
}

/// LSP location (uri + range)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// LSP progress kind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspProgressKind {
    Begin,
    Report,
    End,
}

// ============================================================================
// Error Code Mapping (Backend -> UI)
// ============================================================================

/// Error codes for tool execution (matches backend ToolErrorCode)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolErrorCode {
    NotFound = 1000,
    InvalidArguments = 2000,
    InvalidInput = 2001,
    PermissionDenied = 3000,
    SecurityViolation = 3001,
    RooIgnoreBlocked = 3002,
    ApprovalRequired = 4000,
    ExecutionFailed = 5000,
    Timeout = 5001,
    IoError = 5002,
    Unknown = 9000,
}

impl ToolErrorCode {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        !matches!(self,
            ToolErrorCode::NotFound
            | ToolErrorCode::PermissionDenied
            | ToolErrorCode::SecurityViolation
            | ToolErrorCode::RooIgnoreBlocked
        )
    }
    
    /// Get user-friendly category name
    pub fn category(&self) -> &'static str {
        match self {
            ToolErrorCode::NotFound => "Not Found",
            ToolErrorCode::InvalidArguments | ToolErrorCode::InvalidInput => "Invalid Input",
            ToolErrorCode::PermissionDenied | ToolErrorCode::SecurityViolation | ToolErrorCode::RooIgnoreBlocked => "Permission Denied",
            ToolErrorCode::ApprovalRequired => "Approval Required",
            ToolErrorCode::ExecutionFailed | ToolErrorCode::Timeout | ToolErrorCode::IoError => "Execution Error",
            ToolErrorCode::Unknown => "Unknown Error",
        }
    }
}
