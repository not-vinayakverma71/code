/// Common traits and types for AI Providers

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Completion request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: Option<bool>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<Tool>>,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
    pub metadata: Option<Value>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Stream tokens
#[derive(Debug, Clone)]
pub enum StreamToken {
    Text(String),
    ToolCall { id: String, name: String, arguments: String },
    FunctionCall { name: String, arguments: String },
    Error(String),
    Done,
}

/// Provider capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub functions: bool,
    pub tools: bool,
    pub vision: bool,
    pub embeddings: bool,
    pub rate_limits: Option<RateLimits>,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub concurrent_requests: u32,
}

/// Main AI Provider trait
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Complete a request (non-streaming)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Stream a completion
    async fn stream(&self, request: CompletionRequest) -> Result<BoxStream<'static, Result<StreamToken>>>;
    
    /// Get provider capabilities
    fn get_capabilities(&self) -> ProviderCapabilities;
    
    /// Health check
    async fn health_check(&self) -> Result<()>;
}
