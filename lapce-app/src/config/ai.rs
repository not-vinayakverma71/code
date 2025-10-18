// AI Configuration
// Complete settings for AI features (ported from Codex)
// See AI_SETTINGS_MAP.md for full Codex→Lapce mapping

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AIConfig {
    // ============================================================
    // CRITICAL PRIORITY (10 settings)
    // ============================================================
    
    /// Default AI model to use
    #[serde(default)]
    pub default_model: String,
    
    /// Show model selector in toolbar
    #[serde(default = "default_show_model_selector")]
    pub show_model_selector: bool,
    
    /// Show context panel sidebar
    #[serde(default = "default_show_context_panel")]
    pub show_context_panel: bool,
    
    /// API request timeout in seconds
    #[serde(default = "default_api_request_timeout_secs")]
    pub api_request_timeout_secs: u32,
    
    /// Max image file size in MB
    #[serde(default = "default_max_image_file_mb")]
    pub max_image_file_mb: u32,
    
    /// Max total image size in MB
    #[serde(default = "default_max_total_image_mb")]
    pub max_total_image_mb: u32,
    
    /// Max read file lines (-1 = unlimited)
    #[serde(default = "default_max_read_file_lines")]
    pub max_read_file_lines: i32,
    
    /// History preview collapsed
    #[serde(default)]
    pub history_preview_collapsed: bool,
    
    /// Include task history in enhance
    #[serde(default = "default_include_task_history_in_enhance")]
    pub include_task_history_in_enhance: bool,
    
    /// Show task timeline
    #[serde(default = "default_show_task_timeline")]
    pub show_task_timeline: bool,
    
    /// Show timestamps
    #[serde(default = "default_show_timestamps")]
    pub show_timestamps: bool,
    
    // ============================================================
    // HIGH PRIORITY (15+ settings)
    // ============================================================
    
    // --- Modes ---
    /// Current mode
    #[serde(default)]
    pub mode: String,
    
    /// Custom modes (JSON serialized)
    #[serde(default)]
    pub custom_modes: Vec<serde_json::Value>,
    
    /// Custom mode prompts (JSON)
    #[serde(default)]
    pub custom_mode_prompts: serde_json::Value,
    
    /// Custom support prompts (JSON)
    #[serde(default)]
    pub custom_support_prompts: serde_json::Value,
    
    /// Has opened mode selector (onboarding)
    #[serde(default)]
    pub has_opened_mode_selector: bool,
    
    /// Fast apply model
    #[serde(default)]
    pub fast_apply_model: String,
    
    // --- Auto-Approve Toggles ---
    /// Always allow read-only operations
    #[serde(default = "default_always_allow_read_only")]
    pub always_allow_read_only: bool,
    
    /// Always allow read-only outside workspace
    #[serde(default)]
    pub always_allow_read_only_outside_workspace: bool,
    
    /// Always allow write operations
    #[serde(default = "default_always_allow_write")]
    pub always_allow_write: bool,
    
    /// Always allow write outside workspace
    #[serde(default)]
    pub always_allow_write_outside_workspace: bool,
    
    /// Always allow write to protected files
    #[serde(default)]
    pub always_allow_write_protected: bool,
    
    /// Always allow browser operations
    #[serde(default)]
    pub always_allow_browser: bool,
    
    /// Always allow execute commands
    #[serde(default)]
    pub always_allow_execute: bool,
    
    /// Always allow MCP operations
    #[serde(default)]
    pub always_allow_mcp: bool,
    
    /// Always allow mode switch
    #[serde(default)]
    pub always_allow_mode_switch: bool,
    
    /// Always allow subtasks
    #[serde(default)]
    pub always_allow_subtasks: bool,
    
    /// Always approve resubmit
    #[serde(default)]
    pub always_approve_resubmit: bool,
    
    /// Always allow follow-up questions
    #[serde(default)]
    pub always_allow_followup_questions: bool,
    
    /// Always allow update todo list
    #[serde(default = "default_always_allow_update_todo_list")]
    pub always_allow_update_todo_list: bool,
    
    // --- Auto-Approve Limits & Lists ---
    /// Allowed commands list
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    
    /// Denied commands list
    #[serde(default)]
    pub denied_commands: Vec<String>,
    
    /// Max requests allowed
    #[serde(default)]
    pub allowed_max_requests: Option<u32>,
    
    /// Max cost allowed
    #[serde(default)]
    pub allowed_max_cost: Option<f32>,
    
    /// Request delay in seconds
    #[serde(default = "default_request_delay_seconds")]
    pub request_delay_seconds: u32,
    
    /// Follow-up auto-approve timeout in ms
    #[serde(default)]
    pub followup_auto_approve_timeout_ms: Option<u32>,
    
    /// Show auto-approve menu
    #[serde(default)]
    pub show_auto_approve_menu: bool,
    
    /// Auto-approval enabled (master toggle)
    #[serde(default = "default_auto_approval_enabled")]
    pub auto_approval_enabled: bool,
    
    // --- Task Management ---
    /// New task requires todos
    #[serde(default)]
    pub new_task_require_todos: bool,
    
    /// Max open tabs context
    #[serde(default = "default_max_open_tabs_context")]
    pub max_open_tabs_context: u32,
    
    /// Max workspace files
    #[serde(default = "default_max_workspace_files")]
    pub max_workspace_files: u32,
    
    /// Use agent rules
    #[serde(default = "default_use_agent_rules")]
    pub use_agent_rules: bool,
    
    // ============================================================
    // MEDIUM PRIORITY (33 settings)
    // ============================================================
    
    // --- Providers & API Config ---
    /// Current API config name
    #[serde(default = "default_current_api_config_name")]
    pub current_api_config_name: String,
    
    /// Pinned API configs
    #[serde(default)]
    pub pinned_api_configs: HashMap<String, bool>,
    
    /// Condensing API config ID
    #[serde(default)]
    pub condensing_api_config_id: String,
    
    /// Enhancement API config ID
    #[serde(default)]
    pub enhancement_api_config_id: String,
    
    /// Commit message API config ID
    #[serde(default)]
    pub commit_message_api_config_id: String,
    
    /// API configuration (opaque JSON for IPC)
    #[serde(default)]
    pub api_configuration: serde_json::Value,
    
    // --- Browser ---
    /// Browser tool enabled
    #[serde(default = "default_browser_tool_enabled")]
    pub browser_tool_enabled: bool,
    
    /// Browser viewport size (e.g., "900x600")
    #[serde(default = "default_browser_viewport_size")]
    pub browser_viewport_size: String,
    
    /// Screenshot quality (0-100)
    #[serde(default = "default_screenshot_quality")]
    pub screenshot_quality: u8,
    
    /// Remote browser host
    #[serde(default)]
    pub remote_browser_host: String,
    
    /// Remote browser enabled
    #[serde(default)]
    pub remote_browser_enabled: bool,
    
    // --- Terminal ---
    /// Terminal output line limit
    #[serde(default = "default_terminal_output_line_limit")]
    pub terminal_output_line_limit: u32,
    
    /// Terminal output character limit
    #[serde(default = "default_terminal_output_character_limit")]
    pub terminal_output_character_limit: u32,
    
    /// Terminal shell integration timeout (ms)
    #[serde(default = "default_terminal_shell_integration_timeout")]
    pub terminal_shell_integration_timeout: u32,
    
    /// Terminal shell integration disabled
    #[serde(default)]
    pub terminal_shell_integration_disabled: bool,
    
    /// Terminal ZDOTDIR handling
    #[serde(default)]
    pub terminal_zdotdir: bool,
    
    /// Terminal Zsh Oh My Zsh
    #[serde(default)]
    pub terminal_zsh_oh_my: bool,
    
    /// Terminal Zsh Powerlevel10k
    #[serde(default)]
    pub terminal_zsh_p10k: bool,
    
    /// Terminal compress progress bar
    #[serde(default = "default_terminal_compress_progress_bar")]
    pub terminal_compress_progress_bar: bool,
    
    /// Terminal command delay (ms)
    #[serde(default)]
    pub terminal_command_delay: u32,
    
    /// Terminal PowerShell counter
    #[serde(default)]
    pub terminal_powershell_counter: bool,
    
    /// Terminal Zsh clear EOL mark
    #[serde(default)]
    pub terminal_zsh_clear_eol_mark: bool,
    
    /// Terminal command API config ID
    #[serde(default)]
    pub terminal_command_api_config_id: String,
    
    // --- Display ---
    /// Reasoning block collapsed
    #[serde(default = "default_reasoning_block_collapsed")]
    pub reasoning_block_collapsed: bool,
    
    /// Hide cost below threshold
    #[serde(default)]
    pub hide_cost_below_threshold: f32,
    
    /// Diff enabled
    #[serde(default)]
    pub diff_enabled: bool,
    
    /// Enable checkpoints
    #[serde(default = "default_enable_checkpoints")]
    pub enable_checkpoints: bool,
    
    // --- Notifications ---
    /// Sound enabled
    #[serde(default)]
    pub sound_enabled: bool,
    
    /// Sound volume (0.0-1.0)
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,
    
    /// TTS enabled
    #[serde(default)]
    pub tts_enabled: bool,
    
    /// TTS speed
    #[serde(default = "default_tts_speed")]
    pub tts_speed: f32,
    
    /// System notifications enabled
    #[serde(default)]
    pub system_notifications_enabled: bool,
    
    // --- Context Management ---
    /// Auto condense context
    #[serde(default = "default_auto_condense_context")]
    pub auto_condense_context: bool,
    
    /// Auto condense context percent
    #[serde(default = "default_auto_condense_context_percent")]
    pub auto_condense_context_percent: u8,
    
    /// Write delay in ms
    #[serde(default = "default_write_delay_ms")]
    pub write_delay_ms: u32,
    
    /// Fuzzy match threshold
    #[serde(default = "default_fuzzy_match_threshold")]
    pub fuzzy_match_threshold: f32,
    
    /// Custom condensing prompt
    #[serde(default)]
    pub custom_condensing_prompt: String,
    
    // --- Performance ---
    /// Max concurrent file reads
    #[serde(default = "default_max_concurrent_file_reads")]
    pub max_concurrent_file_reads: u32,
    
    /// Allow very large reads
    #[serde(default)]
    pub allow_very_large_reads: bool,
    
    // --- Diagnostics ---
    /// Include diagnostic messages
    #[serde(default = "default_include_diagnostic_messages")]
    pub include_diagnostic_messages: bool,
    
    /// Max diagnostic messages
    #[serde(default = "default_max_diagnostic_messages")]
    pub max_diagnostic_messages: u32,
    
    // --- Image Generation ---
    /// OpenRouter image API key
    #[serde(default)]
    pub openrouter_image_api_key: String,
    
    /// KiloCode image API key
    #[serde(default)]
    pub kilocode_image_api_key: String,
    
    /// OpenRouter image generation model
    #[serde(default)]
    pub openrouter_image_generation_model: String,
    
    // --- MCP ---
    /// MCP enabled
    #[serde(default = "default_mcp_enabled")]
    pub mcp_enabled: bool,
    
    /// Enable MCP server creation
    #[serde(default)]
    pub enable_mcp_server_creation: bool,
    
    // ============================================================
    // LOW PRIORITY (15 settings)
    // ============================================================
    
    // --- Cloud ---
    /// Cloud is authenticated (read-only)
    #[serde(default)]
    pub cloud_is_authenticated: bool,
    
    /// Cloud organizations (read-only)
    #[serde(default)]
    pub cloud_organizations: Vec<serde_json::Value>,
    
    /// Sharing enabled
    #[serde(default)]
    pub sharing_enabled: bool,
    
    /// Organization allow list
    #[serde(default)]
    pub organization_allow_list: String,
    
    /// Organization settings version
    #[serde(default = "default_organization_settings_version")]
    pub organization_settings_version: i32,
    
    // --- Marketplace ---
    /// Marketplace items (read-only)
    #[serde(default)]
    pub marketplace_items: Vec<serde_json::Value>,
    
    /// Marketplace installed metadata (read-only)
    #[serde(default)]
    pub marketplace_installed_metadata: serde_json::Value,
    
    // --- Misc ---
    /// Language code
    #[serde(default = "default_language")]
    pub language: String,
    
    /// Auto-import settings path
    #[serde(default)]
    pub auto_import_settings_path: String,
    
    /// Custom storage path
    #[serde(default)]
    pub custom_storage_path: String,
    
    /// Enable code actions
    #[serde(default = "default_enable_code_actions")]
    pub enable_code_actions: bool,
    
    /// Prevent completion with open todos
    #[serde(default)]
    pub prevent_completion_with_open_todos: bool,
    
    /// Command execution timeout (seconds)
    #[serde(default)]
    pub command_execution_timeout_secs: u32,
    
    /// Command timeout allowlist
    #[serde(default)]
    pub command_timeout_allowlist: Vec<String>,
    
    /// Code index embedding batch size
    #[serde(default = "default_code_index_embedding_batch_size")]
    pub code_index_embedding_batch_size: u32,
    
    // --- Profile Thresholds ---
    /// Profile thresholds (cost tracking)
    #[serde(default)]
    pub profile_thresholds: HashMap<String, f32>,
}

// ============================================================
// Default value functions
// ============================================================

// Critical
fn default_show_model_selector() -> bool { true }
fn default_show_context_panel() -> bool { true }
fn default_api_request_timeout_secs() -> u32 { 600 }
fn default_max_image_file_mb() -> u32 { 5 }
fn default_max_total_image_mb() -> u32 { 20 }
fn default_max_read_file_lines() -> i32 { -1 }
fn default_include_task_history_in_enhance() -> bool { true }
fn default_show_task_timeline() -> bool { true }
fn default_show_timestamps() -> bool { true }

// High
fn default_always_allow_read_only() -> bool { true }
fn default_always_allow_write() -> bool { true }
fn default_always_allow_update_todo_list() -> bool { true }
fn default_request_delay_seconds() -> u32 { 5 }
fn default_auto_approval_enabled() -> bool { true }
fn default_max_open_tabs_context() -> u32 { 20 }
fn default_max_workspace_files() -> u32 { 200 }
fn default_use_agent_rules() -> bool { true }

// Medium
fn default_current_api_config_name() -> String { "default".to_string() }
fn default_browser_tool_enabled() -> bool { true }
fn default_browser_viewport_size() -> String { "900x600".to_string() }
fn default_screenshot_quality() -> u8 { 75 }
fn default_terminal_output_line_limit() -> u32 { 500 }
fn default_terminal_output_character_limit() -> u32 { 50000 }
fn default_terminal_shell_integration_timeout() -> u32 { 4000 }
fn default_terminal_compress_progress_bar() -> bool { true }
fn default_reasoning_block_collapsed() -> bool { true }
fn default_enable_checkpoints() -> bool { true }
fn default_sound_volume() -> f32 { 0.5 }
fn default_tts_speed() -> f32 { 1.0 }
fn default_auto_condense_context() -> bool { true }
fn default_auto_condense_context_percent() -> u8 { 100 }
fn default_write_delay_ms() -> u32 { 1000 }
fn default_fuzzy_match_threshold() -> f32 { 1.0 }
fn default_max_concurrent_file_reads() -> u32 { 5 }
fn default_include_diagnostic_messages() -> bool { true }
fn default_max_diagnostic_messages() -> u32 { 50 }
fn default_mcp_enabled() -> bool { true }

// Low
fn default_organization_settings_version() -> i32 { -1 }
fn default_language() -> String { "en".to_string() }
fn default_enable_code_actions() -> bool { true }
fn default_code_index_embedding_batch_size() -> u32 { 60 }

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            // Critical
            default_model: String::new(),
            show_model_selector: default_show_model_selector(),
            show_context_panel: default_show_context_panel(),
            api_request_timeout_secs: default_api_request_timeout_secs(),
            max_image_file_mb: default_max_image_file_mb(),
            max_total_image_mb: default_max_total_image_mb(),
            max_read_file_lines: default_max_read_file_lines(),
            history_preview_collapsed: false,
            include_task_history_in_enhance: default_include_task_history_in_enhance(),
            show_task_timeline: default_show_task_timeline(),
            show_timestamps: default_show_timestamps(),
            
            // High - Modes
            mode: String::new(),
            custom_modes: Vec::new(),
            custom_mode_prompts: serde_json::Value::Object(serde_json::Map::new()),
            custom_support_prompts: serde_json::Value::Object(serde_json::Map::new()),
            has_opened_mode_selector: false,
            fast_apply_model: String::new(),
            
            // High - Auto-Approve
            always_allow_read_only: default_always_allow_read_only(),
            always_allow_read_only_outside_workspace: false,
            always_allow_write: default_always_allow_write(),
            always_allow_write_outside_workspace: false,
            always_allow_write_protected: false,
            always_allow_browser: false,
            always_allow_execute: false,
            always_allow_mcp: false,
            always_allow_mode_switch: false,
            always_allow_subtasks: false,
            always_approve_resubmit: false,
            always_allow_followup_questions: false,
            always_allow_update_todo_list: default_always_allow_update_todo_list(),
            allowed_commands: Vec::new(),
            denied_commands: Vec::new(),
            allowed_max_requests: None,
            allowed_max_cost: None,
            request_delay_seconds: default_request_delay_seconds(),
            followup_auto_approve_timeout_ms: None,
            show_auto_approve_menu: false,
            auto_approval_enabled: default_auto_approval_enabled(),
            
            // High - Task Management
            new_task_require_todos: false,
            max_open_tabs_context: default_max_open_tabs_context(),
            max_workspace_files: default_max_workspace_files(),
            use_agent_rules: default_use_agent_rules(),
            
            // Medium - Providers
            current_api_config_name: default_current_api_config_name(),
            pinned_api_configs: HashMap::new(),
            condensing_api_config_id: String::new(),
            enhancement_api_config_id: String::new(),
            commit_message_api_config_id: String::new(),
            api_configuration: serde_json::Value::Object(serde_json::Map::new()),
            
            // Medium - Browser
            browser_tool_enabled: default_browser_tool_enabled(),
            browser_viewport_size: default_browser_viewport_size(),
            screenshot_quality: default_screenshot_quality(),
            remote_browser_host: String::new(),
            remote_browser_enabled: false,
            
            // Medium - Terminal
            terminal_output_line_limit: default_terminal_output_line_limit(),
            terminal_output_character_limit: default_terminal_output_character_limit(),
            terminal_shell_integration_timeout: default_terminal_shell_integration_timeout(),
            terminal_shell_integration_disabled: false,
            terminal_zdotdir: false,
            terminal_zsh_oh_my: false,
            terminal_zsh_p10k: false,
            terminal_compress_progress_bar: default_terminal_compress_progress_bar(),
            terminal_command_delay: 0,
            terminal_powershell_counter: false,
            terminal_zsh_clear_eol_mark: false,
            terminal_command_api_config_id: String::new(),
            
            // Medium - Display
            reasoning_block_collapsed: default_reasoning_block_collapsed(),
            hide_cost_below_threshold: 0.0,
            diff_enabled: false,
            enable_checkpoints: default_enable_checkpoints(),
            
            // Medium - Notifications
            sound_enabled: false,
            sound_volume: default_sound_volume(),
            tts_enabled: false,
            tts_speed: default_tts_speed(),
            system_notifications_enabled: false,
            
            // Medium - Context
            auto_condense_context: default_auto_condense_context(),
            auto_condense_context_percent: default_auto_condense_context_percent(),
            write_delay_ms: default_write_delay_ms(),
            fuzzy_match_threshold: default_fuzzy_match_threshold(),
            custom_condensing_prompt: String::new(),
            
            // Medium - Performance
            max_concurrent_file_reads: default_max_concurrent_file_reads(),
            allow_very_large_reads: false,
            
            // Medium - Diagnostics
            include_diagnostic_messages: default_include_diagnostic_messages(),
            max_diagnostic_messages: default_max_diagnostic_messages(),
            
            // Medium - Image Gen
            openrouter_image_api_key: String::new(),
            kilocode_image_api_key: String::new(),
            openrouter_image_generation_model: String::new(),
            
            // Medium - MCP
            mcp_enabled: default_mcp_enabled(),
            enable_mcp_server_creation: false,
            
            // Low - Cloud
            cloud_is_authenticated: false,
            cloud_organizations: Vec::new(),
            sharing_enabled: false,
            organization_allow_list: String::new(),
            organization_settings_version: default_organization_settings_version(),
            
            // Low - Marketplace
            marketplace_items: Vec::new(),
            marketplace_installed_metadata: serde_json::Value::Object(serde_json::Map::new()),
            
            // Low - Misc
            language: default_language(),
            auto_import_settings_path: String::new(),
            custom_storage_path: String::new(),
            enable_code_actions: default_enable_code_actions(),
            prevent_completion_with_open_todos: false,
            command_execution_timeout_secs: 0,
            command_timeout_allowlist: Vec::new(),
            code_index_embedding_batch_size: default_code_index_embedding_batch_size(),
            profile_thresholds: HashMap::new(),
        }
    }
}

impl AIConfig {
    pub const FIELDS: &'static [&'static str] = &[
        // === CRITICAL (10) ===
        "default-model",
        "show-model-selector",
        "api-request-timeout-secs",
        "max-image-file-mb",
        "max-total-image-mb",
        "max-read-file-lines",
        "history-preview-collapsed",
        "include-task-history-in-enhance",
        "show-task-timeline",
        "show-timestamps",
        
        // === HIGH - Modes (9) ===
        "mode",
        "has-opened-mode-selector",
        "fast-apply-model",
        "custom-modes",
        "custom-mode-prompts",
        "custom-support-prompts",
        "followup-auto-approve-timeout-ms",
        "condensing-api-config-id",
        "enhancement-api-config-id",
        
        // === HIGH - Auto-Approve Core (13) ===
        "auto-approval-enabled",
        "always-allow-read-only",
        "always-allow-read-only-outside-workspace",
        "always-allow-write",
        "always-allow-write-outside-workspace",
        "always-allow-write-protected",
        "always-allow-browser",
        "always-allow-execute",
        "always-allow-mcp",
        "always-allow-mode-switch",
        "always-allow-subtasks",
        "always-approve-resubmit",
        "always-allow-followup-questions",
        "always-allow-update-todo-list",
        
        // === HIGH - Auto-Approve Limits (5) ===
        "request-delay-seconds",
        "show-auto-approve-menu",
        // Note: allowed-commands, denied-commands, allowed-max-requests, allowed-max-cost are complex (handled separately)
        
        // === HIGH - Task Management (4) ===
        "new-task-require-todos",
        "max-open-tabs-context",
        "max-workspace-files",
        "use-agent-rules",
        
        // === MEDIUM - Providers (2) ===
        "current-api-config-name",
        // Note: pinned-api-configs, api-configuration are complex (handled separately)
        
        // === MEDIUM - Browser (5) ===
        "browser-tool-enabled",
        "browser-viewport-size",
        "screenshot-quality",
        "remote-browser-host",
        "remote-browser-enabled",
        
        // === MEDIUM - Terminal (12) ===
        "terminal-output-line-limit",
        "terminal-output-character-limit",
        "terminal-shell-integration-timeout",
        "terminal-shell-integration-disabled",
        "terminal-zdotdir",
        "terminal-zsh-oh-my",
        "terminal-zsh-p10k",
        "terminal-compress-progress-bar",
        "terminal-command-delay",
        "terminal-powershell-counter",
        "terminal-zsh-clear-eol-mark",
        "terminal-command-api-config-id",
        
        // === MEDIUM - Display (4) ===
        "reasoning-block-collapsed",
        "hide-cost-below-threshold",
        "diff-enabled",
        "enable-checkpoints",
        
        // === MEDIUM - Notifications (5) ===
        "sound-enabled",
        "sound-volume",
        "tts-enabled",
        "tts-speed",
        "system-notifications-enabled",
        
        // === MEDIUM - Context (5) ===
        "auto-condense-context",
        "auto-condense-context-percent",
        "write-delay-ms",
        "fuzzy-match-threshold",
        "custom-condensing-prompt",
        
        // === MEDIUM - Performance (2) ===
        "max-concurrent-file-reads",
        "allow-very-large-reads",
        
        // === MEDIUM - Diagnostics (2) ===
        "include-diagnostic-messages",
        "max-diagnostic-messages",
        
        // === MEDIUM - Image Generation (3) ===
        "openrouter-image-api-key",
        "kilocode-image-api-key",
        "openrouter-image-generation-model",
        
        // === MEDIUM - MCP (2) ===
        "mcp-enabled",
        "enable-mcp-server-creation",
        
        // === LOW - Cloud (5) ===
        "cloud-is-authenticated",
        "sharing-enabled",
        "organization-allow-list",
        "organization-settings-version",
        // Note: cloud-organizations is complex array (handled separately)
        
        // === LOW - Misc (8) ===
        "language",
        "auto-import-settings-path",
        "custom-storage-path",
        "enable-code-actions",
        "prevent-completion-with-open-todos",
        "command-execution-timeout-secs",
        "code-index-embedding-batch-size",
        // Note: command-timeout-allowlist, marketplace-items, marketplace-installed-metadata, profile-thresholds are complex
    ];

    pub const DESCS: &'static [&'static str] = &[
        // === CRITICAL (10) ===
        "Default AI model ID (e.g., 'gpt-4', 'claude-3-opus')",
        "Show model selector in AI chat toolbar",
        "API request timeout in seconds",
        "Max image file size in MB",
        "Max total image size in MB",
        "Max read file lines (-1 = unlimited)",
        "Collapse history preview by default",
        "Include task history when enhancing prompts",
        "Show task timeline in chat",
        "Show timestamps in chat messages",
        
        // === HIGH - Modes (9) ===
        "Current AI mode",
        "Has user opened mode selector (onboarding)",
        "Fast apply model ID",
        "Custom modes (JSON array of ModeConfig objects)",
        "Custom mode prompts (JSON object mapping mode slugs to prompts)",
        "Custom support prompts (JSON object for support messages)",
        "Follow-up questions auto-approve timeout (milliseconds)",
        "API config ID for condensing context",
        "API config ID for prompt enhancement",
        
        // === HIGH - Auto-Approve Core (13) ===
        "Enable auto-approval system (master toggle)",
        "Always allow read-only file operations",
        "Always allow read-only operations outside workspace",
        "Always allow write file operations",
        "Always allow write operations outside workspace",
        "Always allow write to protected files",
        "Always allow browser tool operations",
        "Always allow command execution",
        "Always allow MCP server operations",
        "Always allow mode switching",
        "Always allow subtask creation",
        "Always approve retry/resubmit requests",
        "Always allow follow-up questions",
        "Always allow todo list updates",
        
        // === HIGH - Auto-Approve Limits (5) ===
        "Request delay in seconds before retry",
        "Show auto-approve menu in UI",
        
        // === HIGH - Task Management (4) ===
        "Require todos when creating new task",
        "Max open tabs to include in context",
        "Max workspace files to index",
        "Use .clinerules agent rules files",
        
        // === MEDIUM - Providers (2) ===
        "Current API configuration profile name",
        
        // === MEDIUM - Browser (5) ===
        "Enable browser automation tool",
        "Browser viewport size (e.g., '900x600')",
        "Screenshot quality (0-100)",
        "Remote browser host URL",
        "Enable remote browser mode",
        
        // === MEDIUM - Terminal (12) ===
        "Terminal output line limit",
        "Terminal output character limit",
        "Shell integration timeout in ms",
        "Disable shell integration",
        "Handle ZDOTDIR for Zsh",
        "Detect Oh My Zsh",
        "Detect Powerlevel10k theme",
        "Compress progress bar output",
        "Command execution delay in ms",
        "Enable PowerShell counter",
        "Clear Zsh end-of-line mark",
        "API config for terminal commands",
        
        // === MEDIUM - Display (4) ===
        "Collapse reasoning blocks by default",
        "Hide costs below threshold (USD)",
        "Enable diff view",
        "Enable checkpoint system",
        
        // === MEDIUM - Notifications (5) ===
        "Enable sound notifications",
        "Sound volume (0.0-1.0)",
        "Enable text-to-speech",
        "TTS speed multiplier",
        "Enable system notifications",
        
        // === MEDIUM - Context (5) ===
        "Auto-condense context when full",
        "Auto-condense at % of context limit",
        "Write delay in ms (debounce)",
        "Fuzzy match threshold (0.0-1.0)",
        "Custom condensing prompt override",
        
        // === MEDIUM - Performance (2) ===
        "Max concurrent file read operations",
        "Allow reading very large files (>100MB)",
        
        // === MEDIUM - Diagnostics (2) ===
        "Include diagnostic messages in context",
        "Max diagnostic messages to include",
        
        // === MEDIUM - Image Generation (3) ===
        "OpenRouter image API key",
        "KiloCode image API key",
        "OpenRouter image generation model ID",
        
        // === MEDIUM - MCP (2) ===
        "Enable Model Context Protocol",
        "Allow MCP server creation",
        
        // === LOW - Cloud (5) ===
        "Cloud authentication status (read-only)",
        "Enable cloud sharing features",
        "Organization allow list",
        "Organization settings version",
        
        // === LOW - Misc (8) ===
        "UI language code (e.g., 'en')",
        "Auto-import settings from file path",
        "Custom storage directory path",
        "Enable code action suggestions",
        "Prevent task completion with open todos",
        "Command execution timeout in seconds",
        "Code index embedding batch size",
    ];
}
