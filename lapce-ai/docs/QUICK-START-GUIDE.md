# QUICK START GUIDE: Begin Rust Translation

## Prerequisites Check

```bash
# Verify Rust toolchain
rustc --version  # Should be 1.70+
cargo --version

# Clone Codex for reference
cd /home/verma/lapce
# Codex already at: /home/verma/lapce/Codex

# Target location ready
# Backend: /home/verma/lapce/lapce-ai-rust
# Lapce plugin: /home/verma/lapce/lapce-app/
```

## Step 1: Create Project Structure (5 minutes)

```bash
cd /home/verma/lapce/lapce-ai-rust

# Create directory structure
mkdir -p src/{types,api,tools,task,storage,config}
mkdir -p web-ui/src
mkdir -p lapce-plugin/src

# Initialize Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "lapce-ai-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-stream = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# API providers
anthropic = "0.1"  # Check crates.io for actual name
async-openai = "0.18"
reqwest = { version = "0.11", features = ["json", "stream"] }

# Web server
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Utilities
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
thiserror = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Storage
keyring = "2.2"
fs2 = "0.4"  # File locking

[workspace]
members = ["lapce-plugin"]
EOF
```

## Step 2: Port Core Types (Task T001)

Create `src/types/mod.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Main message types between UI and backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    #[serde(rename = "webviewDidLaunch")]
    WebviewDidLaunch,
    
    #[serde(rename = "newTask")]
    NewTask {
        text: Option<String>,
        images: Option<Vec<String>>,
    },
    
    #[serde(rename = "askResponse")]
    AskResponse {
        #[serde(rename = "askResponse")]
        ask_response: ClineAskResponse,
        text: Option<String>,
        images: Option<Vec<String>>,
    },
    
    #[serde(rename = "apiConfiguration")]
    ApiConfiguration {
        #[serde(rename = "apiConfiguration")]
        api_configuration: ProviderSettings,
    },
    
    // ... add remaining 60+ variants from WebviewMessage.ts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClineAskResponse {
    #[serde(rename = "yesButtonTapped")]
    YesButtonTapped,
    #[serde(rename = "noButtonTapped")]
    NoButtonTapped,
    #[serde(rename = "messageResponse")]
    MessageResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
    pub ts: u64,
    
    #[serde(rename = "type")]
    pub message_type: MessageType,
    
    pub say: Option<String>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub partial: Option<bool>,
    
    // ... add all fields from ClineMessage type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    #[serde(rename = "say")]
    Say,
    #[serde(rename = "ask")]
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub api_provider: String,
    pub api_model_id: Option<String>,
    pub api_key: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub openai_base_url: Option<String>,
    // ... add all provider settings
}

// Test with actual JSON
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_webview_message_parsing() {
        let json = r#"{"type":"newTask","text":"Hello","images":null}"#;
        let msg: WebviewMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            WebviewMessage::NewTask { text, .. } => {
                assert_eq!(text, Some("Hello".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }
}
```

**Validation:** Run this test with actual JSON from Codex extension logs.

## Step 3: Create ApiHandler Trait (Task T011)

Create `src/api/mod.rs`:

```rust
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use anyhow::Result;

pub type ApiStream = Pin<Box<dyn Stream<Item = Result<ApiStreamChunk>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiStreamChunk {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "usage")]
    Usage {
        #[serde(rename = "inputTokens")]
        input_tokens: u32,
        #[serde(rename = "outputTokens")]
        output_tokens: u32,
        #[serde(rename = "cacheReadTokens")]
        cache_read_tokens: Option<u32>,
        #[serde(rename = "cacheWriteTokens")]
        cache_write_tokens: Option<u32>,
    },
    
    #[serde(rename = "error")]
    Error { error: String, message: String },
}

#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn create_message(
        &self,
        system_prompt: String,
        messages: Vec<MessageParam>,
        metadata: Option<ApiHandlerMetadata>,
    ) -> Result<ApiStream>;
    
    fn get_model(&self) -> ModelInfo;
    
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<u32>;
}

#[derive(Debug, Clone)]
pub struct MessageParam {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}
```

## Step 4: Implement First Provider (Task T021)

Create `src/api/providers/anthropic.rs`:

```rust
use super::*;
use anthropic::{Client, Message, MessageRequest};

pub struct AnthropicHandler {
    client: Client,
    model_id: String,
    max_tokens: u32,
}

impl AnthropicHandler {
    pub fn new(api_key: String, model_id: String) -> Self {
        let client = Client::new(api_key);
        Self {
            client,
            model_id,
            max_tokens: 8192,
        }
    }
}

#[async_trait]
impl ApiHandler for AnthropicHandler {
    async fn create_message(
        &self,
        system_prompt: String,
        messages: Vec<MessageParam>,
        _metadata: Option<ApiHandlerMetadata>,
    ) -> Result<ApiStream> {
        // Convert MessageParam to Anthropic format
        let anthropic_messages = messages.into_iter()
            .map(|msg| /* convert */)
            .collect();
        
        let request = MessageRequest {
            model: self.model_id.clone(),
            max_tokens: self.max_tokens,
            system: Some(system_prompt),
            messages: anthropic_messages,
            stream: true,
        };
        
        let stream = self.client.messages().create_stream(request).await?;
        
        // Map Anthropic stream chunks to ApiStreamChunk
        let mapped = stream.map(|chunk_result| {
            let chunk = chunk_result?;
            match chunk {
                AnthropicChunk::ContentBlockDelta { delta } => {
                    Ok(ApiStreamChunk::Text { text: delta.text })
                }
                AnthropicChunk::MessageDelta { usage } => {
                    Ok(ApiStreamChunk::Usage {
                        input_tokens: usage.input_tokens,
                        output_tokens: usage.output_tokens,
                        cache_read_tokens: usage.cache_read_tokens,
                        cache_write_tokens: usage.cache_write_tokens,
                    })
                }
                _ => Ok(ApiStreamChunk::Text { text: String::new() }),
            }
        });
        
        Ok(Box::pin(mapped))
    }
    
    fn get_model(&self) -> ModelInfo {
        ModelInfo {
            id: self.model_id.clone(),
            max_tokens: self.max_tokens,
            // ...
        }
    }
    
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<u32> {
        // Use tiktoken or Anthropic's counting API
        Ok(0) // Placeholder
    }
}
```

## Step 5: Test End-to-End (Manual)

Create `examples/basic_test.rs`:

```rust
use lapce_ai_backend::api::*;
use lapce_ai_backend::types::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // Initialize provider
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let handler = AnthropicHandler::new(api_key, "claude-3-5-sonnet-20241022".to_string());
    
    // Create simple message
    let messages = vec![
        MessageParam {
            role: Role::User,
            content: vec![
                ContentBlock::Text {
                    text: "Hello! Please respond with 'Hello, world!'".to_string()
                }
            ],
        }
    ];
    
    // Stream response
    let mut stream = handler.create_message(
        "You are a helpful assistant.".to_string(),
        messages,
        None,
    ).await?;
    
    use futures::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        match chunk {
            ApiStreamChunk::Text { text } => print!("{}", text),
            ApiStreamChunk::Usage { input_tokens, output_tokens, .. } => {
                println!("\n[Tokens: {} in, {} out]", input_tokens, output_tokens);
            }
            ApiStreamChunk::Error { error, message } => {
                eprintln!("Error: {} - {}", error, message);
            }
        }
    }
    
    Ok(())
}
```

Run:
```bash
export ANTHROPIC_API_KEY="your-key"
cargo run --example basic_test
```

**Expected output:**
```
Hello, world!
[Tokens: 15 in, 5 out]
```

## Step 6: Iterate on Remaining Tasks

With types and basic API handler working, proceed to:

1. **T002-T005:** Complete type system
2. **T006-T010:** Storage layer
3. **T036-T050:** Tool system
4. **T056-T070:** Task engine
5. **T071-T085:** Web UI + HTTP API
6. **T086-T090:** Lapce plugin

## Validation Checkpoints

After each phase, validate:

```bash
# Phase 1: Types compile and parse JSON correctly
cargo test --lib types

# Phase 2: API provider streams work
cargo test --lib api

# Phase 3: Tools execute successfully
cargo test --lib tools

# Phase 4: Task orchestration works
cargo test --lib task

# Phase 5: HTTP API responds
cargo run --bin server &
curl http://localhost:3000/api/health

# Phase 6: Lapce plugin loads
cd lapce-plugin && cargo build
```

## Common Issues & Solutions

**Issue:** Anthropic crate doesn't exist  
**Solution:** Use reqwest HTTP client with anthropic.com API docs

**Issue:** Streaming performance slow  
**Solution:** Use tokio::spawn for CPU-bound work, Pin<Box<dyn Stream>> for streaming

**Issue:** Type mismatch errors  
**Solution:** Add #[serde(rename_all = "camelCase")] and validate with actual JSON

**Issue:** Lapce plugin API unclear  
**Solution:** Start with minimal plugin, expand as API becomes clearer

## Getting Help

1. Reference documentation in `/home/verma/lapce/lapce-ai-rust/docs/`
2. Check original TypeScript in `/home/verma/lapce/Codex/src/`
3. Validate JSON schemas with test suite
4. Benchmark performance early

## Success Criteria

- [ ] Types parse actual Codex JSON without errors
- [ ] Anthropic handler streams responses
- [ ] One tool (read_file) executes successfully
- [ ] Task loop completes one iteration
- [ ] Web UI connects to backend
- [ ] Lapce plugin launches everything

**Ready to begin? Start with Task T001!**
