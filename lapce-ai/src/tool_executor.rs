// Tool Executor - CHUNK-02 Integration with existing tools
// Connects AssistantMessageContent to actual tool execution

use std::sync::Arc;
use std::path::PathBuf;
use anyhow::Result;
use serde_json::{Value, json};
use tracing::{info, debug, warn, error};

use crate::task_exact_translation::{Task, AssistantMessageContent};
use crate::core::tools::traits::{Tool, ToolContext};
use crate::core::tools::registry::ToolRegistry;
use crate::tool_repetition_detector::{ToolRepetitionDetector, RepetitionResult};
use crate::task_orchestrator_metrics::global_metrics;

// Import existing tools
use crate::core::tools::fs::{
    ReadFileTool, WriteFileTool, EditFileTool, SearchFilesTool,
    ListFilesTool, InsertContentTool, SearchAndReplaceTool,
};
use crate::core::tools::execute_command::ExecuteCommandTool;
use crate::core::tools::diff_tool::DiffTool;

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
}

/// Tool executor orchestrator
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    repetition_detector: ToolRepetitionDetector,
    workspace: PathBuf,
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new(workspace: PathBuf) -> Result<Self> {
        let registry = Arc::new(ToolRegistry::new());
        
        // Register all available tools
        Self::register_all_tools(&registry)?;
        
        Ok(Self {
            registry,
            repetition_detector: ToolRepetitionDetector::new(),
            workspace,
        })
    }
    
    /// Register all available tools from core/tools
    fn register_all_tools(registry: &ToolRegistry) -> Result<()> {
        // File system tools
        registry.register(ReadFileTool)?;
        registry.register(WriteFileTool)?;
        registry.register(EditFileTool)?;
        registry.register(SearchFilesTool)?;
        registry.register(ListFilesTool)?;
        registry.register(InsertContentTool)?;
        registry.register(SearchAndReplaceTool)?;
        
        // Command execution
        registry.register(ExecuteCommandTool)?;
        
        // Diff tool
        registry.register(DiffTool)?;
        
        // Register expanded tools (10+ additional)
        crate::core::tools::expanded_tools::register_expanded_tools(registry)?;
        
        info!("Registered {} tools", registry.list_tools().len());
        Ok(())
    }
    
    /// Execute tool from AssistantMessageContent
    pub async fn execute_tool_use(
        &mut self,
        task: Arc<Task>,
        content: &AssistantMessageContent,
    ) -> Result<ToolExecutionResult> {
        match content {
            AssistantMessageContent::ToolUse { name, input } => {
                info!("Executing tool: {}", name);
                
                // Check for repetition
                let params_json = serde_json::to_string(input)?;
                let repetition = self.repetition_detector.record_call(name, &params_json);
                
                match repetition {
                    RepetitionResult::None | RepetitionResult::NoRepetition => {
                        debug!("No repetition detected for {}", name);
                    }
                    RepetitionResult::SameTool { count, .. } | RepetitionResult::SameToolRepeated { count, .. } => {
                        warn!("Tool {} repeated {} times", name, count);
                        task.increment_tool_mistakes();
                    }
                    RepetitionResult::IdenticalCalls { count, .. } | RepetitionResult::IdenticalCall { count, .. } => {
                        warn!("Identical call to {} detected ({} times)", name, count);
                        task.increment_tool_mistakes();
                        return Ok(ToolExecutionResult {
                            tool_name: name.clone(),
                            success: false,
                            output: json!({"error": "Identical tool call detected"}),
                            error: Some("Tool call appears to be stuck in a loop".to_string()),
                        });
                    }
                    RepetitionResult::CyclicPattern { pattern_length, .. } => {
                        warn!("Cyclic pattern detected (length {})", pattern_length);
                        task.increment_tool_mistakes();
                    }
                }
                
                // Create tool context
                let context = self.create_context(task.clone());
                
                // Execute tool
                let result = self.registry.execute(name, input.clone(), context).await?;
                
                match result {
                    Ok(output) => {
                        info!("Tool {} executed successfully", name);
                        global_metrics().record_tool_success(name);
                        
                        Ok(ToolExecutionResult {
                            tool_name: name.clone(),
                            success: true,
                            output: output.result.clone(),
                            error: None,
                        })
                    }
                    Err(e) => {
                        error!("Tool {} failed: {:?}", name, e);
                        global_metrics().record_tool_failure(name);
                        task.increment_tool_mistakes();
                        
                        Ok(ToolExecutionResult {
                            tool_name: name.clone(),
                            success: false,
                            output: Value::Null,
                            error: Some(format!("{:?}", e)),
                        })
                    }
                }
            }
            _ => {
                Err(anyhow::anyhow!("Content is not a tool use"))
            }
        }
    }
    
    /// Execute all tools from assistant message content blocks
    pub async fn execute_all_tools(
        &mut self,
        task: Arc<Task>,
        content_blocks: &[AssistantMessageContent],
    ) -> Result<Vec<ToolExecutionResult>> {
        let mut results = Vec::new();
        
        for content in content_blocks {
            if let AssistantMessageContent::ToolUse { .. } = content {
                let result = self.execute_tool_use(task.clone(), content).await?;
                results.push(result);
            }
        }
        
        Ok(results)
    }
    
    /// Create tool context from task
    fn create_context(&self, task: Arc<Task>) -> ToolContext {
        ToolContext {
            workspace: self.workspace.clone(),
            user_id: "task_user".to_string(),
            session_id: task.task_id.clone(),
            execution_id: uuid::Uuid::new_v4().to_string(),
            require_approval: false, // Task handles approval separately
            dry_run: false,
            allow_local_network: false,
            metadata: std::collections::HashMap::new(),
            permissions: Default::default(),
            rooignore: None,
            permission_manager: None,
            config: Arc::new(crate::core::tools::config::get_config().clone()),
            log_context: None,
            adapters: std::collections::HashMap::new(),
            event_emitters: Vec::new(),
            diff_controllers: Vec::new(),
        }
    }
    
    /// Get tool registry reference
    pub fn registry(&self) -> Arc<ToolRegistry> {
        self.registry.clone()
    }
    
    /// Reset repetition detector
    pub fn reset_detector(&mut self) {
        self.repetition_detector.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::task_exact_translation::{TaskOptions, ExtensionContext};
    use tokio::sync::RwLock;
    
    fn create_test_task() -> Arc<Task> {
        let options = TaskOptions {
            task: Some("Test task".to_string()),
            assistant_message_info: None,
            assistant_metadata: None,
            custom_variables: None,
            images: None,
            start_with: None,
            project_path: None,
            automatically_approve_api_requests: None,
            context_files_content: None,
            context_files: None,
            experiments: None,
            start_task: Some(false),
            root_task: None,
            parent_task: None,
            task_number: None,
            on_created: None,
            initial_todos: None,
            context: Some(ExtensionContext {
                global_storage_uri: PathBuf::from("/tmp"),
                workspace_state: Arc::new(RwLock::new(HashMap::<String, serde_json::Value>::new())),
            }),
            provider: None,
            api_configuration: None,
            enable_diff: None,
            enable_checkpoints: None,
            enable_task_bridge: None,
            fuzzy_match_threshold: None,
            consecutive_mistake_limit: None,
            history_item: None,
        };
        
        Task::new(options)
    }
    
    #[tokio::test]
    async fn test_executor_creation() {
        let executor = ToolExecutor::new(PathBuf::from("/tmp")).unwrap();
        assert!(executor.registry().count() > 0);
    }
    
    #[tokio::test]
    async fn test_tool_execution() {
        let mut executor = ToolExecutor::new(PathBuf::from("/tmp")).unwrap();
        let task = create_test_task();
        
        let content = AssistantMessageContent::ToolUse {
            name: "read_file".to_string(),
            input: json!({
                "path": "test.txt"
            }),
        };
        
        let result = executor.execute_tool_use(task, &content).await;
        assert!(result.is_ok());
    }
}
