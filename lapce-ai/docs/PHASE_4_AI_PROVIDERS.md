# Phase 4: AI Provider Implementation (2 weeks)
## 8 Strategic Providers with Minimal Memory Footprint

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Provider Parity**: All 8 providers work IDENTICALLY to Codex
- [ ] **Memory Target**: < 8MB for all providers combined
- [ ] **Streaming**: EXACT SSE format per provider (OpenAI, Claude, etc.)
- [ ] **Response Time**: < 5ms dispatch overhead per provider
- [ ] **Error Handling**: SAME error messages as Codex (character-match)
- [ ] **Auth Methods**: Preserve all quirks (Bearer, API keys, headers)
- [ ] **Rate Limiting**: Adaptive throttling without breaking requests
- [ ] **Load Test**: Handle 1K concurrent requests across providers

⚠️ **GATE**: Phase 5 starts ONLY when AI responses are INDISTINGUISHABLE from Codex.

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED: TYPESCRIPT → RUST TRANSLATION ONLY
**YEARS OF PRODUCTION-TESTED PROVIDER LOGIC - TRANSLATE EXACTLY**

**LINE-BY-LINE TRANSLATION** from `/home/verma/lapce/Codex/providers/`:
- `openai.ts` → `openai.rs` - SAME logic, different syntax
- `anthropic.ts` → `anthropic.rs` - SAME quirks preserved
- `gemini.ts` → `gemini.rs` - SAME structure maintained
- `bedrock.ts` → `bedrock.rs` - SAME AWS handling
- `openrouter.ts` → `openrouter.rs` - SAME routing logic
- `xai.ts` → `xai.rs` - SAME format
- `base-provider.ts` → `base_provider.rs` - SAME interface

**TRANSLATION RULES**:
1. Copy function by function
2. TypeScript types → Rust types
3. async/await → async/await (same!)
4. Same variable names (snake_case)
5. Same error messages
6. Same retry logic
7. Same timeout values
8. Same EVERYTHING - just Rust syntax

**DO NOT**: Optimize, improve, refactor, or change ANYTHING except syntax

### Week 1: Core Provider Architecture & Primary Providers
**Goal:** Implement unified provider trait with 4 primary providers
**Memory Target:** < 8MB for all providers combined

### Unified Provider Trait System
```rust
use async_trait::async_trait;
use futures::stream::{Stream, BoxStream};

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn max_tokens(&self) -> usize;
    fn supports_streaming(&self) -> bool;
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> Result<BoxStream<'_, Result<Token>>>;
    
    // Resource management
    fn memory_estimate(&self) -> usize;
    fn connection_pool_size(&self) -> usize;
}

pub struct ProviderManager {
    providers: DashMap<String, Arc<dyn AIProvider>>,
    connection_pool: Arc<ConnectionPool>,
    rate_limiters: DashMap<String, Arc<RateLimiter>>,
    // Single shared buffer for all providers
    shared_buffer: Arc<Mutex<BytesMut>>,
}

impl ProviderManager {
    pub fn with_minimal_memory() -> Self {
        let pool = ConnectionPool::builder()
            .max_connections(10)  // Shared across all providers
            .idle_timeout(Duration::from_secs(30))
            .connection_reuse(true)
            .build();
            
        Self {
            providers: DashMap::new(),
            connection_pool: Arc::new(pool),
            rate_limiters: DashMap::new(),
            shared_buffer: Arc::new(Mutex::new(BytesMut::with_capacity(64 * 1024))),
        }
    }
}
```

### Provider 1: OpenAI (EXACT PORT FROM openai.ts)
```rust
// READ: codex-reference/providers/openai.ts
// CRITICAL: Streaming format MUST be:
// data: {"choices":[{"delta":{"content":"text"}}]}
// data: [DONE]

pub struct OpenAIProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
    model_cache: Arc<DashMap<String, ModelInfo>>,
}

impl OpenAIProvider {
    pub fn new(api_key: String, pool: Arc<ConnectionPool>) -> Self {
        Self {
            client: pool.get_or_create_client("https://api.openai.com"),
            api_key: SecureString::from(api_key),
            model_cache: Arc::new(DashMap::new()),
        }
    }
    
    async fn stream_sse(&self, request: CompletionRequest) -> Result<BoxStream<'_, Result<Token>>> {
        let response = self.client
            .post("/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": request.model,
                "messages": request.messages,
                "stream": true,
                "temperature": request.temperature,
            }))
            .send()
            .await?;
            
        // Zero-copy SSE parsing
        let stream = response.bytes_stream()
            .map(|chunk| {
                // Parse SSE without allocating strings
                parse_sse_chunk_zero_copy(&chunk?)
            });
            
        Ok(Box::pin(stream))
    }
}
```
**Memory: ~1MB**

### Provider 2: Anthropic (EXACT PORT FROM anthropic.ts)
```rust
// READ: codex-reference/providers/anthropic.ts
// CRITICAL: 
// - Message format: "Human: {msg}\n\nAssistant:"
// - Headers: anthropic-version: 2023-06-01
// - Cache control: anthropic-beta headers

pub struct AnthropicProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
    // Reuse buffer for message formatting
    format_buffer: Arc<Mutex<String>>,
}

impl AnthropicProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Anthropic's unique message format
        let mut buffer = self.format_buffer.lock().await;
        buffer.clear();
        
        for msg in &request.messages {
            write!(&mut buffer, "\n\n{}: {}", 
                if msg.role == "user" { "Human" } else { "Assistant" },
                msg.content
            )?;
        }
        buffer.push_str("\n\nAssistant:");
        
        let response = self.client
            .post("/v1/messages")
            .header("x-api-key", &self.api_key.expose())
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": request.model,
                "messages": request.messages,
                "max_tokens": request.max_tokens,
            }))
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
}
```
**Memory: ~1MB**

### Provider 3: Google Gemini
```rust
pub struct GeminiProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
    // Gemini-specific token counter
    token_counter: Arc<GeminiTokenCounter>,
}

impl GeminiProvider {
    async fn stream(&self, request: CompletionRequest) -> Result<BoxStream<'_, Result<Token>>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1/models/{}:streamGenerateContent",
            request.model
        );
        
        let response = self.client
            .post(&url)
            .query(&[("key", &self.api_key.expose())])
            .json(&json!({
                "contents": self.convert_messages(&request.messages),
                "generationConfig": {
                    "temperature": request.temperature,
                    "maxOutputTokens": request.max_tokens,
                }
            }))
            .send()
            .await?;
            
        // Gemini uses different streaming format
        let stream = response.bytes_stream()
            .map(|chunk| self.parse_gemini_stream(&chunk?));
            
        Ok(Box::pin(stream))
    }
}
```
**Memory: ~1MB**

### Provider 4: AWS Bedrock
```rust
use aws_sdk_bedrockruntime::{Client as BedrockClient, types::*};
use aws_config::BehaviorVersion;

pub struct BedrockProvider {
    client: Arc<BedrockClient>,
    model_registry: Arc<DashMap<String, ModelConfig>>,
}

impl BedrockProvider {
    pub async fn new() -> Result<Self> {
        // Use minimal AWS SDK configuration
        let config = aws_config::defaults(BehaviorVersion::latest())
            .retry_config(RetryConfig::standard().with_max_attempts(2))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(30))
                    .build()
            )
            .load()
            .await;
            
        Ok(Self {
            client: Arc::new(BedrockClient::new(&config)),
            model_registry: Arc::new(Self::load_model_registry()),
        })
    }
    
    async fn invoke_model(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Bedrock requires model-specific payloads
        let payload = match request.model.as_str() {
            model if model.starts_with("anthropic.claude") => {
                self.format_claude_payload(&request)
            }
            model if model.starts_with("amazon.titan") => {
                self.format_titan_payload(&request)
            }
            _ => return Err(anyhow!("Unsupported model")),
        };
        
        let response = self.client
            .invoke_model()
            .model_id(&request.model)
            .body(Blob::new(payload))
            .send()
            .await?;
            
        self.parse_bedrock_response(response.body())
    }
}
```
**Memory: ~2MB (AWS SDK overhead)**

### Week 2: Secondary Providers & Optimization

### Provider 5: Azure OpenAI
```rust
pub struct AzureOpenAIProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
    endpoint: String,
    deployment_id: String,
}

impl AzureOpenAIProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-02-01",
            self.endpoint, self.deployment_id
        );
        
        self.client
            .post(&url)
            .header("api-key", &self.api_key.expose())
            .json(&request)
            .send()
            .await?
            .json()
            .await
    }
}
```
**Memory: ~0.5MB**

### Provider 6: OpenRouter (Multi-Model Router)
```rust
pub struct OpenRouterProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
    model_selector: Arc<ModelSelector>,
}

impl OpenRouterProvider {
    async fn route_request(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // OpenRouter automatically selects best model
        let response = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://lapce.dev")
            .json(&json!({
                "model": request.model.is_empty()
                    .then(|| "auto")
                    .unwrap_or(&request.model),
                "messages": request.messages,
                "stream": request.stream,
            }))
            .send()
            .await?;
            
        response.json().await
    }
}
```
**Memory: ~0.5MB**

### Provider 7: XAI (Grok)
```rust
pub struct XAIProvider {
    client: Arc<HttpClient>,
    api_key: SecureString,
}

impl XAIProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.client
            .post("https://api.x.ai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?
            .json()
            .await
    }
}
```
**Memory: ~0.5MB**

### Provider 8: Vertex AI (Google Cloud)
```rust
use gcp_auth::AuthenticationManager;

pub struct VertexAIProvider {
    client: Arc<HttpClient>,
    auth_manager: Arc<AuthenticationManager>,
    project_id: String,
    location: String,
}

impl VertexAIProvider {
    pub async fn new(project_id: String, location: String) -> Result<Self> {
        let auth_manager = AuthenticationManager::new()
            .await?;
            
        Ok(Self {
            client: Arc::new(HttpClient::new()),
            auth_manager: Arc::new(auth_manager),
            project_id,
            location,
        })
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let token = self.auth_manager.get_token(&["https://www.googleapis.com/auth/cloud-platform"]).await?;
        
        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
            self.location, self.project_id, self.location, request.model
        );
        
        self.client
            .post(&url)
            .bearer_auth(&token.as_str())
            .json(&json!({
                "instances": [{"content": request.messages}],
                "parameters": {
                    "temperature": request.temperature,
                    "maxOutputTokens": request.max_tokens,
                }
            }))
            .send()
            .await?
            .json()
            .await
    }
}
```
**Memory: ~1.5MB**

## Unified Connection Pool
```rust
pub struct ConnectionPool {
    clients: DashMap<String, Arc<HttpClient>>,
    max_connections: usize,
    idle_timeout: Duration,
}

impl ConnectionPool {
    pub fn get_or_create_client(&self, base_url: &str) -> Arc<HttpClient> {
        self.clients.entry(base_url.to_string())
            .or_insert_with(|| {
                Arc::new(
                    reqwest::Client::builder()
                        .pool_max_idle_per_host(2)  // Minimal connections
                        .pool_idle_timeout(self.idle_timeout)
                        .timeout(Duration::from_secs(30))
                        .use_rustls_tls()
                        .build()
                        .unwrap()
                )
            })
            .clone()
    }
}
```

## Rate Limiting & Error Handling
```rust
use governor::{Quota, RateLimiter};

pub struct ProviderRateLimiter {
    limiters: DashMap<String, Arc<RateLimiter<NotKeyed, InMemoryState>>>,
}

impl ProviderRateLimiter {
    pub async fn check_rate_limit(&self, provider: &str) -> Result<()> {
        let limiter = self.limiters.entry(provider.to_string())
            .or_insert_with(|| {
                Arc::new(RateLimiter::direct(
                    Quota::per_minute(NonZeroU32::new(60).unwrap())
                ))
            });
            
        limiter.until_ready().await;
        Ok(())
    }
}

// Exponential backoff with jitter
pub struct RetryPolicy {
    max_retries: u32,
    base_delay: Duration,
}

impl RetryPolicy {
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut delay = self.base_delay;
        
        for attempt in 0..self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.max_retries - 1 => {
                    // Add jitter
                    let jitter = rand::random::<f64>() * delay.as_secs_f64() * 0.1;
                    tokio::time::sleep(delay + Duration::from_secs_f64(jitter)).await;
                    delay *= 2;
                }
                Err(e) => return Err(e),
            }
        }
        
        unreachable!()
    }
}
```

## Memory Optimization Techniques
1. **Shared Connection Pool**: All providers share 10 connections max
2. **Buffer Reuse**: Single 64KB buffer for all formatting
3. **Lazy Loading**: Models loaded only when used
4. **Token Streaming**: No buffering, direct stream processing
5. **Zero-Copy Parsing**: Parse responses without string allocation

## Dependencies
```toml
[dependencies]
# HTTP clients
reqwest = { version = "0.12", features = ["stream", "rustls-tls", "json"] }

# AWS Bedrock
aws-sdk-bedrockruntime = "1.47"
aws-config = "1.5"

# Google Cloud Auth
gcp_auth = "0.12"

# Rate limiting
governor = "0.6"

# Secure string handling
secstr = "0.5"
```

## Expected Results - Phase 4
- **Total Memory**: 8MB for all 8 providers
- **Connection Overhead**: 2MB (shared pool)
- **Response Latency**: < 5ms provider dispatch
- **Streaming Performance**: Zero-allocation token processing
- **Error Recovery**: Automatic retry with exponential backoff

## Testing Strategy
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_usage() {
        let manager = ProviderManager::with_minimal_memory();
        
        // Add all providers
        manager.add_provider("openai", OpenAIProvider::new(...));
        manager.add_provider("anthropic", AnthropicProvider::new(...));
        // ... add all 8
        
        // Measure memory
        let memory = get_process_memory();
        assert!(memory < 10 * 1024 * 1024, "Providers using > 10MB");
    }
    
    #[tokio::test]
    async fn test_concurrent_providers() {
        // Test all providers can work simultaneously
        let futures = vec![
            provider1.complete(request.clone()),
            provider2.complete(request.clone()),
            // ... all 8
        ];
        
        let results = futures::future::join_all(futures).await;
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
```
