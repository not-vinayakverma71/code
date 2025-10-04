/// Complete translation of tool.ts from Codex/packages/types/src/tool.ts
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// COMPLETE tool.ts TRANSLATION START
// ============================================================================

/// ToolGroup - All 6 tool groups from tool.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolGroup {
    Read,
    Edit,
    Browser,
    Command,
    Mcp,
    Modes,
}

/// ToolName - All 22 tool names from tool.ts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

/// ToolStats - For ToolUsage record type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub attempts: u32,
    pub failures: u32,
}

/// ToolUsage - Record type from tool.ts: z.record(toolNamesSchema, z.object({...}))
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    #[serde(flatten)]
    pub tools: HashMap<ToolName, ToolStats>,
}

impl Default for ToolUsage {
    fn default() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
}

impl ToolUsage {
    /// Create a new empty ToolUsage
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record a tool attempt
    pub fn record_attempt(&mut self, tool: ToolName) {
        self.tools
            .entry(tool)
            .or_insert(ToolStats { attempts: 0, failures: 0 })
            .attempts += 1;
    }
    
    /// Record a tool failure
    pub fn record_failure(&mut self, tool: ToolName) {
        self.tools
            .entry(tool)
            .or_insert(ToolStats { attempts: 0, failures: 0 })
            .failures += 1;
    }
    
    /// Get stats for a specific tool
    pub fn get_stats(&self, tool: ToolName) -> Option<&ToolStats> {
        self.tools.get(&tool)
    }
}

// Tool group categorization helpers
impl ToolName {
    pub fn group(&self) -> ToolGroup {
        match self {
            Self::ReadFile | 
            Self::SearchFiles | 
            Self::ListFiles | 
            Self::ListCodeDefinitionNames |
            Self::CodebaseSearch => ToolGroup::Read,
            
            Self::WriteToFile |
            Self::ApplyDiff |
            Self::InsertContent |
            Self::SearchAndReplace |
            Self::EditFile |
            Self::UpdateTodoList => ToolGroup::Edit,
            
            Self::BrowserAction => ToolGroup::Browser,
            
            Self::ExecuteCommand => ToolGroup::Command,
            
            Self::UseMcpTool |
            Self::AccessMcpResource => ToolGroup::Mcp,
            
            Self::AskFollowupQuestion |
            Self::AttemptCompletion |
            Self::SwitchMode |
            Self::NewTask |
            Self::FetchInstructions |
            Self::NewRule |
            Self::ReportBug |
            Self::Condense => ToolGroup::Modes,
        }
    }
    
    pub fn is_read_only(&self) -> bool {
        matches!(self.group(), ToolGroup::Read)
    }
    
    pub fn is_write(&self) -> bool {
        matches!(self.group(), ToolGroup::Edit)
    }
}

// ============================================================================
// END tool.ts TRANSLATION
// ============================================================================
