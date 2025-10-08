/// Message Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/message.ts
// Don't re-export ClineMessage here - it's already in ipc_messages

use serde::{Deserialize, Serialize};

/// TokenUsage structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// ClineAsk is defined in ipc_messages.rs - remove duplicate definition
use crate::ipc_messages::ClineAsk;

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
    DiffExceededThreshold, 
}

/// ClineMessage - Re-exported from ipc_messages.rs (ClineAsk already imported above)
pub use crate::ipc_messages::ClineMessage;
