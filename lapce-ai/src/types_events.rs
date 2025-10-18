/// Event Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/events.ts
use serde::{Deserialize, Serialize};
use crate::ipc_messages::ClineMessage;
use crate::types_tool::{ToolName, ToolUsage};

/// RooCodeEventName - Direct translation from TypeScript
/// Lines 10-42 from events.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RooCodeEventName {
    // Task Provider Lifecycle
    TaskCreated,
    
    // Task Lifecycle
    TaskStarted,
    TaskCompleted,
    TaskAborted,
    TaskFocused,
    TaskUnfocused,
    TaskActive,
    TaskInteractive,
    TaskResumable,
    TaskIdle,
    
    // Subtask Lifecycle
    TaskPaused,
    TaskUnpaused,
    TaskSpawned,
    
    // Task Execution
    Message,
    TaskModeSwitched,
    TaskAskResponded,
    
    // Task Analytics
    TaskTokenUsageUpdated,
    TaskToolFailed,
    
    // Evals
    EvalPass,
    EvalFail,
}

/// TokenUsage - placeholder (need to translate from message.ts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

/// TaskEvent - Direct translation from TypeScript discriminated union
/// Lines 92-207 from events.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "eventName")]
pub enum TaskEvent {
    // Task Provider Lifecycle
    #[serde(rename = "taskCreated")]
    TaskCreated {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    // Task Lifecycle
    #[serde(rename = "taskStarted")]
    TaskStarted {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskCompleted")]
    TaskCompleted {
        payload: TaskCompletedPayload,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskAborted")]
    TaskAborted {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskFocused")]
    TaskFocused {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskUnfocused")]
    TaskUnfocused {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskActive")]
    TaskActive {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskInteractive")]
    TaskInteractive {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskResumable")]
    TaskResumable {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskIdle")]
    TaskIdle {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    // Subtask Lifecycle
    #[serde(rename = "taskPaused")]
    TaskPaused {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskUnpaused")]
    TaskUnpaused {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskSpawned")]
    TaskSpawned {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    // Task Execution
    #[serde(rename = "message")]
    Message {
        payload: Vec<MessagePayload>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskModeSwitched")]
    TaskModeSwitched {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskAskResponded")]
    TaskAskResponded {
        payload: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    // Task Analytics
    #[serde(rename = "taskToolFailed")]
    TaskToolFailed {
        payload: TaskToolFailedPayload,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
    
    #[serde(rename = "taskTokenUsageUpdated")]
    TaskTokenUsageUpdated {
        payload: Vec<TokenUsage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<u32>,
    },
}

/// TaskCompletedPayload - from TypeScript lines 52-59
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletedPayload {
    pub task_id: String,
    pub token_usage: TokenUsage,
    pub tool_usage: ToolUsage,
    pub is_subtask: bool,
}

/// MessagePayload - from TypeScript lines 72-78
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    pub task_id: String,
    pub action: MessageAction,
    pub message: ClineMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageAction {
    Created,
    Updated,
}

/// TaskToolFailedPayload - from TypeScript line 82
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolFailedPayload {
    pub task_id: String,
    pub tool_name: ToolName,
    pub error: String,
}
