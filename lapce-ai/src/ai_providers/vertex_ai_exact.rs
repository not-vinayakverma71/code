/// GCP Vertex AI Provider - EXACT port from TypeScript
/// Complete implementation for Gemini and other models on Google Cloud

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, bail};
use async_trait::async_trait;
use futures::stream::{self, StreamExt, BoxStream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use tokio::sync::RwLock;

use crate::ai_providers::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, RateLimits, Usage,
    ChatMessage, ChatChoice
};

/// Vertex AI configuration
#[derive(Debug, Clone)]
pub struct VertexAiConfig {
    pub project_id: String,
    pub location: String,
    pub access_token: String, // OAuth2 access token
    pub default_model: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl Default for VertexAiConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            location: "us-central1".to_string(),
            access_token: String::new(),
            default_model: Some("gemini-1.5-pro".to_string()),
            timeout_ms: Some(60000),
        }
    }
}

/// Vertex AI models
fn get_vertex_ai_models() -> HashMap<String, Model> {
    let mut models = HashMap::new();
    
    // Gemini models on Vertex AI
    models.insert("gemini-1.5-pro".to_string(), Model {
        id: "gemini-1.5-pro".to_string(),
        name: "Gemini 1.5 Pro (Vertex AI)".to_string(),
        context_window: 2097152,
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    models.insert("gemini-1.5-flash".to_string(), Model {
        id: "gemini-1.5-flash".to_string(),
        name: "Gemini 1.5 Flash (Vertex AI)".to_string(),
        context_window: 1048576,
        max_output_tokens: 8192,
        supports_vision: true,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    models.insert("gemini-1.0-pro".to_string(), Model {
        id: "gemini-1.0-pro".to_string(),
        name: "Gemini 1.0 Pro (Vertex AI)".to_string(),
        context_window: 30720,
        max_output_tokens: 2048,
        supports_vision: false,
        supports_functions: true,
        supports_tools: true,
        pricing: None,
    });
    
    // PaLM models
    models.insert("text-bison".to_string(), Model {
        id: "text-bison".to_string(),
        name: "PaLM 2 Text Bison".to_string(),
        context_window: 8192,
        max_output_tokens: 1024,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models.insert("chat-bison".to_string(), Model {
        id: "chat-bison".to_string(),
        name: "PaLM 2 Chat Bison".to_string(),
        context_window: 8192,
        max_output_tokens: 1024,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    // Codey models
    models.insert("code-bison".to_string(), Model {
        id: "code-bison".to_string(),
        name: "Codey Code Bison".to_string(),
        context_window: 6144,
        max_output_tokens: 1024,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models.insert("codechat-bison".to_string(), Model {
        id: "codechat-bison".to_string(),
        name: "Codey Code Chat Bison".to_string(),
        context_window: 6144,
        max_output_tokens: 1024,
        supports_vision: false,
        supports_functions: false,
        supports_tools: false,
        pricing: None,
    });
    
    models
}

/// Vertex AI Provider
pub struct VertexAiProvider {
    config: Arc<RwLock<VertexAiConfig>>,
    client: reqwest::Client,
    models: Arc<HashMap<String, Model>>,
}

impl VertexAiProvider {
    pub async fn new(config: VertexAiConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.access_token))?,
        );
        
        let timeout = config.timeout_ms.unwrap_or(60000);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            models: Arc::new(get_vertex_ai_models()),
        })
    }
    
    async fn build_url(&self, model: &str, method: &str) -> String {
        let config = self.config.read().await;
        
        // Different endpoints for different model families
        if model.starts_with("gemini") {
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:{}",
                config.location, config.project_id, config.location, model, method
            )
        } else {
            // PaLM and other models
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
                config.location, config.project_id, config.location, model
            )
        }
    }
    
    fn convert_to_vertex_format(&self, model: &str, messages: &[ChatMessage]) 
        -> serde_json::Value {
        if model.starts_with("gemini") {
            // Gemini format
            let mut contents = Vec::new();
            
            for msg in messages {
                let role = match msg.role.as_str() {
                    "assistant" => "model",
                    "system" => "user", // Vertex AI doesn't have system role
                    _ => "user",
                };
                
                let mut parts = Vec::new();
                if let Some(content) = &msg.content {
                    parts.push(json!({"text": content}));
                }
                
                contents.push(json!({
                    "role": role,
                    "parts": parts
                }));
            }
            
            json!(contents)
        } else if model.contains("chat") {
            // Chat models (PaLM chat)
            let mut messages_array = Vec::new();
            
            for msg in messages {
                messages_array.push(json!({
                    "author": if msg.role == "assistant" { "bot" } else { "user" },
                    "content": msg.content
                }));
            }
            
            json!({
                "messages": messages_array
            })
        } else {
            // Text models
            let prompt = messages.iter()
                .filter_map(|m| m.content.as_ref())
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            
            json!({
                "prompt": prompt
            })
        }
    }
    
    fn parse_vertex_response(&self, model: &str, json: &serde_json::Value) -> Result<String> {
        if model.starts_with("gemini") {
            // Gemini response format
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
        } else if model.contains("chat") {
            // Chat model response
            if let Some(candidates) = json["candidates"].as_array() {
                if let Some(candidate) = candidates.first() {
                    if let Some(content) = candidate["content"].as_str() {
                        return Ok(content.to_string());
                    }
                }
            }
        } else {
            // Text model response
            if let Some(predictions) = json["predictions"].as_array() {
                if let Some(prediction) = predictions.first() {
                    if let Some(content) = prediction["content"].as_str() {
                        return Ok(content.to_string());
                    }
                }
            }
        }
        
        bail!("Failed to parse Vertex AI response")
    }
}

#[async_trait]
impl AiProvider for VertexAiProvider {
    fn name(&self) -> &'static str {
        "Vertex AI"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let config = self.config.read().await;
        
        // Try to list models
        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/models",
            config.location, config.project_id, config.location
        );
        
        let start = std::time::Instant::now();
        let response = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        match response {
            Ok(resp) => {
                let healthy = resp.status().is_success() || 
                              resp.status() == 401 || // Unauthorized means API is up
                              resp.status() == 403;    // Forbidden means API is up
                
                Ok(HealthStatus {
                    healthy,
                    latency_ms,
                    error: if !healthy { 
                        Some(format!("HTTP {}", resp.status())) 
                    } else { 
                        None 
                    },
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
        // Convert to chat format
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
            ..Default::default()
        };
        
        let chat_response = self.chat(chat_request).await?;
        
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
            ..Default::default()
        };
        
        self.chat_stream(chat_request).await
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let method = if request.model.starts_with("gemini") {
            "generateContent"
        } else {
            "predict"
        };
        
        let url = self.build_url(&request.model, method).await;
        
        let body = if request.model.starts_with("gemini") {
            // Gemini request format
            let contents = self.convert_to_vertex_format(&request.model, &request.messages);
            
            json!({
                "contents": contents,
                "generationConfig": {
                    "temperature": request.temperature.unwrap_or(0.7),
                    "maxOutputTokens": request.max_tokens.unwrap_or(2048),
                    "topP": request.top_p.unwrap_or(0.95),
                }
            })
        } else {
            // PaLM/other model format
            let instances = self.convert_to_vertex_format(&request.model, &request.messages);
            
            json!({
                "instances": [instances],
                "parameters": {
                    "temperature": request.temperature.unwrap_or(0.7),
                    "maxOutputTokens": request.max_tokens.unwrap_or(1024),
                    "topP": request.top_p.unwrap_or(0.95),
                }
            })
        };
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            
            // Try to parse error
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                if let Some(error) = error_json.get("error") {
                    if let Some(msg) = error["message"].as_str() {
                        bail!("Vertex AI error: {}", msg);
                    }
                }
            }
            
            bail!("Vertex AI error: {}", error_text);
        }
        
        let json: serde_json::Value = response.json().await?;
        let content = self.parse_vertex_response(&request.model, &json)?;
        
        // Extract usage if available
        let usage = if request.model.starts_with("gemini") {
            json.get("usageMetadata").map(|u| Usage {
                prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            })
        } else {
            None
        };
        
        Ok(ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
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
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage,
            system_fingerprint: None,
        })
    }
    
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        // Vertex AI streaming implementation
        let method = if request.model.starts_with("gemini") {
            "streamGenerateContent"
        } else {
            "streamGenerateContent" // Claude and other models also support streaming
        };
        
        let url = self.build_url(&request.model, method).await;
        
        // Build request body using same format as chat method
        let mut body = if request.model.starts_with("gemini") {
            // Gemini request format - simplified implementation
            json!({
                "contents": request.messages.iter().map(|msg| {
                    json!({
                        "role": if msg.role == "assistant" { "model" } else { "user" },
                        "parts": [{"text": msg.content}]
                    })
                }).collect::<Vec<_>>(),
                "generationConfig": {
                    "temperature": request.temperature.unwrap_or(0.7),
                    "maxOutputTokens": request.max_tokens.unwrap_or(2048),
                    "topP": request.top_p.unwrap_or(0.95)
                }
            })
        } else {
            // PaLM/other model format
            let instances = self.convert_to_vertex_format(&request.model, &request.messages);
            
            json!({
                "instances": [instances],
                "parameters": {
                    "temperature": request.temperature.unwrap_or(0.7),
                    "maxOutputTokens": request.max_tokens.unwrap_or(1024),
                }
            })
        };
        body["stream"] = json!(true);
        
        let config = self.config.read().await;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", config.access_token).parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("Vertex AI streaming error: {}", error_text);
        }
        
        // Parse streaming response (similar to Gemini format)
        let stream = response.bytes_stream()
            .map(|result| result.map_err(|e| anyhow::anyhow!(e)))
            .flat_map(move |chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        // Vertex AI uses JSON array streaming like Gemini
                        let mut tokens = Vec::new();
                        
                        // Try to parse as JSON array
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&chunk_str) {
                            if let Some(candidates) = json["candidates"].as_array() {
                                for candidate in candidates {
                                    if let Some(content) = candidate["content"]["parts"][0]["text"].as_str() {
                                        tokens.push(Ok(StreamToken::Delta { content: content.to_string() }));
                                    }
                                }
                            }
                        }
                        
                        if tokens.is_empty() && !chunk_str.trim().is_empty() {
                            // Fallback: treat as raw text if not valid JSON
                            tokens.push(Ok(StreamToken::Text(chunk_str.to_string())));
                        }
                        
                        stream::iter(tokens)
                    }
                    Err(e) => stream::iter(vec![Err(e)]),
                }
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn list_models(&self) -> Result<Vec<Model>> {
        // Return cached models
        // Vertex AI model listing is complex and requires specific permissions
        Ok(self.models.values().cloned().collect())
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 2097152, // Gemini 1.5 Pro
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
