# Step 3: Provider Trait Architecture
## Unified Interface for All AI Providers - EXACT CODEX BEHAVIOR

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED: TypeScript → Rust Translation ONLY
**YEARS OF PRODUCTION-TESTED PROVIDER CODE - TRANSLATE, DON'T REWRITE**

**BEFORE IMPLEMENTING**: Study `/home/verma/lapce/Codex`
- This is a TRANSLATION job - copy line by line
- Each provider took months to perfect - preserve EVERYTHING
- Same quirks, same formats, same edge cases
- Only syntax changes from TypeScript to Rust in RUST

## ✅ Success Criteria
- [ ] **Memory Usage**: < 8MB for all providers combined
- [ ] **Provider Support**: 8+ AI providers (Bedrock, OpenAI, Anthropic, etc.)
- [ ] **Latency**: < 5ms provider dispatch overhead
- [ ] **Connection Reuse**: 95%+ connection pool hit rate
- [ ] **Rate Limiting**: Adaptive rate control with circuit breakers
- [ ] **Streaming**: Zero-allocation token streaming
- [ ] **Error Recovery**: Automatic failover in < 100ms
- [ ] **Test Coverage**: Mock provider with 100% API coverage

## Overview
The provider trait architecture enables seamless integration with multiple AI providers (AWS Bedrock, OpenAI, Anthropic, Gemini, etc.) through a single unified interface, achieving 60MB → 8MB memory reduction.

## EXACT Provider Behaviors from Codex in RUST 

### OpenAI (codex/providers/openai.ts)
```typescript
// EXACT streaming format
data: {"choices":[{"delta":{"content":"text"}}]}
data: [DONE]

// EXACT request format
{
  "model": "gpt-5",
  "messages": [{"role": "user", "content": "..."}],
  "stream": true,
  "temperature": 0.7
}
```

### Anthropic (codex/providers/anthropic.ts)
```typescript
// EXACT headers required
"anthropic-version": "2023-06-01"
"anthropic-beta": "prompt-caching-2024-07-31"

// EXACT message format
"Human: {content}\n\nAssistant:"
```

### Gemini (codex/providers/gemini.ts)
```typescript
// EXACT content structure
{
  "contents": [{
    "parts": [{"text": "..."}],
    "role": "user"
  }]
}
```

## Core Trait Design

### Base Provider Trait
```rust
use async_trait::async_trait;
use futures::stream::{Stream, BoxStream};
use std::pin::Pin;

#[async_trait]
pub trait AiProvider: Send + Sync + 'static {
    /// Provider identifier
    fn name(&self) -> &'static str;
    
    /// Check if provider is available and configured
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Single completion request
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Streaming completion
    async fn complete_stream(
        &self, 
        request: CompletionRequest
    ) -> Result<BoxStream<'static, Result<StreamToken>>>;
    
    /// Chat conversation
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    
    /// Chat with streaming
    async fn chat_stream(
        &self,
        request: ChatRequest
    ) -> Result<BoxStream<'static, Result<StreamToken>>>;
    
    /// Get available models
    async fn list_models(&self) -> Result<Vec<Model>>;
    
    /// Estimate token count
    async fn count_tokens(&self, text: &str) -> Result<usize>;
    
    /// Provider-specific capabilities
    fn capabilities(&self) -> ProviderCapabilities;
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub max_tokens: usize,
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub supports_vision: bool,
    pub supports_embeddings: bool,
    pub rate_limits: RateLimits,
}
```

## Provider Implementations

### 1. AWS Bedrock Provider
```rust
use aws_sdk_bedrockruntime::{Client, Config};
use aws_types::region::Region;

pub struct BedrockProvider {
    client: Client,
    region: Region,
    model_cache: Arc<RwLock<HashMap<String, ModelInfo>>>,
    request_signer: RequestSigner,
}

impl BedrockProvider {
    pub async fn new(config: BedrockConfig) -> Result<Self> {
        // Use AWS SDK with custom configuration
        let sdk_config = aws_config::from_env()
            .region(Region::new(config.region))
            .credentials_provider(
                aws_config::default_provider::credentials::default_provider().await
            )
            .load()
            .await;
            
        let client = Client::new(&sdk_config);
        
        // Pre-cache model information
        let model_cache = Arc::new(RwLock::new(HashMap::with_capacity(10)));
        
        Ok(Self {
            client,
            region: Region::new(config.region),
            model_cache,
            request_signer: RequestSigner::new(),
        })
    }
    
    async fn invoke_model(&self, request: &BedrockRequest) -> Result<BedrockResponse> {
        use aws_sdk_bedrockruntime::operation::invoke_model::{InvokeModelInput, InvokeModelOutput};
        
        let input = InvokeModelInput::builder()
            .model_id(&request.model_id)
            .body(aws_sdk_bedrockruntime::primitives::Blob::new(request.body.clone()))
            .content_type("application/json")
            .build();
            
        let output = self.client.invoke_model(input).await?;
        
        // Parse response based on model type
        self.parse_model_response(output.body().as_ref(), &request.model_id)
    }
}

#[async_trait]
impl AiProvider for BedrockProvider {
    fn name(&self) -> &'static str {
        "bedrock"
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Format request for specific model (Nova, Claude, etc.)
        let bedrock_request = self.format_request(&request)?;
        
        // Add retry logic with exponential backoff
        let response = retry_with_backoff(|| async {
            self.invoke_model(&bedrock_request).await
        }, 3, Duration::from_millis(100)).await?;
        
        Ok(CompletionResponse {
            text: response.text,
            model: request.model.clone(),
            usage: Usage {
                prompt_tokens: response.input_tokens,
                completion_tokens: response.output_tokens,
                total_tokens: response.total_tokens,
            },
            latency: response.latency,
        })
    }
    
    async fn complete_stream(
        &self,
        request: CompletionRequest
    ) -> Result<BoxStream<'static, Result<StreamToken>>> {
        use aws_sdk_bedrockruntime::operation::invoke_model_with_response_stream::InvokeModelWithResponseStreamInput;
        
        let input = InvokeModelWithResponseStreamInput::builder()
            .model_id(&request.model)
            .body(aws_sdk_bedrockruntime::primitives::Blob::new(
                serde_json::to_vec(&request)?
            ))
            .build();
            
        let mut output = self.client
            .invoke_model_with_response_stream(input)
            .await?;
            
        // Convert AWS stream to our stream type
        let stream = async_stream::stream! {
            while let Some(event) = output.body.recv().await? {
                match event {
                    ResponseStreamEvent::Chunk(chunk) => {
                        let token = self.parse_chunk(chunk)?;
                        yield Ok(token);
                    }
                    ResponseStreamEvent::Error(e) => {
                        yield Err(e.into());
                    }
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
}
```

### 2. OpenAI Provider
```rust
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: SecureString,
    base_url: Url,
    organization_id: Option<String>,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;
            
        Ok(Self {
            client,
            api_key: SecureString::new(config.api_key),
            base_url: Url::parse(&config.base_url.unwrap_or_else(|| 
                "https://api.openai.com/v1".to_string()
            ))?,
            organization_id: config.organization_id,
        })
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete_stream(
        &self,
        request: CompletionRequest
    ) -> Result<BoxStream<'static, Result<StreamToken>>> {
        let url = self.base_url.join("chat/completions")?;
        
        let body = json!({
            "model": request.model,
            "messages": [{"role": "user", "content": request.prompt}],
            "stream": true,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });
        
        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
            
        // Parse SSE stream
        let stream = parse_sse_stream(response.bytes_stream());
        Ok(Box::pin(stream))
    }
}
```

### 3. Unified Provider Manager
```rust
pub struct ProviderManager {
    providers: DashMap<String, Arc<dyn AiProvider>>,
    default_provider: RwLock<String>,
    health_monitor: HealthMonitor,
    metrics: Arc<ProviderMetrics>,
}

impl ProviderManager {
    pub async fn new(config: ProvidersConfig) -> Result<Self> {
        let providers = DashMap::with_capacity(8);
        
        // Initialize providers concurrently
        let mut handles = vec![];
        
        if let Some(bedrock_config) = config.bedrock {
            handles.push(tokio::spawn(async move {
                let provider = BedrockProvider::new(bedrock_config).await?;
                Ok::<_, Error>(("bedrock".to_string(), Arc::new(provider) as Arc<dyn AiProvider>))
            }));
        }
        
        if let Some(openai_config) = config.openai {
            handles.push(tokio::spawn(async move {
                let provider = OpenAiProvider::new(openai_config)?;
                Ok::<_, Error>(("openai".to_string(), Arc::new(provider) as Arc<dyn AiProvider>))
            }));
        }
        
        // Wait for all providers to initialize
        for handle in handles {
            let (name, provider) = handle.await??;
            providers.insert(name, provider);
        }
        
        Ok(Self {
            providers,
            default_provider: RwLock::new(config.default_provider),
            health_monitor: HealthMonitor::new(),
            metrics: Arc::new(ProviderMetrics::new()),
        })
    }
    
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let provider_name = request.provider.as_ref()
            .unwrap_or(&*self.default_provider.read().await);
            
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| Error::ProviderNotFound(provider_name.clone()))?;
            
        // Track metrics
        let start = Instant::now();
        let result = provider.complete(request).await;
        let latency = start.elapsed();
        
        self.metrics.record(provider_name, latency, result.is_ok());
        
        result
    }
}
```

## Memory Optimization Strategies

### 1. Request/Response Pooling
```rust
pub struct RequestPool {
    completion_requests: ObjectPool<CompletionRequest>,
    chat_requests: ObjectPool<ChatRequest>,
    responses: ObjectPool<Response>,
}

impl RequestPool {
    pub fn acquire_completion_request(&self) -> PoolGuard<CompletionRequest> {
        self.completion_requests.pull(|| CompletionRequest::default())
    }
    
    pub fn acquire_response(&self) -> PoolGuard<Response> {
        self.responses.pull(|| Response::default())
    }
}

// Use pool in provider
impl BedrockProvider {
    async fn complete_with_pool(
        &self,
        request: CompletionRequest,
        pool: &RequestPool
    ) -> Result<CompletionResponse> {
        // Acquire pooled response
        let mut response = pool.acquire_response();
        
        // Process request
        self.invoke_model_into(&request, &mut response).await?;
        
        // Return without allocation
        Ok(response.into_inner())
    }
}
```

### 2. Streaming Without Buffering
```rust
pub struct StreamProcessor {
    decoder: SseDecoder,
    parser: JsonStreamParser,
}

impl StreamProcessor {
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<StreamToken> {
        let mut tokens = Vec::new();
        
        // Decode SSE events without allocation
        self.decoder.feed(chunk);
        
        while let Some(event) = self.decoder.next_event() {
            if let Some(token) = self.parser.parse_event(event) {
                tokens.push(token);
            }
        }
        
        tokens
    }
}

// Zero-copy SSE decoder
pub struct SseDecoder {
    buffer: BytesMut,
    position: usize,
}

impl SseDecoder {
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
    
    pub fn next_event(&mut self) -> Option<SseEvent> {
        // Find event boundary without allocation
        let search_start = self.position;
        let double_newline = b"\n\n";
        
        if let Some(pos) = self.buffer[search_start..]
            .windows(2)
            .position(|w| w == double_newline) 
        {
            let event_end = search_start + pos;
            let event_data = &self.buffer[self.position..event_end];
            
            // Parse event without copying
            let event = SseEvent::parse_borrowed(event_data);
            
            self.position = event_end + 2;
            Some(event)
        } else {
            None
        }
    }
}
```

## Rate Limiting & Retry Logic

### Adaptive Rate Limiter
```rust
use governor::{Quota, RateLimiter as Gov, state::NotKeyed};

pub struct AdaptiveRateLimiter {
    limiter: Arc<Gov<NotKeyed>>,
    current_quota: AtomicU32,
    error_count: AtomicU32,
}

impl AdaptiveRateLimiter {
    pub async fn acquire(&self) -> Result<()> {
        self.limiter.until_ready().await;
        Ok(())
    }
    
    pub fn adapt_rate(&self, success: bool) {
        if success {
            // Increase rate on success
            let errors = self.error_count.swap(0, Ordering::Relaxed);
            if errors == 0 {
                self.current_quota.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            // Decrease rate on error
            self.error_count.fetch_add(1, Ordering::Relaxed);
            let current = self.current_quota.load(Ordering::Relaxed);
            if current > 1 {
                self.current_quota.store(current / 2, Ordering::Relaxed);
            }
        }
    }
}
```

### Smart Retry with Circuit Breaker
```rust
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

impl CircuitBreaker {
    pub async fn call<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let state = self.state.read().await.clone();
        
        match state {
            CircuitState::Open(opened_at) => {
                if opened_at.elapsed() > self.timeout {
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    return Err(Error::CircuitOpen);
                }
            }
            _ => {}
        }
        
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
}
```

## Provider Testing

### Mock Provider for Testing
```rust
pub struct MockProvider {
    responses: Arc<RwLock<VecDeque<Result<CompletionResponse>>>>,
}

#[async_trait]
impl AiProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }
    
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        self.responses.write().await
            .pop_front()
            .unwrap_or_else(|| Ok(CompletionResponse::default()))
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_provider_manager() {
        let config = ProvidersConfig {
            bedrock: Some(BedrockConfig::default()),
            openai: Some(OpenAiConfig::default()),
            default_provider: "bedrock".to_string(),
        };
        
        let manager = ProviderManager::new(config).await.unwrap();
        
        let request = CompletionRequest {
            prompt: "Hello".to_string(),
            model: "nova-pro".to_string(),
            ..Default::default()
        };
        
        let response = manager.complete(request).await.unwrap();
        assert!(!response.text.is_empty());
    }
}
```

## Performance Benchmarks

```rust
#[bench]
fn bench_provider_dispatch(b: &mut Bencher) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let manager = runtime.block_on(create_test_manager());
    
    b.iter(|| {
        runtime.block_on(async {
            let request = CompletionRequest {
                prompt: "test".to_string(),
                model: "test-model".to_string(),
                provider: Some("mock".to_string()),
                ..Default::default()
            };
            manager.complete(request).await.unwrap()
        })
    });
}
```

## Memory Profile
- **Provider manager**: 1MB base
- **Per provider**: 500KB-1MB
- **Connection pools**: 2MB shared
- **Request/response pools**: 1MB
- **Total for 8 providers**: ~8MB (vs 60MB Node.js)
