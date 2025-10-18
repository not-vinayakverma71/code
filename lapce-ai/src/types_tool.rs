/// Tool Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/tool.ts
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ToolGroup - Direct translation from TypeScript
/// Lines 7-11 from tool.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolGroup {
    Read,
    Edit,
    Browser,
    Command,
    Mcp,
    Modes,
}

/// ToolParameter for tool invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub value: String,
}

/// ToolName - Direct translation from TypeScript
/// Lines 17-43 from tool.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    ExecuteCommand,
    ReadFile,
    WriteToFile,
    ApplyDiff,
    InsertContent,
    SearchAndReplace,
    SearchFiles,
    ListFiles,
    ListCodeDefinitionNames,
    BrowserAction,
    UseMcpTool,
    AccessMcpResource,
    AskFollowupQuestion,
    AttemptCompletion,
    SwitchMode,
    NewTask,
    FetchInstructions,
    CodebaseSearch,
    // kilocode_change start
    EditFile,
    NewRule,
    ReportBug,
    Condense,
    // kilocode_change end
    UpdateTodoList,
}

/// ToolUsage - Direct translation from TypeScript
/// Lines 53-61 from tool.ts
pub type ToolUsage = HashMap<ToolName, ToolUsageEntry>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageEntry {
    pub attempts: u32,
    pub failures: u32,
}
