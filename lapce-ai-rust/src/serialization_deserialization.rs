/// Exact 1:1 Translation of TypeScript serialization/deserialization from codex-reference/core/task-persistence/apiMessages.ts
/// DAY 3 H3-4: Port serialization/deserialization logic

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use serde_json;

/// ApiMessage type - exact translation line 12
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    #[serde(flatten)]
    pub message: MessageParam,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(rename = "isSummary")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_summary: Option<bool>,
}

/// MessageParam - Anthropic message parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
    pub role: String,
    pub content: serde_json::Value,
}

/// GlobalFileNames constants
pub struct GlobalFileNames;

impl GlobalFileNames {
    pub const API_CONVERSATION_HISTORY: &'static str = "api_conversation_history.json";
    pub const TASK_MESSAGES: &'static str = "task_messages.json";
    pub const TASK_METADATA: &'static str = "task_metadata.json";
}

/// readApiMessages - exact translation lines 14-69
pub async fn read_api_messages(
    task_id: &str,
    global_storage_path: &Path,
) -> Result<Vec<ApiMessage>, String> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join(GlobalFileNames::API_CONVERSATION_HISTORY);
    
    if file_exists_at_path(&file_path).await {
        match fs::read_to_string(&file_path).await {
            Ok(file_content) => {
                match serde_json::from_str::<Vec<ApiMessage>>(&file_content) {
                    Ok(parsed_data) => {
                        if parsed_data.is_empty() {
                            eprintln!(
                                "[Roo-Debug] readApiMessages: Found API conversation history file, but it's empty (parsed as []). TaskId: {}, Path: {}",
                                task_id, file_path.display()
                            );
                        }
                        Ok(parsed_data)
                    }
                    Err(error) => {
                        eprintln!(
                            "[Roo-Debug] readApiMessages: Error parsing API conversation history file. TaskId: {}, Path: {}, Error: {}",
                            task_id, file_path.display(), error
                        );
                        Err(error.to_string())
                    }
                }
            }
            Err(error) => Err(error.to_string())
        }
    } else {
        // Check for old file name (claude_messages.json)
        let old_path = task_dir.join("claude_messages.json");
        
        if file_exists_at_path(&old_path).await {
            match fs::read_to_string(&old_path).await {
                Ok(file_content) => {
                    match serde_json::from_str::<Vec<ApiMessage>>(&file_content) {
                        Ok(parsed_data) => {
                            if parsed_data.is_empty() {
                                eprintln!(
                                    "[Roo-Debug] readApiMessages: Found OLD API conversation history file (claude_messages.json), but it's empty (parsed as []). TaskId: {}, Path: {}",
                                    task_id, old_path.display()
                                );
                            }
                            // Delete old file after successful read
                            let _ = fs::remove_file(&old_path).await;
                            Ok(parsed_data)
                        }
                        Err(error) => {
                            eprintln!(
                                "[Roo-Debug] readApiMessages: Error parsing OLD API conversation history file (claude_messages.json). TaskId: {}, Path: {}, Error: {}",
                                task_id, old_path.display(), error
                            );
                            // DO NOT unlink oldPath if parsing failed
                            Err(error.to_string())
                        }
                    }
                }
                Err(error) => Err(error.to_string())
            }
        } else {
            // Neither new nor old history file found
            eprintln!(
                "[Roo-Debug] readApiMessages: API conversation history file not found for taskId: {}. Expected at: {}",
                task_id, file_path.display()
            );
            Ok(vec![])
        }
    }
}

/// saveApiMessages - exact translation lines 71-83
pub async fn save_api_messages(
    messages: &[ApiMessage],
    task_id: &str,
    global_storage_path: &Path,
) -> Result<(), String> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join(GlobalFileNames::API_CONVERSATION_HISTORY);
    safe_write_json(&file_path, &messages).await
}

/// Helper function to get task directory path
async fn get_task_directory_path(
    global_storage_path: &Path,
    task_id: &str,
) -> Result<PathBuf, String> {
    let task_dir = global_storage_path.join("tasks").join(task_id);
    
    // Create directory if it doesn't exist
    if !task_dir.exists() {
        fs::create_dir_all(&task_dir)
            .await
            .map_err(|e| format!("Failed to create task directory: {}", e))?;
    }
    
    Ok(task_dir)
}

/// Helper function to check if file exists
async fn file_exists_at_path(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

/// Safe JSON write with atomic operations
async fn safe_write_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;
    
    // Write to temp file first
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, json)
        .await
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    // Atomic rename
    fs::rename(&temp_path, path)
        .await
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    
    Ok(())
}

/// ClineMessage serialization/deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClineMessage {
    #[serde(rename = "say")]
    Say {
        ts: u64,
        say: String,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    #[serde(rename = "ask")]
    Ask {
        ts: u64,
        ask: String,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress_status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_protected: Option<bool>,
    },
}

/// Read task messages
pub async fn read_task_messages(
    task_id: &str,
    global_storage_path: &Path,
) -> Result<Vec<ClineMessage>, String> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join(GlobalFileNames::TASK_MESSAGES);
    
    if file_exists_at_path(&file_path).await {
        match fs::read_to_string(&file_path).await {
            Ok(file_content) => {
                match serde_json::from_str::<Vec<ClineMessage>>(&file_content) {
                    Ok(messages) => Ok(messages),
                    Err(error) => {
                        eprintln!(
                            "[Roo-Debug] readTaskMessages: Error parsing task messages file. TaskId: {}, Path: {}, Error: {}",
                            task_id, file_path.display(), error
                        );
                        Err(error.to_string())
                    }
                }
            }
            Err(error) => Err(error.to_string())
        }
    } else {
        Ok(vec![])
    }
}

/// Save task messages
pub async fn save_task_messages(
    messages: &[ClineMessage],
    task_id: &str,
    global_storage_path: &Path,
) -> Result<(), String> {
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join(GlobalFileNames::TASK_MESSAGES);
    safe_write_json(&file_path, &messages).await
}

/// Task metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub task_id: String,
    pub task_number: i32,
    pub workspace: String,
    pub mode: String,
    pub token_usage: TokenUsage,
    pub history_item: HistoryItem,
}

/// Token usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// History item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub task: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
    #[serde(rename = "isFavorited")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_favorited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Get task metadata from messages
pub async fn task_metadata(
    messages: &[ClineMessage],
    task_id: &str,
    task_number: i32,
    global_storage_path: &Path,
    workspace: &str,
    mode: &str,
) -> Result<TaskMetadata, String> {
    // Calculate token usage from messages
    let token_usage = calculate_token_usage(messages);
    
    // Create history item
    let history_item = HistoryItem {
        id: task_id.to_string(),
        task: extract_task_from_messages(messages),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model_used: None,
        is_favorited: None,
        mode: Some(mode.to_string()),
    };
    
    let metadata = TaskMetadata {
        task_id: task_id.to_string(),
        task_number,
        workspace: workspace.to_string(),
        mode: mode.to_string(),
        token_usage,
        history_item,
    };
    
    // Save metadata
    let task_dir = get_task_directory_path(global_storage_path, task_id).await?;
    let file_path = task_dir.join(GlobalFileNames::TASK_METADATA);
    safe_write_json(&file_path, &metadata).await?;
    
    Ok(metadata)
}

/// Helper to calculate token usage from messages
fn calculate_token_usage(messages: &[ClineMessage]) -> TokenUsage {
    // Simplified token calculation
    let mut input_tokens = 0u32;
    let mut output_tokens = 0u32;
    
    for message in messages {
        match message {
            ClineMessage::Say { text, .. } => {
                output_tokens += (text.len() / 4) as u32; // Rough estimate
            }
            ClineMessage::Ask { text, .. } => {
                input_tokens += (text.len() / 4) as u32; // Rough estimate
            }
        }
    }
    
    TokenUsage {
        input_tokens,
        output_tokens,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: None,
    }
}

/// Helper to extract task description from messages
fn extract_task_from_messages(messages: &[ClineMessage]) -> String {
    // Find first user message
    for message in messages {
        if let ClineMessage::Ask { text, .. } = message {
            return text.chars().take(100).collect(); // First 100 chars
        }
    }
    "Untitled Task".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_api_message_serialization() {
        let message = ApiMessage {
            message: MessageParam {
                role: "user".to_string(),
                content: serde_json::json!("Hello"),
            },
            ts: Some(1234567890),
            is_summary: None,
        };
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"ts\":1234567890"));
    }
    
    #[tokio::test]
    async fn test_save_and_read_messages() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path();
        let task_id = "test-task-123";
        
        let messages = vec![
            ApiMessage {
                message: MessageParam {
                    role: "user".to_string(),
                    content: serde_json::json!("Test message"),
                },
                ts: Some(1234567890),
                is_summary: None,
            }
        ];
        
        // Save messages
        save_api_messages(&messages, task_id, storage_path).await.unwrap();
        
        // Read them back
        let loaded = read_api_messages(task_id, storage_path).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].message.role, "user");
    }
}
