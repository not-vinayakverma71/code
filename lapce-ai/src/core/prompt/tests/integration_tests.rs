//! Integration Tests (P14)
//!
//! End-to-end tests for prompt building:
//! - Build prompts for each mode with real workspace
//! - Verify token counts are reasonable
//! - Verify section order matches specification
//! - Verify presence/absence of gated sections
//! - Test with various settings configurations
//! - Test error recovery paths

use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

use crate::core::prompt::{
    builder::PromptBuilder,
    modes::get_mode_by_slug,
    settings::SystemPromptSettings,
};

// ============================================================================
// Basic Integration Tests
// ============================================================================

#[tokio::test]
async fn test_build_code_mode_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    let result = builder.build().await;
    assert!(result.is_ok(), "Failed to build code mode prompt: {:?}", result.err());
    
    let prompt = result.unwrap();
    
    // Verify it's not empty
    assert!(!prompt.is_empty(), "Prompt should not be empty");
    
    // Verify it contains key sections
    assert!(prompt.contains("MARKDOWN RULES"));
    assert!(prompt.contains("TOOL USE"));
    assert!(prompt.contains("CAPABILITIES"));
    assert!(prompt.contains("OBJECTIVE"));
    assert!(prompt.contains("SYSTEM INFORMATION"));
}

#[tokio::test]
async fn test_build_architect_mode_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("architect").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    let result = builder.build().await;
    assert!(result.is_ok());
    
    let prompt = result.unwrap();
    
    // Architect mode should have architect-specific role
    assert!(prompt.contains(mode.role_definition));
    
    // Should have tool descriptions
    assert!(prompt.contains("# Tools"));
}

#[tokio::test]
async fn test_build_ask_mode_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("ask").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    let result = builder.build().await;
    assert!(result.is_ok());
    
    let prompt = result.unwrap();
    
    // Ask mode should have fewer tools (no Edit group)
    assert!(!prompt.contains("write_to_file") || !prompt.contains("## write_to_file"));
    
    // But should have read tools
    assert!(prompt.contains("## read_file"));
}

#[tokio::test]
async fn test_build_debug_mode_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("debug").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    let result = builder.build().await;
    assert!(result.is_ok());
    
    let prompt = result.unwrap();
    
    // Debug mode should have debugging-related role
    assert!(prompt.contains(mode.role_definition));
}

#[tokio::test]
async fn test_build_orchestrator_mode_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("orchestrator").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    let result = builder.build().await;
    assert!(result.is_ok());
    
    let prompt = result.unwrap();
    
    // Orchestrator has NO tool groups, so should have minimal tools
    // Only always-available tools should be present
    assert!(prompt.contains("## attempt_completion"));
    assert!(prompt.contains("## ask_followup_question"));
}

// ============================================================================
// Section Ordering Tests
// ============================================================================

#[tokio::test]
async fn test_section_order_matches_spec() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Find positions of key sections
    let markdown_pos = prompt.find("MARKDOWN RULES").expect("MARKDOWN RULES not found");
    let tool_use_pos = prompt.find("TOOL USE\n").expect("TOOL USE not found");
    let tools_pos = prompt.find("# Tools").expect("# Tools not found");
    let guidelines_pos = prompt.find("TOOL USE GUIDELINES").expect("TOOL USE GUIDELINES not found");
    let capabilities_pos = prompt.find("CAPABILITIES").expect("CAPABILITIES not found");
    let objective_pos = prompt.find("OBJECTIVE").expect("OBJECTIVE not found");
    let system_info_pos = prompt.find("SYSTEM INFORMATION").expect("SYSTEM INFORMATION not found");
    
    // Verify order per spec (Codex system.ts assembly order)
    assert!(markdown_pos < tool_use_pos, "MARKDOWN RULES should come before TOOL USE");
    assert!(tool_use_pos < tools_pos, "TOOL USE should come before # Tools");
    assert!(tools_pos < guidelines_pos, "# Tools should come before TOOL USE GUIDELINES");
    assert!(guidelines_pos < capabilities_pos, "TOOL USE GUIDELINES should come before CAPABILITIES");
    assert!(capabilities_pos < objective_pos, "CAPABILITIES should come before OBJECTIVE");
    assert!(objective_pos < system_info_pos, "OBJECTIVE should come before SYSTEM INFORMATION");
}

// ============================================================================
// Token Count Tests
// ============================================================================

#[tokio::test]
async fn test_token_counts_reasonable() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    
    for mode_slug in modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
        let prompt = builder.build().await.unwrap();
        
        // Estimate tokens (rough heuristic: chars / 4)
        let estimated_tokens = prompt.len() / 4;
        
        // Prompts should be between 2k and 20k tokens
        assert!(
            estimated_tokens > 2000 && estimated_tokens < 20000,
            "Mode {} has unusual token count: ~{} tokens",
            mode_slug, estimated_tokens
        );
    }
}

#[tokio::test]
async fn test_orchestrator_mode_shorter() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Build code mode (full tools)
    let code_mode = get_mode_by_slug("code").unwrap();
    let code_builder = PromptBuilder::new(code_mode.clone(), workspace.to_path_buf(), SystemPromptSettings::default());
    let code_prompt = code_builder.build().await.unwrap();
    
    // Build orchestrator mode (minimal tools)
    let orch_mode = get_mode_by_slug("orchestrator").unwrap();
    let orch_builder = PromptBuilder::new(orch_mode.clone(), workspace.to_path_buf(), SystemPromptSettings::default());
    let orch_prompt = orch_builder.build().await.unwrap();
    
    // Orchestrator should be significantly shorter
    assert!(
        orch_prompt.len() < code_prompt.len(),
        "Orchestrator mode should be shorter than code mode"
    );
}

// ============================================================================
// Custom Instructions Integration Tests
// ============================================================================

#[tokio::test]
async fn test_with_custom_instructions() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create .kilocode/rules/ directory with a rule
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("custom.txt"), "Always use TypeScript").await.unwrap();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Should include custom instructions
    assert!(prompt.contains("USER'S CUSTOM INSTRUCTIONS"));
    assert!(prompt.contains("Always use TypeScript"));
}

#[tokio::test]
async fn test_with_agents_md() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "# Agent Rules\n\nBe concise.").await.unwrap();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Should include AGENTS.md content
    assert!(prompt.contains("Agent Rules Standard"));
    assert!(prompt.contains("Be concise"));
}

#[tokio::test]
async fn test_agents_md_can_be_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "Should not appear").await.unwrap();
    
    let mode = get_mode_by_slug("code").unwrap();
    let mut settings = SystemPromptSettings::default();
    settings.use_agent_rules = false;
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Should NOT include AGENTS.md when disabled
    assert!(!prompt.contains("Should not appear"));
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_build_with_retry_on_success() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    // build_with_retry should succeed on first try
    let result = builder.build_with_retry().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_nonexistent_workspace_handled() {
    // Use a path that doesn't exist
    let workspace = Path::new("/nonexistent/workspace/path");
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    
    // Should handle gracefully (may succeed with empty custom instructions)
    let result = builder.build().await;
    
    // Either succeeds or fails gracefully
    match result {
        Ok(prompt) => {
            // If it succeeds, should still have core sections
            assert!(prompt.contains("MARKDOWN RULES"));
        }
        Err(_) => {
            // If it fails, that's also acceptable
            // (depends on implementation)
        }
    }
}

// ============================================================================
// Settings Variations Tests
// ============================================================================

#[tokio::test]
async fn test_different_max_concurrent_reads() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    
    // Test with different max_concurrent_file_reads
    for max_reads in vec![1, 5, 50, 100] {
        let mut settings = SystemPromptSettings::default();
        settings.max_concurrent_file_reads = max_reads;
        
        let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
        let result = builder.build().await;
        
        assert!(result.is_ok(), "Failed with max_concurrent_file_reads={}", max_reads);
        
        let prompt = result.unwrap();
        // Should mention the setting in read_file description
        assert!(prompt.contains(&max_reads.to_string()));
    }
}

#[tokio::test]
async fn test_todo_list_enabled_setting() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    
    // With todo list enabled
    let mut settings_enabled = SystemPromptSettings::default();
    settings_enabled.todo_list_enabled = true;
    
    let builder_enabled = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings_enabled);
    let prompt_enabled = builder_enabled.build().await.unwrap();
    
    // Should have update_todo_list tool
    assert!(prompt_enabled.contains("update_todo_list"));
    
    // With todo list disabled
    let mut settings_disabled = SystemPromptSettings::default();
    settings_disabled.todo_list_enabled = false;
    
    let builder_disabled = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings_disabled);
    let prompt_disabled = builder_disabled.build().await.unwrap();
    
    // Should NOT have update_todo_list tool
    assert!(!prompt_disabled.contains("## update_todo_list"));
}

// ============================================================================
// Workspace Boundary Tests
// ============================================================================

#[tokio::test]
async fn test_workspace_path_included() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Workspace path should appear in system info and tool descriptions
    let workspace_str = workspace.display().to_string();
    assert!(prompt.contains(&workspace_str), "Workspace path should be in prompt");
}

// ============================================================================
// Feature Gating Tests
// ============================================================================

#[tokio::test]
async fn test_browser_action_not_available_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // browser_action should NOT be present (supports_browser defaults to false)
    assert!(!prompt.contains("## browser_action"));
}

// ============================================================================
// Completeness Tests
// ============================================================================

#[tokio::test]
async fn test_all_always_available_tools_present() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Even orchestrator mode (no tool groups) should have always-available tools
    let mode = get_mode_by_slug("orchestrator").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // These tools should ALWAYS be present
    let always_available = vec![
        "ask_followup_question",
        "attempt_completion",
        "switch_mode",
        "new_task",
    ];
    
    for tool in always_available {
        assert!(
            prompt.contains(&format!("## {}", tool)),
            "Always-available tool {} missing from orchestrator mode",
            tool
        );
    }
}

#[tokio::test]
async fn test_all_modes_build_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    
    for mode_slug in modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
        let result = builder.build().await;
        
        assert!(
            result.is_ok(),
            "Mode {} failed to build: {:?}",
            mode_slug,
            result.err()
        );
        
        let prompt = result.unwrap();
        assert!(!prompt.is_empty(), "Mode {} produced empty prompt", mode_slug);
    }
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_realistic_workspace_setup() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create a realistic workspace structure
    // 1. .kilocode/rules/ with multiple files
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    fs::write(rules_dir.join("style.txt"), "Use 4 spaces for indentation").await.unwrap();
    fs::write(rules_dir.join("naming.txt"), "Use camelCase for variables").await.unwrap();
    
    // 2. Mode-specific rules
    let code_rules = workspace.join(".kilocode/rules-code");
    fs::create_dir_all(&code_rules).await.unwrap();
    fs::write(code_rules.join("testing.txt"), "Write unit tests").await.unwrap();
    
    // 3. AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "Be helpful and concise").await.unwrap();
    
    // 4. Some source files (shouldn't affect prompt)
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).await.unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() {}").await.unwrap();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    let builder = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt = builder.build().await.unwrap();
    
    // Verify all custom instructions are included
    assert!(prompt.contains("USER'S CUSTOM INSTRUCTIONS"));
    assert!(prompt.contains("Use 4 spaces for indentation"));
    assert!(prompt.contains("Use camelCase for variables"));
    assert!(prompt.contains("Write unit tests"));
    assert!(prompt.contains("Be helpful and concise"));
    
    // Verify core sections are present
    assert!(prompt.contains("MARKDOWN RULES"));
    assert!(prompt.contains("# Tools"));
    assert!(prompt.contains("CAPABILITIES"));
    
    // Source files should NOT be in the prompt
    assert!(!prompt.contains("fn main() {}"));
}

#[tokio::test]
async fn test_prompt_is_deterministic() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    // Build the same prompt twice
    let builder1 = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings.clone());
    let prompt1 = builder1.build().await.unwrap();
    
    let builder2 = PromptBuilder::new(mode.clone(), workspace.to_path_buf(), settings);
    let prompt2 = builder2.build().await.unwrap();
    
    // Should be identical
    assert_eq!(prompt1, prompt2, "Prompts should be deterministic");
}
