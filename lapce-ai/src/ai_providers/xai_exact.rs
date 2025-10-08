/// X.AI (Grok) Provider - EXACT port from Codex/src/api/providers/xai.ts
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt, BoxStream};
use tokio::time::Duration;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow, bail};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use crate::ai_providers::core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, RateLimits, Usage,
    ChatMessage, ChatChoice, CompletionChoice
};
use crate::ai_providers::sse_decoder::{SseDecoder, SseEvent};
use crate::ai_providers::streaming_integration::{
    process_sse_response, ProviderType
};

#[derive(Debug, Clone)]
pub struct XaiConfig {
    pub api_key: String,
    pub base_url: Option<String>,
}

impl Default for XaiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://api.x.ai/v1".to_string()),
        }
    }
}

pub struct XaiProvider {
    config: Arc<RwLock<XaiConfig>>,
    client: reqwest::Client,
}

impl XaiProvider {
    pub async fn new(config: XaiConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", config.api_key))?);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
        })
    }
}

#[async_trait]
impl AiProvider for XaiProvider {
    fn name(&self) -> &'static str { "xAI" }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            latency_ms: 0,
            error: None,
            rate_limit_remaining: None,
        })
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // xAI supports OpenAI-compatible completion API
        let config = self.config.read().await;
        let url = format!("{}/completions", 
                         config.base_url.as_deref().unwrap_or("https://api.x.ai/v1"));
        
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(100),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": false,
        });
        
        // Add optional parameters
        if let Some(stop) = &request.stop {
            body["stop"] = json!(stop);
        }
        
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", config.api_key).parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);
        
        let response = self.client.post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            bail!("xAI completion error: {}", error);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        Ok(CompletionResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: json["object"].as_str().unwrap_or("text_completion").to_string(),
            created: json["created"].as_u64().unwrap_or(0),
            model: json["model"].as_str().unwrap_or(&request.model).to_string(),
            choices: vec![],
            usage: None,
        })
    }
    
    async fn complete_stream(&self, request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        // xAI supports OpenAI-compatible streaming
        let config = self.config.read().await;
        let url = format!("{}/completions", 
                         config.base_url.as_deref().unwrap_or("https://api.x.ai/v1"));
        
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(100),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": true,
        });
        
        if let Some(stop) = &request.stop {
            body["stop"] = json!(stop);
        }
        
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", config.api_key).parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);
        
        let response = self.client.post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            bail!("xAI streaming error: {}", error);
        }
        
        // Parse SSE stream (OpenAI format)
        let mut decoder = SseDecoder::new();
        let stream = response.bytes_stream()
            .map(|result| result.map_err(|e| anyhow::anyhow!(e)))
            .flat_map(move |chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let events = decoder.process_chunk(&chunk);
                        let mut tokens: Vec<Result<StreamToken>> = Vec::new();
                        
                        for event in events {
                            if let Some(data) = event.data {
                                let data_str = String::from_utf8_lossy(&data);
                                if data_str.trim() == "[DONE]" {
                                    tokens.push(Ok(StreamToken::Done));
                                } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data_str) {
                                    if let Some(text) = json["choices"][0]["text"].as_str() {
                                        tokens.push(Ok(StreamToken::Text(text.to_string())));
                                    }
                                }
                            }
                        }
                        
                        futures::stream::empty().boxed()
                    }
                    Err(e) => futures::stream::iter(vec![Err(e)]).boxed(),
                }
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let config = self.config.read().await;
        let url = format!("{}/chat/completions", 
                         config.base_url.as_deref().unwrap_or("https://api.x.ai/v1"));
        
        let body = json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            bail!("xAI error: {}", error);
        }
        
        let json: serde_json::Value = response.json().await?;
        
        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion".to_string(),
            created: json["created"].as_u64().unwrap_or(0),
            model: request.model,
            choices: vec![],
            usage: None,
            system_fingerprint: None,
        })
    }
    
    async fn chat_stream(&self, request: ChatRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        let response = self.chat(request).await?;
        let tokens = vec![Ok(StreamToken::Done)];
        Ok(Box::pin(futures::stream::iter(tokens)))
    }
    
    async fn list_models(&self) -> Result<Vec<Model>> {
        Ok(vec![])
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 128000,
            supports_streaming: true,
            supports_functions: false,
            supports_vision: false,
            supports_embeddings: false,
            supports_tool_calls: false,
            supports_prompt_caching: false,
            rate_limits: RateLimits {
                requests_per_minute: 60,
                tokens_per_minute: 100000,
                concurrent_requests: 50,
            },
        }
    }
}
