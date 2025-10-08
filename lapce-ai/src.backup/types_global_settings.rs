/// Global Settings - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/global-settings.ts
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default delay in milliseconds after writes to allow diagnostics to detect potential problems.
/// Line 24 from global-settings.ts
pub const DEFAULT_WRITE_DELAY_MS: u32 = 1000;

/// Default terminal output character limit constant.
/// Line 31 from global-settings.ts
pub const DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT: usize = 50_000;

/// GlobalSettings - Direct translation from TypeScript
/// Lines 37-133 from global-settings.ts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_api_config_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_api_config_meta: Option<Vec<ProviderSettingsEntry>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_api_configs: Option<HashMap<String, bool>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_shown_announcement_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_history: Option<Vec<HistoryItem>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condensing_api_config_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_condensing_prompt: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_approval_enabled: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_read_only: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_read_only_outside_workspace: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_write: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_write_outside_workspace: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_write_protected: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_delay_ms: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_browser: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_approve_resubmit: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_delay_seconds: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_mcp: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_mode_switch: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_subtasks: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_execute: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_followup_questions: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_auto_approve_timeout_ms: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_allow_update_todo_list: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_commands: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_commands: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_execution_timeout: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_timeout_allowlist: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prevent_completion_with_open_todos: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_max_requests: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_max_cost: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_condense_context: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_condense_context_percent: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_file_reads: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_very_large_reads: Option<bool>, // kilocode_change
    
    /// Whether to include diagnostic messages (errors, warnings) in tool outputs
    /// Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_diagnostic_messages: Option<bool>,
    
    /// Maximum number of diagnostic messages to include in tool outputs
    /// Default: 50
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_diagnostic_messages: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_tool_enabled: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_viewport_size: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_auto_approve_menu: Option<bool>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_task_timeline: Option<bool>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_workflow_toggles: Option<HashMap<String, bool>>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_workflow_toggles: Option<HashMap<String, bool>>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_rules_toggles: Option<HashMap<String, bool>>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_rules_toggles: Option<HashMap<String, bool>>, // kilocode_change
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_quality: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_browser_enabled: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_browser_host: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_chrome_host_url: Option<String>,
}

/// Placeholder types - will translate these from their respective TypeScript files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettingsEntry {
    // TODO: Translate from provider-settings.ts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    // TODO: Translate from history.ts
}

/// RooCodeSettings - Direct translation from TypeScript
/// Lines 157-259 from global-settings.ts  
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RooCodeSettings {
    pub global_settings: GlobalSettings,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_settings: Option<serde_json::Value>, // ProviderSettings type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry_settings: Option<serde_json::Value>, // TelemetrySettings type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghost_service_settings: Option<serde_json::Value>, // GhostServiceSettings type (kilocode_change)
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_models: Option<serde_json::Value>, // CodebaseIndexModels type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_config: Option<serde_json::Value>, // CodebaseIndexConfig type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiments: Option<serde_json::Value>, // Experiments type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_configs: Option<serde_json::Value>, // ModeConfigs type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_mode_prompts: Option<serde_json::Value>, // CustomModePrompts type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_support_prompts: Option<serde_json::Value>, // CustomSupportPrompts type
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<serde_json::Value>, // Languages type
}
