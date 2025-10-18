// Diff view streaming for UI integration - P1-8
// Provides real-time diff updates to the UI

use std::sync::Arc;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::core::tools::streaming::{DiffStreamUpdate, DiffLine, DiffOperation};

/// Diff view manager for streaming updates
pub struct DiffViewManager {
    tx: mpsc::UnboundedSender<DiffStreamUpdate>,
    rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<DiffStreamUpdate>>>,
}

impl DiffViewManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }
    
    /// Stream a diff file
    pub async fn stream_diff(
        &self,
        correlation_id: String,
        file_path: String,
        diff_content: &str,
    ) -> Result<()> {
        let lines: Vec<&str> = diff_content.lines().collect();
        let total_chunks = (lines.len() + 49) / 50; // 50 lines per chunk
        
        for (chunk_index, chunk) in lines.chunks(50).enumerate() {
            let diff_lines: Vec<DiffLine> = chunk.iter().enumerate().map(|(i, line)| {
                let line_number = chunk_index * 50 + i + 1;
                let operation = if line.starts_with('+') {
                    DiffOperation::Add
                } else if line.starts_with('-') {
                    DiffOperation::Remove
                } else if line.starts_with('@') {
                    DiffOperation::Modified
                } else {
                    DiffOperation::Context
                };
                
                DiffLine {
                    line_number,
                    operation,
                    content: line.to_string(),
                }
            }).collect();
            
            let update = DiffStreamUpdate {
                correlation_id: correlation_id.clone(),
                file_path: file_path.clone(),
                chunk_index,
                total_chunks,
                diff_lines,
                is_complete: chunk_index == total_chunks - 1,
            };
            
            self.tx.send(update)?;
            
            // Small delay to simulate streaming
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        
        Ok(())
    }
    
    /// Get receiver for UI consumption
    pub fn get_receiver(&self) -> Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<DiffStreamUpdate>>> {
        self.rx.clone()
    }
}

/// Diff view state for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffViewState {
    pub file_path: String,
    pub original_content: Vec<String>,
    pub modified_content: Vec<String>,
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
}

/// Diff hunk representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub original_start: usize,
    pub original_count: usize,
    pub modified_start: usize,
    pub modified_count: usize,
    pub lines: Vec<DiffLine>,
}

/// Diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
    pub files_changed: usize,
}

impl DiffStats {
    pub fn from_lines(lines: &[DiffLine]) -> Self {
        let mut stats = Self {
            additions: 0,
            deletions: 0,
            modifications: 0,
            files_changed: 1,
        };
        
        for line in lines {
            match line.operation {
                DiffOperation::Add => stats.additions += 1,
                DiffOperation::Remove => stats.deletions += 1,
                DiffOperation::Modified => stats.modifications += 1,
                _ => {}
            }
        }
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_diff_streaming() {
        let manager = DiffViewManager::new();
        
        let diff_content = r#"
@@ -1,3 +1,3 @@
 line 1
-line 2
+line 2 modified
 line 3
"#;
        
        let result = manager.stream_diff(
            "test-123".to_string(),
            "test.txt".to_string(),
            diff_content,
        ).await;
        
        assert!(result.is_ok());
    }
}
