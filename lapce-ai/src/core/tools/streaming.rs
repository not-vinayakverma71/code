// Streaming support for tool execution - P1-4
// Provides progress updates and partial results

use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tracing;

/// Tool execution progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionProgress {
    pub correlation_id: String,
    pub tool_name: String,
    pub progress_type: ProgressType,
    pub message: String,
    pub percentage: Option<u8>,
    pub partial_result: Option<Value>,
    pub timestamp: u64,
}

/// Types of progress updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressType {
    Started,
    Progress,
    PartialResult,
    Completed,
    Failed,
}

/// Command execution status for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionStatus {
    pub correlation_id: String,
    pub command: String,
    pub status: CommandStatus,
    pub stdout_line: Option<String>,
    pub stderr_line: Option<String>,
    pub exit_code: Option<i32>,
    pub timestamp: u64,
}

/// Command execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Running,
    OutputLine,
    ErrorLine,
    Completed,
    Failed,
    Timeout,
}

/// Diff streaming update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStreamUpdate {
    pub correlation_id: String,
    pub file_path: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub diff_lines: Vec<DiffLine>,
    pub is_complete: bool,
}

/// Single diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_number: usize,
    pub operation: DiffOperation,
    pub content: String,
}

/// Diff operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffOperation {
    Add,
    Remove,
    Context,
    Modified,
}

/// Search progress for ripgrep integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProgress {
    pub search_id: String,
    pub files_searched: usize,
    pub matches_found: usize,
    pub current_file: Option<String>,
    pub is_complete: bool,
}

/// Stream emitter for sending updates
pub struct StreamEmitter {
    search_tx: Option<mpsc::UnboundedSender<SearchProgress>>,
}

impl StreamEmitter {
    pub fn new() -> Self {
        Self {
            search_tx: None,
        }
    }
    
    /// Emit a batch of search results
    pub async fn emit_search_batch<T: Serialize>(
        &self,
        search_id: &str,
        batch: Vec<T>,
    ) {
        // In real implementation, this would send to IPC
        // For now, just log internally
        tracing::debug!(
            "Search batch for {}: {} results",
            search_id,
            batch.len()
        );
    }
    
    /// Emit search completion
    pub async fn emit_search_complete(
        &self,
        search_id: &str,
        total_matches: usize,
        files_searched: usize,
        files_with_matches: usize,
        duration: std::time::Duration,
    ) {
        tracing::info!(
            "Search {} complete: {} matches in {} files ({} searched) in {:.2}s",
            search_id,
            total_matches,
            files_with_matches,
            files_searched,
            duration.as_secs_f64()
        );
    }
}

/// Streaming context for tools
pub struct StreamingContext {
    pub progress_tx: Option<mpsc::UnboundedSender<ToolExecutionProgress>>,
    pub command_tx: Option<mpsc::UnboundedSender<CommandExecutionStatus>>,
    pub diff_tx: Option<mpsc::UnboundedSender<DiffStreamUpdate>>,
}

impl StreamingContext {
    pub fn new() -> Self {
        Self {
            progress_tx: None,
            command_tx: None,
            diff_tx: None,
        }
    }
    
    /// Send progress update
    pub async fn send_progress(&self, progress: ToolExecutionProgress) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(progress);
        }
    }
    
    /// Send command status
    pub async fn send_command_status(&self, status: CommandExecutionStatus) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(status);
        }
    }
    
    /// Send diff update
    pub async fn send_diff_update(&self, update: DiffStreamUpdate) {
        if let Some(tx) = &self.diff_tx {
            let _ = tx.send(update);
        }
    }
    
    /// Create progress update
    pub fn create_progress(
        &self,
        correlation_id: String,
        tool_name: String,
        progress_type: ProgressType,
        message: String,
        percentage: Option<u8>,
    ) -> ToolExecutionProgress {
        ToolExecutionProgress {
            correlation_id,
            tool_name,
            progress_type,
            message,
            percentage,
            partial_result: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Trait for tools that support streaming
#[async_trait::async_trait]
pub trait StreamingTool: Send + Sync {
    /// Execute with streaming support
    async fn execute_streaming(
        &self,
        args: Value,
        context: crate::core::tools::traits::ToolContext,
        streaming: StreamingContext,
    ) -> crate::core::tools::traits::ToolResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_streaming_context() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut ctx = StreamingContext::new();
        ctx.progress_tx = Some(tx);
        
        let progress = ctx.create_progress(
            "test-123".to_string(),
            "test_tool".to_string(),
            ProgressType::Started,
            "Starting execution".to_string(),
            Some(0),
        );
        
        ctx.send_progress(progress.clone()).await;
        
        let received = rx.recv().await.unwrap();
        assert_eq!(received.correlation_id, "test-123");
        assert_eq!(received.tool_name, "test_tool");
    }
}
