// Unified Streaming Emitter System - Production-grade with backpressure
// Part of Streaming emitter TODO #8 - pre-IPC

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tokio::sync::{mpsc, broadcast};
use tokio::time::{Duration, Instant};
use futures::stream::Stream;

// Streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    ToolExecutionProgress(ToolExecutionProgress),
    CommandExecutionStatus(CommandExecutionStatus),
    DiffStreamUpdate(DiffStreamUpdate),
    SearchProgress(SearchProgress),
    FileProgress(FileProgress),
    LogMessage(LogMessage),
    Error(ErrorEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionProgress {
    pub tool_name: String,
    pub correlation_id: String,
    pub phase: ExecutionPhase,
    pub progress: f32, // 0.0 to 1.0
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub elapsed_ms: u64,
    pub estimated_remaining_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPhase {
    Initializing,
    Validating,
    Executing,
    Processing,
    Finalizing,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionStatus {
    pub command: String,
    pub correlation_id: String,
    pub status: CommandStatus,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: Option<i32>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandStatus {
    Started,
    Running,
    OutputReceived,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStreamUpdate {
    pub correlation_id: String,
    pub file_path: String,
    pub hunk_index: usize,
    pub total_hunks: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub status: DiffStatus,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffStatus {
    Analyzing,
    ApplyingHunk,
    HunkApplied,
    HunkFailed,
    Complete,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProgress {
    pub correlation_id: String,
    pub query: String,
    pub files_searched: usize,
    pub matches_found: usize,
    pub current_file: Option<String>,
    pub progress: f32,
    pub batch: Option<Vec<SearchMatch>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub file_path: String,
    pub line_number: usize,
    pub column: usize,
    pub match_text: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProgress {
    pub correlation_id: String,
    pub file_path: String,
    pub operation: FileOperation,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    Reading,
    Writing,
    Copying,
    Moving,
    Deleting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_name: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub error: String,
    pub details: Option<String>,
    pub tool_name: String,
    pub correlation_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub buffer_size: usize,
    pub high_watermark: usize,
    pub low_watermark: usize,
    pub drop_policy: DropPolicy,
}

#[derive(Debug, Clone)]
pub enum DropPolicy {
    DropOldest,
    DropNewest,
    Block,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            high_watermark: 800,
            low_watermark: 200,
            drop_policy: DropPolicy::DropOldest,
        }
    }
}

// Unified streaming emitter
pub struct UnifiedStreamEmitter {
    // Broadcast channels for different event types
    tool_tx: broadcast::Sender<StreamEvent>,
    
    // MPSC channels for internal use
    internal_tx: mpsc::UnboundedSender<StreamEvent>,
    
    // Subscribers tracking
    subscribers: Arc<RwLock<HashMap<String, SubscriberInfo>>>,
    
    // Backpressure management
    backpressure: Arc<RwLock<BackpressureState>>,
    
    config: BackpressureConfig,
}

#[derive(Debug)]
struct SubscriberInfo {
    id: String,
    filter: Option<EventFilter>,
    created_at: Instant,
    events_received: u64,
}

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub tool_names: Option<Vec<String>>,
    pub event_types: Option<Vec<String>>,
    pub correlation_ids: Option<Vec<String>>,
}

#[derive(Debug)]
struct BackpressureState {
    current_buffer_size: usize,
    dropped_events: u64,
    is_throttled: bool,
}

impl UnifiedStreamEmitter {
    pub fn new(config: BackpressureConfig) -> Self {
        let (tool_tx, _) = broadcast::channel(config.buffer_size);
        let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
        
        let emitter = Self {
            tool_tx: tool_tx.clone(),
            internal_tx,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            backpressure: Arc::new(RwLock::new(BackpressureState {
                current_buffer_size: 0,
                dropped_events: 0,
                is_throttled: false,
            })),
            config: config.clone(),
        };
        
        // Spawn event dispatcher
        let tool_tx_clone = tool_tx.clone();
        let backpressure = emitter.backpressure.clone();
        let bp_config = config.clone();
        
        tokio::spawn(async move {
            while let Some(event) = internal_rx.recv().await {
                let mut bp_state = backpressure.write();
                
                // Check backpressure
                if bp_state.current_buffer_size >= bp_config.high_watermark {
                    bp_state.is_throttled = true;
                    
                    match bp_config.drop_policy {
                        DropPolicy::DropOldest | DropPolicy::DropNewest => {
                            bp_state.dropped_events += 1;
                            continue;
                        }
                        DropPolicy::Block => {
                            // Wait until low watermark
                            while bp_state.current_buffer_size > bp_config.low_watermark {
                                drop(bp_state);
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                bp_state = backpressure.write();
                            }
                        }
                    }
                }
                
                // Send event
                let _ = tool_tx_clone.send(event);
                bp_state.current_buffer_size = tool_tx_clone.receiver_count();
                
                if bp_state.current_buffer_size <= bp_config.low_watermark {
                    bp_state.is_throttled = false;
                }
            }
        });
        
        emitter
    }
    
    // Emit tool execution progress
    pub async fn emit_tool_progress(
        &self,
        tool_name: &str,
        correlation_id: &str,
        phase: ExecutionPhase,
        progress: f32,
        message: String,
    ) -> Result<()> {
        let event = StreamEvent::ToolExecutionProgress(ToolExecutionProgress {
            tool_name: tool_name.to_string(),
            correlation_id: correlation_id.to_string(),
            phase,
            progress,
            message,
            timestamp: chrono::Utc::now(),
            elapsed_ms: 0,
            estimated_remaining_ms: None,
        });
        
        self.internal_tx.send(event)?;
        Ok(())
    }
    
    // Emit command status
    pub async fn emit_command_status(
        &self,
        command: &str,
        correlation_id: &str,
        status: CommandStatus,
        stdout: Vec<String>,
        stderr: Vec<String>,
    ) -> Result<()> {
        let event = StreamEvent::CommandExecutionStatus(CommandExecutionStatus {
            command: command.to_string(),
            correlation_id: correlation_id.to_string(),
            status,
            stdout,
            stderr,
            exit_code: None,
            timestamp: chrono::Utc::now(),
        });
        
        self.internal_tx.send(event)?;
        Ok(())
    }
    
    // Emit diff update
    pub async fn emit_diff_update(
        &self,
        correlation_id: &str,
        file_path: &str,
        hunk_index: usize,
        total_hunks: usize,
        status: DiffStatus,
    ) -> Result<()> {
        let event = StreamEvent::DiffStreamUpdate(DiffStreamUpdate {
            correlation_id: correlation_id.to_string(),
            file_path: file_path.to_string(),
            hunk_index,
            total_hunks,
            lines_added: 0,
            lines_removed: 0,
            status,
            preview: None,
        });
        
        self.internal_tx.send(event)?;
        Ok(())
    }
    
    // Subscribe to events
    pub fn subscribe(&self, filter: Option<EventFilter>) -> broadcast::Receiver<StreamEvent> {
        let subscriber_id = uuid::Uuid::new_v4().to_string();
        
        self.subscribers.write().insert(
            subscriber_id.clone(),
            SubscriberInfo {
                id: subscriber_id,
                filter,
                created_at: Instant::now(),
                events_received: 0,
            },
        );
        
        self.tool_tx.subscribe()
    }
    
    // Get backpressure statistics
    pub fn get_backpressure_stats(&self) -> BackpressureStats {
        let state = self.backpressure.read();
        BackpressureStats {
            current_buffer_size: state.current_buffer_size,
            dropped_events: state.dropped_events,
            is_throttled: state.is_throttled,
            subscriber_count: self.subscribers.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureStats {
    pub current_buffer_size: usize,
    pub dropped_events: u64,
    pub is_throttled: bool,
    pub subscriber_count: usize,
}

// Global emitter instance
lazy_static::lazy_static! {
    pub static ref STREAM_EMITTER: Arc<UnifiedStreamEmitter> = 
        Arc::new(UnifiedStreamEmitter::new(BackpressureConfig::default()));
}

// Test consumer for local validation
pub struct TestConsumer {
    receiver: broadcast::Receiver<StreamEvent>,
    received_events: Arc<RwLock<Vec<StreamEvent>>>,
}

impl TestConsumer {
    pub fn new(emitter: &UnifiedStreamEmitter) -> Self {
        Self {
            receiver: emitter.subscribe(None),
            received_events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn consume(&mut self) -> Result<()> {
        while let Ok(event) = self.receiver.recv().await {
            self.received_events.write().push(event);
        }
        Ok(())
    }
    
    pub fn get_events(&self) -> Vec<StreamEvent> {
        self.received_events.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_streaming_emission() {
        let emitter = UnifiedStreamEmitter::new(BackpressureConfig::default());
        let mut consumer = TestConsumer::new(&emitter);
        
        // Start consumer
        let consumer_handle = tokio::spawn(async move {
            let _ = consumer.consume().await;
        });
        
        // Emit events
        emitter.emit_tool_progress(
            "test_tool",
            "corr-123",
            ExecutionPhase::Executing,
            0.5,
            "Processing...".to_string(),
        ).await.unwrap();
        
        emitter.emit_command_status(
            "echo test",
            "corr-124",
            CommandStatus::Completed,
            vec!["test".to_string()],
            vec![],
        ).await.unwrap();
        
        // Give consumer time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check stats
        let stats = emitter.get_backpressure_stats();
        assert!(stats.subscriber_count > 0);
    }
    
    #[tokio::test]
    async fn test_backpressure() {
        let mut config = BackpressureConfig::default();
        config.buffer_size = 10;
        config.high_watermark = 8;
        config.low_watermark = 2;
        
        let emitter = UnifiedStreamEmitter::new(config);
        
        // Emit many events rapidly
        for i in 0..20 {
            emitter.emit_tool_progress(
                "flood_tool",
                &format!("corr-{}", i),
                ExecutionPhase::Executing,
                i as f32 / 20.0,
                format!("Event {}", i),
            ).await.unwrap();
        }
        
        let stats = emitter.get_backpressure_stats();
        assert!(stats.dropped_events > 0 || stats.is_throttled);
    }
}
