// Diff adapter for Lapce diff view integration - P0-Adapters

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tokio::sync::mpsc;

/// Diff-related messages for Lapce integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffMessage {
    /// Open diff view with two files
    OpenDiffFiles {
        left_path: PathBuf,
        right_path: PathBuf,
        title: Option<String>,
    },
    
    /// Save diff changes
    DiffSave {
        file_path: PathBuf,
        content: String,
    },
    
    /// Revert diff changes
    DiffRevert {
        file_path: PathBuf,
    },
    
    /// Close diff view
    CloseDiff {
        left_path: PathBuf,
        right_path: PathBuf,
    },
}

/// Diff adapter for Lapce diff view integration
pub struct DiffAdapter {
    /// Channel for sending diff messages
    sender: mpsc::UnboundedSender<DiffMessage>,
    
    /// Workspace root for resolving paths
    workspace: PathBuf,
}

impl DiffAdapter {
    /// Create new diff adapter
    pub fn new(sender: mpsc::UnboundedSender<DiffMessage>, workspace: PathBuf) -> Self {
        Self {
            sender,
            workspace,
        }
    }
    
    /// Open diff view for preview
    pub fn open_diff_preview(
        &self,
        original_path: &PathBuf,
        modified_path: &PathBuf,
        title: Option<String>
    ) -> Result<()> {
        let message = DiffMessage::OpenDiffFiles {
            left_path: self.resolve_path(original_path),
            right_path: self.resolve_path(modified_path),
            title,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Save diff changes
    pub fn save_diff(&self, file_path: &PathBuf, content: &str) -> Result<()> {
        let message = DiffMessage::DiffSave {
            file_path: self.resolve_path(file_path),
            content: content.to_string(),
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Revert diff changes
    pub fn revert_diff(&self, file_path: &PathBuf) -> Result<()> {
        let message = DiffMessage::DiffRevert {
            file_path: self.resolve_path(file_path),
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Close diff view
    pub fn close_diff(&self, left_path: &PathBuf, right_path: &PathBuf) -> Result<()> {
        let message = DiffMessage::CloseDiff {
            left_path: self.resolve_path(left_path),
            right_path: self.resolve_path(right_path),
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Create temporary file for diff preview
    pub fn create_temp_file(&self, original_path: &PathBuf, content: &str) -> Result<PathBuf> {
        // Create temp file in workspace .lapce-ai/temp/ directory
        let temp_dir = self.workspace.join(".lapce-ai").join("temp");
        std::fs::create_dir_all(&temp_dir)?;
        
        // Generate temp file name based on original
        let file_name = original_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unnamed"));
        
        let temp_name = format!(
            "{}.preview.{}",
            uuid::Uuid::new_v4(),
            file_name.to_string_lossy()
        );
        
        let temp_path = temp_dir.join(temp_name);
        std::fs::write(&temp_path, content)?;
        
        Ok(temp_path)
    }
    
    /// Clean up temporary files
    pub fn cleanup_temp_files(&self) -> Result<()> {
        let temp_dir = self.workspace.join(".lapce-ai").join("temp");
        if temp_dir.exists() {
            // Remove old preview files (older than 1 hour)
            let one_hour_ago = std::time::SystemTime::now()
                - std::time::Duration::from_secs(3600);
            
            for entry in std::fs::read_dir(&temp_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified < one_hour_ago {
                            let _ = std::fs::remove_file(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Resolve path relative to workspace
    fn resolve_path(&self, path: &PathBuf) -> PathBuf {
        if path.is_absolute() {
            path.clone()
        } else {
            self.workspace.join(path)
        }
    }
}

/// Helper for managing diff previews
pub struct DiffPreview {
    adapter: DiffAdapter,
    original_path: PathBuf,
    temp_path: Option<PathBuf>,
}

impl DiffPreview {
    /// Create new diff preview
    pub fn new(adapter: DiffAdapter, original_path: PathBuf) -> Self {
        Self {
            adapter,
            original_path,
            temp_path: None,
        }
    }
    
    /// Show preview with modified content
    pub fn show_preview(&mut self, modified_content: &str) -> Result<()> {
        // Create temp file with modified content
        let temp_path = self.adapter.create_temp_file(&self.original_path, modified_content)?;
        
        // Open diff view
        self.adapter.open_diff_preview(
            &self.original_path,
            &temp_path,
            Some(format!("Preview: {}", self.original_path.display()))
        )?;
        
        self.temp_path = Some(temp_path);
        Ok(())
    }
    
    /// Apply changes (save to original file)
    pub fn apply(&self) -> Result<()> {
        if let Some(ref temp_path) = self.temp_path {
            let content = std::fs::read_to_string(temp_path)?;
            self.adapter.save_diff(&self.original_path, &content)?;
        }
        Ok(())
    }
    
    /// Cancel preview (revert and cleanup)
    pub fn cancel(&self) -> Result<()> {
        if let Some(ref temp_path) = self.temp_path {
            self.adapter.close_diff(&self.original_path, temp_path)?;
            let _ = std::fs::remove_file(temp_path);
        }
        Ok(())
    }
}

impl Drop for DiffPreview {
    fn drop(&mut self) {
        // Cleanup temp file on drop
        if let Some(ref temp_path) = self.temp_path {
            let _ = std::fs::remove_file(temp_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_diff_messages() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let temp_dir = TempDir::new().unwrap();
        let adapter = DiffAdapter::new(tx, temp_dir.path().to_path_buf());
        
        // Test open diff
        adapter.open_diff_preview(
            &PathBuf::from("original.txt"),
            &PathBuf::from("modified.txt"),
            Some("Test Diff".to_string())
        ).unwrap();
        
        let msg = rx.recv().await.unwrap();
        match msg {
            DiffMessage::OpenDiffFiles { left_path, right_path, title } => {
                assert!(left_path.ends_with("original.txt"));
                assert!(right_path.ends_with("modified.txt"));
                assert_eq!(title, Some("Test Diff".to_string()));
            }
            _ => panic!("Expected OpenDiffFiles message"),
        }
        
        // Test save diff
        adapter.save_diff(&PathBuf::from("file.txt"), "new content").unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            DiffMessage::DiffSave { file_path, content } => {
                assert!(file_path.ends_with("file.txt"));
                assert_eq!(content, "new content");
            }
            _ => panic!("Expected DiffSave message"),
        }
        
        // Test revert diff
        adapter.revert_diff(&PathBuf::from("file.txt")).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            DiffMessage::DiffRevert { file_path } => {
                assert!(file_path.ends_with("file.txt"));
            }
            _ => panic!("Expected DiffRevert message"),
        }
        
        // Test close diff
        adapter.close_diff(
            &PathBuf::from("left.txt"),
            &PathBuf::from("right.txt")
        ).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            DiffMessage::CloseDiff { left_path, right_path } => {
                assert!(left_path.ends_with("left.txt"));
                assert!(right_path.ends_with("right.txt"));
            }
            _ => panic!("Expected CloseDiff message"),
        }
    }
    
    #[test]
    fn test_temp_file_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let temp_dir = TempDir::new().unwrap();
        let adapter = DiffAdapter::new(tx, temp_dir.path().to_path_buf());
        
        let original_path = PathBuf::from("test.rs");
        let content = "fn main() { println!(\"Hello\"); }";
        
        let temp_path = adapter.create_temp_file(&original_path, content).unwrap();
        
        assert!(temp_path.exists());
        assert!(temp_path.to_string_lossy().contains("preview"));
        assert!(temp_path.to_string_lossy().contains("test.rs"));
        
        let read_content = std::fs::read_to_string(&temp_path).unwrap();
        assert_eq!(read_content, content);
        
        // Cleanup
        std::fs::remove_file(temp_path).unwrap();
    }
    
    #[test]
    fn test_diff_preview() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let temp_dir = TempDir::new().unwrap();
        let adapter = DiffAdapter::new(tx, temp_dir.path().to_path_buf());
        
        // Create original file
        let original_path = temp_dir.path().join("original.txt");
        std::fs::write(&original_path, "original content").unwrap();
        
        let mut preview = DiffPreview::new(adapter, original_path.clone());
        
        // Show preview
        preview.show_preview("modified content").unwrap();
        assert!(preview.temp_path.is_some());
        
        let temp_path = preview.temp_path.as_ref().unwrap();
        assert!(temp_path.exists());
        
        let content = std::fs::read_to_string(temp_path).unwrap();
        assert_eq!(content, "modified content");
    }
    
    #[test]
    fn test_cleanup_old_files() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let temp_dir = TempDir::new().unwrap();
        let adapter = DiffAdapter::new(tx, temp_dir.path().to_path_buf());
        
        // Create temp directory
        let temp_files_dir = temp_dir.path().join(".lapce-ai").join("temp");
        std::fs::create_dir_all(&temp_files_dir).unwrap();
        
        // Create an old file
        let old_file = temp_files_dir.join("old.preview.txt");
        std::fs::write(&old_file, "old content").unwrap();
        
        // Set its modification time to 2 hours ago
        let two_hours_ago = std::time::SystemTime::now()
            - std::time::Duration::from_secs(7200);
        filetime::set_file_mtime(
            &old_file,
            filetime::FileTime::from_system_time(two_hours_ago)
        ).unwrap();
        
        // Create a new file
        let new_file = temp_files_dir.join("new.preview.txt");
        std::fs::write(&new_file, "new content").unwrap();
        
        // Run cleanup
        adapter.cleanup_temp_files().unwrap();
        
        // Old file should be removed, new file should remain
        assert!(!old_file.exists());
        assert!(new_file.exists());
    }
}
