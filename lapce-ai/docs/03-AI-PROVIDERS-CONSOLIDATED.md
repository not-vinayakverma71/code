# AI Providers: Unified Architecture and Implementations (Consolidated) - Just Implement 7 Providers (Xai,bedrock,aws,azure,openai,anthropic,gemini)

This document consolidates and reconciles the following sources:
- `docs/03-PROVIDER-TRAIT-ARCHITECTURE.md`
- `docs/03-OPENAI-ANTHROPIC-PROVIDERS.md`
- `docs/03-REMAINING-PROVIDERS.md`

It preserves all rules, behaviors, and constraints as a strict TypeScript → Rust translation, while presenting a single coherent plan.

## ⚠️ Critical Rules: 1:1 Translation Only
- Translate line-by-line from Codex TypeScript sources under `/home/verma/lapce/Codex/`
- Preserve exact request/response formats, headers, streaming behaviors, and error messages
- Do not alter algorithms or flow; only change language syntax to Rust

## ✅ Success Criteria (Combined)
- [ ] Memory usage: < 8MB for all providers combined
- [ ] Latency: < 5ms dispatch overhead per request
- [ ] Streaming: Zero-allocation or near-zero allocation, exact SSE formats per provider
- [ ] Rate limiting: Adaptive per provider with circuit breaker
- [ ] Load: 1K concurrent requests sustained across providers
- [ ] Parity: Character-for-character compatibility with TypeScript output on fixtures
- [ ] Tests: 100% behavior parity on mock and live endpoints

---

## Core Provider Trait Architecture

Borrowed and unified from `03-PROVIDER-TRAIT-ARCHITECTURE.md`

```rust
use async_trait::async_trait;
use futures::stream::{BoxStream};

#[async_trait]
pub trait AiProvider: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    async fn health_check(&self) -> Result<HealthStatus>;
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(&self, request: CompletionRequest) -> Result<BoxStream<'static, Result<StreamToken>>>;
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<StreamToken>>>;
    async fn list_models(&self) -> Result<Vec<Model>>;
    async fn count_tokens(&self, text: &str) -> Result<usize>;
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

### ProviderManager (Dispatch + Metrics)

```rust
pub struct ProviderManager {
    providers: DashMap<String, Arc<dyn AiProvider>>,
    default_provider: RwLock<String>,
    health_monitor: HealthMonitor,
    metrics: Arc<ProviderMetrics>,
}

impl ProviderManager {
    pub async fn new(config: ProvidersConfig) -> Result<Self> { /* initialize concurrently */ }
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> { /* route + record metrics */ }
}
```

### Registry

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn AiProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self { /* register all providers exactly as TS */ }
}
```

---

## OpenAI Provider (Exact Port)

From `03-OPENAI-ANTHROPIC-PROVIDERS.md`

### Streaming Format (MUST MATCH)
```typescript
// codex/providers/openai.ts
data: {"id":"...","choices":[{"delta":{"content":"Hello"}}]}
data: {"id":"...","choices":[{"delta":{"content":" world"}}]}
data: [DONE]
```

### Rust Structure
```rust
pub struct OpenAIProvider {
    client: Arc<reqwest::Client>,
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self { /* EXACT constructor translation */ }
    async fn stream_completion(&self, request: Request) -> Result<TokenStream> { /* EXACT SSE parse */ }
}
```

---

## Anthropic (Claude) Provider (Exact Port)

Key differences from `03-OPENAI-ANTHROPIC-PROVIDERS.md` (PRESERVE ALL):

```typescript
headers: {
  "anthropic-version": "2023-06-01",
  "anthropic-beta": "prompt-caching-2024-07-31",
  "x-api-key": apiKey
}
messages: [{ role: "user", content: "Human: {content}\n\nAssistant:" }]
```

### SSE Event Model (DIFFERENT FROM OpenAI)
```typescript
event: message_start
data: {"type":"message_start","message":{"id":"..."}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

### Rust Structure
```rust
pub struct AnthropicProvider {
    client: Arc<reqwest::Client>,
    api_key: String,
    cache_enabled: bool,
    prompt_caching_beta: bool,
}

impl AnthropicProvider {
    async fn format_messages(&self, messages: Vec<Message>) -> String { /* EXACT Human/Assistant */ }
    async fn parse_sse_event(&self, line: &str) -> Option<Token> { /* handle event: + data: */ }
}
```

---

## Remaining Providers (Exact Ports)

From `03-REMAINING-PROVIDERS.md`

### Gemini
- Request format uses `contents` → `parts` → `text`, not OpenAI’s schema.
```typescript
{
  "contents": [{ "parts": [{"text": "Hello"}], "role": "user" }],
  "generationConfig": { "temperature": 0.7, "maxOutputTokens": 2048 }
}
```

### AWS Bedrock
- AWS SigV4 signing
- Model-specific payloads (Claude/Titan/Llama)
- Same headers/algorithm as TS

### OpenRouter
- Special headers:
```typescript
"HTTP-Referer": "https://yourapp.com"
"X-Title": "Your App Name"
```

### xAI
- Mostly OpenAI compatible; keep xAI-specific deviations encapsulated.

### Perplexity
- Internet search and citations. Preserve citation format.

### Groq
- Ultra-fast endpoints and optimizations.

---

## Streaming & Zero-Allocation Processing

From `03-PROVIDER-TRAIT-ARCHITECTURE.md`

```rust
pub struct StreamProcessor {
    decoder: SseDecoder,
    parser: JsonStreamParser,
}

impl StreamProcessor {
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<StreamToken> { /* decode without allocation */ }
}

pub struct SseDecoder { buffer: BytesMut, position: usize }
```

---

## Rate Limiting, Retry, and Circuit Breakers

```rust
pub struct AdaptiveRateLimiter { /* governor-based, adaptive */ }
pub struct CircuitBreaker { /* Closed/Open/HalfOpen */ }
```

- Acquire tokens per provider
- Backoff and failover behaviors mimic TypeScript exactly

---

## Testing Requirements (Consolidated)

- Character-by-character parity for streaming outputs vs. TypeScript fixtures
- Header sets match exactly
- Registry contains all providers
- Load tests at 1K concurrent

```rust
#[tokio::test]
async fn provider_streaming_matches_ts() { /* compare fixtures 1:1 */ }
```

---

## Memory Optimization (After Parity Only)

- Use `Arc` for shared state
- Reuse HTTP clients and buffers
- Object pools for request/response structs

---

## Implementation Checklist (Combined)
- [ ] Implement `AiProvider` trait and `ProviderManager`
- [ ] OpenAI port with exact SSE
- [ ] Anthropic port (event-based SSE)
- [ ] Gemini exact request schema
- [ ] Bedrock signing + model handlers
- [ ] OpenRouter headers
- [ ] xAI compatibility wrapper
- [ ] Perplexity citation parsing
- [ ] Groq optimizations
- [ ] Registry and tests for 1:1 parity
- [ ] Rate limiting and circuit breaker
- [ ] Memory: < 8MB total
