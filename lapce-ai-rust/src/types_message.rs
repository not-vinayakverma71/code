/// Message Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/message.ts
use serde::{Deserialize, Serialize};

/// TokenUsage structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// ClineAsk - Direct translation from TypeScript
/// Lines 29-45 from message.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    FollowUp,  // Alias for compatibility
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ApiCostLimit,  // Added for compatibility
    RequestCostLimit,  // Added for compatibility
    Confirmation,  // Added for compatibility
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    BrowserActionLaunch,
    UseMcpServer,
    AutoApprovalMaxReqReached,
    PaymentRequiredPrompt, // kilocode_change: Added for the low credits dialog
    ReportBug,              // kilocode_change
    Condense,               // kilocode_change
}

/// IdleAsks - Direct translation from TypeScript
/// Lines 61-67
pub const IDLE_ASKS: &[ClineAsk] = &[
    ClineAsk::CompletionResult,
    ClineAsk::ApiReqFailed,
    ClineAsk::ResumeCompletedTask,
    ClineAsk::MistakeLimitReached,
    ClineAsk::AutoApprovalMaxReqReached,
];

pub fn is_idle_ask(ask: ClineAsk) -> bool {
    IDLE_ASKS.contains(&ask)
}

/// ResumableAsks - Direct translation from TypeScript
/// Line 81
pub const RESUMABLE_ASKS: &[ClineAsk] = &[ClineAsk::ResumeTask];

pub fn is_resumable_ask(ask: ClineAsk) -> bool {
    RESUMABLE_ASKS.contains(&ask)
}

/// InteractiveAsks - Direct translation from TypeScript
/// Lines 95-100
pub const INTERACTIVE_ASKS: &[ClineAsk] = &[
    ClineAsk::Command,
    ClineAsk::Tool,
    ClineAsk::BrowserActionLaunch,
    ClineAsk::UseMcpServer,
];

pub fn is_interactive_ask(ask: ClineAsk) -> bool {
    INTERACTIVE_ASKS.contains(&ask)
}

/// ClineSay - Direct translation from TypeScript
/// Lines 114-126
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineSay {
    Followup,
    TextEditor,
    UserFeedback,
    ApiReqStarted,
    ApiReqFinished,
    ApiReqRetried,
    ApiReqFailed,
    RetryApiRequest,
    MistakesExceededThreshold,
    BrowserActionLaunch,
    McpServerResponse,
    // kilocode_change: Removed `tool` from ClineSay
    DiffExceededThreshold, // kilocode_change
}

/// ClineMessage - Direct translation from TypeScript
/// Lines 169-181
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClineMessage {
    #[serde(rename = "ask")]
    Ask {
        #[serde(skip_serializing_if = "Option::is_none")]
        ts: Option<u64>,
        ask: ClineAsk,
        text: Option<String>,
        /// kilocode_change: Made this Option since it's optional in TypeScript
        #[serde(skip_serializing_if = "Option::is_none")]
        kilocode_meta_data: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress_status: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_protected: Option<bool>,
    },
    #[serde(rename = "say")]
    Say {
        say: ClineSay,
        text: Option<String>,
        /// Optional diff information that gets saved to conversation
        #[serde(skip_serializing_if = "Option::is_none")]
        diff: Option<String>,
        /// Save current HTML if present
        #[serde(skip_serializing_if = "Option::is_none")]
        current_html: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress_status: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_protected: Option<bool>,
    },
}

impl ClineMessage {
    pub fn is_partial(&self) -> bool {
        match self {
            ClineMessage::Ask { partial, .. } => partial.unwrap_or(false),
            ClineMessage::Say { partial, .. } => partial.unwrap_or(false),
        }
    }
    
    pub fn is_ask_type(&self, ask_type: ClineAsk) -> bool {
        match self {
            ClineMessage::Ask { ask, .. } => *ask == ask_type,
            _ => false,
        }
    }
    
    pub fn get_ask_type(&self) -> Option<ClineAsk> {
        match self {
            ClineMessage::Ask { ask, .. } => Some(*ask),
            _ => None,
        }
    }
    
    pub fn set_text(&mut self, new_text: Option<String>) {
        match self {
            ClineMessage::Ask { text, .. } => *text = new_text,
            ClineMessage::Say { text, .. } => *text = new_text,
        }
    }
    
    pub fn set_partial(&mut self, is_partial: Option<bool>) {
        match self {
            ClineMessage::Ask { partial, .. } => *partial = is_partial,
            ClineMessage::Say { partial, .. } => *partial = is_partial,
        }
    }
    
    pub fn set_progress_status(&mut self, status: Option<serde_json::Value>) {
        match self {
            ClineMessage::Ask { progress_status, .. } => *progress_status = status,
            ClineMessage::Say { progress_status, .. } => *progress_status = status,
        }
    }
    
    pub fn set_is_protected(&mut self, protected: Option<bool>) {
        match self {
            ClineMessage::Ask { is_protected, .. } => *is_protected = protected,
            ClineMessage::Say { is_protected, .. } => *is_protected = protected,
        }
    }
}
