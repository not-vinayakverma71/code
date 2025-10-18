// Context Tracker Adapter for tool integration
// Wires context tracking into tool execution flow
// Part of PORT-CT-25: Integrate context-tracking with tools

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::core::context_tracking::{FileContextTracker, RecordSource};

/// Adapter that integrates context tracking with tool execution
pub struct ContextTrackerAdapter {
    tracker: Arc<RwLock<FileContextTracker>>,
}

impl ContextTrackerAdapter {
    pub fn new(tracker: Arc<RwLock<FileContextTracker>>) -> Self {
        Self { tracker }
    }
    
    /// Track file read operation
    pub async fn track_read(&self, file_path: &str) -> Result<(), String> {
        let mut tracker = self.tracker.write().await;
        tracker.track_file_context(file_path.to_string(), RecordSource::ReadTool).await
    }
    
    /// Track file write operation
    pub async fn track_write(&self, file_path: &str) -> Result<(), String> {
        let mut tracker = self.tracker.write().await;
        tracker.track_file_context(file_path.to_string(), RecordSource::WriteTool).await
    }
    
    /// Track diff apply operation
    pub async fn track_diff_apply(&self, file_path: &str) -> Result<(), String> {
        let mut tracker = self.tracker.write().await;
        tracker.track_file_context(file_path.to_string(), RecordSource::DiffApply).await
    }
    
    /// Track file mention (e.g., in search results)
    pub async fn track_mention(&self, file_path: &str) -> Result<(), String> {
        let mut tracker = self.tracker.write().await;
        tracker.track_file_context(file_path.to_string(), RecordSource::Mention).await
    }
    
    /// Mark file as edited by AI (prevents false user-edit detection)
    pub async fn mark_ai_edited(&self, file_path: &str) -> Result<(), String> {
        let mut tracker = self.tracker.write().await;
        tracker.mark_file_as_edited_by_roo(file_path.to_string());
        Ok(())
    }
}

/// Helper to get context tracker from ToolContext adapters
pub fn get_context_tracker(
    context: &crate::core::tools::traits::ToolContext
) -> Option<Arc<ContextTrackerAdapter>> {
    context.adapters
        .get("context_tracker")
        .and_then(|adapter| adapter.clone().downcast::<ContextTrackerAdapter>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_track_read() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            PathBuf::from(temp_dir.path()),
        );
        
        let adapter = ContextTrackerAdapter::new(Arc::new(RwLock::new(tracker)));
        
        let result = adapter.track_read("src/main.rs").await;
        assert!(result.is_ok());
        
        // Verify tracking happened
        let tracker = adapter.tracker.read().await;
        let metadata = tracker.get_task_metadata("test-task").await;
        assert_eq!(metadata.files_in_context.len(), 1);
        assert_eq!(metadata.files_in_context[0].path, "src/main.rs");
    }
    
    #[tokio::test]
    async fn test_track_write() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            PathBuf::from(temp_dir.path()),
        );
        
        let adapter = ContextTrackerAdapter::new(Arc::new(RwLock::new(tracker)));
        
        let result = adapter.track_write("output.txt").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_track_diff_apply() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            PathBuf::from(temp_dir.path()),
        );
        
        let adapter = ContextTrackerAdapter::new(Arc::new(RwLock::new(tracker)));
        
        let result = adapter.track_diff_apply("modified.rs").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_mark_ai_edited() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let tracker = FileContextTracker::new(
            "test-task".to_string(),
            PathBuf::from(temp_dir.path()),
        );
        
        let adapter = ContextTrackerAdapter::new(Arc::new(RwLock::new(tracker)));
        
        // Track write first
        adapter.track_write("generated.rs").await.unwrap();
        
        // Mark as AI-edited
        let result = adapter.mark_ai_edited("generated.rs").await;
        assert!(result.is_ok());
    }
}
