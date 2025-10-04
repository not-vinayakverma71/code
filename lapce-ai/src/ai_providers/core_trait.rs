/// EXACT AI Provider Trait from 03-AI-PROVIDERS-CONSOLIDATED.md
/// This is the EXACT specification - not simplified

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Stream token types for exact SSE compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamToken {
    Text(String),
    Delta { content: String },
    Event { event_type: String, data: serde_json::Value },
    FunctionCall { name: String, arguments: String },
    ToolCall { id: String, name: String, arguments: String },
    Citation { text: String, sources: Vec<String> },
    Done,
    Error(String),
}

/// Health status for providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub rate_limit_remaining: Option<u32>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_vision: bool,
    pub supports_functions: bool,
    pub supports_tools: bool,
    pub pricing: Option<ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub max_tokens: usize,
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub supports_vision: bool,
    pub supports_embeddings: bool,
    pub supports_tool_calls: bool,
    pub supports_prompt_caching: bool,
    pub rate_limits: RateLimits,
}

/// Rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub concurrent_requests: u32,
}

/// Completion request (EXACT from TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stream: Option<bool>,
    pub stop: Option<Vec<String>>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub logit_bias: Option<serde_json::Value>,
    pub user: Option<String>,
    pub suffix: Option<String>,
    pub echo: Option<bool>,
    pub n: Option<u32>,
    pub best_of: Option<u32>,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChoice {
    pub text: String,
    pub index: u32,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: Option<String>,
}

/// Chat request (EXACT from TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stream: Option<bool>,
    pub stop: Option<Vec<String>>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub functions: Option<Vec<Function>>,
    pub function_call: Option<serde_json::Value>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<serde_json::Value>,
    pub response_format: Option<ResponseFormat>,
    pub seed: Option<u64>,
    pub user: Option<String>,
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub name: Option<String>,
    pub function_call: Option<FunctionCall>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    pub r#type: String,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// THE EXACT AI Provider Trait (from spec)
#[async_trait]
pub trait AiProvider: Send + Sync + 'static {
    /// Provider name
    fn name(&self) -> &'static str;
    
    /// Health check
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Completion API (legacy)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Completion streaming
    async fn complete_stream(&self, request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>>;
    
    /// Chat API
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    
    /// Chat streaming
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<Model>>;
    
    /// Count tokens in text
    async fn count_tokens(&self, text: &str) -> Result<usize>;
    
    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;
}
