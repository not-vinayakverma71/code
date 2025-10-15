// Task Manager - Engine-only orchestration (CHUNK-03: T03)
// Manages task lifecycle without IPC bridge dependency

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use anyhow::{Result, bail};
use tracing::{info, warn, error, debug};

use crate::task_exact_translation::{Task, TaskOptions, TaskStatus};
use crate::events_exact_translation::{TaskEvent, TaskEventBus, global_event_bus};

/// Task Manager for coordinating multiple tasks
/// Provides create/start/abort/list operations without IPC
pub struct TaskManager {
    /// Active tasks indexed by task_id
    tasks: Arc<RwLock<HashMap<String, Arc<Task>>>>,
    
    /// Event bus for publishing task events
    event_bus: TaskEventBus,
    
    /// Counter for generating task numbers
    task_counter: Arc<RwLock<i32>>,
}

impl TaskManager {
    /// Create a new TaskManager
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            event_bus: global_event_bus().clone(),
            task_counter: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Create a new task with given options
    /// Returns task_id on success
    pub async fn create_task(&self, mut options: TaskOptions) -> Result<String> {
        // Assign task number if not provided
        if options.task_number.is_none() {
            let mut counter = self.task_counter.write().await;
            *counter += 1;
            options.task_number = Some(*counter);
        }
        
        // Create the task
        let task = Task::new(options);
        let task_id = task.task_id.clone();
        
        // Store in active tasks
        {
            let mut tasks = self.tasks.write().await;
            if tasks.contains_key(&task_id) {
                bail!("Task with ID {} already exists", task_id);
            }
            tasks.insert(task_id.clone(), task.clone());
        }
        
        // Publish TaskCreated event
        let event = TaskEvent::TaskCreated {
            payload: (task_id.clone(),),
            task_id: None,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish TaskCreated event: {}", e);
        }
        
        info!("Task created: {}", task_id);
        Ok(task_id)
    }
    
    /// Start a task by ID
    /// This would normally trigger the orchestration loop
    /// For now, just updates state and publishes event
    pub async fn start_task(&self, task_id: &str) -> Result<()> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
        };
        
        // Mark as initialized (engine-only marker)
        {
            let mut is_init = task.is_initialized.write().await;
            *is_init = true;
        }
        
        // Publish TaskStarted event
        let event = TaskEvent::TaskStarted {
            payload: (task_id.to_string(),),
            task_id: None,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish TaskStarted event: {}", e);
        }
        
        info!("Task started: {}", task_id);
        Ok(())
    }
    
    /// Abort a task by ID
    /// Sets abort flag and publishes event
    pub async fn abort_task(&self, task_id: &str) -> Result<()> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
        };
        
        // Set abort flag
        {
            let mut abort = task.abort.write().await;
            if *abort {
                // Already aborted, idempotent
                debug!("Task {} already aborted", task_id);
                return Ok(());
            }
            *abort = true;
        }
        
        // Publish TaskAborted event
        let event = TaskEvent::TaskAborted {
            payload: (task_id.to_string(),),
            task_id: None,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish TaskAborted event: {}", e);
        }
        
        info!("Task aborted: {}", task_id);
        Ok(())
    }
    
    /// Pause a task by ID
    pub async fn pause_task(&self, task_id: &str) -> Result<()> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
        };
        
        // Set pause flag
        {
            let mut is_paused = task.is_paused.write().await;
            if *is_paused {
                debug!("Task {} already paused", task_id);
                return Ok(());
            }
            *is_paused = true;
        }
        
        // Publish TaskPaused event
        let event = TaskEvent::TaskPaused {
            payload: (task_id.to_string(),),
            task_id: None,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish TaskPaused event: {}", e);
        }
        
        info!("Task paused: {}", task_id);
        Ok(())
    }
    
    /// Resume a paused task
    pub async fn resume_task(&self, task_id: &str) -> Result<()> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?
        };
        
        // Clear pause flag
        {
            let mut is_paused = task.is_paused.write().await;
            if !*is_paused {
                debug!("Task {} not paused", task_id);
                return Ok(());
            }
            *is_paused = false;
        }
        
        // Publish TaskUnpaused event
        let event = TaskEvent::TaskUnpaused {
            payload: (task_id.to_string(),),
            task_id: None,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish TaskUnpaused event: {}", e);
        }
        
        info!("Task resumed: {}", task_id);
        Ok(())
    }
    
    /// Check if a task exists by ID
    pub async fn has_task(&self, task_id: &str) -> bool {
        self.tasks.read().await.contains_key(task_id)
    }
    
    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Arc<Task>> {
        self.tasks.read().await.get(task_id).cloned()
    }
    
    pub fn get_task_blocking(&self, task_id: &str) -> Option<Arc<Task>> {
        futures::executor::block_on(async {
            self.tasks.read().await.get(task_id).cloned()
        })
    }
    
    /// List all active tasks
    pub async fn list_tasks(&self) -> Vec<String> {
        self.tasks.read().await.keys().cloned().collect()
    }
    
    /// Get task count
    pub async fn task_count(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.len()
    }
    
    /// Subscribe to task events
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_bus.subscribe()
    }
    
    /// Remove a completed/aborted task from tracking
    pub async fn cleanup_task(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        if tasks.remove(task_id).is_some() {
            self.event_bus.cleanup_task(task_id);
            debug!("Cleaned up task: {}", task_id);
        }
    }
    
    /// Determine task status (engine-only, based on flags)
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let task = self.get_task(task_id).await?;
        
        // Check abort flag
        if *task.abort.read().await {
            return Some(TaskStatus::Aborted);
        }
        
        // Check pause flag
        if *task.is_paused.read().await {
            return Some(TaskStatus::Paused);
        }
        
        // Check if waiting for response
        if task.idle_ask.read().await.is_some() {
            return Some(TaskStatus::Idle);
        }
        
        if task.resumable_ask.read().await.is_some() {
            return Some(TaskStatus::Resumable);
        }
        
        if task.interactive_ask.read().await.is_some() {
            return Some(TaskStatus::Interactive);
        }
        
        // Check if initialized
        if *task.is_initialized.read().await {
            return Some(TaskStatus::Active);
        }
        
        // Default to idle
        Some(TaskStatus::Idle)
    }
    
    // ========================================================================
    // CHUNK-03 T10: Crash Recovery
    // ========================================================================
    
    /// Restore tasks from disk snapshot (crash recovery)
    /// This is idempotent - can be called multiple times safely
    pub async fn restore_from_snapshot(
        &self,
        persistence: &crate::task_persistence::TaskPersistence,
    ) -> Result<Vec<String>> {
        
        
        // Load snapshot of active task IDs
        let task_ids = persistence.load_snapshot()
            .unwrap_or_else(|e| {
                warn!("Failed to load snapshot: {}", e);
                Vec::new()
            });
        
        if task_ids.is_empty() {
            info!("No tasks to restore from snapshot");
            return Ok(Vec::new());
        }
        
        info!("Attempting to restore {} tasks from snapshot", task_ids.len());
        let mut restored = Vec::new();
        
        for task_id in task_ids {
            // Skip if already loaded (idempotency)
            if self.get_task(&task_id).await.is_some() {
                info!("Task {} already loaded, skipping", task_id);
                continue;
            }
            
            // Load persisted state
            match persistence.load_task(&task_id) {
                Ok(state) => {
                    match self.restore_task_from_state(state).await {
                        Ok(id) => {
                            restored.push(id);
                        }
                        Err(e) => {
                            error!("Failed to restore task {}: {}", task_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to load task state {}: {}", task_id, e);
                }
            }
        }
        
        info!("Restored {} tasks from snapshot", restored.len());
        Ok(restored)
    }
    
    /// Restore a single task from persisted state
    async fn restore_task_from_state(
        &self,
        state: crate::task_persistence::PersistedTaskState,
    ) -> Result<String> {
        use crate::task_exact_translation::{ExtensionContext, HistoryItem};
        use std::path::PathBuf;
        
        // Create task options from persisted state
        let history_item = HistoryItem {
            id: state.task_id.clone(),
            task: state.metadata.task.clone(),
            ts: state.saved_at as i64,
            is_favorited: None,
        };
        
        let options = TaskOptions {
            task: None,
            assistant_message_info: None,
            assistant_metadata: None,
            custom_variables: None,
            images: state.metadata.images.clone(),
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
            initial_todos: state.todo_list.clone(),
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
            history_item: Some(history_item),
        };
        
        // Create task
        let task = Task::new(options);
        let task_id = task.task_id.clone();
        
        // Restore state from persisted data
        self.restore_task_state(&task, &state).await;
        
        // Register existing task in manager
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }
        
        info!("Restored task: {}", task_id);
        Ok(task_id)
    }
    
    /// Restore task internal state from persisted state
    async fn restore_task_state(
        &self,
        task: &Arc<Task>,
        state: &crate::task_persistence::PersistedTaskState,
    ) {
        // Restore flags
        if state.is_aborted {
            task.request_abort().await;
        }
        if state.is_paused {
            let _ = task.pause().await;
        }
        if state.is_initialized {
            task.mark_initialized();
        }
        if state.is_abandoned {
            task.mark_abandoned();
        }
        
        // Restore messages
        {
            let mut messages = task.cline_messages.blocking_write();
            *messages = state.cline_messages.clone();
        }
        
        // Restore API conversation
        {
            let mut api_msgs = task.api_conversation_history.blocking_write();
            *api_msgs = state.api_messages.clone();
        }
        
        // Restore last message timestamp
        {
            let mut last_ts = task.last_message_ts.blocking_write();
            *last_ts = state.last_message_ts;
        }
        
        // Restore mistake tracking
        {
            let mut mistakes = task.consecutive_mistake_count.blocking_write();
            *mistakes = state.consecutive_mistakes;
        }
        
        {
            let mut tool_mistakes = task.consecutive_mistake_count_for_apply_diff.blocking_write();
            *tool_mistakes = state.tool_mistakes.clone();
        }
        
        // Restore tool usage
        {
            let mut usage = task.tool_usage.blocking_write();
            usage.usage_count = state.tool_usage.clone();
        }
        
        // Restore task mode if present
        if let Some(mode) = &state.task_mode {
            task.set_task_mode(mode.clone()).await;
        }
    }
    
    /// Save current snapshot of active tasks
    pub async fn save_snapshot(
        &self,
        persistence: &crate::task_persistence::TaskPersistence,
    ) -> Result<()> {
        let task_ids = self.list_tasks().await;
        persistence.save_snapshot(&task_ids)
    }
    
    /// Save all active tasks to disk
    pub async fn save_all_tasks(
        &self,
        persistence: &crate::task_persistence::TaskPersistence,
    ) -> Result<()> {
        let tasks_guard = self.tasks.read().await;
        for (task_id, task) in tasks_guard.iter() {
            let state = crate::task_persistence::task_to_persisted_state(task);
            if let Err(e) = persistence.save_task(&state) {
                error!("Failed to save task {}: {}", task_id, e);
            }
        }
        
        Ok(())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_exact_translation::ExtensionContext;
    use std::path::PathBuf;
    
    fn create_test_options(task_text: Option<String>) -> TaskOptions {
        TaskOptions {
            task: task_text,
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
                global_storage_uri: PathBuf::from("/tmp/test"),
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
    
    #[tokio::test]
    async fn test_task_manager_creation() {
        let manager = TaskManager::new();
        assert_eq!(manager.task_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_create_task() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        assert!(!task_id.is_empty());
        assert_eq!(manager.task_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_create_duplicate_task_fails() {
        let manager = TaskManager::new();
        
        // Create first task with history_item to control ID
        use crate::task_exact_translation::HistoryItem;
        let mut options = create_test_options(None);
        options.history_item = Some(HistoryItem {
            id: "fixed-id".to_string(),
            task: Some("Test".to_string()),
            ts: 0,
            is_favorited: None,
        });
        
        manager.create_task(options.clone()).await.unwrap();
        
        // Try to create again with same ID
        let result = manager.create_task(options).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_start_task() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        manager.start_task(&task_id).await.unwrap();
        
        // Verify task is marked as initialized
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(*task.is_initialized.read().await);
    }
    
    #[tokio::test]
    async fn test_abort_task() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        manager.abort_task(&task_id).await.unwrap();
        
        // Verify abort flag is set
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(*task.abort.read().await);
    }
    
    #[tokio::test]
    async fn test_abort_task_idempotent() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        
        // Abort twice
        manager.abort_task(&task_id).await.unwrap();
        manager.abort_task(&task_id).await.unwrap();
        
        // Should succeed both times
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(*task.abort.read().await);
    }
    
    #[tokio::test]
    async fn test_pause_resume_task() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        
        // Pause
        manager.pause_task(&task_id).await.unwrap();
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(*task.is_paused.read().await);
        
        // Resume
        manager.resume_task(&task_id).await.unwrap();
        assert!(!*task.is_paused.read().await);
    }
    
    #[tokio::test]
    async fn test_list_tasks() {
        let manager = TaskManager::new();
        
        // Create multiple tasks
        for i in 0..5 {
            let options = create_test_options(Some(format!("Task {}", i)));
            manager.create_task(options).await.unwrap();
        }
        
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 5);
    }
    
    #[tokio::test]
    async fn test_cleanup_task() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        assert_eq!(manager.task_count().await, 1);
        
        manager.cleanup_task(&task_id).await;
        assert_eq!(manager.task_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let manager = TaskManager::new();
        let mut rx = manager.subscribe();
        
        let options = create_test_options(Some("Test task".to_string()));
        let task_id = manager.create_task(options).await.unwrap();
        
        // Should receive TaskCreated event
        let event = rx.recv().await.unwrap();
        match event {
            TaskEvent::TaskCreated { payload, .. } => {
                assert_eq!(payload.0, task_id);
            }
            _ => panic!("Expected TaskCreated event"),
        }
    }
    
    #[tokio::test]
    async fn test_get_task_status() {
        let manager = TaskManager::new();
        let options = create_test_options(Some("Test task".to_string()));
        
        let task_id = manager.create_task(options).await.unwrap();
        
        // Initial status should be Idle
        let status = manager.get_task_status(&task_id).await.unwrap();
        assert_eq!(status, TaskStatus::Idle);
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        let manager = Arc::new(TaskManager::new());
        let mut handles = vec![];
        
        // Create tasks concurrently
        for i in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                let options = create_test_options(Some(format!("Concurrent task {}", i)));
                manager_clone.create_task(options).await
            });
            handles.push(handle);
        }
        
        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // All tasks should be created
        assert_eq!(manager.task_count().await, 10);
    }
}
