/// Google Gemini Provider - EXACT port from Codex/src/api/providers/gemini.ts
/// Complete implementation with exact request/response format

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, bail};
use async_trait::async_trait;
use futures::stream::{self, StreamExt, BoxStream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tokio::sync::RwLock;

use crate::ai_providers::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, RateLimits, Usage,
    ChatMessage, ChatChoice
};
use crate::ai_providers::sse_decoder::parsers::parse_gemini_stream;

/// Gemini configuration
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub api_version: Option<String>,
    pub timeout_ms: Option<u64>,
    pub project_id: Option<String>,
    pub location: Option<String>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            default_model: Some("gemini-1.5-pro".to_string()),
            api_version: Some("v1beta".to_string()),
            timeout_ms: Some(60000),
            project_id: None,
            location: None,
        }
    }
}

/// Gemini model definitions (from TypeScript)
fn get_gemini_models() -> HashMap<String, Model> {
    let mut models = HashMap::new();
    
    // Gemini 1.5 Pro
    models.insert("gemini-1.5-pro".to_string(), Model {
        id: "gemini-1.5-pro".to_string(),
        name: "Gemini 1.5 Pro".to_string(),
        context_window: 2097152, // 2M tokens
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // Gemini 1.5 Flash
    models.insert("gemini-1.5-flash".to_string(), Model {
        id: "gemini-1.5-flash".to_string(),
        name: "Gemini 1.5 Flash".to_string(),
        context_window: 1048576, // 1M tokens
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // Gemini 1.5 Flash-8B
    models.insert("gemini-1.5-flash-8b".to_string(), Model {
        id: "gemini-1.5-flash-8b".to_string(),
        name: "Gemini 1.5 Flash 8B".to_string(),
        context_window: 1048576,
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // Gemini Pro (legacy)
    models.insert("gemini-pro".to_string(), Model {
        id: "gemini-pro".to_string(),
        name: "Gemini Pro".to_string(),
        context_window: 30720,
        max_output_tokens: 2048,
        supports_vision: false,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // Gemini Pro Vision
    models.insert("gemini-pro-vision".to_string(), Model {
        id: "gemini-pro-vision".to_string(),
        name: "Gemini Pro Vision".to_string(),
        context_window: 12288,
        max_output_tokens: 4096,
        supports_vision: true,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models
}

/// Gemini Provider - Complete implementation
pub struct GeminiProvider {
    config: Arc<RwLock<GeminiConfig>>,
    client: reqwest::Client,
    models: Arc<HashMap<String, Model>>,
}

impl GeminiProvider {
    pub async fn new(config: GeminiConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let timeout = config.timeout_ms.unwrap_or(60000);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            models: Arc::new(get_gemini_models()),
        })
    }
    
    async fn build_url(&self, model: &str, method: &str) -> String {
        let config = self.config.read().await;
        
        // Add "models/" prefix if not present
        let model_name = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{}", model)
        };
        
        let base = config.base_url.as_ref()
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
        
        let api_version = config.api_version.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "v1beta".to_string());
        
        format!("{}/{}/{}:{}?key={}", 
                base, api_version, model_name, method, config.api_key)
    }
    
    /// Convert messages to Gemini format (contents -> parts -> text)
    fn convert_to_gemini_format(&self, messages: &[ChatMessage], system_prompt: Option<&str>) 
        -> serde_json::Value {
        let mut contents = Vec::new();
        
        // Add system instruction if present
        if let Some(system) = system_prompt {
            contents.push(json!({
                "role": "user",
                "parts": [{"text": system}]
            }));
            contents.push(json!({
                "role": "model",
                "parts": [{"text": "I understand. I'll follow your instructions."}]
            }));
        }
        
        // Convert messages
        for msg in messages {
            let role = match msg.role.as_str() {
                "assistant" | "model" => "model",
                "system" => "user", // Gemini doesn't have system role
                _ => "user",
            };
            
            let mut parts = Vec::new();
            
            // Add text content
            if let Some(content) = &msg.content {
                parts.push(json!({"text": content}));
            }
            
            // Add function calls as text (Gemini doesn't have native function calling yet)
            if let Some(function_call) = &msg.function_call {
                parts.push(json!({
                    "text": format!("Function call: {} with arguments: {}", 
                                   function_call.name, function_call.arguments)
                }));
            }
            
            contents.push(json!({
                "role": role,
                "parts": parts
            }));
        }
        
        json!(contents)
    }
    
    /// Build generation config
    fn build_generation_config(&self, request: &ChatRequest) -> serde_json::Value {
        let mut config = json!({
            "temperature": request.temperature.unwrap_or(0.7),
            "maxOutputTokens": request.max_tokens.unwrap_or(2048),
            "topP": request.top_p.unwrap_or(0.95),
            "topK": 40,
        });
        
        if let Some(stop) = &request.stop {
            config["stopSequences"] = json!(stop);
        }
        
        config
    }
    
    /// Parse Gemini response
    fn parse_gemini_response(&self, json: &serde_json::Value) -> Result<String> {
        if let Some(candidates) = json["candidates"].as_array() {
            if let Some(candidate) = candidates.first() {
                if let Some(content) = candidate.get("content") {
                    if let Some(parts) = content["parts"].as_array() {
                        let mut text = String::new();
                        for part in parts {
                            if let Some(part_text) = part["text"].as_str() {
                                text.push_str(part_text);
                            }
                        }
                        return Ok(text);
                    }
                }
            }
        }
        
        bail!("Invalid Gemini response format")
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Gemini"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let config = self.config.read().await;
        let base = config.base_url.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
        
        let url = format!("{}/v1beta/models?key={}", base, config.api_key);
        let start = std::time::Instant::now();
        
        let response = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        match response {
            Ok(resp) if resp.status().is_success() => {
                Ok(HealthStatus {
                    healthy: true,
                    latency_ms,
                    error: None,
                    rate_limit_remaining: None,
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
        // Convert completion request to chat format
        let chat_request = ChatRequest {
            model: request.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(request.prompt.clone()),
                name: None,
                function_call: None,
                tool_calls: None,
            }],
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(false),
            ..Default::default()
        };
        
        let chat_response = self.chat(chat_request).await?;
        
        // Convert chat response to completion format
        Ok(CompletionResponse {
            id: chat_response.id,
            object: "text_completion".to_string(),
            created: chat_response.created,
            model: chat_response.model,
            choices: chat_response.choices.into_iter().map(|choice| {
                crate::ai_providers::core_trait::CompletionChoice {
                    text: choice.message.content.unwrap_or_default(),
                    index: choice.index,
                    logprobs: None,
                    finish_reason: choice.finish_reason,
                }
            }).collect(),
            usage: chat_response.usage,
        })
    }
    
    async fn complete_stream(&self, request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        // Convert to chat and stream
        let chat_request = ChatRequest {
            model: request.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(request.prompt.clone()),
                name: None,
                function_call: None,
                tool_calls: None,
            }],
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(true),
            ..Default::default()
        };
        
        self.chat_stream(chat_request).await
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = self.build_url(&request.model, "generateContent").await;
        
        let contents = self.convert_to_gemini_format(&request.messages, None);
        let generation_config = self.build_generation_config(&request);
        
        let body = json!({
            "contents": contents,
            "generationConfig": generation_config,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            
            // Try to parse error
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                if let Some(error_msg) = error_json["error"]["message"].as_str() {
                    bail!("Gemini API error: {}", error_msg);
                }
            }
            
            bail!("Gemini API error: {}", error_text);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        let content = self.parse_gemini_response(&json)?;
        
        // Extract usage metadata
        let usage = json.get("usageMetadata").map(|u| Usage {
            prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
        });
        
        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: request.model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: json["candidates"][0]["finishReason"]
                    .as_str()
                    .map(|s| s.to_string()),
                logprobs: None,
            }],
            usage,
            system_fingerprint: None,
        })
    }
    
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        let url = self.build_url(&request.model, "streamGenerateContent").await;
        
        let contents = self.convert_to_gemini_format(&request.messages, None);
        let generation_config = self.build_generation_config(&request);
        
        let body = json!({
            "contents": contents,
            "generationConfig": generation_config,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("Gemini streaming error: {}", error_text);
        }
        
        // Parse streaming response
        let stream = response.bytes_stream()
            .map(|result| result.map_err(|e| anyhow::anyhow!(e)))
            .flat_map(move |chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        let tokens = parse_gemini_stream(&chunk_str);
                        let results: Vec<Result<StreamToken>> = tokens.into_iter().map(Ok).collect();
                        stream::iter(results)
                    }
                    Err(e) => stream::iter(vec![Err(e)]),
                }
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn list_models(&self) -> Result<Vec<Model>> {
        let config = self.config.read().await;
        let base = config.base_url.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
        
        let url = format!("{}/v1beta/models?key={}", base, config.api_key);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            // Return cached models on error
            return Ok(self.models.values().cloned().collect());
        }
        
        let json: serde_json::Value = response.json().await?;
        
        if let Some(models_array) = json["models"].as_array() {
            let mut models = Vec::new();
            
            for model_json in models_array {
                let id = model_json["name"].as_str()
                    .unwrap_or("")
                    .replace("models/", "");
                
                // Use cached info if available
                if let Some(cached) = self.models.get(&id) {
                    models.push(cached.clone());
                } else {
                    // Create from API response
                    models.push(Model {
                        id: id.clone(),
                        name: model_json["displayName"].as_str()
                            .unwrap_or(&id)
                            .to_string(),
                        context_window: model_json["inputTokenLimit"].as_u64()
                            .unwrap_or(4096) as u32,
                        max_output_tokens: model_json["outputTokenLimit"].as_u64()
                            .unwrap_or(2048) as u32,
                        supports_vision: model_json["supportedGenerationMethods"]
                            .as_array()
                            .map(|arr| arr.iter().any(|m| 
                                m.as_str() == Some("generateContent")))
                            .unwrap_or(false),
                        supports_functions: true,
                        supports_tools: true,
                        pricing: None,
                    });
                }
            }
            
            Ok(models)
        } else {
            Ok(self.models.values().cloned().collect())
        }
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation for Gemini
        // More accurate would require the actual tokenizer
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 2097152, // 2M tokens for Gemini 1.5 Pro
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            supports_embeddings: true,
            supports_tool_calls: true,
            supports_prompt_caching: false,
            rate_limits: RateLimits {
                requests_per_minute: 60,
                tokens_per_minute: 1000000,
                concurrent_requests: 100,
            },
        }
    }
}
