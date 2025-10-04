// Common types for MCP tools
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub line_ranges: Vec<(usize, usize)>,
}

impl FileEntry {
    pub fn new(path: String) -> Self {
        Self {
            path,
            line_ranges: Vec::new(),
        }
    }
}

// Task and Provider types for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOptions {
    pub auto_complete: bool,
    pub notify: bool,
    pub track_time: bool,
}

// Cline types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineProvider {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineAsk {
    pub question: String,
    pub options: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineAskResponse {
    pub response: String,
    pub metadata: Option<serde_json::Value>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
}

// API Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

impl ApiMessage {
    pub fn new(role: String, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: get_current_timestamp(),
        }
    }
}

// User content type for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    pub content_type: String,
    pub text: String,
    pub metadata: Option<serde_json::Value>,
}

// Helper functions
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
