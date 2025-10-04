# Step 14: Remaining Provider Implementations
## Gemini, Bedrock, OpenRouter, xAI, Perplexity, Groq

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**PRESERVE YEARS OF PROVIDER-SPECIFIC QUIRKS**

**TRANSLATE LINE-BY-LINE FROM**:
- `home/verma/lapce/Codex`

## ✅ Success Criteria
- [ ] **Memory Usage**: < 6MB for all 6 providers combined
- [ ] **Provider Quirks**: ALL unique behaviors preserved
- [ ] **Auth Methods**: Exact authentication per provider
- [ ] **Request Formats**: CHARACTER-FOR-CHARACTER match
- [ ] **Streaming**: Each provider's unique format
- [ ] **Error Handling**: Same error codes and messages
- [ ] **Rate Limiting**: Provider-specific limits
- [ ] **Test Coverage**: 100% behavior parity

## Gemini Provider (UNIQUE STRUCTURE)

### Request Format (DIFFERENT!)
```typescript
// From codex-reference/providers/gemini.ts
{
  "contents": [{
    "parts": [{"text": "Hello"}],
    "role": "user"
  }],
  "generationConfig": {
    "temperature": 0.7,
    "maxOutputTokens": 2048
  }
}
```

### Rust Translation
```rust
pub struct GeminiProvider {
    client: Arc<Client>,
    api_key: String,
    // Gemini-specific fields
    safety_settings: Vec<SafetySetting>,
}

impl GeminiProvider {
    fn format_request(&self, messages: Vec<Message>) -> GeminiRequest {
        // EXACT "contents" structure from TypeScript
        // "parts" array with "text" field
        // DO NOT simplify to OpenAI format
    }
}
```

## AWS Bedrock Provider (COMPLEX AUTH)

### Critical AWS Requirements
```typescript
// From codex-reference/providers/bedrock.ts
// AWS Signature V4 signing
// Model-specific payloads
// Different formats for Claude vs Titan vs Llama
```

### Rust Translation
```rust
use aws_sigv4::http_request::SignableRequest;
use aws_credential_types::Credentials;

pub struct BedrockProvider {
    client: Arc<Client>,
    credentials: Credentials,
    region: String,
    // Model-specific formatters
    model_handlers: HashMap<String, Box<dyn ModelHandler>>,
}

impl BedrockProvider {
    async fn sign_request(&self, request: &mut Request) {
        // EXACT AWS Signature V4 from TypeScript
        // Same headers, same algorithm
    }
    
    fn get_model_payload(&self, model: &str, messages: Vec<Message>) -> Value {
        // Different format per model family
        match model {
            m if m.contains("claude") => self.format_claude_payload(messages),
            m if m.contains("titan") => self.format_titan_payload(messages),
            m if m.contains("llama") => self.format_llama_payload(messages),
            _ => panic!("Unknown model family"),
        }
    }
}
```

## OpenRouter Provider (ROUTING LOGIC)

### Special Headers
```typescript
// From codex-reference/providers/openrouter.ts
headers: {
    "HTTP-Referer": "https://yourapp.com",
    "X-Title": "Your App Name"
}
```

### Rust Translation
```rust
pub struct OpenRouterProvider {
    client: Arc<Client>,
    api_key: String,
    app_name: String,
    site_url: String,
}

impl OpenRouterProvider {
    fn add_routing_headers(&self, request: &mut Request) {
        // EXACT headers from TypeScript
        request.headers_mut().insert(
            "HTTP-Referer",
            self.site_url.parse().unwrap()
        );
        request.headers_mut().insert(
            "X-Title",
            self.app_name.parse().unwrap()
        );
    }
}
```

## xAI Provider (OPENAI COMPATIBLE)

### Standard but with quirks
```rust
pub struct XAIProvider {
    // Mostly OpenAI-compatible
    // BUT check for xAI-specific differences
    base_provider: OpenAICompatibleProvider,
    xai_specific_options: XAIOptions,
}
```

## Perplexity Provider (SEARCH INTEGRATION)

### Unique Features
```typescript
// From codex-reference/providers/perplexity.ts
// Has internet search capability
// Different response format with citations
```

```rust
pub struct PerplexityProvider {
    client: Arc<Client>,
    api_key: String,
    search_enabled: bool,
}

impl PerplexityProvider {
    fn parse_citations(&self, response: &Response) -> Vec<Citation> {
        // EXACT citation parsing from TypeScript
        // Preserve format for AI to understand
    }
}
```

## Groq Provider (ULTRA-FAST)

### Speed optimizations
```rust
pub struct GroqProvider {
    client: Arc<Client>,
    api_key: String,
    // Groq-specific optimizations
    use_dedicated_endpoint: bool,
}
```

## Common Provider Registry

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn AIProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut providers = HashMap::new();
        
        // Register ALL providers EXACTLY as TypeScript
        providers.insert("openai", Box::new(OpenAIProvider::new()));
        providers.insert("anthropic", Box::new(AnthropicProvider::new()));
        providers.insert("gemini", Box::new(GeminiProvider::new()));
        providers.insert("bedrock", Box::new(BedrockProvider::new()));
        providers.insert("openrouter", Box::new(OpenRouterProvider::new()));
        providers.insert("xai", Box::new(XAIProvider::new()));
        providers.insert("perplexity", Box::new(PerplexityProvider::new()));
        providers.insert("groq", Box::new(GroqProvider::new()));
        
        Self { providers }
    }
}
```

## Testing Requirements

```rust
#[tokio::test]
async fn all_providers_match_typescript() {
    for provider_name in ["gemini", "bedrock", "openrouter", "xai", "perplexity", "groq"] {
        let ts_response = load_typescript_fixture(&format!("{}_response.json", provider_name));
        let rust_response = registry.get(provider_name).complete(request).await;
        assert_eq!(ts_response, rust_response, "Provider {} mismatch", provider_name);
    }
}
```

## Implementation Checklist
- [ ] Port each provider TypeScript file line-by-line
- [ ] Preserve ALL unique behaviors
- [ ] Match request/response formats exactly
- [ ] Keep provider-specific headers
- [ ] Maintain auth quirks
- [ ] Test against real APIs
- [ ] Memory < 6MB total
- [ ] All providers registered in registry
