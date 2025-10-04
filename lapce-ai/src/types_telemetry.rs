/// Telemetry Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/telemetry.ts
use serde::{Deserialize, Serialize};

/// TelemetrySetting - Direct translation from TypeScript
/// Lines 10-14 from telemetry.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetrySetting {
    Unset,
    Enabled,
    Disabled,
}

/// TelemetryEventName - Direct translation from TypeScript
/// Lines 20-110 from telemetry.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryEventName {
    // kilocode_change start
    #[serde(rename = "Commit Message Generated")]
    CommitMsgGenerated,
    #[serde(rename = "Inline Assist Quick Task")]
    InlineAssistQuickTask,
    #[serde(rename = "Inline Assist Auto Task")]
    InlineAssistAutoTask,
    #[serde(rename = "Inline Assist Accept Suggestion")]
    InlineAssistAcceptSuggestion,
    #[serde(rename = "Inline Assist Reject Suggestion")]
    InlineAssistRejectSuggestion,
    #[serde(rename = "Checkpoint Failure")]
    CheckpointFailure,
    #[serde(rename = "Excessive Recursion")]
    ExcessiveRecursion,
    #[serde(rename = "Notification Clicked")]
    NotificationClicked,
    #[serde(rename = "Webview Memory Usage")]
    WebviewMemoryUsage,
    #[serde(rename = "Free Models Link Clicked")]
    FreeModelsLinkClicked,
    #[serde(rename = "Switch To Kilo Code Clicked")]
    SwitchToKiloCodeClicked,
    #[serde(rename = "Suggestion Button Clicked")]
    SuggestionButtonClicked,
    #[serde(rename = "No Assistant Messages")]
    NoAssistantMessages,
    // kilocode_change end
    
    #[serde(rename = "Task Created")]
    TaskCreated,
    #[serde(rename = "Task Reopened")]
    TaskRestarted,
    #[serde(rename = "Task Completed")]
    TaskCompleted,
    #[serde(rename = "Task Message")]
    TaskMessage,
    #[serde(rename = "Conversation Message")]
    TaskConversationMessage,
    #[serde(rename = "LLM Completion")]
    LlmCompletion,
    #[serde(rename = "Mode Switched")]
    ModeSwitch,
    #[serde(rename = "Mode Selector Opened")]
    ModeSelectorOpened,
    #[serde(rename = "Tool Used")]
    ToolUsed,
    
    #[serde(rename = "Checkpoint Created")]
    CheckpointCreated,
    #[serde(rename = "Checkpoint Restored")]
    CheckpointRestored,
    #[serde(rename = "Checkpoint Diffed")]
    CheckpointDiffed,
    
    #[serde(rename = "Tab Shown")]
    TabShown,
    #[serde(rename = "Mode Setting Changed")]
    ModeSettingsChanged,
    #[serde(rename = "Custom Mode Created")]
    CustomModeCreated,
    
    #[serde(rename = "Context Condensed")]
    ContextCondensed,
    #[serde(rename = "Sliding Window Truncation")]
    SlidingWindowTruncation,
    
    #[serde(rename = "Code Action Used")]
    CodeActionUsed,
    #[serde(rename = "Prompt Enhanced")]
    PromptEnhanced,
    
    #[serde(rename = "Title Button Clicked")]
    TitleButtonClicked,
    
    #[serde(rename = "Authentication Initiated")]
    AuthenticationInitiated,
    
    #[serde(rename = "Marketplace Item Installed")]
    MarketplaceItemInstalled,
    #[serde(rename = "Marketplace Item Removed")]
    MarketplaceItemRemoved,
    #[serde(rename = "Marketplace Tab Viewed")]
    MarketplaceTabViewed,
    #[serde(rename = "Marketplace Install Button Clicked")]
    MarketplaceInstallButtonClicked,
    
    #[serde(rename = "Share Button Clicked")]
    ShareButtonClicked,
    #[serde(rename = "Share Organization Clicked")]
    ShareOrganizationClicked,
    #[serde(rename = "Share Public Clicked")]
    SharePublicClicked,
    #[serde(rename = "Share Connect To Cloud Clicked")]
    ShareConnectToCloudClicked,
    
    #[serde(rename = "Account Connect Clicked")]
    AccountConnectClicked,
    #[serde(rename = "Account Connect Success")]
    AccountConnectSuccess,
    #[serde(rename = "Account Logout Clicked")]
    AccountLogoutClicked,
    #[serde(rename = "Account Logout Success")]
    AccountLogoutSuccess,
    
    #[serde(rename = "Schema Validation Error")]
    SchemaValidationError,
}
