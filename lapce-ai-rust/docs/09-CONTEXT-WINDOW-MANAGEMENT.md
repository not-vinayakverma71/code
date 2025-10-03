# Step 15: Context Window & Token Management
## Sliding Window, Token Counting, Message Truncation

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**YEARS OF CALIBRATED CONTEXT LOGIC - PRESERVE EXACTLY**

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex`


## ✅ Success Criteria
- [ ] **Every Single backend fature from Codex**: Must 100% - No Compromise - 1000s of files that needed to be translated
- [ ] **Memory Usage**: < 2MB for context management
- [ ] **Token Counting**: EXACT tiktoken.ts behavior
- [ ] **Context Limits**: Same per model (128k, 200k, etc.)
- [ ] **Prioritization**: IDENTICAL selection algorithm
- [ ] **Truncation**: Same logic for message cutting
- [ ] **Performance**: < 10ms for context preparation
- [ ] **Test Coverage**: CHARACTER-match with TypeScript
- [ ] **Accuracy**: 100% same context as Codex

## Overview
Context window management is the HEART of AI behavior. Any deviation will break years of tuning.

## Token Counting (EXACT TIKTOKEN PORT)

### TypeScript Reference
```typescript
// From codex-reference/tiktoken.ts
export function countTokens(text: string, model: string): number {
    // Exact tokenization logic
    // Model-specific adjustments
    // Special token handling
}
```

### Rust Translation
```rust
use tiktoken_rs::{cl100k_base, p50k_base, r50k_base};

pub struct TokenCounter {
    // Cache tokenizers per model
    encoders: HashMap<String, Arc<CoreBPE>>,
}

impl TokenCounter {
    pub fn count_tokens(&self, text: &str, model: &str) -> usize {
        // EXACT translation of tiktoken.ts
        // Same model mappings
        // Same special token handling
        
        let encoder = match model {
            m if m.contains("gpt-5") => &self.cl100k_base,
            m if m.contains("gemini") => &self.cl100k_base,
            m if m.contains("claude") => &self.cl100k_base, // Claude uses similar
            _ => &self.p50k_base,
        };
        
        // Count EXACTLY as TypeScript does
        encoder.encode_ordinary(text).len()
    }
}
```

## Sliding Window Implementation

### Context Prioritization (PRESERVE ORDER)
```typescript
// From codex-reference/sliding-window/index.ts
// Priority order (MUST MATCH):
// 1. System prompt
// 2. Recent messages
// 3. Pinned context
// 4. Tool definitions
// 5. Older messages (truncated)
```

### Rust Translation
```rust
pub struct SlidingWindow {
    max_tokens: HashMap<String, usize>, // Model limits
    token_counter: Arc<TokenCounter>,
    // SAME fields as TypeScript
}

impl SlidingWindow {
    pub fn prepare_context(&self, 
        messages: Vec<Message>,
        model: &str,
        tools: Option<Vec<Tool>>,
        system_prompt: Option<String>
    ) -> Vec<Message> {
        // EXACT algorithm from TypeScript
        
        let limit = self.max_tokens.get(model).unwrap_or(&128000);
        let mut token_count = 0;
        let mut result = Vec::new();
        
        // 1. System prompt ALWAYS first
        if let Some(prompt) = system_prompt {
            let tokens = self.token_counter.count_tokens(&prompt, model);
            result.push(Message::system(prompt));
            token_count += tokens;
        }
        
        // 2. Recent messages (work backwards)
        for message in messages.iter().rev() {
            let msg_tokens = self.count_message_tokens(message, model);
            if token_count + msg_tokens > limit {
                // Truncate EXACTLY as TypeScript
                break;
            }
            result.insert(1, message.clone()); // After system
            token_count += msg_tokens;
        }
        
        result
    }
}
```

## Message Condensing (COMPLEX LOGIC)

### Conversation Summarization
```typescript
// From codex-reference/condense/
// When context full, summarize older messages
// Preserve key information
// Specific format for summaries
```

### Rust Translation
```rust
pub struct MessageCondenser {
    summary_prompt: String, // EXACT prompt from TypeScript
}

impl MessageCondenser {
    pub async fn condense_messages(&self, 
        messages: Vec<Message>,
        preserve_count: usize
    ) -> Vec<Message> {
        // EXACT condensing algorithm
        // Same summary format
        // Same preservation logic
        
        if messages.len() <= preserve_count {
            return messages;
        }
        
        let to_condense = &messages[..messages.len() - preserve_count];
        let preserved = &messages[messages.len() - preserve_count..];
        
        // Create summary using EXACT prompt
        let summary = self.create_summary(to_condense).await;
        
        // Return in EXACT format
        vec![Message::system(summary)]
            .into_iter()
            .chain(preserved.iter().cloned())
            .collect()
    }
}
```

## Model-Specific Limits (DO NOT CHANGE)

```rust
lazy_static! {
    static ref MODEL_LIMITS: HashMap<&'static str, usize> = {
        let mut m = HashMap::new();
        // EXACT limits from TypeScript
        m.insert("gpt-5", 400000);
        m.insert("deepseek-r1", 128000);
        m.insert("qwen-3", 200000);
        m.insert("claude-4-opus", 200000);
        m.insert("kimi-2", 200000);
        m.insert("grok-code", 200000);
        m.insert("gemini-2.5-pro", 1048576); // 1M context
        m.insert("claude-4-sonnet", 1048576);
        m
    };
}
```

## Context Tracking

```rust
pub struct ContextTracker {
    // Track what's in context
    current_context: Vec<ContextItem>,
    token_usage: TokenUsage,
}

#[derive(Clone, Debug)]
pub struct ContextItem {
    pub content: String,
    pub item_type: ContextType,
    pub priority: i32,
    pub tokens: usize,
    pub timestamp: SystemTime,
}

#[derive(Clone, Debug)]
pub enum ContextType {
    SystemPrompt,
    UserMessage,
    AssistantMessage,
    ToolDefinition,
    PinnedContext,
    FileContent,
}
```

## Testing Requirements

```rust
#[tokio::test]
async fn context_selection_matches_typescript() {
    // Load test cases from TypeScript
    let test_cases = load_typescript_fixtures("context_tests.json");
    
    for case in test_cases {
        let rust_context = window.prepare_context(
            case.messages,
            case.model,
            case.tools,
            case.system_prompt
        );
        
        // Must match CHARACTER-FOR-CHARACTER
        assert_eq!(case.expected_context, rust_context);
    }
}

#[tokio::test]
async fn token_counting_exact() {
    // Test against known token counts
    assert_eq!(counter.count_tokens("Hello world", "gpt-5"), 2);
    assert_eq!(counter.count_tokens("", "gpt-5"), 0);
    // More test cases from TypeScript
}
```

## Implementation Checklist
- [ ] Every single feature - copy full backend of Codex
- [ ] Port sliding-window/ line-by-line
- [ ] Port context/ line-by-line
- [ ] Port tiktoken.ts exactly
- [ ] Port condense/ logic
- [ ] Preserve prioritization order
- [ ] Match model limits exactly
- [ ] Test token counting accuracy
- [ ] Verify context selection matches
