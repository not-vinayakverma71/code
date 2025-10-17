//! Tool Descriptions Registry
//!
//! 1:1 Translation from Codex `src/shared/tools.ts` and `src/core/prompts/tools/index.ts`
//!
//! Maps tool names to description generators with mode-based filtering

use std::collections::HashSet;
use crate::core::prompt::modes::{ToolGroup, GroupEntry};

/// Tool group with Modes added (not in modes.rs but needed for tool registry)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedToolGroup {
    Read,
    Edit,
    Browser,
    Command,
    Mcp,
    Modes,
}

impl ExtendedToolGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtendedToolGroup::Read => "read",
            ExtendedToolGroup::Edit => "edit",
            ExtendedToolGroup::Browser => "browser",
            ExtendedToolGroup::Command => "command",
            ExtendedToolGroup::Mcp => "mcp",
            ExtendedToolGroup::Modes => "modes",
        }
    }
    
    pub fn from_tool_group(tg: &ToolGroup) -> Self {
        match tg {
            ToolGroup::Read => ExtendedToolGroup::Read,
            ToolGroup::Edit => ExtendedToolGroup::Edit,
            ToolGroup::Browser => ExtendedToolGroup::Browser,
            ToolGroup::Command => ExtendedToolGroup::Command,
            ToolGroup::Mcp => ExtendedToolGroup::Mcp,
        }
    }
}

/// Tool group configuration
#[derive(Debug, Clone)]
pub struct ToolGroupConfig {
    pub tools: Vec<&'static str>,
    pub always_available: bool,
}

/// Get tool groups definition
///
/// Translation of TOOL_GROUPS from tools.ts (lines 233-268)
pub fn get_tool_groups() -> Vec<(ExtendedToolGroup, ToolGroupConfig)> {
    vec![
        (
            ExtendedToolGroup::Read,
            ToolGroupConfig {
                tools: vec![
                    "read_file",
                    "fetch_instructions",
                    "search_files",
                    "list_files",
                    "list_code_definition_names",
                    "codebase_search",
                ],
                always_available: false,
            },
        ),
        (
            ExtendedToolGroup::Edit,
            ToolGroupConfig {
                tools: vec![
                    "apply_diff",
                    "edit_file",
                    "write_to_file",
                    "insert_content",
                    "search_and_replace",
                    "generate_image",
                ],
                always_available: false,
            },
        ),
        (
            ExtendedToolGroup::Browser,
            ToolGroupConfig {
                tools: vec!["browser_action"],
                always_available: false,
            },
        ),
        (
            ExtendedToolGroup::Command,
            ToolGroupConfig {
                tools: vec!["execute_command"],
                always_available: false,
            },
        ),
        (
            ExtendedToolGroup::Mcp,
            ToolGroupConfig {
                tools: vec!["use_mcp_tool", "access_mcp_resource"],
                always_available: false,
            },
        ),
        (
            ExtendedToolGroup::Modes,
            ToolGroupConfig {
                tools: vec!["switch_mode", "new_task"],
                always_available: true,
            },
        ),
    ]
}

/// Tools that are always available
///
/// Translation of ALWAYS_AVAILABLE_TOOLS from tools.ts (lines 271-280)
pub const ALWAYS_AVAILABLE_TOOLS: &[&str] = &[
    "ask_followup_question",
    "attempt_completion",
    "switch_mode",
    "new_task",
    "update_todo_list",
    "run_slash_command",
];

/// Get tools for a mode by collecting from its groups
pub fn get_tools_for_mode(mode_groups: &[GroupEntry]) -> HashSet<String> {
    let mut tools = HashSet::new();
    let tool_groups = get_tool_groups();
    
    // Add tools from mode's groups
    for mode_group_entry in mode_groups {
        let mode_group = mode_group_entry.get_group_name();
        let ext_group = ExtendedToolGroup::from_tool_group(mode_group);
        
        if let Some((_, config)) = tool_groups.iter().find(|(g, _)| g == &ext_group) {
            for tool in &config.tools {
                tools.insert(tool.to_string());
            }
        }
    }
    
    // Add always available tools
    for tool in ALWAYS_AVAILABLE_TOOLS {
        tools.insert(tool.to_string());
    }
    
    tools
}

/// Filter tools based on feature gates
///
/// Translation of filtering logic from index.ts (lines 135-166)
pub fn filter_tools_by_features(
    mut tools: HashSet<String>,
    codebase_search_available: bool,
    fast_apply_available: bool,
    todo_list_enabled: bool,
    image_generation_enabled: bool,
    run_slash_command_enabled: bool,
) -> HashSet<String> {
    // Conditionally exclude codebase_search if not available
    if !codebase_search_available {
        tools.remove("codebase_search");
    }
    
    // When Morph (edit_file) is enabled, disable traditional editing tools
    if fast_apply_available {
        tools.remove("apply_diff");
        tools.remove("write_to_file");
        tools.remove("insert_content");
        tools.remove("search_and_replace");
    } else {
        tools.remove("edit_file");
    }
    
    // Conditionally exclude update_todo_list if disabled
    if !todo_list_enabled {
        tools.remove("update_todo_list");
    }
    
    // Conditionally exclude generate_image if experiment is not enabled
    if !image_generation_enabled {
        tools.remove("generate_image");
    }
    
    // Conditionally exclude run_slash_command if experiment is not enabled
    if !run_slash_command_enabled {
        tools.remove("run_slash_command");
    }
    
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_groups() {
        let groups = get_tool_groups();
        assert_eq!(groups.len(), 6);
        
        // Verify read group
        let read_group = groups.iter().find(|(g, _)| *g == ToolGroup::Read).unwrap();
        assert!(read_group.1.tools.contains(&"read_file"));
        assert!(read_group.1.tools.contains(&"search_files"));
    }
    
    #[test]
    fn test_always_available_tools() {
        assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"ask_followup_question"));
        assert!(ALWAYS_AVAILABLE_TOOLS.contains(&"attempt_completion"));
    }
    
    #[test]
    fn test_filter_codebase_search() {
        let mut tools = HashSet::new();
        tools.insert("read_file".to_string());
        tools.insert("codebase_search".to_string());
        
        let filtered = filter_tools_by_features(tools, false, false, true, false, false);
        assert!(!filtered.contains("codebase_search"));
        assert!(filtered.contains("read_file"));
    }
    
    #[test]
    fn test_filter_fast_apply() {
        let mut tools = HashSet::new();
        tools.insert("edit_file".to_string());
        tools.insert("write_to_file".to_string());
        tools.insert("apply_diff".to_string());
        
        // When fast_apply is enabled, traditional tools are removed
        let filtered = filter_tools_by_features(tools.clone(), false, true, true, false, false);
        assert!(filtered.contains("edit_file"));
        assert!(!filtered.contains("write_to_file"));
        assert!(!filtered.contains("apply_diff"));
        
        // When fast_apply is disabled, edit_file is removed
        let filtered = filter_tools_by_features(tools, false, false, true, false, false);
        assert!(!filtered.contains("edit_file"));
        assert!(filtered.contains("write_to_file"));
        assert!(filtered.contains("apply_diff"));
    }
}
