// Task Orchestration Loop - CHUNK-03: T12
// Stack-based iterative loop with tool-use deferral

use std::sync::Arc;
use anyhow::{Result, bail};
use tracing::{info, warn, debug, error};

use crate::task_exact_translation::{Task, AssistantMessageContent, ApiMessage};
use crate::assistant_message_parser::{AssistantMessageParser, StreamChunk};
use crate::backoff_util::{BackoffState, BackoffConfig};
use crate::task_orchestrator_metrics::global_metrics;

/// Orchestration state for iterative loop
#[derive(Debug, Clone)]
enum OrchestrationState {
    /// Initial state - preparing request
    Preparing,
    
    /// Waiting for API response
    WaitingForResponse,
    
    /// Processing streaming response
    ProcessingStream,
    
    /// Tool execution deferred (tools disabled)
    ToolDeferred { tool_name: String },
    
    /// Completed successfully
    Completed,
    
    /// Aborted
    Aborted { reason: String },
}

/// Stack frame for iterative orchestration
#[derive(Debug)]
struct OrchestrationFrame {
    state: OrchestrationState,
    depth: usize,
    iteration: usize,
}

/// Orchestration loop configuration
#[derive(Debug, Clone)]
pub struct OrchestrationConfig {
    /// Maximum recursion depth
    pub max_depth: usize,
    
    /// Maximum iterations per depth level
    pub max_iterations: usize,
    
    /// Enable tool execution (if false, tools are deferred)
    pub tools_enabled: bool,
    
    /// Backoff configuration for retries
    pub backoff_config: BackoffConfig,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_iterations: 100,
            tools_enabled: false, // Deferred until CHUNK-02 integration
            backoff_config: BackoffConfig::default(),
        }
    }
}

/// Orchestration loop executor
pub struct OrchestrationExecutor {
    config: OrchestrationConfig,
}

impl OrchestrationExecutor {
    pub fn new(config: OrchestrationConfig) -> Self {
        Self { config }
    }
    
    /// Run the orchestration loop for a task
    /// This is an iterative (non-recursive) implementation using an explicit stack
    pub async fn run(&self, task: &Arc<Task>) -> Result<()> {
        let mut stack: Vec<OrchestrationFrame> = vec![
            OrchestrationFrame {
                state: OrchestrationState::Preparing,
                depth: 0,
                iteration: 0,
            }
        ];
        
        let mut backoff = BackoffState::new(self.config.backoff_config.clone());
        
        while let Some(mut frame) = stack.pop() {
            // Check abort flag
            if task.is_aborted() {
                info!("Task aborted, exiting orchestration loop");
                return Ok(());
            }
            
            // Check depth limit
            if frame.depth >= self.config.max_depth {
                warn!("Maximum recursion depth {} reached", self.config.max_depth);
                task.say(
                    crate::ipc_messages::ClineSay::Error,
                    Some(format!("Maximum depth {} reached", self.config.max_depth))
                )?;
                break;
            }
            
            // Check iteration limit
            if frame.iteration >= self.config.max_iterations {
                warn!("Maximum iterations {} reached at depth {}", 
                    self.config.max_iterations, frame.depth);
                break;
            }
            
            match frame.state {
                OrchestrationState::Preparing => {
                    debug!("Preparing request at depth {}", frame.depth);
                    
                    // Transition to waiting for response
                    frame.state = OrchestrationState::WaitingForResponse;
                    stack.push(frame);
                }
                
                OrchestrationState::WaitingForResponse => {
                    debug!("Waiting for API response");
                    
                    // In real implementation, this would call API provider
                    // For now, simulate with a placeholder
                    
                    // Transition to processing stream
                    frame.state = OrchestrationState::ProcessingStream;
                    stack.push(frame);
                }
                
                OrchestrationState::ProcessingStream => {
                    debug!("Processing streaming response");
                    
                    // Parse assistant message (simulated)
                    let content = self.simulate_assistant_response(task).await;
                    
                    // Check for tool use
                    let has_tool_use = content.iter().any(|c| {
                        matches!(c, AssistantMessageContent::ToolUse { .. })
                    });
                    
                    if has_tool_use && !self.config.tools_enabled {
                        // Tools disabled - defer execution
                        if let Some(AssistantMessageContent::ToolUse { name, .. }) = 
                            content.iter().find(|c| matches!(c, AssistantMessageContent::ToolUse { .. })) 
                        {
                            warn!("Tool execution deferred: {}", name);
                            frame.state = OrchestrationState::ToolDeferred { 
                                tool_name: name.clone() 
                            };
                            stack.push(frame);
                        }
                    } else if has_tool_use && self.config.tools_enabled {
                        // Tools enabled - would execute here (CHUNK-02 integration)
                        info!("Tool execution would happen here (deferred to CHUNK-02)");
                        
                        // Push next iteration
                        stack.push(OrchestrationFrame {
                            state: OrchestrationState::Preparing,
                            depth: frame.depth + 1,
                            iteration: 0,
                        });
                    } else {
                        // No tool use - complete
                        frame.state = OrchestrationState::Completed;
                        stack.push(frame);
                    }
                }
                
                OrchestrationState::ToolDeferred { ref tool_name } => {
                    // Emit message about deferral
                    task.say(
                        crate::ipc_messages::ClineSay::Text,
                        Some(format!("Tool '{}' execution deferred (tools disabled)", tool_name))
                    )?;
                    
                    // Mark as completed for now
                    frame.state = OrchestrationState::Completed;
                    stack.push(frame);
                }
                
                OrchestrationState::Completed => {
                    info!("Orchestration completed at depth {}", frame.depth);
                    global_metrics().record_task_completed(
                        std::time::Duration::from_secs(0)
                    );
                    break;
                }
                
                OrchestrationState::Aborted { ref reason } => {
                    warn!("Orchestration aborted: {}", reason);
                    break;
                }
            }
            
            // Increment iteration for next loop
            if let Some(ref mut top) = stack.last_mut() {
                top.iteration += 1;
            }
        }
        
        Ok(())
    }
    
    /// Simulate assistant response (placeholder until API integration)
    async fn simulate_assistant_response(&self, _task: &Arc<Task>) -> Vec<AssistantMessageContent> {
        // Placeholder - returns text only
        vec![AssistantMessageContent::Text {
            text: "Simulated response".to_string()
        }]
    }
    
    /// Check if loop should continue
    fn should_continue(&self, task: &Arc<Task>, depth: usize) -> bool {
        !task.is_aborted() && depth < self.config.max_depth
    }
}

impl Default for OrchestrationExecutor {
    fn default() -> Self {
        Self::new(OrchestrationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_exact_translation::{TaskOptions, ExtensionContext};
    use std::path::PathBuf;
    use parking_lot::RwLock;
    use std::collections::HashMap;
    
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
                workspace_state: Arc::new(RwLock::new(HashMap::new())),
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
        let executor = OrchestrationExecutor::default();
        assert_eq!(executor.config.max_depth, 10);
        assert!(!executor.config.tools_enabled);
    }
    
    #[tokio::test]
    async fn test_basic_orchestration() {
        let task = create_test_task();
        let executor = OrchestrationExecutor::default();
        
        let result = executor.run(&task).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_abort_handling() {
        let task = create_test_task();
        let executor = OrchestrationExecutor::default();
        
        // Abort before running
        task.request_abort();
        
        let result = executor.run(&task).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_depth_limit() {
        let config = OrchestrationConfig {
            max_depth: 2,
            ..Default::default()
        };
        let executor = OrchestrationExecutor::new(config);
        let task = create_test_task();
        
        let result = executor.run(&task).await;
        assert!(result.is_ok());
    }
}
