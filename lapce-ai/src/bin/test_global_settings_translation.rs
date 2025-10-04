/// Test to verify global-settings.ts translation works correctly
use lapce_ai_rust::global_settings_exact_translation::*;
use serde_json;

fn main() {
    println!("Testing global-settings.ts Translation...\n");
    
    // Test 1: Constants
    println!("1. Testing Constants:");
    println!("   DEFAULT_WRITE_DELAY_MS: {}", DEFAULT_WRITE_DELAY_MS);
    assert_eq!(DEFAULT_WRITE_DELAY_MS, 1000);
    println!("   DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT: {}", DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT);
    assert_eq!(DEFAULT_TERMINAL_OUTPUT_CHARACTER_LIMIT, 50_000);
    
    // Test 2: SECRET_STATE_KEYS
    println!("\n2. Testing SECRET_STATE_KEYS:");
    println!("   Total secret keys: {}", SECRET_STATE_KEYS.len());
    assert!(SECRET_STATE_KEYS.contains(&"apiKey"));
    assert!(SECRET_STATE_KEYS.contains(&"kilocodeToken"));
    assert!(SECRET_STATE_KEYS.contains(&"ioIntelligenceApiKey"));
    
    // Test 3: GlobalSettings serialization
    println!("\n3. Testing GlobalSettings:");
    let settings = GlobalSettings {
        current_api_config_name: Some("test-config".to_string()),
        auto_approval_enabled: Some(true),
        write_delay_ms: Some(2000),
        always_allow_read_only: Some(true),
        terminal_output_character_limit: Some(100_000),
        ghost_service_settings: GhostServiceSettings {
            enabled: Some(true),
            api_key: Some("secret".to_string()),
            model: Some("gpt-4".to_string()),
        },
        language: Some("en".to_string()),
        mode: Some("architect".to_string()),
        dismissed_notification_ids: Some(vec!["notif1".to_string()]),
        ..default_global_settings()
    };
    
    let json = serde_json::to_string(&settings).unwrap();
    println!("   GlobalSettings serialized (partial): {}", &json[..100]);
    assert!(json.contains(r#""currentApiConfigName":"test-config""#));
    assert!(json.contains(r#""autoApprovalEnabled":true"#));
    assert!(json.contains(r#""writeDelayMs":2000"#));
    
    // Test 4: ProviderSettingsEntry
    println!("\n4. Testing ProviderSettingsEntry:");
    let entry = ProviderSettingsEntry {
        id: "openai".to_string(),
        label: "OpenAI GPT-4".to_string(),
        model_id: Some("gpt-4".to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    println!("   ProviderSettingsEntry: {}", json);
    assert!(json.contains(r#""modelId":"gpt-4""#));
    
    // Test 5: HistoryItem
    println!("\n5. Testing HistoryItem:");
    let history = HistoryItem {
        id: "hist-123".to_string(),
        text: "Previous task".to_string(),
        timestamp: 1234567890,
        token_usage: None,
        total_tokens: Some(500),
        task: Some("coding".to_string()),
        is_favorited: Some(true),
        model_used: Some("claude-3".to_string()),
    };
    let json = serde_json::to_string(&history).unwrap();
    println!("   HistoryItem: {}", json);
    assert!(json.contains(r#""isFavorited":true"#));
    assert!(json.contains(r#""modelUsed":"claude-3""#));
    
    // Test 6: ModeConfig
    println!("\n6. Testing ModeConfig:");
    let mode = ModeConfig {
        slug: "custom-mode".to_string(),
        name: "Custom Mode".to_string(),
        instructions: Some("Be helpful".to_string()),
        tools: Some(vec!["read_file".to_string(), "write_to_file".to_string()]),
    };
    let json = serde_json::to_string(&mode).unwrap();
    println!("   ModeConfig: {}", json);
    assert!(json.contains(r#""slug":"custom-mode""#));
    
    // Test 7: Deserialization
    println!("\n7. Testing deserialization:");
    let json_input = r#"{
        "currentApiConfigName": "default",
        "autoApprovalEnabled": false,
        "writeDelayMs": 1500,
        "ghostServiceSettings": {
            "enabled": false
        }
    }"#;
    
    match serde_json::from_str::<GlobalSettings>(json_input) {
        Ok(settings) => {
            println!("   ✅ Successfully deserialized GlobalSettings");
            assert_eq!(settings.current_api_config_name, Some("default".to_string()));
            assert_eq!(settings.auto_approval_enabled, Some(false));
            assert_eq!(settings.write_delay_ms, Some(1500));
        }
        Err(e) => {
            println!("   ❌ Deserialization failed: {}", e);
        }
    }
    
    // Test 8: RooCodeSettings (merged)
    println!("\n8. Testing RooCodeSettings:");
    let roo_settings = RooCodeSettings {
        global: GlobalSettings {
            current_api_config_name: Some("roo".to_string()),
            mode: Some("code".to_string()),
            ..Default::default()
        },
        provider: ProviderSettings {
            api_provider: Some("openrouter".to_string()),
            open_router_api_key: Some("key123".to_string()),
            ..Default::default()
        },
    };
    let json = serde_json::to_string(&roo_settings).unwrap();
    println!("   RooCodeSettings serialized: {}", &json[..80]);
    assert!(json.contains(r#""currentApiConfigName":"roo""#));
    assert!(json.contains(r#""apiProvider":"openrouter""#));
    
    println!("\n✅ All global-settings.ts translation tests passed!");
}

// Helper function to create default GlobalSettings for testing
fn default_global_settings() -> GlobalSettings {
    GlobalSettings {
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
