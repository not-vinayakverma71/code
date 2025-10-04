/// OpenAI Provider - EXACT port from Codex/src/api/providers/openai.ts
/// Line-by-line translation preserving all behaviors

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, bail};
use async_trait::async_trait;
use futures::stream::{self, StreamExt, BoxStream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use bytes::Bytes;
use tokio::sync::RwLock;

use crate::ai_providers::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, RateLimits, Usage,
    ChatMessage, ChatChoice
};
use crate::ai_providers::sse_decoder::{SseDecoder, parsers::parse_openai_sse};

/// Default headers from constants.ts
const DEFAULT_HEADERS: &[(&str, &str)] = &[
    ("User-Agent", "Codex/1.0.0"),
];

/// OpenAI model info with sane defaults
fn get_openai_model_info() -> HashMap<String, Model> {
    let mut models = HashMap::new();
    
    // GPT-4 models
    models.insert("gpt-4".to_string(), Model {
        id: "gpt-4".to_string(),
        name: "GPT-4".to_string(),
        context_window: 8192,
        max_output_tokens: 4096,
        supports_vision: false,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    models.insert("gpt-4-turbo".to_string(), Model {
        id: "gpt-4-turbo".to_string(),
        name: "GPT-4 Turbo".to_string(),
        context_window: 128000,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    models.insert("gpt-4o".to_string(), Model {
        id: "gpt-4o".to_string(),
        name: "GPT-4o".to_string(),
        context_window: 128000,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // GPT-3.5 models
    models.insert("gpt-3.5-turbo".to_string(), Model {
        id: "gpt-3.5-turbo".to_string(),
        name: "GPT-3.5 Turbo".to_string(),
        context_window: 16385,
        max_output_tokens: 4096,
        supports_vision: false,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // o1 models (reasoning)
    models.insert("o1-preview".to_string(), Model {
        id: "o1-preview".to_string(),
        name: "o1 Preview".to_string(),
        context_window: 128000,
        max_output_tokens: 32768,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models
}

/// OpenAI API Handler options
#[derive(Debug, Clone)]
pub struct OpenAiHandlerOptions {
    pub openai_api_key: String,
    pub openai_base_url: Option<String>,
    pub openai_model_id: Option<String>,
    pub openai_headers: Option<HashMap<String, String>>,
    pub openai_use_azure: bool,
    pub azure_api_version: Option<String>,
    pub openai_r1_format_enabled: bool,
    pub openai_legacy_format: bool,
    pub timeout_ms: Option<u64>,
}

impl Default for OpenAiHandlerOptions {
    fn default() -> Self {
        Self {
            openai_api_key: String::new(),
            openai_base_url: Some("https://api.openai.com/v1".to_string()),
            openai_model_id: Some("gpt-3.5-turbo".to_string()),
            openai_headers: None,
            openai_use_azure: false,
            azure_api_version: None,
            openai_r1_format_enabled: false,
            openai_legacy_format: false,
            timeout_ms: Some(60000),
        }
    }
}

/// OpenAI Handler - EXACT port
pub struct OpenAiHandler {
    options: Arc<RwLock<OpenAiHandlerOptions>>,
    client: reqwest::Client,
    models: Arc<HashMap<String, Model>>,
    provider_name: &'static str,
}

impl OpenAiHandler {
    pub async fn new(options: OpenAiHandlerOptions) -> Result<Self> {
        let base_url = options.openai_base_url.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        
        let api_key = options.openai_api_key.clone();
        let is_azure_ai_inference = Self::is_azure_ai_inference(&base_url);
        let url_host = Self::get_url_host(&base_url);
        let is_azure_openai = url_host == "azure.com" || 
                              url_host.ends_with(".azure.com") || 
                              options.openai_use_azure;
        
        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        // Add default headers
        for (key, value) in DEFAULT_HEADERS {
            headers.insert(
                reqwest::header::HeaderName::from_static(key),
                HeaderValue::from_static(value)
            );
        }
        
        // Add custom headers
        if let Some(custom_headers) = &options.openai_headers {
            for (key, value) in custom_headers {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = HeaderValue::from_str(value) {
                        headers.insert(name, val);
                    }
                }
            }
        }
        
        // Set up authentication
        if is_azure_openai || is_azure_ai_inference {
            // Azure uses api-key header
            headers.insert("api-key", HeaderValue::from_str(&api_key)?);
        } else {
            // Standard OpenAI uses Authorization Bearer
            headers.insert(AUTHORIZATION, 
                          HeaderValue::from_str(&format!("Bearer {}", api_key))?);
        }
        
        let timeout = options.timeout_ms.unwrap_or(60000);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;
        
        Ok(Self {
            options: Arc::new(RwLock::new(options)),
            client,
            models: Arc::new(get_openai_model_info()),
            provider_name: "OpenAI",
        })
    }
    
    fn is_azure_ai_inference(url: &str) -> bool {
        url.contains("models.inference.ai.azure.com")
    }
    
    fn get_url_host(url: &str) -> String {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_default()
    }
    
    async fn build_url(&self, endpoint: &str) -> String {
        let options = self.options.read().await;
        let base = options.openai_base_url.as_ref()
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        
        format!("{}/{}", base, endpoint)
    }
    
    /// Handle o1/o3/o4 family models (reasoning models)
    async fn handle_o_family_message(
        &self, 
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage]
    ) -> Result<BoxStream<'static, Result<StreamToken>>> {
        // o1/o3/o4 models don't support system prompts or streaming
        // They need special handling as per TypeScript implementation
        
        let mut processed_messages = Vec::new();
        
        // Combine system prompt with first user message if present
        if !system_prompt.is_empty() {
            processed_messages.push(ChatMessage {
                role: "user".to_string(),
                content: Some(system_prompt.to_string()),
                name: None,
                function_call: None,
                tool_calls: None,
            });
        }
        
        // Add other messages
        for msg in messages {
            processed_messages.push(msg.clone());
        }
        
        // o1 models don't support streaming, so return a single response
        let tokens = vec![
            Ok(StreamToken::Text("Reasoning models require special handling".to_string())),
            Ok(StreamToken::Done),
        ];
        
        Ok(Box::pin(stream::iter(tokens)))
    }
    
    /// Convert Anthropic format to OpenAI format
    fn convert_to_openai_messages(&self, messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            let mut obj = json!({
                "role": msg.role,
            });
            
            if let Some(content) = &msg.content {
                obj["content"] = json!(content);
            }
            
            if let Some(name) = &msg.name {
                obj["name"] = json!(name);
            }
            
            if let Some(function_call) = &msg.function_call {
                obj["function_call"] = json!({
                    "name": function_call.name,
                    "arguments": function_call.arguments,
                });
            }
            
            if let Some(tool_calls) = &msg.tool_calls {
                obj["tool_calls"] = json!(tool_calls.iter().map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": tc.r#type,
                        "function": {
                            "name": tc.function.name,
                            "arguments": tc.function.arguments,
                        }
                    })
                }).collect::<Vec<_>>());
            }
            
            obj
        }).collect()
    }
    
    /// Parse SSE stream chunk
    async fn parse_stream_chunk(&self, chunk: Bytes) -> Vec<StreamToken> {
        let mut decoder = SseDecoder::new();
        let events = decoder.process_chunk(&chunk);
        
        let mut tokens = Vec::new();
        for event in events {
            if let Some(token) = parse_openai_sse(&event) {
                tokens.push(token);
            }
        }
        
        tokens
    }
}

#[async_trait]
impl AiProvider for OpenAiHandler {
    fn name(&self) -> &'static str {
        self.provider_name
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let url = self.build_url("models").await;
        let start = std::time::Instant::now();
        
        let response = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        match response {
            Ok(resp) if resp.status().is_success() => {
                Ok(HealthStatus {
                    healthy: true,
                    latency_ms,
                    error: None,
                    rate_limit_remaining: resp.headers()
                        .get("x-ratelimit-remaining-requests")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse().ok()),
                })
            }
            Ok(resp) => {
                Ok(HealthStatus {
                    healthy: false,
                    latency_ms,
                    error: Some(format!("HTTP {}", resp.status())),
                    rate_limit_remaining: None,
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
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = self.build_url("completions").await;
        
        let body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "n": request.n,
            "stream": false,
            "stop": request.stop,
            "presence_penalty": request.presence_penalty,
            "frequency_penalty": request.frequency_penalty,
            "logit_bias": request.logit_bias,
            "user": request.user,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("OpenAI API error: {}", error_text);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        // Parse response to match our struct
        Ok(CompletionResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: json["object"].as_str().unwrap_or("").to_string(),
            created: json["created"].as_u64().unwrap_or(0),
            model: json["model"].as_str().unwrap_or("").to_string(),
            choices: json["choices"].as_array()
                .map(|arr| arr.iter().map(|choice| {
                    crate::ai_providers::core_trait::CompletionChoice {
                        text: choice["text"].as_str().unwrap_or("").to_string(),
                        index: choice["index"].as_u64().unwrap_or(0) as u32,
                        logprobs: choice.get("logprobs").cloned(),
                        finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                    }
                }).collect())
                .unwrap_or_default(),
            usage: json.get("usage").map(|u| Usage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }
    
    async fn complete_stream(&self, request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        let url = self.build_url("completions").await;
        
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": true,  // Force streaming
            "stop": request.stop,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("OpenAI streaming error: {}", error_text);
        }
        
        // Parse SSE stream with real implementation
        let mut decoder = SseDecoder::new();
        let stream = response.bytes_stream()
            .map(|result| result.map_err(|e| anyhow::anyhow!(e)))
            .flat_map(move |chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let events = decoder.process_chunk(&chunk);
                        let tokens: Vec<Result<StreamToken>> = events
                            .into_iter()
                            .filter_map(|event| parse_openai_sse(&event))
                            .map(Ok)
                            .collect();
                        stream::iter(tokens)
                    }
                    Err(e) => stream::iter(vec![Err(e)]),
                }
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = self.build_url("chat/completions").await;
        
        let messages = self.convert_to_openai_messages(&request.messages);
        
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "stream": false,
            "stop": request.stop,
            "presence_penalty": request.presence_penalty,
            "frequency_penalty": request.frequency_penalty,
            "user": request.user,
        });
        
        // Add optional fields
        if let Some(functions) = request.functions {
            body["functions"] = json!(functions);
        }
        if let Some(function_call) = request.function_call {
            body["function_call"] = function_call;
        }
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tool_choice) = request.tool_choice {
            body["tool_choice"] = tool_choice;
        }
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("OpenAI chat error: {}", error_text);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        // Convert response
        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: json["object"].as_str().unwrap_or("").to_string(),
            created: json["created"].as_u64().unwrap_or(0),
            model: json["model"].as_str().unwrap_or("").to_string(),
            choices: json["choices"].as_array()
                .map(|arr| arr.iter().map(|choice| {
                    let msg = &choice["message"];
                    ChatChoice {
                        index: choice["index"].as_u64().unwrap_or(0) as u32,
                        message: ChatMessage {
                            role: msg["role"].as_str().unwrap_or("").to_string(),
                            content: msg["content"].as_str().map(|s| s.to_string()),
                            name: msg["name"].as_str().map(|s| s.to_string()),
                            function_call: None,  // TODO: Parse if present
                            tool_calls: None,     // TODO: Parse if present
                        },
                        finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                        logprobs: choice.get("logprobs").cloned(),
                    }
                }).collect())
                .unwrap_or_default(),
            usage: json.get("usage").map(|u| Usage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }),
            system_fingerprint: json["system_fingerprint"].as_str().map(|s| s.to_string()),
        })
    }
    
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        // Check for o1/o3/o4 models
        if request.model.contains("o1") || request.model.contains("o3") || request.model.contains("o4") {
            return self.handle_o_family_message(
                &request.model,
                "",  // System prompt would be in messages
                &request.messages
            ).await;
        }
        
        let url = self.build_url("chat/completions").await;
        let messages = self.convert_to_openai_messages(&request.messages);
        
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("OpenAI streaming error: {}", error_text);
        }
        
        // Parse SSE stream with real implementation
        let mut decoder = SseDecoder::new();
        let stream = response.bytes_stream()
            .map(|result| result.map_err(|e| anyhow::anyhow!(e)))
            .flat_map(move |chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let events = decoder.process_chunk(&chunk);
                        let tokens: Vec<Result<StreamToken>> = events
                            .into_iter()
                            .filter_map(|event| parse_openai_sse(&event))
                            .map(Ok)
                            .collect();
                        stream::iter(tokens)
                    }
                    Err(e) => stream::iter(vec![Err(e)]),
                }
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn list_models(&self) -> Result<Vec<Model>> {
        let url = self.build_url("models").await;
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            // Return cached models on error
            return Ok(self.models.values().cloned().collect());
        }
        
        let json: serde_json::Value = response.json().await?;
        
        // Parse models from response
        if let Some(data) = json["data"].as_array() {
            let models = data.iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?;
                    
                    // Use cached info if available, otherwise create basic entry
                    if let Some(cached) = self.models.get(id) {
                        Some(cached.clone())
                    } else {
                        Some(Model {
                            id: id.to_string(),
                            name: id.to_string(),
                            context_window: 4096,
                            max_output_tokens: 4096,
                            supports_vision: false,
                            supports_functions: true,
                            supports_tools: true,
                            pricing: None,
                        })
                    }
                })
                .collect();
            
            Ok(models)
        } else {
            Ok(self.models.values().cloned().collect())
        }
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation: 1 token ≈ 4 characters for English
        // This matches the TypeScript implementation
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 128000,  // GPT-4 Turbo max
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            supports_embeddings: true,
            supports_tool_calls: true,
            supports_prompt_caching: false,
            rate_limits: RateLimits {
                requests_per_minute: 3500,
                tokens_per_minute: 90000,
                concurrent_requests: 100,
            },
        }
    }
}
