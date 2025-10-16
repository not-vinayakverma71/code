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

    /// History results
    HistoryResults {
        tasks: Vec<TaskSummary>,
        total: usize,
    },

    /// Error notification
    Error { message: String, recoverable: bool },

    /// Connection status change
    ConnectionStatus { status: ConnectionStatusType },
}

/// Chat message structure (mirrors Codex ClineMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatusType {
    Disconnected,
    Connecting,
    Connected,
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
