# Step 13: OpenAI & Anthropic Provider Implementation
## Core AI Providers with Exact Behavior Preservation

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**THIS IS NOT A REWRITE - IT'S A TRANSLATION**

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/lapce-ai-rust/codex-reference/providers/openai.ts`
- `/home/verma/lapce/ex-reference/providers/anthropic.ts`
- Years of production quirks - PRESERVE ALL
- Same streaming formats, same headers, same everything

## ✅ Success Criteria
- [ ] **Memory Usage**: < 2MB for both providers combined
- [ ] **Streaming Format**: EXACT SSE parsing per provider
- [ ] **Response Time**: < 5ms dispatch overhead
- [ ] **Error Messages**: CHARACTER-FOR-CHARACTER match
- [ ] **Rate Limiting**: Same backoff strategies
- [ ] **Auth Handling**: Exact header formats
- [ ] **Test Coverage**: 100% parity with TypeScript tests
- [ ] **Load Test**: 1K concurrent requests without breaking

## Overview
OpenAI and Anthropic are the most critical providers, handling 80% of AI requests. Their implementation must be IDENTICAL to Codex behavior.

## OpenAI Provider (EXACT PORT)

### Streaming Format (MUST MATCH)
```typescript
// From codex-reference/providers/openai.ts
data: {"id":"...","choices":[{"delta":{"content":"Hello"}}]}
data: {"id":"...","choices":[{"delta":{"content":" world"}}]}
data: [DONE]
```

### Rust Translation
```rust
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

pub struct OpenAIProvider {
    client: Arc<Client>,
    api_key: String,
    base_url: String,
    // SAME fields as TypeScript
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        // TRANSLATE openai.ts constructor EXACTLY
        Self {
            client: Arc::new(Client::new()),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
    
    async fn stream_completion(&self, request: Request) -> Result<TokenStream> {
        // EXACT translation of streaming logic
        // Parse SSE EXACTLY as TypeScript does
        // Handle "data: [DONE]" the same way
    }
}
```

## Anthropic Provider (EXACT PORT)

### Critical Differences (PRESERVE ALL)
```typescript
// From codex-reference/providers/anthropic.ts
// EXACT headers required
headers: {
    "anthropic-version": "2023-06-01",
    "anthropic-beta": "prompt-caching-2024-07-31",
    "x-api-key": apiKey
}

// EXACT message format
messages: [{
    role: "user",
    content: "Human: {content}\n\nAssistant:"
}]
```

### Streaming Format (DIFFERENT FROM OpenAI)
```typescript
event: message_start
data: {"type":"message_start","message":{"id":"..."}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

### Rust Translation
```rust
pub struct AnthropicProvider {
    client: Arc<Client>,
    api_key: String,
    // Claude-specific fields
    cache_enabled: bool,
    prompt_caching_beta: bool,
}

impl AnthropicProvider {
    async fn format_messages(&self, messages: Vec<Message>) -> String {
        // EXACT Human/Assistant format from TypeScript
        // DO NOT "improve" the formatting
    }
    
    async fn parse_sse_event(&self, line: &str) -> Option<Token> {
        // Different SSE format than OpenAI
        // Handle "event:" lines
        // Parse "data:" differently
    }
}
```

## Common Pitfalls to AVOID

### DO NOT Change:
- Error message text (AI expects exact strings)
- Retry timeouts (calibrated over years)
- Header names or values
- URL endpoints
- Streaming chunk sizes
- Token counting logic

## Testing Requirements

### Integration Tests
```rust
#[tokio::test]
async fn openai_streaming_matches_typescript() {
    // Compare output CHARACTER-BY-CHARACTER with TypeScript
    let typescript_output = load_test_fixture("openai_stream.txt");
    let rust_output = openai.stream(request).await;
    assert_eq!(typescript_output, rust_output);
}

#[tokio::test]
async fn anthropic_headers_exact() {
    // Verify headers match EXACTLY
    assert!(request.headers.contains("anthropic-version"));
    assert_eq!(request.headers["anthropic-beta"], "prompt-caching-2024-07-31");
}
```

## Memory Optimization (ONLY AFTER EXACT PORT)
```rust
// Use Arc for shared data
// Pool connections
// Reuse buffers
// BUT ONLY after behavior matches 100%
```

## Implementation Checklist
- [ ] Port openai.ts line-by-line
- [ ] Port anthropic.ts line-by-line
- [ ] Preserve ALL quirks and edge cases
- [ ] Match error messages exactly
- [ ] Test against real API responses
- [ ] Compare with TypeScript output
- [ ] Memory usage < 2MB combined
- [ ] Performance 10x better than Node.js
