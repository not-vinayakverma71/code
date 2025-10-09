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
// P0-2: Tool Execution Lifecycle Messages
// ============================================================================

/// Tool execution lifecycle states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Started {
        tool_name: String,
        correlation_id: String,
        timestamp: u64,
    },
    Progress {
        correlation_id: String,
        message: String,
        percentage: Option<u8>,
    },
    Completed {
        correlation_id: String,
        result: serde_json::Value,
        duration_ms: u64,
    },
    Failed {
        correlation_id: String,
        error: String,
        duration_ms: u64,
    },
}

/// Command execution status for execute_command tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionStatus {
    Started {
        command: String,
        args: Vec<String>,
        correlation_id: String,
    },
    OutputChunk {
        correlation_id: String,
        chunk: String,
        stream_type: StreamType,
    },
    Exit {
        correlation_id: String,
        exit_code: i32,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Diff operations for diff_tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffOperation {
    OpenDiffFiles {
        left_path: String,
        right_path: String,
        correlation_id: String,
    },
    SaveDiff {
        correlation_id: String,
        target_path: String,
    },
    RevertDiff {
        correlation_id: String,
    },
    CloseDiff {
        correlation_id: String,
    },
}

/// Approval flow messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMessage {
    ApprovalRequested {
        tool_name: String,
        operation: String,
        details: serde_json::Value,
        correlation_id: String,
        timeout_ms: Option<u64>,
    },
    ApprovalDecision {
        correlation_id: String,
        approved: bool,
        reason: Option<String>,
    },
}

/// Internal command for Lapce integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InternalCommand {
    OpenDiffFiles {
        left_path: String,
        right_path: String,
    },
    ExecuteProcess {
        program: String,
        arguments: Vec<String>,
    },
}

// ============================================================================
// COMPLETE message.ts TRANSLATION START
// ============================================================================

/// ClineAsk - All 15 ask types from message.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    BrowserActionLaunch,
    UseMcpServer,
    AutoApprovalMaxReqReached,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<ClineAsk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub say: Option<ClineSay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Message structure - Exact match of TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool Call - Exact match of TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
    pub id: String,
}

/// Tool structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

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
    
    #[test]
    fn test_tool_execution_status_roundtrip() {
        let statuses = vec![
            ToolExecutionStatus::Started {
                tool_name: "test_tool".to_string(),
                correlation_id: "corr-123".to_string(),
                timestamp: 1234567890,
            },
            ToolExecutionStatus::Progress {
                correlation_id: "corr-123".to_string(),
                message: "Processing...".to_string(),
                percentage: Some(50),
            },
            ToolExecutionStatus::Completed {
                correlation_id: "corr-123".to_string(),
                result: serde_json::json!({"success": true}),
                duration_ms: 1500,
            },
            ToolExecutionStatus::Failed {
                correlation_id: "corr-123".to_string(),
                error: "Something went wrong".to_string(),
                duration_ms: 500,
            },
        ];
        
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: ToolExecutionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }
    
    #[test]
    fn test_command_execution_status_roundtrip() {
        let statuses = vec![
            CommandExecutionStatus::Started {
                command: "echo".to_string(),
                args: vec!["hello".to_string()],
                correlation_id: "cmd-456".to_string(),
            },
            CommandExecutionStatus::OutputChunk {
                correlation_id: "cmd-456".to_string(),
                chunk: "hello world\n".to_string(),
                stream_type: StreamType::Stdout,
            },
            CommandExecutionStatus::Exit {
                correlation_id: "cmd-456".to_string(),
                exit_code: 0,
                duration_ms: 100,
            },
        ];
        
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: CommandExecutionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }
    
    #[test]
    fn test_diff_operation_roundtrip() {
        let ops = vec![
            DiffOperation::OpenDiffFiles {
                left_path: "/tmp/left.txt".to_string(),
                right_path: "/tmp/right.txt".to_string(),
                correlation_id: "diff-789".to_string(),
            },
            DiffOperation::SaveDiff {
                correlation_id: "diff-789".to_string(),
                target_path: "/tmp/merged.txt".to_string(),
            },
            DiffOperation::RevertDiff {
                correlation_id: "diff-789".to_string(),
            },
            DiffOperation::CloseDiff {
                correlation_id: "diff-789".to_string(),
            },
        ];
        
        for op in ops {
            let json = serde_json::to_string(&op).unwrap();
            let deserialized: DiffOperation = serde_json::from_str(&json).unwrap();
            assert_eq!(op, deserialized);
        }
    }
    
    #[test]
    fn test_approval_message_roundtrip() {
        let messages = vec![
            ApprovalMessage::ApprovalRequested {
                tool_name: "write_file".to_string(),
                operation: "create".to_string(),
                details: serde_json::json!({"path": "/tmp/test.txt"}),
                correlation_id: "appr-111".to_string(),
                timeout_ms: Some(30000),
            },
            ApprovalMessage::ApprovalDecision {
                correlation_id: "appr-111".to_string(),
                approved: true,
                reason: Some("User approved".to_string()),
            },
        ];
        
        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let deserialized: ApprovalMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(msg, deserialized);
        }
    }
    
    #[test]
    fn test_backward_compatibility() {
        // Test that old IpcMessage formats still deserialize
        let old_ack_json = r#"{
            "type": "Ack",
            "origin": "client",
            "data": {
                "clientId": "client-123",
                "pid": 1234,
                "ppid": 1000
            }
        }"#;
        
        let msg: IpcMessage = serde_json::from_str(old_ack_json).unwrap();
        match msg {
            IpcMessage::Ack { origin, data } => {
                assert_eq!(origin, IpcOrigin::Client);
                assert_eq!(data.client_id, "client-123");
                assert_eq!(data.pid, 1234);
            }
            _ => panic!("Expected Ack message"),
        }
        
        // Test old TaskCommand format
        let old_task_json = r#"{
            "type": "TaskCommand",
            "origin": "client",
            "clientId": "client-456",
            "data": {
                "commandName": "CancelTask",
                "data": "task-789"
            }
        }"#;
        
        let msg: IpcMessage = serde_json::from_str(old_task_json).unwrap();
        match msg {
            IpcMessage::TaskCommand { origin, client_id, data } => {
                assert_eq!(origin, IpcOrigin::Client);
                assert_eq!(client_id, "client-456");
                match data {
                    TaskCommand::CancelTask { data } => {
                        assert_eq!(data, "task-789");
                    }
                    _ => panic!("Expected CancelTask"),
                }
            }
            _ => panic!("Expected TaskCommand message"),
        }
    }
    
    #[test]
    fn test_internal_command_serialization() {
        let cmds = vec![
            InternalCommand::OpenDiffFiles {
                left_path: "/tmp/original.txt".to_string(),
                right_path: "/tmp/modified.txt".to_string(),
            },
            InternalCommand::ExecuteProcess {
                program: "ls".to_string(),
                arguments: vec!["-la".to_string(), "/tmp".to_string()],
            },
        ];
        
        for cmd in cmds {
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: InternalCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }
}
