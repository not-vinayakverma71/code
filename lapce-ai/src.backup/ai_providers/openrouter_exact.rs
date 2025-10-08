/// OpenRouter Provider - EXACT port of Codex OpenRouter handler
/// Implements full production-grade integration with https://openrouter.ai API

use std::str;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use futures::stream::BoxStream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tiktoken_rs::cl100k_base;

use crate::ai_providers::core_trait::{
    AiProvider, ChatChoice, ChatMessage, ChatRequest, ChatResponse, CompletionChoice,
    CompletionRequest, CompletionResponse, Function, HealthStatus, Model,
    ModelPricing, ProviderCapabilities, RateLimits, StreamToken, Tool, Usage,
};
use crate::ai_providers::sse_decoder::{SseDecoder, SseEvent};

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";
const DEFAULT_MODEL: &str = "openrouter/auto";
const DEFAULT_REFERER: &str = "https://lapce.dev";
const DEFAULT_APP_TITLE: &str = "Lapce IDE";

/// Provider configuration derived from TypeScript options
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub specific_provider: Option<String>,
    pub allow_fallbacks: bool,
    pub data_collection: Option<String>,
    pub provider_sort: Option<String>,
    pub use_middle_out_transform: bool,
    pub timeout_ms: u64,
    pub referer: String,
    pub app_title: String,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            default_model: DEFAULT_MODEL.to_string(),
            specific_provider: None,
            allow_fallbacks: true,
            data_collection: None,
            provider_sort: None,
            use_middle_out_transform: true,
            timeout_ms: 30_000,
            referer: DEFAULT_REFERER.to_string(),
            app_title: DEFAULT_APP_TITLE.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OpenRouterModelInfo {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context_length: Option<u64>,
    #[serde(default)]
    architecture: Option<ArchitectureInfo>,
    #[serde(default)]
    pricing: Option<PricingInfo>,
    #[serde(default)]
    capabilities: Option<CapabilitiesInfo>,
}

#[derive(Debug, Clone, Deserialize)]
struct ArchitectureInfo {
    #[serde(default)]
    max_tokens: Option<u64>,
    #[serde(default)]
    vision: Option<bool>,
    #[serde(default)]
    tool_calls: Option<bool>,
    #[serde(default)]
    functions: Option<bool>,
    #[serde(default)]
    embeddings: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct PricingInfo {
    #[serde(default)]
    prompt: Option<f64>,
    #[serde(default)]
    completion: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct CapabilitiesInfo {
    #[serde(default)]
    vision: Option<bool>,
    #[serde(default)]
    tool_calls: Option<bool>,
    #[serde(default)]
    functions: Option<bool>,
    #[serde(default)]
    embeddings: Option<bool>,
    #[serde(default)]
    prompt_caching: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelsResponse {
    data: Vec<OpenRouterModelInfo>,
}

pub struct OpenRouterProvider {
    client: Client,
    config: Arc<RwLock<OpenRouterConfig>>,
    model_cache: Arc<RwLock<Vec<Model>>>,
}

impl OpenRouterProvider {
    pub async fn new(mut config: OpenRouterConfig) -> Result<Self> {
        if config.base_url.trim().is_empty() {
            config.base_url = DEFAULT_BASE_URL.to_string();
        }
        if config.default_model.trim().is_empty() {
            config.default_model = DEFAULT_MODEL.to_string();
        }
        if config.referer.trim().is_empty() {
            config.referer = DEFAULT_REFERER.to_string();
        }
        if config.app_title.trim().is_empty() {
            config.app_title = DEFAULT_APP_TITLE.to_string();
        }

        let timeout = Duration::from_millis(config.timeout_ms);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build OpenRouter HTTP client")?;

        let provider = Self {
            client,
            config: Arc::new(RwLock::new(config)),
            model_cache: Arc::new(RwLock::new(Vec::new())),
        };

        if let Ok(models) = provider.fetch_models().await {
            let mut cache = provider.model_cache.write().await;
            *cache = models;
        }

        Ok(provider)
    }

    async fn auth_headers(&self, include_json: bool) -> Result<HeaderMap> {
        let config = self.config.read().await;

        let mut headers = HeaderMap::new();
        let bearer = format!("Bearer {}", config.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&bearer).context("invalid OpenRouter API key")?,
        );
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_str(&config.referer).context("invalid HTTP-Referer header")?,
        );
        headers.insert(
            "X-Title",
            HeaderValue::from_str(&config.app_title).context("invalid X-Title header")?,
        );

        if include_json {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
        }

        Ok(headers)
    }

    async fn fetch_models(&self) -> Result<Vec<Model>> {
        let config = self.config.read().await;
        let url = format!("{}/models", config.base_url.trim_end_matches('/'));
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers(false).await?)
            .send()
            .await
            .context("failed to fetch OpenRouter models")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter model fetch failed: {}", error_text));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .context("failed to deserialize OpenRouter model list")?;

        let models = models_response
            .data
            .into_iter()
            .map(|info| self.convert_model_info(info))
            .collect::<Vec<_>>();

        Ok(models)
    }

    fn convert_model_info(&self, info: OpenRouterModelInfo) -> Model {
        let context_window = info
            .context_length
            .or_else(|| info.architecture.as_ref().and_then(|a| a.max_tokens))
            .unwrap_or(0) as u32;

        let supports_vision = info
            .capabilities
            .as_ref()
            .and_then(|c| c.vision)
            .or_else(|| info.architecture.as_ref().and_then(|a| a.vision))
            .unwrap_or(false);
        let supports_functions = info
            .capabilities
            .as_ref()
            .and_then(|c| c.functions)
            .or_else(|| info.architecture.as_ref().and_then(|a| a.functions))
            .unwrap_or(true);
        let supports_tool_calls = info
            .capabilities
            .as_ref()
            .and_then(|c| c.tool_calls)
            .or_else(|| info.architecture.as_ref().and_then(|a| a.tool_calls))
            .unwrap_or(true);
        let supports_embeddings = info
            .capabilities
            .as_ref()
            .and_then(|c| c.embeddings)
            .or_else(|| info.architecture.as_ref().and_then(|a| a.embeddings))
            .unwrap_or(false);

        let pricing = info.pricing.map(|pricing| ModelPricing {
            input_per_1k: pricing.prompt.unwrap_or_default(),
            output_per_1k: pricing.completion.unwrap_or_default(),
        });

        Model {
            id: info.id.clone(),
            name: info.name.unwrap_or(info.id),
            context_window,
            max_output_tokens: info
                .architecture
                .and_then(|a| a.max_tokens)
                .unwrap_or(context_window as u64)
                as u32,
            supports_vision,
            supports_functions,
            supports_tools: supports_tool_calls,
            pricing,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn build_chat_messages(&self, request: &ChatRequest) -> Vec<Value> {
        request
            .messages
            .iter()
            .map(|message| {
                let mut value = json!({ "role": message.role });

                if let Some(content) = &message.content {
                    value["content"] = json!(content);
                }

                if let Some(function_call) = &message.function_call {
                    value["function_call"] = json!({
                        "name": function_call.name,
                        "arguments": function_call.arguments,
                    });
                }

                if let Some(tool_calls) = &message.tool_calls {
                    let serialized = tool_calls
                        .iter()
                        .map(|call| {
                            json!({
                                "id": call.id,
                                "type": call.r#type,
                                "function": {
                                    "name": call.function.name,
                                    "arguments": call.function.arguments,
                                }
                            })
                        })
                        .collect::<Vec<_>>();
                    value["tool_calls"] = json!(serialized);
                }

                value
            })
            .collect()
    }

    fn apply_chat_overrides(&self, body: &mut Value, request: &ChatRequest) {
        if let Some(stop) = &request.stop {
            body["stop"] = json!(stop);
        }
        if let Some(functions) = &request.functions {
            body["functions"] = json!(functions.iter().map(|f| function_to_json(f)).collect::<Vec<_>>());
        }
        if let Some(tools) = &request.tools {
            body["tools"] = json!(tools.iter().map(|t| tool_to_json(t)).collect::<Vec<_>>());
        }
        if let Some(response_format) = &request.response_format {
            body["response_format"] = json!(response_format);
        }
    }

    async fn provider_params(&self) -> Value {
        let config = self.config.read().await;

        if let Some(provider) = &config.specific_provider {
            return json!({
                "provider": {
                    "order": [provider],
                    "only": [provider],
                    "allow_fallbacks": config.allow_fallbacks,
                    "data_collection": config.data_collection,
                }
            });
        }

        let mut provider = serde_json::Map::new();
        if let Some(data_collection) = &config.data_collection {
            provider.insert("data_collection".to_string(), json!(data_collection));
        }
        if let Some(sort) = &config.provider_sort {
            provider.insert("sort".to_string(), json!(sort));
        }
        if !provider.is_empty() {
            json!({ "provider": Value::Object(provider) })
        } else {
            json!({})
        }
    }

    async fn merge_body_with_provider_params(&self, mut body: Value) -> Value {
        if let Value::Object(provider_params) = self.provider_params().await {
            if let Value::Object(body_map) = &mut body {
                for (key, value) in provider_params {
                    body_map.insert(key, value);
                }
            }
        }
        body
    }

    async fn post_json(&self, path: &str, body: &Value) -> Result<reqwest::Response> {
        let config = self.config.read().await;
        let url = format!("{}/{}", config.base_url.trim_end_matches('/'), path.trim_start_matches('/'));
        let response = self
            .client
            .post(&url)
            .headers(self.auth_headers(true).await?)
            .json(body)
            .send()
            .await
            .with_context(|| format!("OpenRouter POST {} failed", path))?;
        Ok(response)
    }

    fn build_completion_response(model: &str, json: &Value) -> Result<CompletionResponse> {
        let choices = json["choices"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .enumerate()
            .map(|(index, choice)| CompletionChoice {
                text: choice["message"]["content"].as_str().unwrap_or_default().to_string(),
                index: index as u32,
                logprobs: if choice["logprobs"].is_null() { None } else { Some(choice["logprobs"].clone()) },
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            })
            .collect::<Vec<_>>();

        Ok(CompletionResponse {
            id: json["id"].as_str().unwrap_or_default().to_string(),
            object: json["object"].as_str().unwrap_or("text_completion").to_string(),
            created: json["created"].as_u64().unwrap_or_else(Self::now),
            model: json["model"].as_str().unwrap_or(model).to_string(),
            choices,
            usage: json.get("usage").map(|v| parse_usage(v.clone())),
        })
    }

    fn build_chat_response(model: &str, json: &Value) -> Result<ChatResponse> {
        let choices = json["choices"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .enumerate()
            .map(|(index, choice)| ChatChoice {
                index: index as u32,
                message: ChatMessage {
                    role: choice["message"]["role"].as_str().unwrap_or("assistant").to_string(),
                    content: choice["message"]["content"].as_str().map(|s| s.to_string()),
                    name: choice["message"]["name"].as_str().map(|s| s.to_string()),
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                logprobs: if choice["logprobs"].is_null() { None } else { Some(choice["logprobs"].clone()) },
            })
            .collect::<Vec<_>>();

        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or_default().to_string(),
            object: json["object"].as_str().unwrap_or("chat.completion").to_string(),
            created: json["created"].as_u64().unwrap_or_else(Self::now),
            model: json["model"].as_str().unwrap_or(model).to_string(),
            choices,
            usage: json.get("usage").map(|v| parse_usage(v.clone())),
            system_fingerprint: json["system_fingerprint"].as_str().map(|s| s.to_string()),
        })
    }

    fn parse_stream_event(event: &SseEvent) -> Vec<Result<StreamToken>> {
        let mut tokens = Vec::new();
        if let Some(data) = &event.data {
            if data.as_ref() == b"[DONE]" {
                tokens.push(Ok(StreamToken::Done));
                return tokens;
            }

            if let Ok(data_str) = str::from_utf8(&data) {
                match serde_json::from_str::<Value>(data_str) {
                    Ok(json) => {
                        if json.get("error").is_some() {
                            let error_msg = json["error"]["message"].as_str().unwrap_or(data_str);
                            tokens.push(Ok(StreamToken::Error(error_msg.to_string())));
                        }

                        if let Some(usage) = json.get("usage") {
                            tokens.push(Ok(StreamToken::Event {
                                event_type: "usage".to_string(),
                                data: usage.clone(),
                            }));
                        }

                        if let Some(choices) = json["choices"].as_array() {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(reasoning) = delta.get("reasoning") {
                                        tokens.push(Ok(StreamToken::Event {
                                            event_type: "reasoning".to_string(),
                                            data: reasoning.clone(),
                                        }));
                                    }

                                    if let Some(reasoning_content) = delta.get("reasoning_content") {
                                        tokens.push(Ok(StreamToken::Event {
                                            event_type: "reasoning".to_string(),
                                            data: reasoning_content.clone(),
                                        }));
                                    }

                                    if let Some(content) = delta.get("content") {
                                        if let Some(text) = content.as_str() {
                                            tokens.push(Ok(StreamToken::Delta {
                                                content: text.to_string(),
                                            }));
                                        }
                                    }

                                    if let Some(tool_calls) = delta.get("tool_calls") {
                                        tokens.push(Ok(StreamToken::Event {
                                            event_type: "tool_calls".to_string(),
                                            data: tool_calls.clone(),
                                        }));
                                    }
                                }

                                if let Some(finish_reason) = choice.get("finish_reason") {
                                    tokens.push(Ok(StreamToken::Event {
                                        event_type: "finish_reason".to_string(),
                                        data: finish_reason.clone(),
                                    }));
                                }
                            }
                        }
                    }
                    Err(_) => tokens.push(Ok(StreamToken::Text(data_str.to_string()))),
                }
            }
        }

        tokens
    }

    async fn stream_completion(&self, body: Value) -> Result<BoxStream<'static, Result<StreamToken>>> {
        let response = self.post_json("/chat/completions", &body).await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter streaming error: {}", error_text));
        }

        let mut decoder = SseDecoder::new();
        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(|e| anyhow!(e)))
            .flat_map(move |chunk_result| match chunk_result {
                Ok(chunk) => {
                    let events = decoder.process_chunk(&chunk);
                    let mut tokens = Vec::new();
                    for event in events {
                        tokens.extend(Self::parse_stream_event(&event));
                    }
                    stream::iter(tokens)
                }
                Err(e) => stream::iter(vec![Err(e)]),
            });

        Ok(Box::pin(stream))
    }
}

fn function_to_json(function: &Function) -> Value {
    json!({
        "name": function.name,
        "description": function.description,
        "parameters": function.parameters,
    })
}

fn tool_to_json(tool: &Tool) -> Value {
    json!({
        "type": tool.r#type,
        "function": function_to_json(&tool.function),
    })
}

fn parse_usage(value: Value) -> Usage {
    Usage {
        prompt_tokens: value["prompt_tokens"].as_u64().unwrap_or_default() as u32,
        completion_tokens: value["completion_tokens"].as_u64().unwrap_or_default() as u32,
        total_tokens: value["total_tokens"].as_u64().unwrap_or_default() as u32,
    }
}

#[async_trait]
impl AiProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let start = SystemTime::now();
        let result = self.fetch_models().await;
        let latency = start.elapsed().unwrap_or_default().as_millis() as u64;

        match result {
            Ok(models) => {
                *self.model_cache.write().await = models;
                Ok(HealthStatus {
                    healthy: true,
                    latency_ms: latency,
                    error: None,
                    rate_limit_remaining: None,
                })
            }
            Err(err) => Ok(HealthStatus {
                healthy: false,
                latency_ms: latency,
                error: Some(err.to_string()),
                rate_limit_remaining: None,
            }),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let config = self.config.read().await;
        let model = if request.model.is_empty() {
            config.default_model.clone()
        } else {
            request.model.clone()
        };
        let use_middle_out = config.use_middle_out_transform;
        drop(config);

        let mut body = json!({
            "model": model,
            "messages": [
                { "role": "user", "content": request.prompt },
            ],
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": false,
        });

        if use_middle_out {
            body["transforms"] = json!(["middle-out"]);
        }

        body = self.merge_body_with_provider_params(body).await;

        let response = self.post_json("/chat/completions", &body).await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter completion error: {}", error_text));
        }

        let json: Value = response.json().await.context("invalid OpenRouter completion JSON")?;
        Self::build_completion_response(&model, &json)
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<BoxStream<'static, Result<StreamToken>>> {
        let config = self.config.read().await;
        let model = if request.model.is_empty() {
            config.default_model.clone()
        } else {
            request.model.clone()
        };
        let use_middle_out = config.use_middle_out_transform;
        drop(config);

        let mut body = json!({
            "model": model,
            "messages": [
                { "role": "user", "content": request.prompt },
            ],
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        if use_middle_out {
            body["transforms"] = json!(["middle-out"]);
        }

        body = self.merge_body_with_provider_params(body).await;
        self.stream_completion(body).await
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let config = self.config.read().await;
        let model = if request.model.is_empty() {
            config.default_model.clone()
        } else {
            request.model.clone()
        };
        let use_middle_out = config.use_middle_out_transform;
        drop(config);

        let messages = self.build_chat_messages(&request);

        let mut body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": false,
        });
        self.apply_chat_overrides(&mut body, &request);
        if use_middle_out {
            body["transforms"] = json!(["middle-out"]);
        }
        body = self.merge_body_with_provider_params(body).await;

        let response = self.post_json("/chat/completions", &body).await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter chat error: {}", error_text));
        }

        let json: Value = response.json().await.context("invalid OpenRouter chat JSON")?;
        Self::build_chat_response(&model, &json)
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<StreamToken>>> {
        let config = self.config.read().await;
        let model = if request.model.is_empty() {
            config.default_model.clone()
        } else {
            request.model.clone()
        };
        let use_middle_out = config.use_middle_out_transform;
        drop(config);

        let messages = self.build_chat_messages(&request);

        let mut body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": true,
            "stream_options": { "include_usage": true },
        });
        self.apply_chat_overrides(&mut body, &request);
        if use_middle_out {
            body["transforms"] = json!(["middle-out"]);
        }
        body = self.merge_body_with_provider_params(body).await;

        self.stream_completion(body).await
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        match self.fetch_models().await {
            Ok(models) => {
                *self.model_cache.write().await = models.clone();
                Ok(models)
            }
            Err(err) => {
                let cache = self.model_cache.read().await;
                if cache.is_empty() {
                    Err(err)
                } else {
                    Ok(cache.clone())
                }
            }
        }
    }

    async fn count_tokens(&self, text: &str) -> Result<usize> {
        let bpe = cl100k_base().map_err(|e| anyhow!("failed to load tokenizer: {}", e))?;
        Ok(bpe.encode_with_special_tokens(text).len())
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_tokens: 1_000_000,
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            supports_embeddings: true,
            supports_tool_calls: true,
            supports_prompt_caching: true,
            rate_limits: RateLimits {
                requests_per_minute: 300,
                tokens_per_minute: 1_200_000,
                concurrent_requests: 64,
            },
        }
    }
}
