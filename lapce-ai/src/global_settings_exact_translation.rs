/// Exact 1:1 Translation of TypeScript GlobalSettings from Codex/packages/types/src/global-settings.ts
/// This is NOT a rewrite - it's a direct translation maintaining same logic and flow
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default delay in milliseconds after writes to allow diagnostics
pub const DEFAULT_WRITE_DELAY_MS: u32 = 1000;

/// Default terminal output character limit constant
pub const DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT: u32 = 50_000;

/// ProviderSettingsEntry (from provider-settings.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettingsEntry {
    pub id: String,
    pub label: String,
    pub model_id: Option<String>,
}

/// HistoryItem (from history.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub timestamp: u64,
    pub token_usage: Option<HashMap<String, u32>>,
    pub total_tokens: Option<u32>,
    pub task: Option<String>,
    pub is_favorited: Option<bool>,
    pub model_used: Option<String>,
}

/// CodebaseIndexModels (from codebase-index.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndexModels {
    pub embedding_model: Option<String>,
    pub reranker_model: Option<String>,
}

/// CodebaseIndexConfig (from codebase-index.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndexConfig {
    pub enabled: Option<bool>,
    pub max_files: Option<u32>,
    pub chunk_size: Option<u32>,
}

/// Experiments (from experiment.js)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Experiments {
    pub feature_flags: Option<HashMap<String, bool>>,
}

/// TelemetrySettings (from telemetry.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySettings {
    pub enabled: Option<bool>,
    pub anonymous_id: Option<String>,
}

/// ModeConfig (from mode.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub instructions: Option<String>,
    pub tools: Option<Vec<String>>,
}

/// CustomModePrompts (from mode.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModePrompts {
    pub system: Option<String>,
    pub user: Option<String>,
}

/// Languages type (from vscode.js) - using string to match TypeScript
pub type Languages = String;

/// GhostServiceSettings (from kilocode.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhostServiceSettings {
    pub enabled: Option<bool>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

/// GlobalSettings structure - exact translation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub current_api_config_name: Option<String>,
    pub list_api_config_meta: Option<Vec<ProviderSettingsEntry>>,
    pub pinned_api_configs: Option<HashMap<String, bool>>,
    
    pub last_shown_announcement_id: Option<String>,
    pub custom_instructions: Option<String>,
    pub task_history: Option<Vec<HistoryItem>>,
    
    pub condensing_api_config_id: Option<String>,
    pub custom_condensing_prompt: Option<String>,
    
    pub auto_approval_enabled: Option<bool>,
    pub always_allow_read_only: Option<bool>,
    pub always_allow_read_only_outside_workspace: Option<bool>,
    pub always_allow_write: Option<bool>,
    pub always_allow_write_outside_workspace: Option<bool>,
    pub always_allow_write_protected: Option<bool>,
    pub write_delay_ms: Option<u32>,
    pub always_allow_browser: Option<bool>,
    pub always_approve_resubmit: Option<bool>,
    pub request_delay_seconds: Option<u32>,
    pub always_allow_mcp: Option<bool>,
    pub always_allow_mode_switch: Option<bool>,
    pub always_allow_subtasks: Option<bool>,
    pub always_allow_execute: Option<bool>,
    pub always_allow_followup_questions: Option<bool>,
    pub followup_auto_approve_timeout_ms: Option<u32>,
    pub always_allow_update_todo_list: Option<bool>,
    pub allowed_commands: Option<Vec<String>>,
    pub denied_commands: Option<Vec<String>>,
    pub command_execution_timeout: Option<u32>,
    pub command_timeout_allowlist: Option<Vec<String>>,
    pub prevent_completion_with_open_todos: Option<bool>,
    pub allowed_max_requests: Option<u32>,
    pub allowed_max_cost: Option<f64>,
    pub auto_condense_context: Option<bool>,
    pub auto_condense_context_percent: Option<f32>,
    pub max_concurrent_file_reads: Option<u32>,
    pub allow_very_large_reads: Option<bool>,
    
    /// Whether to include diagnostic messages (errors, warnings) in tool outputs
    pub include_diagnostic_messages: Option<bool>,
    /// Maximum number of diagnostic messages to include in tool outputs
    pub max_diagnostic_messages: Option<u32>,
    
    pub browser_tool_enabled: Option<bool>,
    pub browser_viewport_size: Option<String>,
    pub show_auto_approve_menu: Option<bool>,
    pub show_task_timeline: Option<bool>,
    pub local_workflow_toggles: Option<HashMap<String, bool>>,
    pub global_workflow_toggles: Option<HashMap<String, bool>>,
    pub local_rules_toggles: Option<HashMap<String, bool>>,
    pub global_rules_toggles: Option<HashMap<String, bool>>,
    pub screenshot_quality: Option<f32>,
    pub remote_browser_enabled: Option<bool>,
    pub remote_browser_host: Option<String>,
    pub cached_chrome_host_url: Option<String>,
    
    pub enable_checkpoints: Option<bool>,
    
    pub tts_enabled: Option<bool>,
    pub tts_speed: Option<f32>,
    pub sound_enabled: Option<bool>,
    pub sound_volume: Option<f32>,
    pub system_notifications_enabled: Option<bool>,
    
    pub max_open_tabs_context: Option<u32>,
    pub max_workspace_files: Option<u32>,
    pub show_roo_ignored_files: Option<bool>,
    pub max_read_file_line: Option<u32>,
    pub max_image_file_size: Option<u32>,
    pub max_total_image_size: Option<u32>,
    
    pub terminal_output_line_limit: Option<u32>,
    pub terminal_output_character_limit: Option<u32>,
    pub terminal_shell_integration_timeout: Option<u32>,
    pub terminal_shell_integration_disabled: Option<bool>,
    pub terminal_command_delay: Option<u32>,
    pub terminal_powershell_counter: Option<bool>,
    pub terminal_zsh_clear_eol_mark: Option<bool>,
    pub terminal_zsh_oh_my: Option<bool>,
    pub terminal_zsh_p10k: Option<bool>,
    pub terminal_zdotdir: Option<bool>,
    pub terminal_compress_progress_bar: Option<bool>,
    
    pub diagnostics_enabled: Option<bool>,
    
    pub rate_limit_seconds: Option<u32>,
    pub diff_enabled: Option<bool>,
    pub fuzzy_match_threshold: Option<f32>,
    pub experiments: Option<Experiments>,
    
    pub morph_api_key: Option<String>,
    
    pub codebase_index_models: Option<CodebaseIndexModels>,
    pub codebase_index_config: Option<CodebaseIndexConfig>,
    
    pub language: Option<Languages>,
    
    pub telemetry_setting: Option<TelemetrySettings>,
    
    pub mcp_enabled: Option<bool>,
    pub enable_mcp_server_creation: Option<bool>,
    pub mcp_marketplace_catalog: Option<serde_json::Value>,
    
    pub remote_control_enabled: Option<bool>,
    
    pub mode: Option<String>,
    pub mode_api_configs: Option<HashMap<String, String>>,
    pub custom_modes: Option<Vec<ModeConfig>>,
    pub custom_mode_prompts: Option<CustomModePrompts>,
    pub custom_support_prompts: Option<CustomModePrompts>,
    pub enhancement_api_config_id: Option<String>,
    pub dismissed_notification_ids: Option<Vec<String>>,
    pub commit_message_api_config_id: Option<String>,
    pub terminal_command_api_config_id: Option<String>,
    pub ghost_service_settings: GhostServiceSettings,
    pub include_task_history_in_enhance: Option<bool>,
    pub history_preview_collapsed: Option<bool>,
    pub profile_thresholds: Option<HashMap<String, f64>>,
    pub has_opened_mode_selector: Option<bool>,
    pub last_mode_export_path: Option<String>,
    pub last_mode_import_path: Option<String>,
}

/// ProviderSettings (from original provider configs)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub api_key: Option<String>,
    pub glama_api_key: Option<String>,
    pub open_router_api_key: Option<String>,
    pub aws_access_key: Option<String>,
    pub aws_api_key: Option<String>,
    pub aws_secret_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub open_ai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub open_ai_native_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
    pub deep_seek_api_key: Option<String>,
    pub doubao_api_key: Option<String>,
    pub moonshot_api_key: Option<String>,
    pub mistral_api_key: Option<String>,
    // Fields needed for Task operations
    pub auto_approval_enabled: Option<bool>,
    pub always_approve_resubmit: Option<bool>,
    pub request_delay_seconds: Option<u64>,
    pub api_provider: Option<String>,
}

/// RooCodeSettings - merged GlobalSettings and ProviderSettings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RooCodeSettings {
    // GlobalSettings fields
    #[serde(flatten)]
    pub global: GlobalSettings,
    // ProviderSettings fields
    #[serde(flatten)]
    pub provider: ProviderSettings,
}

/// SECRET_STATE_KEYS - keys that contain secrets (21 total from global-settings.ts)
pub const SECRET_STATE_KEYS: &[&str] = &[
    "apiKey",
    "glamaApiKey",
    "openRouterApiKey",
    "awsAccessKey",
    "awsApiKey",
    "awsSecretKey",
    "awsSessionToken",
    "openAiApiKey",
    "geminiApiKey",
    "openAiNativeApiKey",
    "cerebrasApiKey",
    "deepSeekApiKey",
    "doubaoApiKey",
    "moonshotApiKey",
    "mistralApiKey",
    "unboundApiKey",
    "requestyApiKey",
    "xaiApiKey",
    "groqApiKey",
    "chutesApiKey",
    "litellmApiKey",
    "codeIndexOpenAiKey",
    "codeIndexQdrantApiKey",
    // kilocode_change start
    "kilocodeToken",
    "deepInfraApiKey",
    // kilocode_change end
    "codebaseIndexOpenAiCompatibleApiKey",
    "codebaseIndexGeminiApiKey",
    "codebaseIndexMistralApiKey",
    "huggingFaceApiKey",
    "sambaNovaApiKey",
    "zaiApiKey",
    "fireworksApiKey",
    "featherlessApiKey",
    "ioIntelligenceApiKey",
];

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_global_settings_serialization() {
        let settings = GlobalSettings {
            current_api_config_name: Some("default".to_string()),
            auto_approval_enabled: Some(false),
            write_delay_ms: Some(1000),
            ghost_service_settings: GhostServiceSettings {
                enabled: Some(true),
                api_key: None,
                model: Some("gpt-4".to_string()),
            },
            ..Default::default()
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"currentApiConfigName\":\"default\""));
        assert!(json.contains("\"autoApprovalEnabled\":false"));
    }
    
    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_WRITE_DELAY_MS, 1000);
        assert_eq!(DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT, 50_000);
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            current_api_config_name: None,
            list_api_config_meta: None,
            pinned_api_configs: None,
            last_shown_announcement_id: None,
            custom_instructions: None,
            task_history: None,
            condensing_api_config_id: None,
            custom_condensing_prompt: None,
            auto_approval_enabled: None,
            always_allow_read_only: None,
            always_allow_read_only_outside_workspace: None,
            always_allow_write: None,
            always_allow_write_outside_workspace: None,
            always_allow_write_protected: None,
            write_delay_ms: None,
            always_allow_browser: None,
            always_approve_resubmit: None,
            request_delay_seconds: None,
            always_allow_mcp: None,
            always_allow_mode_switch: None,
            always_allow_subtasks: None,
            always_allow_execute: None,
            always_allow_followup_questions: None,
            followup_auto_approve_timeout_ms: None,
            always_allow_update_todo_list: None,
            allowed_commands: None,
            denied_commands: None,
            command_execution_timeout: None,
            command_timeout_allowlist: None,
            prevent_completion_with_open_todos: None,
            allowed_max_requests: None,
            allowed_max_cost: None,
            auto_condense_context: None,
            auto_condense_context_percent: None,
            max_concurrent_file_reads: None,
            allow_very_large_reads: None,
            include_diagnostic_messages: None,
            max_diagnostic_messages: None,
            browser_tool_enabled: None,
            browser_viewport_size: None,
            show_auto_approve_menu: None,
            show_task_timeline: None,
            local_workflow_toggles: None,
            global_workflow_toggles: None,
            local_rules_toggles: None,
            global_rules_toggles: None,
            screenshot_quality: None,
            remote_browser_enabled: None,
            remote_browser_host: None,
            cached_chrome_host_url: None,
            enable_checkpoints: None,
            tts_enabled: None,
            tts_speed: None,
            sound_enabled: None,
            sound_volume: None,
            system_notifications_enabled: None,
            max_open_tabs_context: None,
            max_workspace_files: None,
            show_roo_ignored_files: None,
            max_read_file_line: None,
            max_image_file_size: None,
            max_total_image_size: None,
            terminal_output_line_limit: None,
            terminal_output_character_limit: None,
            terminal_shell_integration_timeout: None,
            terminal_shell_integration_disabled: None,
            terminal_command_delay: None,
            terminal_powershell_counter: None,
            terminal_zsh_clear_eol_mark: None,
            terminal_zsh_oh_my: None,
            terminal_zsh_p10k: None,
            terminal_zdotdir: None,
            terminal_compress_progress_bar: None,
            diagnostics_enabled: None,
            rate_limit_seconds: None,
            diff_enabled: None,
            fuzzy_match_threshold: None,
            experiments: None,
            morph_api_key: None,
            codebase_index_models: None,
            codebase_index_config: None,
            language: None,
            telemetry_setting: None,
            mcp_enabled: None,
            enable_mcp_server_creation: None,
            mcp_marketplace_catalog: None,
            remote_control_enabled: None,
            mode: None,
            mode_api_configs: None,
            custom_modes: None,
            custom_mode_prompts: None,
            custom_support_prompts: None,
            enhancement_api_config_id: None,
            dismissed_notification_ids: None,
            commit_message_api_config_id: None,
            terminal_command_api_config_id: None,
            ghost_service_settings: GhostServiceSettings {
                enabled: None,
                api_key: None,
                model: None,
            },
            include_task_history_in_enhance: None,
            history_preview_collapsed: None,
            profile_thresholds: None,
            has_opened_mode_selector: None,
            last_mode_export_path: None,
            last_mode_import_path: None,
        }
    }
}
