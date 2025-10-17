//! AI Modes Configuration
//!
//! 1:1 Translation from Codex `src/shared/modes.ts` and `packages/types/src/mode.ts`
//!
//! Reference: /home/verma/lapce/Codex/packages/types/src/mode.ts (lines 137-212)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::core::prompt::errors::PromptError;

/// Mode slug - string identifier for AI modes
pub type Mode = String;

/// Tool group names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolGroup {
    Read,
    Edit,
    Browser,
    Command,
    Mcp,
}

impl ToolGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolGroup::Read => "read",
            ToolGroup::Edit => "edit",
            ToolGroup::Browser => "browser",
            ToolGroup::Command => "command",
            ToolGroup::Mcp => "mcp",
        }
    }
}

/// Group options for file restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupOptions {
    /// Regex pattern for file restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_regex: Option<String>,
    
    /// Human-readable description of the restriction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Group entry can be just a group name or a tuple with options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroupEntry {
    Simple(ToolGroup),
    WithOptions(ToolGroup, GroupOptions),
}

impl GroupEntry {
    pub fn get_group_name(&self) -> &ToolGroup {
        match self {
            GroupEntry::Simple(group) => group,
            GroupEntry::WithOptions(group, _) => group,
        }
    }
    
    pub fn get_options(&self) -> Option<&GroupOptions> {
        match self {
            GroupEntry::Simple(_) => None,
            GroupEntry::WithOptions(_, options) => Some(options),
        }
    }
}

/// Mode configuration - defines AI behavior and available tools
///
/// Translation of ModeConfig from `packages/types/src/mode.ts`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Unique identifier (e.g., "code", "architect", "ask")
    pub slug: String,
    
    /// Display name
    pub name: String,
    
    /// Icon name (Codicon identifier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_name: Option<String>,
    
    /// Role definition - main system prompt for this mode
    pub role_definition: String,
    
    /// When to use this mode (UI hint)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    
    /// Description (short summary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Custom instructions for this mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
    
    /// Tool groups available in this mode
    pub groups: Vec<GroupEntry>,
    
    /// Source (global, project, organization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Default mode slug
pub const DEFAULT_MODE_SLUG: &str = "code";

/// Built-in modes - exact translation from DEFAULT_MODES in mode.ts (lines 137-211)
pub fn get_default_modes() -> Vec<ModeConfig> {
    vec![
        // Architect mode (lines 138-152)
        ModeConfig {
            slug: "architect".to_string(),
            name: "Architect".to_string(),
            icon_name: Some("codicon-type-hierarchy-sub".to_string()),
            role_definition: "You are Kilo Code, an experienced technical leader who is inquisitive and an excellent planner. Your goal is to gather information and get context to create a detailed plan for accomplishing the user's task, which the user will review and approve before they switch into another mode to implement the solution.".to_string(),
            when_to_use: Some("Use this mode when you need to plan, design, or strategize before implementation. Perfect for breaking down complex problems, creating technical specifications, designing system architecture, or brainstorming solutions before coding.".to_string()),
            description: Some("Plan and design before implementation".to_string()),
            groups: vec![
                GroupEntry::Simple(ToolGroup::Read),
                GroupEntry::WithOptions(
                    ToolGroup::Edit,
                    GroupOptions {
                        file_regex: Some("\\.md$".to_string()),
                        description: Some("Markdown files only".to_string()),
                    }
                ),
                GroupEntry::Simple(ToolGroup::Browser),
                GroupEntry::Simple(ToolGroup::Mcp),
            ],
            custom_instructions: Some("1. Do some information gathering (using provided tools) to get more context about the task.\n\n2. You should also ask the user clarifying questions to get a better understanding of the task.\n\n3. Once you've gained more context about the user's request, break down the task into clear, actionable steps and create a todo list using the `update_todo_list` tool. Each todo item should be:\n   - Specific and actionable\n   - Listed in logical execution order\n   - Focused on a single, well-defined outcome\n   - Clear enough that another mode could execute it independently\n\n   **Note:** If the `update_todo_list` tool is not available, write the plan to a markdown file (e.g., `plan.md` or `todo.md`) instead.\n\n4. As you gather more information or discover new requirements, update the todo list to reflect the current understanding of what needs to be accomplished.\n\n5. Ask the user if they are pleased with this plan, or if they would like to make any changes. Think of this as a brainstorming session where you can discuss the task and refine the todo list.\n\n6. Include Mermaid diagrams if they help clarify complex workflows or system architecture. Please avoid using double quotes (\"\") and parentheses () inside square brackets ([]) in Mermaid diagrams, as this can cause parsing errors.\n\n7. Use the switch_mode tool to request that the user switch to another mode to implement the solution.\n\n**IMPORTANT: Focus on creating clear, actionable todo lists rather than lengthy markdown documents. Use the todo list as your primary planning tool to track and organize the work that needs to be done.**".to_string()),
            source: None,
        },
        
        // Code mode (lines 153-165)
        ModeConfig {
            slug: "code".to_string(),
            name: "Code".to_string(),
            icon_name: Some("codicon-code".to_string()),
            role_definition: "You are Kilo Code, a highly skilled software engineer with extensive knowledge in many programming languages, frameworks, design patterns, and best practices.".to_string(),
            when_to_use: Some("Use this mode when you need to write, modify, or refactor code. Ideal for implementing features, fixing bugs, creating new files, or making code improvements across any programming language or framework.".to_string()),
            description: Some("Write, modify, and refactor code".to_string()),
            groups: vec![
                GroupEntry::Simple(ToolGroup::Read),
                GroupEntry::Simple(ToolGroup::Edit),
                GroupEntry::Simple(ToolGroup::Browser),
                GroupEntry::Simple(ToolGroup::Command),
                GroupEntry::Simple(ToolGroup::Mcp),
            ],
            custom_instructions: None,
            source: None,
        },
        
        // Ask mode (lines 166-180)
        ModeConfig {
            slug: "ask".to_string(),
            name: "Ask".to_string(),
            icon_name: Some("codicon-question".to_string()),
            role_definition: "You are Kilo Code, a knowledgeable technical assistant focused on answering questions and providing information about software development, technology, and related topics.".to_string(),
            when_to_use: Some("Use this mode when you need explanations, documentation, or answers to technical questions. Best for understanding concepts, analyzing existing code, getting recommendations, or learning about technologies without making changes.".to_string()),
            description: Some("Get answers and explanations".to_string()),
            groups: vec![
                GroupEntry::Simple(ToolGroup::Read),
                GroupEntry::Simple(ToolGroup::Browser),
                GroupEntry::Simple(ToolGroup::Mcp),
            ],
            custom_instructions: Some("You can analyze code, explain concepts, and access external resources. Always answer the user's questions thoroughly, and do not switch to implementing code unless explicitly requested by the user. Include Mermaid diagrams when they clarify your response.".to_string()),
            source: None,
        },
        
        // Debug mode (lines 181-195)
        ModeConfig {
            slug: "debug".to_string(),
            name: "Debug".to_string(),
            icon_name: Some("codicon-bug".to_string()),
            role_definition: "You are Kilo Code, an expert software debugger specializing in systematic problem diagnosis and resolution.".to_string(),
            when_to_use: Some("Use this mode when you're troubleshooting issues, investigating errors, or diagnosing problems. Specialized in systematic debugging, adding logging, analyzing stack traces, and identifying root causes before applying fixes.".to_string()),
            description: Some("Diagnose and fix software issues".to_string()),
            groups: vec![
                GroupEntry::Simple(ToolGroup::Read),
                GroupEntry::Simple(ToolGroup::Edit),
                GroupEntry::Simple(ToolGroup::Browser),
                GroupEntry::Simple(ToolGroup::Command),
                GroupEntry::Simple(ToolGroup::Mcp),
            ],
            custom_instructions: Some("Reflect on 5-7 different possible sources of the problem, distill those down to 1-2 most likely sources, and then add logs to validate your assumptions. Explicitly ask the user to confirm the diagnosis before fixing the problem.".to_string()),
            source: None,
        },
        
        // Orchestrator mode (lines 196-211)
        ModeConfig {
            slug: "orchestrator".to_string(),
            name: "Orchestrator".to_string(),
            icon_name: Some("codicon-run-all".to_string()),
            role_definition: "You are Kilo Code, a strategic workflow orchestrator who coordinates complex tasks by delegating them to appropriate specialized modes. You have a comprehensive understanding of each mode's capabilities and limitations, allowing you to effectively break down complex problems into discrete tasks that can be solved by different specialists.".to_string(),
            when_to_use: Some("Use this mode for complex, multi-step projects that require coordination across different specialties. Ideal when you need to break down large tasks into subtasks, manage workflows, or coordinate work that spans multiple domains or expertise areas.".to_string()),
            description: Some("Coordinate tasks across multiple modes".to_string()),
            groups: vec![], // Empty groups - orchestrator delegates to other modes
            custom_instructions: Some("Your role is to coordinate complex workflows by delegating tasks to specialized modes. As an orchestrator, you should:\n\n1. When given a complex task, break it down into logical subtasks that can be delegated to appropriate specialized modes.\n\n2. For each subtask, use the `new_task` tool to delegate. Choose the most appropriate mode for the subtask's specific goal and provide comprehensive instructions in the `message` parameter. These instructions must include:\n    *   All necessary context from the parent task or previous subtasks required to complete the work.\n    *   A clearly defined scope, specifying exactly what the subtask should accomplish.\n    *   An explicit statement that the subtask should *only* perform the work outlined in these instructions and not deviate.\n    *   An instruction for the subtask to signal completion by using the `attempt_completion` tool, providing a concise yet thorough summary of the outcome in the `result` parameter, keeping in mind that this summary will be the source of truth used to keep track of what was completed on this project.\n    *   A statement that these specific instructions supersede any conflicting general instructions the subtask's mode might have.\n\n3. Track and manage the progress of all subtasks. When a subtask is completed, analyze its results and determine the next steps.\n\n4. Help the user understand how the different subtasks fit together in the overall workflow. Provide clear reasoning about why you're delegating specific tasks to specific modes.\n\n5. When all subtasks are completed, synthesize the results and provide a comprehensive overview of what was accomplished.\n\n6. Ask clarifying questions when necessary to better understand how to break down complex tasks effectively.\n\n7. Suggest improvements to the workflow based on the results of completed subtasks.\n\nUse subtasks to maintain clarity. If a request significantly shifts focus or requires a different expertise (mode), consider creating a subtask for that work.".to_string()),
            source: None,
        },
    ]
}

/// Get mode configuration by slug
///
/// Translation of `getModeBySlug()` from modes.ts (lines 70-78)
pub fn get_mode_by_slug(slug: &str) -> Result<ModeConfig, PromptError> {
    get_default_modes()
        .into_iter()
        .find(|mode| mode.slug == slug)
        .ok_or_else(|| PromptError::ModeNotFound(slug.to_string()))
}

/// Get role definition and base instructions for a mode
///
/// Translation of `getModeSelection()` from modes.ts (lines 130-151)
pub fn get_mode_selection(mode: &ModeConfig) -> (String, String) {
    let role_definition = mode.role_definition.clone();
    let base_instructions = mode.custom_instructions.clone().unwrap_or_default();
    
    (role_definition, base_instructions)
}

/// Get all tools for a mode based on its groups
///
/// Translation of `getToolsForMode()` from modes.ts (lines 47-61)
pub fn get_tools_for_mode(groups: &[GroupEntry]) -> HashSet<String> {
    let mut tools = HashSet::new();
    
    // Add tools from each group (will be implemented when tool registry is done)
    for group in groups {
        let _group_name = group.get_group_name();
        // TODO: Add tools from TOOL_GROUPS[group_name]
    }
    
    // Add always-available tools (will be implemented when tool registry is done)
    // TODO: Add ALWAYS_AVAILABLE_TOOLS
    
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_modes_count() {
        let modes = get_default_modes();
        assert_eq!(modes.len(), 5, "Should have 5 default modes");
    }
    
    #[test]
    fn test_default_mode_slugs() {
        let modes = get_default_modes();
        let slugs: Vec<_> = modes.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(slugs, vec!["architect", "code", "ask", "debug", "orchestrator"]);
    }
    
    #[test]
    fn test_get_mode_by_slug() {
        let code_mode = get_mode_by_slug("code").unwrap();
        assert_eq!(code_mode.name, "Code");
        assert_eq!(code_mode.groups.len(), 5);
    }
    
    #[test]
    fn test_get_mode_by_slug_not_found() {
        let result = get_mode_by_slug("nonexistent");
        assert!(matches!(result, Err(PromptError::ModeNotFound(_))));
    }
    
    #[test]
    fn test_architect_mode_file_restriction() {
        let arch_mode = get_mode_by_slug("architect").unwrap();
        
        // Find the edit group
        let edit_group = arch_mode.groups.iter()
            .find(|g| matches!(g.get_group_name(), ToolGroup::Edit))
            .unwrap();
        
        // Verify it has file regex restriction
        let options = edit_group.get_options().unwrap();
        assert_eq!(options.file_regex, Some("\\.md$".to_string()));
        assert_eq!(options.description, Some("Markdown files only".to_string()));
    }
    
    #[test]
    fn test_orchestrator_mode_empty_groups() {
        let orch_mode = get_mode_by_slug("orchestrator").unwrap();
        assert_eq!(orch_mode.groups.len(), 0, "Orchestrator should have no tool groups");
    }
    
    #[test]
    fn test_mode_selection() {
        let code_mode = get_mode_by_slug("code").unwrap();
        let (role, instructions) = get_mode_selection(&code_mode);
        
        assert!(role.contains("highly skilled software engineer"));
        assert_eq!(instructions, ""); // Code mode has no custom instructions
    }
}
