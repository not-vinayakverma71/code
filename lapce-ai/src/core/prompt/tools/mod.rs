//! Tool Descriptions
//!
//! 1:1 Translation from Codex `src/core/prompts/tools/`
//!
//! Generates tool descriptions filtered by mode and feature gates

use std::path::Path;

pub mod registry;
pub mod descriptions;

use crate::core::prompt::modes::ModeConfig;
use registry::*;
use descriptions::*;

/// Arguments for tool description generation
pub struct ToolDescriptionContext<'a> {
    pub workspace: &'a Path,
    pub supports_browser: bool,
    pub codebase_search_available: bool,
    pub fast_apply_available: bool,
    pub max_concurrent_file_reads: usize,
    pub partial_reads_enabled: bool,
    pub todo_list_enabled: bool,
    pub image_generation_enabled: bool,
    pub run_slash_command_enabled: bool,
    pub browser_viewport_size: String,
    pub new_task_require_todos: bool,
}

impl Default for ToolDescriptionContext<'static> {
    fn default() -> Self {
        Self {
            workspace: Path::new("."),
            supports_browser: false,
            codebase_search_available: false,
            fast_apply_available: false,
            max_concurrent_file_reads: 5,
            partial_reads_enabled: false,
            todo_list_enabled: true,
            image_generation_enabled: false,
            run_slash_command_enabled: false,
            browser_viewport_size: "900x600".to_string(),
            new_task_require_todos: false,
        }
    }
}

/// Generate tool descriptions for a mode
///
/// Translation of getToolDescriptionsForMode() from index.ts (lines 76-182)
pub fn get_tool_descriptions_for_mode(
    mode: &ModeConfig,
    context: &ToolDescriptionContext,
) -> String {
    // Get tools for mode's groups
    let mut tools = get_tools_for_mode(&mode.groups);
    
    // Apply feature filtering
    tools = filter_tools_by_features(
        tools,
        context.codebase_search_available,
        context.fast_apply_available,
        context.todo_list_enabled,
        context.image_generation_enabled,
        context.run_slash_command_enabled,
    );
    
    // Generate descriptions for allowed tools
    let mut descriptions = Vec::new();
    
    // Sort tools for deterministic output
    let mut sorted_tools: Vec<_> = tools.iter().collect();
    sorted_tools.sort();
    
    for tool_name in sorted_tools {
        if let Some(description) = get_tool_description(tool_name, context) {
            descriptions.push(description);
        }
    }
    
    format!("# Tools\n\n{}", descriptions.join("\n\n"))
}

/// Get description for a single tool
///
/// Translation of toolDescriptionMap from index.ts (lines 43-74)
fn get_tool_description(
    tool_name: &str,
    context: &ToolDescriptionContext,
) -> Option<String> {
    let args = ToolDescriptionArgs {
        workspace: context.workspace,
        supports_browser: context.supports_browser,
        max_concurrent_file_reads: context.max_concurrent_file_reads,
        partial_reads_enabled: context.partial_reads_enabled,
    };
    
    match tool_name {
        "read_file" => Some(read_file_description(&args)),
        "write_to_file" => Some(write_to_file_description(context.workspace)),
        "execute_command" => Some(execute_command_description(context.workspace)),
        "list_files" => Some(list_files_description(context.workspace)),
        "search_files" => Some(search_files_description(context.workspace)),
        "insert_content" => Some(insert_content_description(context.workspace)),
        "search_and_replace" => Some(search_and_replace_description(context.workspace)),
        "ask_followup_question" => Some(ask_followup_question_description()),
        "attempt_completion" => Some(attempt_completion_description()),
        "list_code_definition_names" => Some(list_code_definition_names_description(context.workspace)),
        "browser_action" => {
            if context.supports_browser {
                Some(browser_action_description(context.workspace, &context.browser_viewport_size))
            } else {
                None
            }
        },
        "codebase_search" => Some(codebase_search_description(context.workspace)),
        "switch_mode" => Some(switch_mode_description()),
        "new_task" => Some(new_task_description(context.new_task_require_todos)),
        "update_todo_list" => Some(update_todo_list_description()),
        // TODO: Complex tools requiring strategy/MCP integration
        // "apply_diff" => handled by diff strategy in capabilities section
        // "edit_file" => Morph fast apply (post-IPC)
        // "use_mcp_tool" => MCP integration (post-IPC)
        // "access_mcp_resource" => MCP integration (post-IPC)
        // "generate_image" => Image generation (post-IPC)
        // "run_slash_command" => Slash commands (post-IPC)
        _ => None,
    }
}

pub use registry::{ExtendedToolGroup, get_tool_groups, get_tools_for_mode};
pub use descriptions::ToolDescriptionArgs;

#[cfg(test)]
mod tests;
