/// Exact 1:1 Translation of TypeScript Events from Codex/packages/types/src/events.ts
/// This is NOT a rewrite - it's a direct translation maintaining same logic and flow
use serde::{Serialize, Deserialize};
use crate::ipc_messages::ClineMessage;

/// RooCodeEventName enum - exact translation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// TokenUsage structure - exact match from message.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    #[serde(default)]
    pub total_tokens_in: u32,
    #[serde(default)]
    pub total_tokens_out: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    #[serde(default)]
    pub total_cost: f64,
    #[serde(default)]
    pub context_tokens: u32,
}

/// ToolUsage structure - from tool.ts as Record type
/// In TypeScript: z.record(toolNamesSchema, z.object({ attempts: z.number(), failures: z.number() }))
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    // Using HashMap to match TypeScript Record type
    #[serde(flatten)]
    pub tools: std::collections::HashMap<String, ToolStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub attempts: u32,
    pub failures: u32,
}

/// ToolNames enum (from tool.js)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolNames {
    ExecuteCommand,
    ReadFile,
    WriteFile,
    SearchFiles,
    ListFiles,
    ListCodeDefinitionNames,
    BrowserAction,
    AskFollowupQuestion,
    AttemptCompletion,
    UrlScreenshot,
}

// ClineMessage and ClineAsk are imported from ipc_messages.rs

/// MessageAction for Message event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageAction {
    Created,
    Updated,
}

/// MessageEventPayload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEventPayload {
    pub task_id: String,
    pub action: MessageAction,
    pub message: ClineMessage,
}

/// TaskCompletedPayload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompletedPayload {
    pub task_id: String,
    pub token_usage: TokenUsage,
    pub tool_usage: ToolUsage,
    pub is_subtask: bool,
}

/// Metadata for TaskCompleted event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompletedMetadata {
    pub is_subtask: bool,
}

/// TaskEvent - exact translation with discriminated union matching TypeScript structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "eventName")]
pub enum TaskEvent {
    // Task Provider Lifecycle
    #[serde(rename = "taskCreated")]
    TaskCreated {
        payload: (String,), // z.tuple([z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    
    // Task Lifecycle
    #[serde(rename = "taskStarted")]
    TaskStarted {
        payload: (String,), // z.tuple([z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskCompleted")]
    TaskCompleted {
        payload: (String, TokenUsage, ToolUsage, TaskCompletedMetadata), // 4-tuple
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskAborted")]
    TaskAborted {
        payload: (String,), // z.tuple([z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskFocused")]
    TaskFocused {
        payload: (String,), // z.tuple([z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskUnfocused")]
    TaskUnfocused {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskActive")]
    TaskActive {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskInteractive")]
    TaskInteractive {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskResumable")]
    TaskResumable {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskIdle")]
    TaskIdle {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    
    // Subtask Lifecycle
    #[serde(rename = "taskPaused")]
    TaskPaused {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskUnpaused")]
    TaskUnpaused {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskSpawned")]
    TaskSpawned {
        payload: (String, String), // z.tuple([z.string(), z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    
    // Task Execution
    #[serde(rename = "message")]
    Message {
        payload: (MessageEventPayload,), // z.tuple([z.object({...})])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskModeSwitched")]
    TaskModeSwitched {
        payload: (String, String), // z.tuple([z.string(), z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskAskResponded")]
    TaskAskResponded {
        payload: (String,),
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    
    // Task Analytics
    #[serde(rename = "taskToolFailed")]
    TaskToolFailed {
        payload: (String, ToolNames, String), // z.tuple([z.string(), toolNamesSchema, z.string()])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    #[serde(rename = "taskTokenUsageUpdated")]
    TaskTokenUsageUpdated {
        payload: (String, TokenUsage), // z.tuple([z.string(), tokenUsageSchema])
        #[serde(rename = "taskId")]
        task_id: Option<u32>,
    },
    
    // Evals
    #[serde(rename = "evalPass")]
    EvalPass {
        payload: (), // z.undefined() - empty tuple in Rust
        #[serde(rename = "taskId")]
        task_id: u32, // Required for eval events (not optional)
    },
    #[serde(rename = "evalFail")]
    EvalFail {
        payload: (), // z.undefined() - empty tuple in Rust
        #[serde(rename = "taskId")]
        task_id: u32, // Required for eval events (not optional)
    },
}

/// RooCodeEvents trait - representing TypeScript event structure
pub trait RooCodeEvents {
    fn on_task_created(&mut self, task_id: String);
    fn on_task_started(&mut self, task_id: String);
    fn on_task_completed(&mut self, task_id: String, token_usage: TokenUsage, tool_usage: ToolUsage, metadata: TaskCompletedMetadata);
    fn on_task_aborted(&mut self, task_id: String);
    fn on_task_focused(&mut self, task_id: String);
    fn on_task_unfocused(&mut self, task_id: String);
    fn on_task_active(&mut self, task_id: String);
    fn on_task_interactive(&mut self, task_id: String);
    fn on_task_resumable(&mut self, task_id: String);
    fn on_task_idle(&mut self, task_id: String);
    fn on_task_paused(&mut self, task_id: String);
    fn on_task_unpaused(&mut self, task_id: String);
    fn on_task_spawned(&mut self, task_id: String, spawned_task_id: String);
    fn on_message(&mut self, payload: MessageEventPayload);
    fn on_task_mode_switched(&mut self, task_id: String, mode: String);
    fn on_task_ask_responded(&mut self, task_id: String);
    fn on_task_tool_failed(&mut self, task_id: String, tool: ToolNames, error: String);
    fn on_task_token_usage_updated(&mut self, task_id: String, usage: TokenUsage);
    fn on_eval_pass(&mut self, task_id: u32);
    fn on_eval_fail(&mut self, task_id: u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_serialization() {
        let event = TaskEvent::TaskStarted {
            payload: ("task-123".to_string(),),
            task_id: Some(1),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"eventName\":\"taskStarted\""));
        assert!(json.contains("\"taskId\":1"));
    }
    
    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage {
            total_tokens_in: 100,
            total_tokens_out: 200,
            cache_write_tokens: Some(50),
            cache_read_tokens: None,
            input_tokens: 100,
            output_tokens: 200,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            total_cost: 0.05,
            context_tokens: 300,
        };
        
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"totalTokensIn\":100"));
        assert!(json.contains("\"totalTokensOut\":200"));
    }
}
