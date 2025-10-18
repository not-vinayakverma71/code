/// Anthropic Claude Provider - EXACT port from Codex/src/api/providers/anthropic.ts
/// Complete implementation with event-based SSE streaming

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, bail};
use async_trait::async_trait;
use futures::stream::{StreamExt, BoxStream};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tokio::sync::RwLock;

use crate::ai_providers::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    HealthStatus, Model, ProviderCapabilities, RateLimits, Usage,
    ChatMessage, ChatChoice
};
use crate::streaming_pipeline::StreamToken;
use crate::ai_providers::sse_decoder::SseEvent;

/// Anthropic configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub version: String,
    pub beta_features: Vec<String>,
    pub default_model: Option<String>,
    pub cache_enabled: bool,
    pub timeout_ms: Option<u64>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://api.anthropic.com".to_string()),
            version: "2023-06-01".to_string(),
            beta_features: vec!["prompt-caching-2024-07-31".to_string()],
            default_model: Some("claude-3-sonnet-20240229".to_string()),
            cache_enabled: true,
            timeout_ms: Some(60000),
        }
    }
}

/// Anthropic model definitions
fn get_anthropic_models() -> HashMap<String, Model> {
    let mut models = HashMap::new();
    
    // Claude 3 Opus
    models.insert("claude-3-opus-20240229".to_string(), Model {
        id: "claude-3-opus-20240229".to_string(),
        name: "Claude 3 Opus".to_string(),
        context_window: 200000,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: false,
        supports_tools: true,
        pricing: None,
    });
    
    // Claude 3 Sonnet
    models.insert("claude-3-sonnet-20240229".to_string(), Model {
        id: "claude-3-sonnet-20240229".to_string(),
        name: "Claude 3 Sonnet".to_string(),
        context_window: 200000,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: false,
        supports_tools: true,
        pricing: None,
    });
    
    // Claude 3.5 Sonnet
    models.insert("claude-3-5-sonnet-20241022".to_string(), Model {
        id: "claude-3-5-sonnet-20241022".to_string(),
        name: "Claude 3.5 Sonnet".to_string(),
        context_window: 200000,
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: false,
        supports_tools: true,
        pricing: None,
    });
    
    // Claude 3 Haiku
    models.insert("claude-3-haiku-20240307".to_string(), Model {
        id: "claude-3-haiku-20240307".to_string(),
        name: "Claude 3 Haiku".to_string(),
        context_window: 200000,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: false,
        supports_tools: true,
        pricing: None,
    });
    
    // Claude 2.1
    models.insert("claude-2.1".to_string(), Model {
        id: "claude-2.1".to_string(),
        name: "Claude 2.1".to_string(),
        context_window: 100000,
        max_output_tokens: 4096,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    // Claude Instant
    models.insert("claude-instant-1.2".to_string(), Model {
        id: "claude-instant-1.2".to_string(),
        name: "Claude Instant 1.2".to_string(),
        context_window: 100000,
        max_output_tokens: 4096,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models
}

/// Anthropic Provider - Complete implementation
pub struct AnthropicProvider {
    config: Arc<RwLock<AnthropicConfig>>,
    client: reqwest::Client,
    models: Arc<HashMap<String, Model>>,
}

impl AnthropicProvider {
    pub async fn new(config: AnthropicConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&config.api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_str(&config.version)?);
        
        // Add beta features
        if !config.beta_features.is_empty() {
            let beta_header = config.beta_features.join(",");
            headers.insert("anthropic-beta", HeaderValue::from_str(&beta_header)?);
        }
        
        let timeout = config.timeout_ms.unwrap_or(60000);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            models: Arc::new(get_anthropic_models()),
        })
    }
    
    async fn build_url(&self, endpoint: &str) -> String {
        let config = self.config.read().await;
        let base = config.base_url.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://api.anthropic.com");
        
        format!("{}/v1/{}", base, endpoint)
    }
    
    /// Format messages for Anthropic API with Human/Assistant prefixes
    /// Ensures alternating user/assistant messages as required
    fn format_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        let mut formatted = Vec::new();
        
        for msg in messages {
            let role = match msg.role.as_str() {
                "system" => "user",  // Anthropic treats system as user
                "assistant" => "assistant",
                _ => "user"
            };
            
            let mut content = msg.content.clone().unwrap_or_default();
            
            // Add Human/Assistant prefixes as per Anthropic format
            if msg.role == "system" {
                // System messages are formatted as user messages with special prefix
                content = format!("Human: [System]: {}\n\nAssistant: I understand the system instructions.", content);
            } else if role == "user" {
                // Add Human: prefix for user messages
                content = format!("Human: {}", content);
            } else if role == "assistant" {
                // Add Assistant: prefix for assistant messages
                content = format!("Assistant: {}", content);
            }
            
            formatted.push(json!({
                "role": role,
                "content": content,
            }));
        }
        
        // Ensure alternating user/assistant messages
        let mut final_messages: Vec<serde_json::Value> = Vec::new();
        let mut last_role = String::new();
        
        for msg in formatted {
            let role = msg["role"].as_str().unwrap_or("").to_string();
            
            if role == last_role && role == "user" {
                // Merge consecutive user messages
                if let Some(last) = final_messages.last_mut() {
                    let prev_content = last["content"].as_str().unwrap_or("");
                    let new_content = msg["content"].as_str().unwrap_or("");
                    last["content"] = json!(format!("{}\n{}", prev_content, new_content));
                }
            } else if role == last_role && role == "assistant" {
                // Add a user message between consecutive assistant messages
                final_messages.push(json!({
                    "role": "user",
                    "content": "Continue."
                }));
                final_messages.push(msg);
            } else {
                final_messages.push(msg);
            }
            
            last_role = role;
        }
        
        // Ensure we end with a user message if needed
        if let Some(last) = final_messages.last() {
            if last["role"] == "assistant" {
                final_messages.push(json!({
                    "role": "user",
                    "content": "Please respond."
                }));
            }
        }
        
        final_messages
    }
}

/// Parse Anthropic event-based SSE format - EXACT implementation
/// Handles: event: message_start, content_block_delta, message_stop
fn parse_anthropic_sse(event: &SseEvent) -> Option<StreamToken> {
    // Anthropic uses event-based SSE
    match event.event.as_deref() {
        Some("message_start") => {
            // Beginning of message - could extract metadata if needed
            None
        }
        Some("content_block_start") => {
            // Start of content block
            None
        }
        Some("content_block_delta") => {
            // Content delta - this is where the actual text comes through
            if let Some(data) = &event.data {
                if let Ok(data_str) = std::str::from_utf8(data) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_str) {
                        if let Some(delta) = json.get("delta") {
                            if let Some(text) = delta["text"].as_str() {
                                use crate::streaming_pipeline::stream_token::TextDelta;
                                return Some(StreamToken::Delta(TextDelta {
                                    content: text.to_string(),
                                    index: 0,
                                    logprob: None,
                                }));
                            }
                        }
                    }
                }
            }
            None
        }
        Some("content_block_stop") => {
            // End of content block
            None
        }
        Some("message_delta") => {
            // Message metadata updates (usage, stop reason, etc.)
            None
        }
        Some("message_stop") => {
            // End of message
            Some(StreamToken::Done)
        }
        Some("error") => {
            // Error event
            if let Some(data) = &event.data {
                if let Ok(data_str) = std::str::from_utf8(data) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_str) {
                        if let Some(error) = json.get("error") {
                            let message = error["message"].as_str().unwrap_or("Unknown error");
                            return Some(StreamToken::Error(message.to_string()));
                        }
                    }
                }
            }
            Some(StreamToken::Error("Unknown error".to_string()))
        }
        _ => None
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Anthropic"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        // Anthropic doesn't have a dedicated health endpoint
        // We'll try to list models with a minimal request
        let start = std::time::Instant::now();
        let url = self.build_url("messages").await;
        
        // Send a minimal request to check connectivity
        let body = json!({
            "model": "claude-3-haiku-20240307",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 1,
        });
        
        let response = self.client.post(&url).json(&body).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        match response {
            Ok(resp) => {
                let healthy = resp.status().is_success() || 
                              resp.status() == 401 || // Unauthorized means API is up
                              resp.status() == 429;    // Rate limited means API is up
                
                Ok(HealthStatus {
                    healthy,
                    latency_ms,
                    error: if !healthy { 
                        Some(format!("HTTP {}", resp.status())) 
                    } else { 
                        None 
                    },
                    rate_limit_remaining: resp.headers()
                        .get("x-ratelimit-remaining")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse().ok()),
                })
            }
            Err(e) => {
                Ok(HealthStatus {
                    healthy: false,
                    latency_ms,
                    error: Some(e.to_string()),
                    rate_limit_remaining: None,
                })
            }
        }
    }
    
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        bail!("Anthropic doesn't support the legacy completion API. Use chat() instead.")
    }
    
    async fn complete_stream(&self, _request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        bail!("Anthropic doesn't support the legacy completion API. Use chat_stream() instead.")
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = self.build_url("messages").await;
        
        let messages = self.format_messages(&request.messages);
        
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });
        
        // Add optional parameters
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        
        if let Some(stop) = request.stop {
            body["stop_sequences"] = json!(stop);
        }
        
        // Add system prompt if present
        let system_prompt = request.messages.iter()
            .find(|m| m.role == "system")
            .and_then(|m| m.content.as_ref());
        
        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }
        
        // Add tools if present
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools.iter().map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "input_schema": t.function.parameters,
                })
            }).collect::<Vec<_>>());
        }
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("Anthropic API error: {}", error_text);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        // Convert Anthropic response to our format
        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: json["model"].as_str().unwrap_or("").to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
                logprobs: None,
            }],
            usage: json.get("usage").map(|u| Usage {
                prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (u["input_tokens"].as_u64().unwrap_or(0) + 
                              u["output_tokens"].as_u64().unwrap_or(0)) as u32,
            }),
            system_fingerprint: None,
        })
    }
    
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        let url = self.build_url("messages").await;
        
        let messages = self.format_messages(&request.messages);
        
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });
        
        // Add optional parameters
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("Anthropic streaming error: {}", error_text);
        }
        
        // Use event-based SSE streaming with Anthropic parser
        use crate::ai_providers::streaming_integration::{process_sse_response, ProviderType};
        use crate::ai_providers::sse_decoder::parsers;
        
        process_sse_response(response, ProviderType::Anthropic, parsers::parse_anthropic_sse).await
    }
    
    async fn list_models(&self) -> Result<Vec<Model>> {
        // Anthropic doesn't have a list models endpoint
        // Return our cached models
        Ok(self.models.values().cloned().collect())
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation for Claude
        // More accurate would require the actual tokenizer
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 200000,
            supports_streaming: true,
            supports_functions: false,
            supports_vision: true,
            supports_embeddings: false,
            supports_tool_calls: true,
            supports_prompt_caching: true,
            rate_limits: RateLimits {
                requests_per_minute: 50,
                tokens_per_minute: 100000,
                concurrent_requests: 50,
            },
        }
    }
}
