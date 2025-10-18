// Orchestrator Integration Bridge - Connects API + Tools + Orchestration Loop
// This is the production-ready orchestration layer

use std::sync::Arc;
use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, error};

use crate::task_exact_translation::Task;
use crate::task_manager::TaskManager;
use crate::api_provider_integration::{ApiOrchestrationBridge, ApiRequest, ApiProvider};
use crate::tool_executor::{ToolExecutor, ToolExecutionResult};
use crate::task_orchestration_loop::OrchestrationExecutor;
use crate::task_orchestrator_metrics::global_metrics;

/// Full orchestrator with API + Tools integration
pub struct IntegratedOrchestrator {
    task_manager: Arc<TaskManager>,
    api_bridge: ApiOrchestrationBridge,
    tool_executor: ToolExecutor,
    orchestration_executor: OrchestrationExecutor,
}

impl IntegratedOrchestrator {
    pub fn new(workspace: PathBuf) -> Result<Self> {
        Ok(Self {
            task_manager: Arc::new(TaskManager::new()),
            api_bridge: ApiOrchestrationBridge::new(),
            tool_executor: ToolExecutor::new(workspace)?,
            orchestration_executor: OrchestrationExecutor::new(),
        })
    }
    
    /// Run full orchestration loop for a task
    pub async fn run_task(&mut self, task: Arc<Task>) -> Result<()> {
        info!("Starting integrated orchestration for task {}", task.task_id);
        
        // Step 1: Execute orchestration loop
        self.orchestration_executor.run(task.clone()).await?;
        
        // Step 2: Get API request from task's conversation
        let api_request = self.build_api_request(task.clone());
        
        // Step 3: Execute API request with streaming
        let content_blocks = self.api_bridge.execute_request(task.clone(), api_request).await?;
        
        // Step 4: Execute all tools from response
        let tool_results = self.tool_executor.execute_all_tools(task.clone(), &content_blocks).await?;
        
        // Step 5: Process results and continue if needed
        for result in &tool_results {
            if !result.success {
                error!("Tool {} failed: {:?}", result.tool_name, result.error);
                task.increment_mistakes();
            }
        }
        
        global_metrics().record_task_lifecycle("completed");
        info!("Orchestration completed for task {}", task.task_id);
        
        Ok(())
    }
    
    fn build_api_request(&self, task: Arc<Task>) -> ApiRequest {
        ApiRequest {
            provider: ApiProvider::Anthropic,
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: task.get_api_conversation(),
            max_tokens: Some(8192),
            temperature: Some(0.7),
            system_prompt: Some("You are a helpful coding assistant.".to_string()),
        }
    }
    
    pub fn task_manager(&self) -> Arc<TaskManager> {
        self.task_manager.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_exact_translation::{TaskOptions, ExtensionContext};
    use parking_lot::RwLock;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_integrated_orchestrator() {
        let orchestrator = IntegratedOrchestrator::new(PathBuf::from("/tmp")).unwrap();
        assert!(orchestrator.task_manager().task_count() == 0);
    }
}
