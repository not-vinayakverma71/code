// Task Persistence - CHUNK-03: T09
// Save/load task state and conversation history to disk

use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn};

use crate::ipc_messages::ClineMessage;
use crate::task_exact_translation::{ApiMessage, TaskMetadata, TodoItem};

/// Version for persistence format
const PERSISTENCE_VERSION: u32 = 1;

/// Persisted task state (versioned)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTaskState {
    /// Version of persistence format
    pub version: u32,
    
    /// Task ID
    pub task_id: String,
    
    /// Task metadata
    pub metadata: TaskMetadata,
    
    /// Conversation messages
    pub cline_messages: Vec<ClineMessage>,
    
    /// API conversation history
    pub api_messages: Vec<ApiMessage>,
    
    /// Last message timestamp
    pub last_message_ts: Option<u64>,
    
    /// Task status flags
    pub is_aborted: bool,
    pub is_paused: bool,
    pub is_initialized: bool,
    pub is_abandoned: bool,
    
    /// Todo list
    pub todo_list: Option<Vec<TodoItem>>,
    
    /// Consecutive mistake count
    pub consecutive_mistakes: u32,
    
    /// Per-file mistake counts
    pub tool_mistakes: std::collections::HashMap<String, u32>,
    
    /// Tool usage statistics
    pub tool_usage: std::collections::HashMap<String, u32>,
    
    /// Task mode
    pub task_mode: Option<String>,
    
    /// Timestamp when saved
    pub saved_at: u64,
}

/// Persistence manager for tasks
pub struct TaskPersistence {
    /// Base storage directory
    storage_path: PathBuf,
}

impl TaskPersistence {
    /// Create a new persistence manager
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        // Ensure storage directory exists
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path)
                .context("Failed to create storage directory")?;
        }
        
        Ok(Self { storage_path })
    }
    
    /// Get file path for a task
    fn get_task_file_path(&self, task_id: &str) -> PathBuf {
        self.storage_path.join(format!("task_{}.json", task_id))
    }
    
    /// Get snapshot file path (for crash recovery)
    fn get_snapshot_path(&self) -> PathBuf {
        self.storage_path.join("active_tasks_snapshot.json")
    }
    
    /// Save task state to disk
    pub fn save_task(&self, state: &PersistedTaskState) -> Result<()> {
        let file_path = self.get_task_file_path(&state.task_id);
        
        // Serialize to JSON with pretty printing for debugging
        let json = serde_json::to_string_pretty(state)
            .context("Failed to serialize task state")?;
        
        // Write atomically using temp file + rename
        let temp_path = file_path.with_extension("json.tmp");
        fs::write(&temp_path, json)
            .context("Failed to write task state to temp file")?;
        
        fs::rename(&temp_path, &file_path)
            .context("Failed to rename temp file to final path")?;
        
        info!("Saved task state: {}", state.task_id);
        Ok(())
    }
    
    /// Load task state from disk
    pub fn load_task(&self, task_id: &str) -> Result<PersistedTaskState> {
        let file_path = self.get_task_file_path(task_id);
        
        if !file_path.exists() {
            anyhow::bail!("Task state file not found: {}", task_id);
        }
        
        let json = fs::read_to_string(&file_path)
            .context("Failed to read task state file")?;
        
        let state: PersistedTaskState = serde_json::from_str(&json)
            .context("Failed to deserialize task state")?;
        
        // Version compatibility check
        if state.version > PERSISTENCE_VERSION {
            warn!(
                "Task state version {} is newer than supported version {}",
                state.version, PERSISTENCE_VERSION
            );
        }
        
        info!("Loaded task state: {}", task_id);
        Ok(state)
    }
    
    /// Delete task state file
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let file_path = self.get_task_file_path(task_id);
        
        if file_path.exists() {
            fs::remove_file(&file_path)
                .context("Failed to delete task state file")?;
            info!("Deleted task state: {}", task_id);
        }
        
        Ok(())
    }
    
    /// Check if task state exists
    pub fn task_exists(&self, task_id: &str) -> bool {
        self.get_task_file_path(task_id).exists()
    }
    
    /// List all persisted task IDs
    pub fn list_tasks(&self) -> Result<Vec<String>> {
        let mut task_ids = Vec::new();
        
        if !self.storage_path.exists() {
            return Ok(task_ids);
        }
        
        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy();
                    if name.starts_with("task_") && name.ends_with(".json") {
                        // Extract task ID from filename
                        let task_id = name.trim_start_matches("task_")
                            .trim_end_matches(".json")
                            .to_string();
                        task_ids.push(task_id);
                    }
                }
            }
        }
        
        Ok(task_ids)
    }
    
    /// Save snapshot of active task IDs for crash recovery
    pub fn save_snapshot(&self, active_task_ids: &[String]) -> Result<()> {
        let snapshot_path = self.get_snapshot_path();
        
        #[derive(Serialize)]
        struct Snapshot {
            version: u32,
            timestamp: u64,
            active_tasks: Vec<String>,
        }
        
        let snapshot = Snapshot {
            version: PERSISTENCE_VERSION,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            active_tasks: active_task_ids.to_vec(),
        };
        
        let json = serde_json::to_string_pretty(&snapshot)?;
        
        // Atomic write
        let temp_path = snapshot_path.with_extension("json.tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &snapshot_path)?;
        
        info!("Saved snapshot with {} active tasks", active_task_ids.len());
        Ok(())
    }
    
    /// Load snapshot of active task IDs
    pub fn load_snapshot(&self) -> Result<Vec<String>> {
        let snapshot_path = self.get_snapshot_path();
        
        if !snapshot_path.exists() {
            return Ok(Vec::new());
        }
        
        #[derive(Deserialize)]
        struct Snapshot {
            version: u32,
            active_tasks: Vec<String>,
        }
        
        let json = fs::read_to_string(&snapshot_path)?;
        let snapshot: Snapshot = serde_json::from_str(&json)?;
        
        info!("Loaded snapshot with {} active tasks", snapshot.active_tasks.len());
        Ok(snapshot.active_tasks)
    }
    
    /// Clean up old task files (older than threshold)
    pub fn cleanup_old_tasks(&self, threshold_days: u64) -> Result<usize> {
        let threshold_secs = threshold_days * 24 * 60 * 60;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut cleaned = 0;
        
        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_secs = modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        
                        if now - modified_secs > threshold_secs {
                            if let Err(e) = fs::remove_file(&path) {
                                warn!("Failed to delete old task file: {}", e);
                            } else {
                                cleaned += 1;
                            }
                        }
                    }
                }
            }
        }
        
        info!("Cleaned up {} old task files", cleaned);
        Ok(cleaned)
    }
}

/// Helper to convert Task to PersistedTaskState
pub fn task_to_persisted_state(task: &crate::task_exact_translation::Task) -> PersistedTaskState {
    PersistedTaskState {
        version: PERSISTENCE_VERSION,
        task_id: task.task_id.clone(),
        metadata: task.metadata.clone(),
        cline_messages: task.get_messages(),
        api_messages: task.get_api_conversation(),
        last_message_ts: task.get_last_message_ts(),
        is_aborted: task.is_aborted(),
        is_paused: task.is_paused(),
        is_initialized: task.is_initialized(),
        is_abandoned: task.is_abandoned(),
        todo_list: task.todo_list.clone(),
        consecutive_mistakes: task.get_consecutive_mistakes(),
        tool_mistakes: {
            let map = std::collections::HashMap::new();
            // Would iterate through all known file paths, but we don't track them
            // This is a limitation - in real impl, we'd need to store the keys
            map
        },
        tool_usage: task.get_all_tool_usage().usage_count,
        task_mode: None, // Would need async access
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_state() -> PersistedTaskState {
        PersistedTaskState {
            version: PERSISTENCE_VERSION,
            task_id: "test-task-123".to_string(),
            metadata: TaskMetadata {
                task: Some("Test task".to_string()),
                images: None,
            },
            cline_messages: vec![],
            api_messages: vec![],
            last_message_ts: Some(1234567890),
            is_aborted: false,
            is_paused: false,
            is_initialized: true,
            is_abandoned: false,
            todo_list: None,
            consecutive_mistakes: 0,
            tool_mistakes: std::collections::HashMap::new(),
            tool_usage: std::collections::HashMap::new(),
            task_mode: Some("default".to_string()),
            saved_at: 1234567890,
        }
    }
    
    #[test]
    fn test_persistence_creation() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(temp_dir.path().exists());
    }
    
    #[test]
    fn test_save_and_load_task() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        let state = create_test_state();
        persistence.save_task(&state).unwrap();
        
        let loaded = persistence.load_task("test-task-123").unwrap();
        assert_eq!(loaded.task_id, state.task_id);
        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.is_initialized, state.is_initialized);
    }
    
    #[test]
    fn test_task_exists() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        assert!(!persistence.task_exists("nonexistent"));
        
        let state = create_test_state();
        persistence.save_task(&state).unwrap();
        
        assert!(persistence.task_exists("test-task-123"));
    }
    
    #[test]
    fn test_delete_task() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        let state = create_test_state();
        persistence.save_task(&state).unwrap();
        assert!(persistence.task_exists("test-task-123"));
        
        persistence.delete_task("test-task-123").unwrap();
        assert!(!persistence.task_exists("test-task-123"));
    }
    
    #[test]
    fn test_list_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Save multiple tasks
        for i in 0..5 {
            let mut state = create_test_state();
            state.task_id = format!("task-{}", i);
            persistence.save_task(&state).unwrap();
        }
        
        let tasks = persistence.list_tasks().unwrap();
        assert_eq!(tasks.len(), 5);
        assert!(tasks.contains(&"task-0".to_string()));
        assert!(tasks.contains(&"task-4".to_string()));
    }
    
    #[test]
    fn test_save_and_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        let active_tasks = vec![
            "task-1".to_string(),
            "task-2".to_string(),
            "task-3".to_string(),
        ];
        
        persistence.save_snapshot(&active_tasks).unwrap();
        
        let loaded = persistence.load_snapshot().unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded, active_tasks);
    }
    
    #[test]
    fn test_roundtrip_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TaskPersistence::new(temp_dir.path().to_path_buf()).unwrap();
        
        let mut state = create_test_state();
        state.consecutive_mistakes = 5;
        state.tool_usage.insert("readFile".to_string(), 10);
        state.tool_usage.insert("writeFile".to_string(), 3);
        
        persistence.save_task(&state).unwrap();
        let loaded = persistence.load_task(&state.task_id).unwrap();
        
        assert_eq!(loaded.consecutive_mistakes, 5);
        assert_eq!(loaded.tool_usage.get("readFile"), Some(&10));
        assert_eq!(loaded.tool_usage.get("writeFile"), Some(&3));
    }
}
