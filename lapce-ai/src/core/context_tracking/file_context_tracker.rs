//! File Context Tracker
//!
//! Direct 1:1 port from Codex/src/core/context-tracking/FileContextTracker.ts
//! Lines 1-228 complete
//!
//! Tracks file operations to prevent stale context in diff editing.
//! Key logic:
//! - Mark existing entries stale when file is re-read
//! - Track roo_edited/user_edited timestamps
//! - File watchers replaced with IPC event endpoints
//! - Persist task_metadata.json with safe writes

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;

use super::file_context_tracker_types::{
    FileMetadataEntry, RecordSource, RecordState, TaskMetadata,
};

/// Constant for task metadata filename
/// TODO: Move to shared constants
pub const TASK_METADATA_FILENAME: &str = "task_metadata.json";

/// File Context Tracker
///
/// Port of FileContextTracker class from FileContextTracker.ts lines 23-227
///
/// This class is responsible for tracking file operations.
/// If the full contents of a file are passed to Roo via a tool, mention, or edit, the file is marked as active.
/// If a file is modified outside of Roo, we detect and track this change to prevent stale context.
pub struct FileContextTracker {
    pub task_id: String,
    
    // File tracking and watching
    // NOTE: VSCode FileSystemWatcher replaced with IPC event integration
    recently_modified_files: Arc<RwLock<HashSet<String>>>,
    recently_edited_by_roo: Arc<RwLock<HashSet<String>>>,
    checkpoint_possible_files: Arc<RwLock<HashSet<String>>>,
    
    // Storage paths
    task_storage_path: PathBuf,
    workspace_root: Option<PathBuf>,
}

impl FileContextTracker {
    /// Creates a new FileContextTracker
    ///
    /// Port of constructor from FileContextTracker.ts lines 33-36
    ///
    /// # Arguments
    /// * `task_id` - Unique task identifier
    /// * `task_storage_path` - Path to task-specific storage directory
    /// * `workspace_root` - Optional workspace root path
    pub fn new(
        task_id: String,
        task_storage_path: PathBuf,
        workspace_root: Option<PathBuf>,
    ) -> Self {
        Self {
            task_id,
            recently_modified_files: Arc::new(RwLock::new(HashSet::new())),
            recently_edited_by_roo: Arc::new(RwLock::new(HashSet::new())),
            checkpoint_possible_files: Arc::new(RwLock::new(HashSet::new())),
            task_storage_path,
            workspace_root,
        }
    }
    
    /// Gets the current working directory or returns None if it cannot be determined
    ///
    /// Port of getCwd() from FileContextTracker.ts lines 38-45
    fn get_cwd(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }
    
    /// Handles file change event from IPC
    ///
    /// Replaces setupFileWatcher and onDidChange logic from FileContextTracker.ts lines 48-77
    /// In original: VSCode watcher detects changes
    /// In Rust: IPC bridge sends file change events to this method
    ///
    /// # Arguments
    /// * `file_path` - The file that changed (relative to workspace)
    pub async fn on_file_changed(&self, file_path: String) {
        let is_roo_edit = {
            let mut edited_by_roo = self.recently_edited_by_roo.write().unwrap();
            if edited_by_roo.contains(&file_path) {
                edited_by_roo.remove(&file_path);
                true
            } else {
                false
            }
        };
        
        if !is_roo_edit {
            // This was a user edit
            self.recently_modified_files.write().unwrap().insert(file_path.clone());
            
            // Update the task metadata with file tracking
            let _ = self.track_file_context(file_path, RecordSource::UserEdited).await;
        }
    }
    
    /// Tracks a file operation in metadata
    ///
    /// Port of trackFileContext() from FileContextTracker.ts lines 81-95
    ///
    /// This is the main entry point for FileContextTracker and is called when a file is passed
    /// to Roo via a tool, mention, or edit.
    ///
    /// # Arguments
    /// * `file_path` - The file to track (relative to workspace)
    /// * `operation` - The operation that triggered tracking
    pub async fn track_file_context(
        &self,
        file_path: String,
        operation: RecordSource,
    ) -> Result<(), String> {
        if self.get_cwd().is_none() {
            return Err("No workspace root available".to_string());
        }
        
        self.add_file_to_file_context_tracker(&self.task_id, file_path, operation)
            .await
    }
    
    /// Gets task metadata from storage
    ///
    /// Port of getTaskMetadata() from FileContextTracker.ts lines 114-126
    pub async fn get_task_metadata(&self, task_id: &str) -> TaskMetadata {
        let file_path = self.task_storage_path.join(TASK_METADATA_FILENAME);
        
        if fs::metadata(&file_path).await.is_ok() {
            if let Ok(content) = fs::read_to_string(&file_path).await {
                if let Ok(metadata) = serde_json::from_str::<TaskMetadata>(&content) {
                    return metadata;
                }
            }
        }
        
        TaskMetadata::default()
    }
    
    /// Saves task metadata to storage
    ///
    /// Port of saveTaskMetadata() from FileContextTracker.ts lines 129-138
    pub async fn save_task_metadata(
        &self,
        task_id: &str,
        metadata: &TaskMetadata,
    ) -> Result<(), String> {
        let file_path = self.task_storage_path.join(TASK_METADATA_FILENAME);
        
        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create task directory: {}", e))?;
        }
        
        // Safe write with atomic operation
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        
        // Write to temp file first, then rename (atomic on Unix)
        let temp_path = file_path.with_extension("json.tmp");
        fs::write(&temp_path, &json)
            .await
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;
        
        Ok(())
    }
    
    /// Adds a file to the metadata tracker
    ///
    /// Port of addFileToFileContextTracker() from FileContextTracker.ts lines 143-200
    ///
    /// This handles the business logic of determining if the file is new, stale, or active.
    /// It also updates the metadata with the latest read/edit dates.
    pub async fn add_file_to_file_context_tracker(
        &self,
        task_id: &str,
        file_path: String,
        source: RecordSource,
    ) -> Result<(), String> {
        let mut metadata = self.get_task_metadata(task_id).await;
        let now = current_timestamp_ms();
        
        // Mark existing entries for this file as stale
        for entry in metadata.files_in_context.iter_mut() {
            if entry.path == file_path && entry.record_state == RecordState::Active {
                entry.record_state = RecordState::Stale;
            }
        }
        
        // Helper to get the latest date for a specific field and file
        let get_latest_date_for_field = |path: &str, field: &str| -> Option<u64> {
            let mut relevant_entries: Vec<&FileMetadataEntry> = metadata
                .files_in_context
                .iter()
                .filter(|entry| entry.path == path)
                .collect();
            
            relevant_entries.sort_by(|a, b| {
                let a_val = match field {
                    "roo_read_date" => a.roo_read_date.unwrap_or(0),
                    "roo_edit_date" => a.roo_edit_date.unwrap_or(0),
                    "user_edit_date" => a.user_edit_date.unwrap_or(0),
                    _ => 0,
                };
                let b_val = match field {
                    "roo_read_date" => b.roo_read_date.unwrap_or(0),
                    "roo_edit_date" => b.roo_edit_date.unwrap_or(0),
                    "user_edit_date" => b.user_edit_date.unwrap_or(0),
                    _ => 0,
                };
                b_val.cmp(&a_val)
            });
            
            if relevant_entries.is_empty() {
                None
            } else {
                match field {
                    "roo_read_date" => relevant_entries[0].roo_read_date,
                    "roo_edit_date" => relevant_entries[0].roo_edit_date,
                    "user_edit_date" => relevant_entries[0].user_edit_date,
                    _ => None,
                }
            }
        };
        
        let mut new_entry = FileMetadataEntry {
            path: file_path.clone(),
            record_state: RecordState::Active,
            record_source: source,
            roo_read_date: get_latest_date_for_field(&file_path, "roo_read_date"),
            roo_edit_date: get_latest_date_for_field(&file_path, "roo_edit_date"),
            user_edit_date: get_latest_date_for_field(&file_path, "user_edit_date"),
        };
        
        match source {
            // user_edited: The user has edited the file
            RecordSource::UserEdited => {
                new_entry.user_edit_date = Some(now);
                self.recently_modified_files.write().unwrap().insert(file_path.clone());
            }
            
            // roo_edited: Roo has edited the file
            RecordSource::RooEdited => {
                new_entry.roo_read_date = Some(now);
                new_entry.roo_edit_date = Some(now);
                self.checkpoint_possible_files.write().unwrap().insert(file_path.clone());
                self.mark_file_as_edited_by_roo(file_path);
            }
            
            // write_tool: Roo has written to the file via a tool
            RecordSource::WriteTool => {
                new_entry.roo_read_date = Some(now);
                new_entry.roo_edit_date = Some(now);
                self.checkpoint_possible_files.write().unwrap().insert(file_path.clone());
                self.mark_file_as_edited_by_roo(file_path);
            }
            
            // diff_apply: Roo has applied a diff to the file
            RecordSource::DiffApply => {
                new_entry.roo_read_date = Some(now);
                new_entry.roo_edit_date = Some(now);
                self.checkpoint_possible_files.write().unwrap().insert(file_path.clone());
                self.mark_file_as_edited_by_roo(file_path);
            }
            
            // mention: File mentioned in conversation/search results
            RecordSource::Mention => {
                new_entry.roo_read_date = Some(now);
            }
            
            // read_tool/file_mentioned: Roo has read the file via a tool or file mention
            RecordSource::ReadTool | RecordSource::FileMentioned => {
                new_entry.roo_read_date = Some(now);
            }
        }
        
        metadata.files_in_context.push(new_entry);
        self.save_task_metadata(task_id, &metadata).await?;
        
        Ok(())
    }
    
    /// Returns (and then clears) the set of recently modified files
    ///
    /// Port of getAndClearRecentlyModifiedFiles() from FileContextTracker.ts lines 203-207
    pub fn get_and_clear_recently_modified_files(&self) -> Vec<String> {
        let mut files_set = self.recently_modified_files.write().unwrap();
        let files: Vec<String> = files_set.iter().cloned().collect();
        files_set.clear();
        files
    }
    
    /// Returns (and then clears) the set of checkpoint-possible files
    ///
    /// Port of getAndClearCheckpointPossibleFile() from FileContextTracker.ts lines 209-213
    pub fn get_and_clear_checkpoint_possible_files(&self) -> Vec<String> {
        let mut files_set = self.checkpoint_possible_files.write().unwrap();
        let files: Vec<String> = files_set.iter().cloned().collect();
        files_set.clear();
        files
    }
    
    /// Marks a file as edited by Roo to prevent false positives in file watchers
    ///
    /// Port of markFileAsEditedByRoo() from FileContextTracker.ts lines 216-218
    pub fn mark_file_as_edited_by_roo(&self, file_path: String) {
        self.recently_edited_by_roo.write().unwrap().insert(file_path);
    }
}

/// Gets current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_track_file_context_read_tool() {
        let temp_dir = TempDir::new().unwrap();
        let task_storage = temp_dir.path().to_path_buf();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            task_storage.clone(),
            Some(temp_dir.path().to_path_buf()),
        );
        
        // Track a file read
        tracker
            .track_file_context("src/main.rs".to_string(), RecordSource::ReadTool)
            .await
            .unwrap();
        
        // Check metadata
        let metadata = tracker.get_task_metadata("test-task").await;
        assert_eq!(metadata.files_in_context.len(), 1);
        assert_eq!(metadata.files_in_context[0].path, "src/main.rs");
        assert_eq!(metadata.files_in_context[0].record_state, RecordState::Active);
        assert!(metadata.files_in_context[0].roo_read_date.is_some());
    }
    
    #[tokio::test]
    async fn test_track_file_context_marks_previous_stale() {
        let temp_dir = TempDir::new().unwrap();
        let task_storage = temp_dir.path().to_path_buf();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            task_storage.clone(),
            Some(temp_dir.path().to_path_buf()),
        );
        
        // Track a file twice
        tracker
            .track_file_context("src/main.rs".to_string(), RecordSource::ReadTool)
            .await
            .unwrap();
        
        tracker
            .track_file_context("src/main.rs".to_string(), RecordSource::ReadTool)
            .await
            .unwrap();
        
        // Check metadata - should have 2 entries, first one stale
        let metadata = tracker.get_task_metadata("test-task").await;
        assert_eq!(metadata.files_in_context.len(), 2);
        assert_eq!(metadata.files_in_context[0].record_state, RecordState::Stale);
        assert_eq!(metadata.files_in_context[1].record_state, RecordState::Active);
    }
    
    #[tokio::test]
    async fn test_roo_edited_adds_to_checkpoint_files() {
        let temp_dir = TempDir::new().unwrap();
        let task_storage = temp_dir.path().to_path_buf();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            task_storage.clone(),
            Some(temp_dir.path().to_path_buf()),
        );
        
        // Track a Roo edit
        tracker
            .track_file_context("src/main.rs".to_string(), RecordSource::RooEdited)
            .await
            .unwrap();
        
        // Should be in checkpoint_possible_files
        let checkpoint_files = tracker.get_and_clear_checkpoint_possible_files();
        assert_eq!(checkpoint_files.len(), 1);
        assert_eq!(checkpoint_files[0], "src/main.rs");
        
        // Should be cleared after get_and_clear
        let checkpoint_files_2 = tracker.get_and_clear_checkpoint_possible_files();
        assert_eq!(checkpoint_files_2.len(), 0);
    }
    
    #[tokio::test]
    async fn test_file_change_event_user_edited() {
        let temp_dir = TempDir::new().unwrap();
        let task_storage = temp_dir.path().to_path_buf();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            task_storage.clone(),
            Some(temp_dir.path().to_path_buf()),
        );
        
        // Simulate file change event from IPC
        tracker.on_file_changed("src/main.rs".to_string()).await;
        
        // Should be in recently_modified_files
        let modified_files = tracker.get_and_clear_recently_modified_files();
        assert_eq!(modified_files.len(), 1);
        assert_eq!(modified_files[0], "src/main.rs");
    }
    
    #[tokio::test]
    async fn test_file_change_event_roo_edited_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let task_storage = temp_dir.path().to_path_buf();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            task_storage.clone(),
            Some(temp_dir.path().to_path_buf()),
        );
        
        // Mark file as edited by Roo
        tracker.mark_file_as_edited_by_roo("src/main.rs".to_string());
        
        // Simulate file change event
        tracker.on_file_changed("src/main.rs".to_string()).await;
        
        // Should NOT be in recently_modified_files (it was a Roo edit)
        let modified_files = tracker.get_and_clear_recently_modified_files();
        assert_eq!(modified_files.len(), 0);
    }
}
