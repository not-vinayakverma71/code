//! System Prompt Settings
//!
//! 1:1 Translation from Codex `src/core/prompts/types.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/types.ts (lines 1-10)

use serde::{Deserialize, Serialize};

/// Settings passed to system prompt generation functions
///
/// Translation of SystemPromptSettings interface from types.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPromptSettings {
    /// Maximum concurrent file reads for context gathering
    pub max_concurrent_file_reads: u32,
    
    /// Whether todo list tool is enabled
    pub todo_list_enabled: bool,
    
    /// Whether to use AGENTS.md and custom rules files
    pub use_agent_rules: bool,
    
    /// Whether new tasks require todos before proceeding
    pub new_task_require_todos: bool,
    
    /// Browser viewport size for browser_action tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_viewport_size: Option<String>,
}

impl Default for SystemPromptSettings {
    fn default() -> Self {
        Self {
            max_concurrent_file_reads: 50,
            todo_list_enabled: true,
            use_agent_rules: true,
            new_task_require_todos: false,
            browser_viewport_size: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_settings() {
        let settings = SystemPromptSettings::default();
        assert_eq!(settings.max_concurrent_file_reads, 50);
        assert!(settings.todo_list_enabled);
        assert!(settings.use_agent_rules);
        assert!(!settings.new_task_require_todos);
    }
    
    #[test]
    fn test_serde_roundtrip() {
        let settings = SystemPromptSettings {
            max_concurrent_file_reads: 100,
            todo_list_enabled: false,
            use_agent_rules: true,
            new_task_require_todos: true,
            browser_viewport_size: Some("1920x1080".to_string()),
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: SystemPromptSettings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.max_concurrent_file_reads, 100);
        assert!(!deserialized.todo_list_enabled);
        assert!(deserialized.use_agent_rules);
        assert!(deserialized.new_task_require_todos);
    }
}
