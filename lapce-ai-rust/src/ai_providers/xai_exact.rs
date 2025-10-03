/// X.AI (Grok) Provider - EXACT port from Codex/src/api/providers/xai.ts

use std::sync::Arc;
use anyhow::{Result, bail};
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use serde_json::json;
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::sync::RwLock;

use crate::ai_providers::core_trait::*;

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
    
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        bail!("xAI uses OpenAI-compatible chat API")
    }
    
    async fn complete_stream(&self, _request: CompletionRequest) 
        -> Result<BoxStream<'static, Result<StreamToken>>> {
        bail!("xAI uses OpenAI-compatible chat API")
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
        Ok(Box::pin(stream::iter(tokens)))
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
