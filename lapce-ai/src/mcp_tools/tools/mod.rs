// MCP Tools Module

pub mod browser_action;
pub mod filesystem;
pub mod git;

// Tool implementations
pub mod read_file {
    pub use super::filesystem::ReadFileTool;
}
pub mod write_file {
    pub use super::filesystem::WriteFileTool;
}
pub mod edit_file {
    pub use super::filesystem::EditFileTool;
}
pub mod list_files {
    pub use super::filesystem::ListFilesTool;
}
pub mod search_files {
    pub use super::filesystem::SearchFilesTool;
}
pub mod execute_command {
    pub use super::filesystem::ExecuteCommandTool;
}
pub mod apply_diff {
    pub use super::git::ApplyDiffTool;
}
pub mod insert_content {
    pub use super::filesystem::InsertContentTool;
}
pub mod search_and_replace {
    pub use super::filesystem::SearchAndReplaceTool;
}
pub mod list_code_definitions {
    pub use super::filesystem::ListCodeDefinitionsTool;
}
pub mod new_task {
    pub use super::filesystem::NewTaskTool;
}
pub mod update_todo_list {
    pub use super::filesystem::UpdateTodoListTool;
}
pub mod attempt_completion {
    pub use super::filesystem::AttemptCompletionTool;
}
pub mod ask_followup_question {
    pub use super::filesystem::AskFollowupQuestionTool;
}

// Re-export main tools
pub use browser_action::BrowserActionTool;
pub use filesystem::ListCodeDefinitionsTool as CodebaseSearchTool;
