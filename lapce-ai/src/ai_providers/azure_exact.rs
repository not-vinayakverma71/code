/// Azure OpenAI Provider - EXACT port from TypeScript
/// Complete implementation with Azure-specific authentication and endpoints

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
use crate::ai_providers::sse_decoder::{SseDecoder, parsers::parse_openai_sse};

/// Azure OpenAI configuration
#[derive(Debug, Clone)]
pub struct AzureOpenAiConfig {
    pub endpoint: String, // https://{resource-name}.openai.azure.com
    pub api_key: String,
    pub deployment_name: String,
    pub api_version: String,
    pub use_entra_id: bool, // Use Microsoft Entra ID (formerly Azure AD)
    pub timeout_ms: Option<u64>,
}

impl Default for AzureOpenAiConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: String::new(),
            deployment_name: String::new(),
            api_version: "2024-02-01".to_string(),
            use_entra_id: false,
            timeout_ms: Some(60000),
        }
    }
}

/// Azure deployment information
#[derive(Debug, Clone)]
pub struct AzureDeployment {
    pub name: String,
    pub model: String,
    pub model_version: String,
    pub context_window: u32,
    pub max_tokens: u32,
    pub supports_functions: bool,
    pub supports_vision: bool,
}

/// Azure OpenAI Provider
pub struct AzureOpenAiProvider {
    config: Arc<RwLock<AzureOpenAiConfig>>,
    client: reqwest::Client,
    deployments: Arc<HashMap<String, AzureDeployment>>,
}

impl AzureOpenAiProvider {
    pub async fn new(config: AzureOpenAiConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        if config.use_entra_id {
            // Use Bearer token for Entra ID
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", config.api_key))?,
            );
        } else {
            // Use api-key header for API key auth
            headers.insert("api-key", HeaderValue::from_str(&config.api_key)?);
        }
        
        let timeout = config.timeout_ms.unwrap_or(60000);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;
        
        let deployments = Self::initialize_deployments();
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            deployments: Arc::new(deployments),
        })
    }
    
    fn initialize_deployments() -> HashMap<String, AzureDeployment> {
        let mut deployments = HashMap::new();
        
        // GPT-4 deployments
        deployments.insert("gpt-4".to_string(), AzureDeployment {
            name: "gpt-4".to_string(),
            model: "gpt-4".to_string(),
            model_version: "0613".to_string(),
            context_window: 8192,
            max_tokens: 4096,
            supports_functions: true,
            supports_vision: false,
        });
        
        deployments.insert("gpt-4-turbo".to_string(), AzureDeployment {
            name: "gpt-4-turbo".to_string(),
            model: "gpt-4".to_string(),
            model_version: "1106-preview".to_string(),
            context_window: 128000,
            max_tokens: 4096,
            supports_functions: true,
            supports_vision: true,
        });
        
        deployments.insert("gpt-4o".to_string(), AzureDeployment {
            name: "gpt-4o".to_string(),
            model: "gpt-4o".to_string(),
            model_version: "2024-05-13".to_string(),
            context_window: 128000,
            max_tokens: 4096,
            supports_functions: true,
            supports_vision: true,
        });
        
        // GPT-3.5 deployments
        deployments.insert("gpt-35-turbo".to_string(), AzureDeployment {
            name: "gpt-35-turbo".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            model_version: "0613".to_string(),
            context_window: 16385,
            max_tokens: 4096,
            supports_functions: true,
            supports_vision: false,
        });
        
        deployments
    }
    
    async fn build_url(&self, operation: &str) -> String {
        let config = self.config.read().await;
        let endpoint = config.endpoint.trim_end_matches('/');
        
        format!(
            "{}/openai/deployments/{}/{}?api-version={}",
            endpoint, config.deployment_name, operation, config.api_version
        )
    }
    
    fn convert_request(&self, req: &ChatRequest) -> serde_json::Value {
        let mut messages = Vec::new();
        
        for msg in &req.messages {
            let mut message = json!({
                "role": msg.role,
            });
            
            if let Some(content) = &msg.content {
                message["content"] = json!(content);
            }
            
            if let Some(name) = &msg.name {
                message["name"] = json!(name);
            }
            
            if let Some(function_call) = &msg.function_call {
                message["function_call"] = json!({
                    "name": function_call.name,
                    "arguments": function_call.arguments,
                });
            }
            
            if let Some(tool_calls) = &msg.tool_calls {
                message["tool_calls"] = json!(tool_calls);
            }
            
            messages.push(message);
        }
        
        let mut body = json!({
            "messages": messages,
        });
        
        // Add optional parameters
        if let Some(temp) = req.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(max_tokens) = req.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        
        if let Some(top_p) = req.top_p {
            body["top_p"] = json!(top_p);
        }
        
        if let Some(stop) = &req.stop {
            body["stop"] = json!(stop);
        }
        
        if let Some(stream) = req.stream {
            body["stream"] = json!(stream);
        }
        
        if let Some(functions) = &req.functions {
            body["functions"] = json!(functions);
        }
        
        if let Some(function_call) = &req.function_call {
            body["function_call"] = function_call.clone();
        }
        
        if let Some(tools) = &req.tools {
            body["tools"] = json!(tools);
        }
        
        if let Some(tool_choice) = &req.tool_choice {
            body["tool_choice"] = tool_choice.clone();
        }
        
        body
    }
}

#[async_trait]
impl AiProvider for AzureOpenAiProvider {
    fn name(&self) -> &'static str {
        "Azure OpenAI"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        // Try to get deployment info
        let config = self.config.read().await;
        let url = format!(
            "{}/openai/deployments/{}?api-version={}",
            config.endpoint, config.deployment_name, config.api_version
        );
        
        let start = std::time::Instant::now();
        let response = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;
        
        match response {
            Ok(resp) => {
                let healthy = resp.status().is_success() || resp.status() == 404;
                Ok(HealthStatus {
                    healthy,
                    latency_ms,
                    error: if !healthy { 
                        Some(format!("HTTP {}", resp.status())) 
                    } else { 
                        None 
                    },
                    rate_limit_remaining: resp.headers()
                        .get("x-ratelimit-remaining-requests")
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
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Azure OpenAI uses chat completions API
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
        let url = self.build_url("chat/completions").await;
        let mut body = self.convert_request(&request);
        body["stream"] = json!(false);
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            
            // Try to parse error
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                if let Some(error) = error_json.get("error") {
                    if let Some(msg) = error["message"].as_str() {
                        bail!("Azure OpenAI error: {}", msg);
                    }
                }
            }
            
            bail!("Azure OpenAI error: {}", error_text);
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
                            function_call: None,
                            tool_calls: None,
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
        let url = self.build_url("chat/completions").await;
        let mut body = self.convert_request(&request);
        body["stream"] = json!(true);
        
        let response = self.client.post(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            bail!("Azure OpenAI streaming error: {}", error_text);
        }
        
        // Parse SSE stream (OpenAI format)
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
        // Return our cached deployments as models
        Ok(self.deployments.values().map(|d| Model {
            id: d.name.clone(),
            name: format!("{} ({})", d.model, d.model_version),
            context_window: d.context_window,
            max_output_tokens: d.max_tokens,
            supports_vision: d.supports_vision,
            supports_functions: d.supports_functions,
            supports_tools: d.supports_functions,
            pricing: None,
        }).collect())
    }
    
    async fn count_tokens(&self, text: &str) -> Result<usize> {
        // Rough approximation
        Ok(text.len() / 4)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 128000,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            supports_embeddings: true,
            supports_tool_calls: true,
            supports_prompt_caching: false,
            rate_limits: RateLimits {
                requests_per_minute: 300,
                tokens_per_minute: 240000,
                concurrent_requests: 100,
            },
        }
    }
}
