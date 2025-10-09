// Subtask Manager - CHUNK-03: T13
// Parent/child task semantics with event propagation

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::Notify;
use anyhow::{Result, bail};
use tracing::{info, warn, debug};

use crate::task_exact_translation::{Task, TaskOptions, TaskStatus};
use crate::task_manager::TaskManager;
use crate::events_exact_translation::{TaskEvent, global_event_bus};

/// Subtask relationship tracker
pub struct SubtaskManager {
    /// Map of parent task ID to list of child task IDs
    parent_to_children: Arc<RwLock<HashMap<String, Vec<String>>>>,
    
    /// Map of child task ID to parent task ID
    child_to_parent: Arc<RwLock<HashMap<String, String>>>,
    
    /// Completion notifiers for tasks
    completion_notifiers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
}

impl SubtaskManager {
    pub fn new() -> Self {
        Self {
            parent_to_children: Arc::new(RwLock::new(HashMap::new())),
            child_to_parent: Arc::new(RwLock::new(HashMap::new())),
            completion_notifiers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Spawn a child task from a parent
    pub fn spawn_child(
        &self,
        parent_task_id: &str,
        child_options: TaskOptions,
        task_manager: &TaskManager,
    ) -> Result<String> {
        // Get parent task
        let parent_task = task_manager.get_task(parent_task_id)
            .ok_or_else(|| anyhow::anyhow!("Parent task not found"))?;
        
        // Create child options with parent reference
        let mut child_opts = child_options;
        child_opts.parent_task = Some(parent_task.clone());
        child_opts.root_task = parent_task.root_task.clone()
            .or(Some(parent_task.clone()));
        
        // Create child task
        let child_id = task_manager.create_task(child_opts)?;
        
        // Register relationship
        {
            let mut parent_map = self.parent_to_children.write();
            parent_map.entry(parent_task_id.to_string())
                .or_insert_with(Vec::new)
                .push(child_id.clone());
        }
        
        {
            let mut child_map = self.child_to_parent.write();
            child_map.insert(child_id.clone(), parent_task_id.to_string());
        }
        
        // Create completion notifier
        {
            let mut notifiers = self.completion_notifiers.write();
            notifiers.insert(child_id.clone(), Arc::new(Notify::new()));
        }
        
        // Publish TaskSpawned event
        let event = TaskEvent::TaskSpawned {
            payload: (parent_task_id.to_string(), child_id.clone()),
            task_id: None,
        };
        
        if let Err(e) = global_event_bus().publish(event) {
            warn!("Failed to publish TaskSpawned event: {}", e);
        }
        
        info!("Spawned child task {} from parent {}", child_id, parent_task_id);
        Ok(child_id)
    }
    
    /// Wait for all child tasks to complete
    pub async fn wait_for_children(
        &self,
        parent_task_id: &str,
        task_manager: &TaskManager,
    ) -> Result<()> {
        let child_ids = {
            let parent_map = self.parent_to_children.read();
            parent_map.get(parent_task_id).cloned().unwrap_or_default()
        };
        
        if child_ids.is_empty() {
            debug!("No children to wait for");
            return Ok(());
        }
        
        info!("Waiting for {} child tasks", child_ids.len());
        
        for child_id in &child_ids {
            // Check current status
            if let Some(status) = task_manager.get_task_status(child_id) {
                if status == TaskStatus::Completed || status == TaskStatus::Aborted {
                    continue;
                }
            }
            
            // Wait for completion notification
            if let Some(notifier) = self.completion_notifiers.read().get(child_id) {
                notifier.notified().await;
            }
        }
        
        info!("All child tasks completed");
        Ok(())
    }
    
    /// Notify that a task has completed
    pub fn notify_completion(&self, task_id: &str) {
        if let Some(notifier) = self.completion_notifiers.read().get(task_id) {
            notifier.notify_waiters();
            debug!("Notified completion for task {}", task_id);
        }
    }
    
    /// Propagate event from child to parent
    pub fn propagate_event(&self, child_task_id: &str, event: TaskEvent) {
        if let Some(parent_id) = self.child_to_parent.read().get(child_task_id) {
            // Re-publish event with parent context
            let parent_event = match event {
                TaskEvent::Message { payload, .. } => {
                    // Wrap child message with parent context
                    TaskEvent::Message {
                        payload: (payload.0,),
                        task_id: Some(1), // Placeholder
                    }
                }
                _ => event,
            };
            
            if let Err(e) = global_event_bus().publish(parent_event) {
                warn!("Failed to propagate event to parent {}: {}", parent_id, e);
            }
        }
    }
    
    /// Get all children of a task
    pub fn get_children(&self, parent_task_id: &str) -> Vec<String> {
        self.parent_to_children.read()
            .get(parent_task_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get parent of a task
    pub fn get_parent(&self, child_task_id: &str) -> Option<String> {
        self.child_to_parent.read().get(child_task_id).cloned()
    }
    
    /// Abort all children of a task
    pub async fn abort_children(
        &self,
        parent_task_id: &str,
        task_manager: &TaskManager,
    ) -> Result<()> {
        let child_ids = self.get_children(parent_task_id);
        
        for child_id in child_ids {
            if let Err(e) = task_manager.abort_task(&child_id).await {
                warn!("Failed to abort child task {}: {}", child_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Clean up completed task relationships
    pub fn cleanup(&self, task_id: &str) {
        // Remove from parent map
        let mut parent_map = self.parent_to_children.write();
        parent_map.remove(task_id);
        
        // Remove from child map
        let mut child_map = self.child_to_parent.write();
        if let Some(parent_id) = child_map.remove(task_id) {
            // Remove from parent's children list
            if let Some(children) = parent_map.get_mut(&parent_id) {
                children.retain(|id| id != task_id);
            }
        }
        
        // Remove notifier
        let mut notifiers = self.completion_notifiers.write();
        notifiers.remove(task_id);
        
        debug!("Cleaned up subtask relationships for {}", task_id);
    }
}

impl Default for SubtaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_exact_translation::ExtensionContext;
    use std::path::PathBuf;
    
    fn create_test_options(task_text: &str) -> TaskOptions {
        TaskOptions {
            task: Some(task_text.to_string()),
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
        }
    }
    
    #[test]
    fn test_subtask_manager_creation() {
        let manager = SubtaskManager::new();
        assert!(manager.get_children("nonexistent").is_empty());
    }
    
    #[test]
    fn test_spawn_child() {
        let task_mgr = TaskManager::new();
        let subtask_mgr = SubtaskManager::new();
        
        // Create parent
        let parent_opts = create_test_options("Parent task");
        let parent_id = task_mgr.create_task(parent_opts).unwrap();
        
        // Spawn child
        let child_opts = create_test_options("Child task");
        let child_id = subtask_mgr.spawn_child(&parent_id, child_opts, &task_mgr).unwrap();
        
        // Verify relationship
        let children = subtask_mgr.get_children(&parent_id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], child_id);
        
        let parent = subtask_mgr.get_parent(&child_id);
        assert_eq!(parent, Some(parent_id));
    }
    
    #[tokio::test]
    async fn test_wait_for_children() {
        let task_mgr = TaskManager::new();
        let subtask_mgr = SubtaskManager::new();
        
        let parent_opts = create_test_options("Parent");
        let parent_id = task_mgr.create_task(parent_opts).unwrap();
        
        // Spawn child
        let child_opts = create_test_options("Child");
        let child_id = subtask_mgr.spawn_child(&parent_id, child_opts, &task_mgr).unwrap();
        
        // Notify completion in background
        let mgr_clone = subtask_mgr.clone();
        let child_id_clone = child_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            mgr_clone.notify_completion(&child_id_clone);
        });
        
        // Wait should complete
        let result = subtask_mgr.wait_for_children(&parent_id, &task_mgr).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cleanup() {
        let task_mgr = TaskManager::new();
        let subtask_mgr = SubtaskManager::new();
        
        let parent_opts = create_test_options("Parent");
        let parent_id = task_mgr.create_task(parent_opts).unwrap();
        
        let child_opts = create_test_options("Child");
        let child_id = subtask_mgr.spawn_child(&parent_id, child_opts, &task_mgr).unwrap();
        
        assert_eq!(subtask_mgr.get_children(&parent_id).len(), 1);
        
        subtask_mgr.cleanup(&child_id);
        
        assert!(subtask_mgr.get_children(&parent_id).is_empty());
        assert!(subtask_mgr.get_parent(&child_id).is_none());
    }
    
    #[tokio::test]
    async fn test_abort_children() {
        let task_mgr = TaskManager::new();
        let subtask_mgr = SubtaskManager::new();
        
        let parent_opts = create_test_options("Parent");
        let parent_id = task_mgr.create_task(parent_opts).unwrap();
        
        // Spawn multiple children
        for i in 0..3 {
            let child_opts = create_test_options(&format!("Child {}", i));
            subtask_mgr.spawn_child(&parent_id, child_opts, &task_mgr).unwrap();
        }
        
        // Abort all children
        subtask_mgr.abort_children(&parent_id, &task_mgr).await.unwrap();
        
        // Verify all aborted
        for child_id in subtask_mgr.get_children(&parent_id) {
            if let Some(task) = task_mgr.get_task(&child_id) {
                assert!(task.is_aborted());
            }
        }
    }
}

impl Clone for SubtaskManager {
    fn clone(&self) -> Self {
        Self {
            parent_to_children: self.parent_to_children.clone(),
            child_to_parent: self.child_to_parent.clone(),
            completion_notifiers: self.completion_notifiers.clone(),
        }
    }
}
