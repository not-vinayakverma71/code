//! Tool Registry and Description Tests
//!
//! P11: Registry tests - per-mode filtering, feature gates, deterministic ordering

use super::*;
use crate::core::prompt::modes::get_mode_by_slug;
use std::path::Path;

#[test]
fn test_tool_groups_count() {
    let groups = get_tool_groups();
    assert_eq!(groups.len(), 6, "Should have 6 tool groups (Read, Edit, Browser, Command, Mcp, Modes)");
}

#[test]
fn test_read_group_tools() {
    let groups = get_tool_groups();
    let read_group = groups.iter()
        .find(|(g, _)| matches!(g, ExtendedToolGroup::Read))
        .expect("Read group should exist");
    
    assert_eq!(read_group.1.tools.len(), 6);
    assert!(read_group.1.tools.contains(&"read_file"));
    assert!(read_group.1.tools.contains(&"list_files"));
    assert!(read_group.1.tools.contains(&"search_files"));
    assert!(read_group.1.tools.contains(&"codebase_search"));
    assert!(!read_group.1.always_available);
}

#[test]
fn test_edit_group_tools() {
    let groups = get_tool_groups();
    let edit_group = groups.iter()
        .find(|(g, _)| matches!(g, ExtendedToolGroup::Edit))
        .expect("Edit group should exist");
    
    assert_eq!(edit_group.1.tools.len(), 6);
    assert!(edit_group.1.tools.contains(&"write_to_file"));
    assert!(edit_group.1.tools.contains(&"insert_content"));
    assert!(edit_group.1.tools.contains(&"search_and_replace"));
    assert!(edit_group.1.tools.contains(&"apply_diff"));
    assert!(!edit_group.1.always_available);
}

#[test]
fn test_modes_group_always_available() {
    let groups = get_tool_groups();
    let modes_group = groups.iter()
        .find(|(g, _)| matches!(g, ExtendedToolGroup::Modes))
        .expect("Modes group should exist");
    
    assert!(modes_group.1.always_available, "Modes group should be always available");
    assert!(modes_group.1.tools.contains(&"switch_mode"));
    assert!(modes_group.1.tools.contains(&"new_task"));
}

#[test]
fn test_always_available_tools() {
    assert_eq!(ALWAYS_AVAILABLE_TOOLS.len(), 6);
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"ask_followup_question"));
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"attempt_completion"));
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"switch_mode"));
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"new_task"));
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"update_todo_list"));
    assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"run_slash_command"));
}

#[test]
fn test_code_mode_tools() {
    let mode = get_mode_by_slug("code").unwrap();
    let tools = get_tools_for_mode(&mode.groups);
    
    // Code mode has all 5 groups: Read, Edit, Browser, Command, Mcp
    // Should have tools from all groups + always available
    assert!(tools.len() >= 15, "Code mode should have at least 15 tools");
    
    // Check key tools
    assert!(tools.contains("read_file"));
    assert!(tools.contains("write_to_file"));
    assert!(tools.contains("execute_command"));
    assert!(tools.contains("browser_action"));
    assert!(tools.contains("ask_followup_question"));
    assert!(tools.contains("attempt_completion"));
}

#[test]
fn test_ask_mode_tools() {
    let mode = get_mode_by_slug("ask").unwrap();
    let tools = get_tools_for_mode(&mode.groups);
    
    // Ask mode only has Read group
    // Should have read tools + always available
    assert!(tools.len() >= 10, "Ask mode should have at least 10 tools");
    
    // Should have read tools
    assert!(tools.contains("read_file"));
    assert!(tools.contains("list_files"));
    
    // Should NOT have edit tools (no Edit group)
    assert!(!tools.contains("write_to_file"));
    assert!(!tools.contains("insert_content"));
    
    // Should still have always available
    assert!(tools.contains("ask_followup_question"));
    assert!(tools.contains("attempt_completion"));
}

#[test]
fn test_orchestrator_mode_no_tools() {
    let mode = get_mode_by_slug("orchestrator").unwrap();
    let tools = get_tools_for_mode(&mode.groups);
    
    // Orchestrator has NO tool groups
    // Should only have always available tools
    assert_eq!(tools.len(), ALWAYS_AVAILABLE_TOOLS.len());
    
    assert!(tools.contains("ask_followup_question"));
    assert!(tools.contains("attempt_completion"));
    assert!(tools.contains("switch_mode"));
    assert!(tools.contains("new_task"));
}

#[test]
fn test_filter_tools_by_features_all_disabled() {
    let mut tools = vec![
        "read_file".to_string(),
        "codebase_search".to_string(),
        "edit_file".to_string(),
        "update_todo_list".to_string(),
        "generate_image".to_string(),
        "run_slash_command".to_string(),
    ].into_iter().collect();
    
    // All features disabled
    tools = filter_tools_by_features(tools, false, false, false, false, false);
    
    // Should remove codebase_search, edit_file, update_todo_list, generate_image, run_slash_command
    assert!(tools.contains("read_file"));
    assert!(!tools.contains("codebase_search"));
    assert!(!tools.contains("edit_file"));
    assert!(!tools.contains("update_todo_list"));
    assert!(!tools.contains("generate_image"));
    assert!(!tools.contains("run_slash_command"));
}

#[test]
fn test_filter_tools_codebase_search_enabled() {
    let mut tools = vec![
        "read_file".to_string(),
        "codebase_search".to_string(),
    ].into_iter().collect();
    
    tools = filter_tools_by_features(tools, true, false, false, false, false);
    
    assert!(tools.contains("codebase_search"), "codebase_search should be available when feature enabled");
}

#[test]
fn test_filter_tools_fast_apply_enabled() {
    let mut tools = vec![
        "edit_file".to_string(),
    ].into_iter().collect();
    
    tools = filter_tools_by_features(tools, false, true, false, false, false);
    
    assert!(tools.contains("edit_file"), "edit_file should be available when fast_apply enabled");
}

#[test]
fn test_filter_tools_todo_list_enabled() {
    let mut tools = vec![
        "update_todo_list".to_string(),
    ].into_iter().collect();
    
    tools = filter_tools_by_features(tools, false, false, true, false, false);
    
    assert!(tools.contains("update_todo_list"), "update_todo_list should be available when enabled");
}

#[test]
fn test_get_tool_descriptions_for_mode_code() {
    let mode = get_mode_by_slug("code").unwrap();
    let context = ToolDescriptionContext::default();
    
    let descriptions = get_tool_descriptions_for_mode(&mode, &context);
    
    // Should start with "# Tools"
    assert!(descriptions.starts_with("# Tools"));
    
    // Should contain core tool descriptions
    assert!(descriptions.contains("## read_file"));
    assert!(descriptions.contains("## write_to_file"));
    assert!(descriptions.contains("## execute_command"));
    assert!(descriptions.contains("## list_files"));
    assert!(descriptions.contains("## search_files"));
    assert!(descriptions.contains("## fetch_instructions"));
    
    // Should NOT contain browser_action (supports_browser is false by default)
    assert!(!descriptions.contains("## browser_action"));
}

#[test]
fn test_get_tool_descriptions_browser_enabled() {
    let mode = get_mode_by_slug("code").unwrap();
    let mut context = ToolDescriptionContext::default();
    context.supports_browser = true;
    
    let descriptions = get_tool_descriptions_for_mode(&mode, &context);
    
    // Should now contain browser_action
    assert!(descriptions.contains("## browser_action"));
    assert!(descriptions.contains("Puppeteer-controlled browser"));
}

#[test]
fn test_tool_descriptions_deterministic_ordering() {
    let mode = get_mode_by_slug("code").unwrap();
    let context = ToolDescriptionContext::default();
    
    // Generate twice
    let desc1 = get_tool_descriptions_for_mode(&mode, &context);
    let desc2 = get_tool_descriptions_for_mode(&mode, &context);
    
    // Should be identical (deterministic)
    assert_eq!(desc1, desc2, "Tool descriptions should be deterministic");
}

#[test]
fn test_tool_descriptions_alphabetically_sorted() {
    let mode = get_mode_by_slug("code").unwrap();
    let context = ToolDescriptionContext::default();
    
    let descriptions = get_tool_descriptions_for_mode(&mode, &context);
    
    // Extract tool names (lines starting with "## ")
    let tool_names: Vec<&str> = descriptions
        .lines()
        .filter(|line| line.starts_with("## "))
        .map(|line| &line[3..])
        .collect();
    
    // Verify alphabetical ordering
    let mut sorted_names = tool_names.clone();
    sorted_names.sort();
    
    assert_eq!(tool_names, sorted_names, "Tools should be alphabetically sorted");
}

#[test]
fn test_read_file_description() {
    let args = ToolDescriptionArgs {
        workspace: Path::new("/test/workspace"),
        supports_browser: false,
        max_concurrent_file_reads: 5,
        partial_reads_enabled: false,
    };
    
    let desc = read_file_description(&args);
    
    assert!(desc.contains("## read_file"));
    assert!(desc.contains("read a file"));
    assert!(desc.contains("/test/workspace"));
    assert!(desc.contains("line_start"));
    assert!(desc.contains("line_end"));
}

#[test]
fn test_write_to_file_description() {
    let desc = write_to_file_description(Path::new("/test/workspace"));
    
    assert!(desc.contains("## write_to_file"));
    assert!(desc.contains("/test/workspace"));
    assert!(desc.contains("line_count"));
    assert!(desc.contains("COMPLETE intended content"));
}

#[test]
fn test_execute_command_description() {
    let desc = execute_command_description(Path::new("/test/workspace"));
    
    assert!(desc.contains("## execute_command"));
    assert!(desc.contains("CLI command"));
    assert!(desc.contains("/test/workspace"));
}

#[test]
fn test_browser_action_description() {
    let desc = browser_action_description(Path::new("/test"), "1920x1080");
    
    assert!(desc.contains("## browser_action"));
    assert!(desc.contains("Puppeteer-controlled browser"));
    assert!(desc.contains("1920x1080"));
    assert!(desc.contains("launch"));
    assert!(desc.contains("click"));
    assert!(desc.contains("close"));
}

#[test]
fn test_codebase_search_description() {
    let desc = codebase_search_description(Path::new("/test/workspace"));
    
    assert!(desc.contains("## codebase_search"));
    assert!(desc.contains("semantic search"));
    assert!(desc.contains("/test/workspace"));
}

#[test]
fn test_switch_mode_description() {
    let desc = switch_mode_description();
    
    assert!(desc.contains("## switch_mode"));
    assert!(desc.contains("mode_slug"));
    assert!(desc.contains("reason"));
}

#[test]
fn test_new_task_description_no_todos() {
    let desc = new_task_description(false);
    
    assert!(desc.contains("## new_task"));
    assert!(desc.contains("mode"));
    assert!(desc.contains("message"));
    assert!(!desc.contains("todos: (required)"));
}

#[test]
fn test_new_task_description_with_todos() {
    let desc = new_task_description(true);
    
    assert!(desc.contains("## new_task"));
    assert!(desc.contains("todos: (required)"));
    assert!(desc.contains("markdown checklist format"));
}

#[test]
fn test_update_todo_list_description() {
    let desc = update_todo_list_description();
    
    assert!(desc.contains("## update_todo_list"));
    assert!(desc.contains("[ ]"));
    assert!(desc.contains("[x]"));
    assert!(desc.contains("[-]"));
    assert!(desc.contains("pending"));
    assert!(desc.contains("completed"));
    assert!(desc.contains("in_progress"));
}

#[test]
fn test_list_code_definition_names_description() {
    let desc = list_code_definition_names_description(Path::new("/test/workspace"));
    
    assert!(desc.contains("## list_code_definition_names"));
    assert!(desc.contains("classes, functions, methods"));
    assert!(desc.contains("/test/workspace"));
}

#[test]
fn test_ask_followup_question_description() {
    let desc = ask_followup_question_description();
    
    assert!(desc.contains("## ask_followup_question"));
    assert!(desc.contains("question"));
    assert!(desc.contains("suggestions"));
}

#[test]
fn test_attempt_completion_description() {
    let desc = attempt_completion_description();
    
    assert!(desc.contains("## attempt_completion"));
    assert!(desc.contains("result"));
    assert!(desc.contains("IMPORTANT NOTE"));
    assert!(desc.contains("previous tool uses were successful"));
}
