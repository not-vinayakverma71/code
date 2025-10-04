/// Mock types to fix compilation errors
/// These replace broken dependencies temporarily

use std::sync::Arc;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use async_trait::async_trait;

// Mock Task type
#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub options: TaskOptions,
}

impl Task {
    pub fn new(options: TaskOptions) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            options,
        }
    }
}

// Mock TaskOptions
#[derive(Clone, Debug)]
pub struct TaskOptions {
    pub name: String,
    pub config: Value,
}

// Mock ClineProvider
#[derive(Clone, Debug)]
pub struct ClineProvider {
    pub id: String,
}

impl ClineProvider {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

// Mock ApiMessage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiMessage {
    pub id: String,
    pub content: String,
    pub message_type: String,
}

// Mock ClineAskResponse
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClineAskResponse {
    pub response: String,
    pub approved: bool,
}

// Mock UserContent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserContent {
    pub text: String,
    pub metadata: Option<Value>,
}

// Mock RegisterCommandOptions
#[derive(Clone, Debug)]
pub struct RegisterCommandOptions {
    pub command: String,
    pub handler: String,
}

// Mock CodeActionProvider
pub struct CodeActionProvider {
    pub id: String,
}

impl CodeActionProvider {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

// Mock ToolParameter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: String,
}

// Helper function
pub fn get_current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
