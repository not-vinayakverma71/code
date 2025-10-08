/// Message structures - Direct 1:1 port from TypeScript
/// Now includes COMPLETE ipc.ts translation
use serde::{Deserialize, Serialize};
use crate::events_exact_translation::TaskEvent;
use crate::global_settings_exact_translation::RooCodeSettings;

// ============================================================================
// COMPLETE ipc.ts TRANSLATION START
// ============================================================================

/// IpcMessageType - Exact translation from ipc.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IpcMessageType {
    #[serde(rename = "Connect")]
    Connect,
    #[serde(rename = "Disconnect")]
    Disconnect,
    #[serde(rename = "Ack")]
    Ack,
    #[serde(rename = "TaskCommand")]
    TaskCommand,
    #[serde(rename = "TaskEvent")]
    TaskEvent,
}

/// IpcOrigin - Exact translation from ipc.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IpcOrigin {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "server")]
    Server,
}

/// Ack structure - Exact translation from ipc.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ack {
    pub client_id: String,
    pub pid: u32,
    pub ppid: u32,
}

/// TaskCommandName - Exact translation from ipc.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskCommandName {
    #[serde(rename = "StartNewTask")]
    StartNewTask,
    #[serde(rename = "CancelTask")]
    CancelTask,
    #[serde(rename = "CloseTask")]
    CloseTask,
    #[serde(rename = "ResumeTask")]
    ResumeTask,
}

/// StartNewTaskData - Part of TaskCommand discriminated union
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartNewTaskData {
    pub configuration: RooCodeSettings,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<bool>,
}

/// TaskCommand - Discriminated union from ipc.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "commandName")]
pub enum TaskCommand {
    #[serde(rename = "StartNewTask")]
    StartNewTask {
        data: StartNewTaskData,
    },
    #[serde(rename = "CancelTask")]
    CancelTask {
        data: String,
    },
    #[serde(rename = "CloseTask")]
    CloseTask {
        data: String,
    },
    #[serde(rename = "ResumeTask")]
    ResumeTask {
        data: String,
    },
}

/// IpcMessage - Discriminated union from ipc.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    #[serde(rename = "Ack")]
    Ack {
        origin: IpcOrigin,
        data: Ack,
    },
    #[serde(rename = "TaskCommand")]
    TaskCommand {
        origin: IpcOrigin,
        #[serde(rename = "clientId")]
        client_id: String,
        data: TaskCommand,
    },
    #[serde(rename = "TaskEvent")]
    TaskEvent {
        origin: IpcOrigin,
        #[serde(rename = "relayClientId")]
        #[serde(skip_serializing_if = "Option::is_none")]
        relay_client_id: Option<String>,
        data: TaskEvent,
    },
}

// IpcClientEvents and IpcServerEvents are TypeScript type mappings
// In Rust, we handle these through the event emitter implementations

// ============================================================================
// END ipc.ts TRANSLATION
// ============================================================================

// ============================================================================
// COMPLETE message.ts TRANSLATION START
// ============================================================================
/// ClineAskResponse - Response types for ClineAsk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClineAskResponse {
    Approved,
    RenameSymbol,
    Cancel,
    SafeModeEnabled,
    DebugModeEnabled,
}

/// ClineAsk - All 15 ask types from message.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    FollowUp,  // Alias
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ApiCostLimit,
    RequestCostLimit,
    Confirmation,
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    AutoApprovalMaxReqReached,
    BrowserActionLaunch,
    UseMcpServer,
    PaymentRequiredPrompt,
    ReportBug,
    Condense,
}

/// ClineSay - All 25 say types from message.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClineSay {
    Error,
    ApiReqStarted,
    ApiReqFinished,
    ApiReqRetried,
    ApiReqRetryDelayed,
    ApiReqDeleted,
    Text,
    Reasoning,
    CompletionResult,
    UserFeedback,
    UserFeedbackDiff,
    CommandOutput,
    ShellIntegrationWarning,
    BrowserAction,
    BrowserActionResult,
    McpServerRequestStarted,
    McpServerResponse,
    SubtaskResult,
    CheckpointSaved,
    RooignoreError,
    DiffError,
    CondenseContext,
    CondenseContextError,
    CodebaseSearchResult,
    UserEditTodos,
}

/// ToolProgressStatus from message.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolProgressStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// ContextCondense from message.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCondense {
    pub cost: f64,
    pub prev_context_tokens: u32,
    pub new_context_tokens: u32,
    pub summary: String,
}

/// KiloCodeMetaData - placeholder for kilocode.ts type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiloCodeMetaData {
    // Will be filled when translating kilocode.ts
}

/// ClineMessage - Complete structure from message.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
    pub ts: u64,
    #[serde(rename = "type")]
    pub msg_type: String, // "ask" | "say"
    pub ask: Option<ClineAsk>,
    pub say: Option<String>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub partial: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_history_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<serde_json::Value>, // Record<string, unknown>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_status: Option<ToolProgressStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_condense: Option<ContextCondense>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_protected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_protocol: Option<String>, // "openai" | "anthropic"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

impl ClineMessage {
    pub fn ts(&self) -> Option<u64> {
        Some(self.ts)
    }
    
    pub fn is_partial(&self) -> bool {
        self.partial.unwrap_or(false)
    }
    
    pub fn is_ask_type(&self, ask_type: &ClineAsk) -> bool {
        if let Some(ref ask) = self.ask {
            std::mem::discriminant(ask) == std::mem::discriminant(ask_type)
        } else {
            false
        }
    }
    
    pub fn set_text(&mut self, new_text: Option<String>) {
        self.text = new_text;
    }
    
    pub fn set_partial(&mut self, is_partial: Option<bool>) {
        self.partial = is_partial;
    }
    
    pub fn set_progress_status(&mut self, status: Option<ToolProgressStatus>) {
        self.progress_status = status;
    }
    
    pub fn set_is_protected(&mut self, protected: Option<bool>) {
        self.is_protected = protected;
    }
}

/// MessageMetadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpt5: Option<Gpt5Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kilo_code: Option<KiloCodeMetaData>,
}

/// Gpt5Metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Gpt5Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_summary: Option<String>,
}

/// TokenUsage - Already defined in events_exact_translation.rs
pub use crate::events_exact_translation::TokenUsage;

/// QueuedMessage interface from message.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// Unique identifier for the queued message
    pub id: String,
    /// The text content of the message  
    pub text: String,
    /// Array of image data URLs attached to the message
    pub images: Vec<String>,
}

// Helper functions from message.ts
impl ClineAsk {
    pub fn is_idle_ask(&self) -> bool {
        matches!(self, 
            ClineAsk::CompletionResult |
            ClineAsk::ApiReqFailed |
            ClineAsk::ResumeCompletedTask |
            ClineAsk::MistakeLimitReached |
            ClineAsk::AutoApprovalMaxReqReached
        )
    }
    
    pub fn is_resumable_ask(&self) -> bool {
        matches!(self, ClineAsk::ResumeTask)
    }
    
    pub fn is_interactive_ask(&self) -> bool {
        matches!(self,
            ClineAsk::Command |
            ClineAsk::Tool |
            ClineAsk::BrowserActionLaunch |
            ClineAsk::UseMcpServer
        )
    }
}

// ============================================================================
// END message.ts TRANSLATION  
// ============================================================================

/// AI Request - Exact match of TypeScript interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRequest {
    pub messages: Vec<Message>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Message role enum - Exact match of TypeScript
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Message structure - Exact match of TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool Call - Exact match of TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
    pub id: String,
}

/// Tool structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

// ============================================================================
// P0-2: Tool Execution Lifecycle Messages
// ============================================================================

use std::path::PathBuf;

/// Tool execution lifecycle status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolExecutionStatus {
    /// Tool execution has started
    Started {
        execution_id: String,
        tool_name: String,
        timestamp: u64,
    },
    
    /// Tool execution progress update
    Progress {
        execution_id: String,
        message: String,
        percentage: Option<u8>,
    },
    
    /// Tool execution completed successfully
    Completed {
        execution_id: String,
        result: serde_json::Value,
        duration_ms: u64,
    },
    
    /// Tool execution failed
    Failed {
        execution_id: String,
        error: String,
        duration_ms: u64,
    },
}

/// Command execution status for terminal operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandExecutionStatusMessage {
    /// Command started
    Started {
        execution_id: String,
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    },
    
    /// Command output line
    Output {
        execution_id: String,
        stream_type: StreamType,
        line: String,
        timestamp: u64,
    },
    
    /// Command completed
    Completed {
        execution_id: String,
        exit_code: i32,
        duration_ms: u64,
    },
    
    /// Command timeout
    Timeout {
        execution_id: String,
        duration_ms: u64,
    },
}

/// Stream type for command output
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Diff operation messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffOperationMessage {
    /// Open diff view with two files
    OpenDiffFiles {
        left_path: PathBuf,
        right_path: PathBuf,
        title: Option<String>,
    },
    
    /// Save diff changes
    DiffSave {
        file_path: PathBuf,
        content: String,
    },
    
    /// Revert diff changes
    DiffRevert {
        file_path: PathBuf,
    },
    
    /// Close diff view
    CloseDiff {
        left_path: PathBuf,
        right_path: PathBuf,
    },
}

/// Tool approval request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalRequest {
    pub execution_id: String,
    pub tool_name: String,
    pub operation: String,
    pub target: String,
    pub details: String,
    pub require_confirmation: bool,
}

/// Tool approval response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalResponse {
    pub execution_id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

/// Extended IPC message for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolIpcMessage {
    #[serde(rename = "ToolExecutionStatus")]
    ToolExecutionStatus {
        origin: IpcOrigin,
        data: ToolExecutionStatus,
    },
    
    #[serde(rename = "CommandExecutionStatus")]
    CommandExecutionStatus {
        origin: IpcOrigin,
        data: CommandExecutionStatusMessage,
    },
    
    #[serde(rename = "DiffOperation")]
    DiffOperation {
        origin: IpcOrigin,
        data: DiffOperationMessage,
    },
    
    #[serde(rename = "ToolApprovalRequest")]
    ToolApprovalRequest {
        origin: IpcOrigin,
        data: ToolApprovalRequest,
    },
    
    #[serde(rename = "ToolApprovalResponse")]
    ToolApprovalResponse {
        origin: IpcOrigin,
        data: ToolApprovalResponse,
    },
}

// ============================================================================
// END P0-2: Tool Execution Lifecycle Messages
// ============================================================================

/// Extension Message type enum - from ExtensionMessage.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionMessageType {
    Action,
    State,
    SelectedImages,
    Theme,
    WorkspaceUpdated,
    Invoke,
    MessageUpdated,
    McpServers,
    EnhancedPrompt,
    CommitSearchResults,
    ListApiConfig,
    RouterModels,
    OpenAiModels,
    OllamaModels,
    LmStudioModels,
    VsCodeLmModels,
    HuggingFaceModels,
    VsCodeLmApiAvailable,
    UpdatePrompt,
    SystemPrompt,
    AutoApprovalEnabled,
    UpdateCustomMode,
    DeleteCustomMode,
    ExportModeResult,
    ImportModeResult,
    CheckRulesDirectoryResult,
    DeleteCustomModeCheck,
    CurrentCheckpointUpdated,
    ShowHumanRelayDialog,
    HumanRelayResponse,
    HumanRelayCancel,
    InsertTextToChatArea,
    BrowserToolEnabled,
    BrowserConnectionResult,
    RemoteBrowserEnabled,
    TtsStart,
    TtsStop,
    MaxReadFileLine,
    FileSearchResults,
    ToggleApiConfigPin,
    McpMarketplaceCatalog,
    McpDownloadDetails,
    ShowSystemNotification,
    OpenInBrowser,
    AcceptInput,
    FocusChatInput,
    SetHistoryPreviewCollapsed,
    CommandExecutionStatus,
    McpExecutionStatus,
    VsCodeSetting,
    ProfileDataResponse,
    BalanceDataResponse,
    UpdateProfileData,
    AuthenticatedUser,
    CondenseTaskContextResponse,
    SingleRouterModelFetchResponse,
    IndexingStatusUpdate,
    IndexCleared,
    CodebaseIndexConfig,
    RulesData,
    MarketplaceInstallResult,
    MarketplaceRemoveResult,
    MarketplaceData,
    MermaidFixResponse,
    ShareTaskSuccess,
    CodeIndexSettingsSaved,
    CodeIndexSecretStatus,
    ShowDeleteMessageDialog,
    ShowEditMessageDialog,
    KilocodeNotificationsResponse,
    UsageDataResponse,
    Commands,
    InsertTextIntoTextarea,
}

/// IPC message types for bi-directional communication

/// Message sent to webview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    pub message_type: String,
    pub payload: serde_json::Value,
}

/// Message types for IPC communication
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MessageType {
    Echo = 0,
    Complete = 1,
    Stream = 2,
    Cancel = 3,
    Heartbeat = 4,
    Shutdown = 5,
    Custom = 99,
}

impl MessageType {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() < 4 {
            return Err(anyhow::anyhow!("Invalid message type bytes"));
        }
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        match value {
            0 => Ok(MessageType::Echo),
            1 => Ok(MessageType::Complete),
            2 => Ok(MessageType::Stream),
            3 => Ok(MessageType::Cancel),
            4 => Ok(MessageType::Heartbeat),
            5 => Ok(MessageType::Shutdown),
            99 => Ok(MessageType::Custom),
            _ => Err(anyhow::anyhow!("Unknown message type: {}", value)),
        }
    }
    
    pub fn to_bytes(&self) -> [u8; 4] {
        (*self as u32).to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // P0-2 Tests: Serialization roundtrip tests for new IPC messages
    
    #[test]
    fn test_tool_execution_status_serialization() {
        let status = ToolExecutionStatus::Started {
            execution_id: "test-123".to_string(),
            tool_name: "readFile".to_string(),
            timestamp: 1234567890,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ToolExecutionStatus = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            ToolExecutionStatus::Started { execution_id, tool_name, timestamp } => {
                assert_eq!(execution_id, "test-123");
                assert_eq!(tool_name, "readFile");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_command_execution_status_serialization() {
        let status = CommandExecutionStatusMessage::Output {
            execution_id: "cmd-456".to_string(),
            stream_type: StreamType::Stdout,
            line: "Hello, world!".to_string(),
            timestamp: 9876543210,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: CommandExecutionStatusMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            CommandExecutionStatusMessage::Output { execution_id, stream_type, line, timestamp } => {
                assert_eq!(execution_id, "cmd-456");
                assert!(matches!(stream_type, StreamType::Stdout));
                assert_eq!(line, "Hello, world!");
                assert_eq!(timestamp, 9876543210);
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_diff_operation_message_serialization() {
        let msg = DiffOperationMessage::OpenDiffFiles {
            left_path: PathBuf::from("/path/to/original.txt"),
            right_path: PathBuf::from("/path/to/modified.txt"),
            title: Some("Test Diff".to_string()),
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DiffOperationMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            DiffOperationMessage::OpenDiffFiles { left_path, right_path, title } => {
                assert_eq!(left_path, PathBuf::from("/path/to/original.txt"));
                assert_eq!(right_path, PathBuf::from("/path/to/modified.txt"));
                assert_eq!(title, Some("Test Diff".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_tool_approval_request_serialization() {
        let request = ToolApprovalRequest {
            execution_id: "exec-789".to_string(),
            tool_name: "writeFile".to_string(),
            operation: "write".to_string(),
            target: "/important/file.txt".to_string(),
            details: "Writing sensitive data".to_string(),
            require_confirmation: true,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"executionId\":\"exec-789\""));
        
        let deserialized: ToolApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_id, "exec-789");
        assert_eq!(deserialized.tool_name, "writeFile");
        assert!(deserialized.require_confirmation);
    }
    
    #[test]
    fn test_tool_approval_response_serialization() {
        let response = ToolApprovalResponse {
            execution_id: "exec-789".to_string(),
            approved: false,
            reason: Some("User rejected the operation".to_string()),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ToolApprovalResponse = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.execution_id, "exec-789");
        assert!(!deserialized.approved);
        assert_eq!(deserialized.reason, Some("User rejected the operation".to_string()));
    }
    
    #[test]
    fn test_tool_ipc_message_serialization() {
        let msg = ToolIpcMessage::ToolExecutionStatus {
            origin: IpcOrigin::Server,
            data: ToolExecutionStatus::Completed {
                execution_id: "test-complete".to_string(),
                result: serde_json::json!({"success": true}),
                duration_ms: 150,
            },
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ToolExecutionStatus\""));
        assert!(json.contains("\"origin\":\"server\""));
        
        let deserialized: ToolIpcMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ToolIpcMessage::ToolExecutionStatus { origin, data } => {
                assert_eq!(origin, IpcOrigin::Server);
                match data {
                    ToolExecutionStatus::Completed { duration_ms, .. } => {
                        assert_eq!(duration_ms, 150);
                    }
                    _ => panic!("Wrong status variant"),
                }
            }
            _ => panic!("Wrong message variant"),
        }
    }
    
    #[test]
    fn test_backward_compatibility() {
        // Test that existing message types still work
        let old_msg = IpcMessage::Ack {
            origin: IpcOrigin::Client,
            data: Ack {
                client_id: "client-123".to_string(),
                pid: 1234,
                ppid: 5678,
            },
        };
        
        let json = serde_json::to_string(&old_msg).unwrap();
        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            IpcMessage::Ack { origin, data } => {
                assert_eq!(origin, IpcOrigin::Client);
                assert_eq!(data.client_id, "client-123");
                assert_eq!(data.pid, 1234);
                assert_eq!(data.ppid, 5678);
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_stream_type_serialization() {
        assert_eq!(serde_json::to_string(&StreamType::Stdout).unwrap(), "\"stdout\"");
        assert_eq!(serde_json::to_string(&StreamType::Stderr).unwrap(), "\"stderr\"");
        
        let stdout: StreamType = serde_json::from_str("\"stdout\"").unwrap();
        let stderr: StreamType = serde_json::from_str("\"stderr\"").unwrap();
        
        assert!(matches!(stdout, StreamType::Stdout));
        assert!(matches!(stderr, StreamType::Stderr));
    }
}
