# CHUNK-17: CORE/SLIDING-WINDOW - CONTEXT WINDOW MANAGEMENT

## 📁 MODULE STRUCTURE

```
Codex/src/core/sliding-window/
├── index.ts                          (176 lines)
└── __tests__/
    └── sliding-window.spec.ts        (1248 lines - comprehensive tests)
```

**Total**: 1,424 lines analyzed

---

## 🎯 PURPOSE

Manage AI conversation context by intelligently truncating or condensing messages when approaching token limits. Prevents context overflow while preserving conversation coherence through two strategies:
1. **Sliding Window**: Remove oldest messages (fast, cheap)
2. **AI Condensing**: Summarize conversation with LLM (smart, costs tokens)

---

## 🔧 CORE CONSTANTS

```typescript
export const TOKEN_BUFFER_PERCENTAGE = 0.1  // 10% safety buffer

// From condense module
export const MAX_CONDENSE_THRESHOLD = 100   // Max % of context window
export const MIN_CONDENSE_THRESHOLD = 50    // Min % of context window
```

**Buffer Strategy**: Reserve 10% of context window to avoid hard limits

---

## 🧮 TOKEN ESTIMATION

### estimateTokenCount()

```typescript
export async function estimateTokenCount(
    content: Array<Anthropic.Messages.ContentBlockParam>,
    apiHandler: ApiHandler,
): Promise<number> {
    if (!content || content.length === 0) return 0
    return apiHandler.countTokens(content)
}
```

**Delegates to Provider**: Each API handler implements token counting
- Text: Use tiktoken (GPT) or Claude tokenizer
- Images: Estimate based on resolution/size
- Mixed: Sum all content blocks

**Example Token Counts** (from tests):
```typescript
// Text: Variable based on tokenizer
"Short text" → ~10-20 tokens
"X".repeat(1000) → ~250 tokens (GPT tokenizer)

// Images: Based on data size
smallImage → sqrt(dataLength) * 1.5
"dummy_data" → ceil(sqrt(10)) * 1.5 ≈ 5 tokens
"X".repeat(1000) → ceil(sqrt(1000)) * 1.5 ≈ 48 tokens
```

---

## ✂️ TRUNCATION STRATEGIES

### Strategy 1: Sliding Window Truncation

```typescript
export function truncateConversation(
    messages: ApiMessage[], 
    fracToRemove: number, 
    taskId: string
): ApiMessage[] {
    TelemetryService.instance.captureSlidingWindowTruncation(taskId)
    
    const truncatedMessages = [messages[0]]  // Always keep first message
    const rawMessagesToRemove = Math.floor((messages.length - 1) * fracToRemove)
    const messagesToRemove = rawMessagesToRemove - (rawMessagesToRemove % 2)  // Round to even
    const remainingMessages = messages.slice(messagesToRemove + 1)
    truncatedMessages.push(...remainingMessages)
    
    return truncatedMessages
}
```

**Key Rules**:
1. **First message always preserved** (usually contains task context)
2. **Remove even number of messages** (maintains user/assistant turn structure)
3. **Default fraction: 0.5** (remove 50% of old messages)
4. **Telemetry tracked** for analytics

**Example**:
```typescript
// Original: 7 messages
const messages = [
    "User: Task description",    // Index 0 - ALWAYS KEPT
    "Assistant: Response 1",     // Index 1 
    "User: Question 1",          // Index 2
    "Assistant: Answer 1",       // Index 3 - Will be kept (after removal)
    "User: Question 2",          // Index 4 - Will be kept
    "Assistant: Answer 2",       // Index 5 - Will be kept
    "User: Latest question",     // Index 6 - Will be kept
]

// After truncation with frac=0.5:
// 6 messages excluding first, 0.5 * 6 = 3, round to 2 (even)
// Remove indices 1, 2
const result = [
    messages[0],  // First message (task)
    messages[3],  // Resume from here
    messages[4],
    messages[5],
    messages[6],
]
```

### Strategy 2: AI-Powered Condensing

**Delegated to**: `summarizeConversation()` from `core/condense/`

**Flow**:
1. Send conversation history to AI with special prompt
2. AI generates summary of removed messages
3. Insert summary as assistant message with `isSummary: true` flag
4. Preserve recent messages

**When Used**:
- `autoCondenseContext: true` AND
- Context percentage ≥ threshold OR
- Total tokens > allowed tokens

---

## 🎛️ MAIN FUNCTION: truncateConversationIfNeeded()

### Input Options

```typescript
type TruncateOptions = {
    messages: ApiMessage[]
    totalTokens: number                      // Tokens EXCLUDING last message
    contextWindow: number                    // Model's max context
    maxTokens?: number | null                // Max tokens for response
    apiHandler: ApiHandler
    autoCondenseContext: boolean             // Enable AI summarization?
    autoCondenseContextPercent: number       // Global threshold (%)
    systemPrompt: string
    taskId: string
    customCondensingPrompt?: string          // Override default summary prompt
    condensingApiHandler?: ApiHandler        // Use different model for summary
    profileThresholds: Record<string, number> // Per-profile thresholds
    currentProfileId: string
}
```

### Return Type

```typescript
type TruncateResponse = {
    messages: ApiMessage[]       // Original or truncated/condensed
    summary: string              // Summary text (if condensed)
    cost: number                 // Cost of condensing operation
    prevContextTokens: number    // Total tokens before truncation
    newContextTokens?: number    // Total tokens after condensing
    error?: string               // Error if condensing failed
}
```

### Decision Flow

```typescript
export async function truncateConversationIfNeeded({
    messages, totalTokens, contextWindow, maxTokens, apiHandler,
    autoCondenseContext, autoCondenseContextPercent,
    systemPrompt, taskId, customCondensingPrompt, condensingApiHandler,
    profileThresholds, currentProfileId,
}: TruncateOptions): Promise<TruncateResponse> {
    
    // 1. Calculate reserved tokens for response
    const reservedTokens = maxTokens || ANTHROPIC_DEFAULT_MAX_TOKENS  // 8192
    
    // 2. Count tokens in last message (always user message)
    const lastMessage = messages[messages.length - 1]
    const lastMessageTokens = await estimateTokenCount(lastMessage.content, apiHandler)
    
    // 3. Calculate total context including last message
    const prevContextTokens = totalTokens + lastMessageTokens
    
    // 4. Calculate allowed tokens (with 10% buffer)
    const allowedTokens = contextWindow * (1 - TOKEN_BUFFER_PERCENTAGE) - reservedTokens
    
    // 5. Determine effective threshold (profile or global)
    let effectiveThreshold = autoCondenseContextPercent
    const profileThreshold = profileThresholds[currentProfileId]
    
    if (profileThreshold !== undefined) {
        if (profileThreshold === -1) {
            // -1 means inherit global setting
            effectiveThreshold = autoCondenseContextPercent
        } else if (profileThreshold >= MIN_CONDENSE_THRESHOLD && 
                   profileThreshold <= MAX_CONDENSE_THRESHOLD) {
            effectiveThreshold = profileThreshold
        } else {
            console.warn(`Invalid profile threshold ${profileThreshold}`)
            effectiveThreshold = autoCondenseContextPercent
        }
    }
    
    // 6. Try AI condensing if enabled
    if (autoCondenseContext) {
        const contextPercent = (100 * prevContextTokens) / contextWindow
        
        if (contextPercent >= effectiveThreshold || prevContextTokens > allowedTokens) {
            const result = await summarizeConversation(
                messages, apiHandler, systemPrompt, taskId,
                prevContextTokens, true, customCondensingPrompt, condensingApiHandler
            )
            
            if (!result.error) {
                return { ...result, prevContextTokens }
            }
            // Continue to sliding window on error
        }
    }
    
    // 7. Fall back to sliding window if needed
    if (prevContextTokens > allowedTokens) {
        const truncatedMessages = truncateConversation(messages, 0.5, taskId)
        return { messages: truncatedMessages, prevContextTokens, summary: "", cost: 0 }
    }
    
    // 8. No truncation needed
    return { messages, summary: "", cost: 0, prevContextTokens }
}
```

---

## 📐 CALCULATIONS EXPLAINED

### Example: Claude Sonnet 3.5 (200k context)

```typescript
contextWindow = 200_000
maxTokens = 8_192  // Default for Claude
TOKEN_BUFFER = 0.1

// Available for conversation history:
allowedTokens = 200_000 * 0.9 - 8_192 
              = 180_000 - 8_192
              = 171_808 tokens

// With 75% threshold:
condenseTrigger = 200_000 * 0.75 = 150_000 tokens

// Condensing triggers when:
// prevContextTokens >= 150_000 OR prevContextTokens > 171_808
```

### Example: GPT-4 (128k context)

```typescript
contextWindow = 128_000
maxTokens = 4_096

allowedTokens = 128_000 * 0.9 - 4_096
              = 115_200 - 4_096  
              = 111_104 tokens

// With 80% threshold:
condenseTrigger = 128_000 * 0.80 = 102_400 tokens
```

---

## 🎨 PROFILE-SPECIFIC THRESHOLDS

### Use Case

Different AI "profiles" (modes) can have different condensing thresholds:
- **Code Mode**: 60% threshold (preserve more code context)
- **Chat Mode**: 80% threshold (condense more aggressively)
- **Research Mode**: 50% threshold (keep full history longer)

### Configuration

```typescript
const profileThresholds = {
    "code-mode": 60,
    "chat-mode": 80,
    "research-mode": 50,
    "default": -1,  // Inherit global setting
}

const currentProfileId = "code-mode"
const globalThreshold = 75

// Result: Will use 60% threshold for code-mode
```

### Special Values

- **-1**: Inherit from global `autoCondenseContextPercent`
- **undefined**: Profile not in map → use global
- **50-100**: Valid custom threshold range
- **< 50 or > 100**: Invalid → fallback to global with warning

---

## 🧪 TEST COVERAGE (1248 lines)

### Core Truncation Tests

```typescript
it("should retain the first message", () => {
    const messages = [
        { role: "user", content: "First message" },
        { role: "assistant", content: "Second message" },
        { role: "user", content: "Third message" },
    ]
    
    const result = truncateConversation(messages, 0.5, taskId)
    
    expect(result[0]).toEqual(messages[0])  // First always kept
})

it("should remove even number of messages", () => {
    const messages = [
        /* 7 messages total */
    ]
    
    // 6 messages after first, 0.3 fraction = 1.8 → rounds to 0 (even)
    const result = truncateConversation(messages, 0.3, taskId)
    expect(result.length).toBe(7)  // No removal
})
```

### Token Estimation Tests

```typescript
it("should return 0 for empty content", async () => {
    expect(await estimateTokenCount([], mockApiHandler)).toBe(0)
})

it("should estimate image tokens based on data size", async () => {
    const smallImage = [{ 
        type: "image", 
        source: { data: "small_data" } 
    }]
    const largeImage = [{ 
        type: "image", 
        source: { data: "X".repeat(1000) } 
    }]
    
    const small = await estimateTokenCount(smallImage, mockApiHandler)
    const large = await estimateTokenCount(largeImage, mockApiHandler)
    
    expect(large).toBeGreaterThan(small)
    expect(large).toBe(48)  // ceil(sqrt(1000)) * 1.5
})
```

### Truncation Decision Tests

```typescript
it("should not truncate if below threshold", async () => {
    const result = await truncateConversationIfNeeded({
        totalTokens: 69_999,  // Below 70k threshold
        contextWindow: 100_000,
        maxTokens: 30_000,
        autoCondenseContext: false,
        // ...
    })
    
    expect(result.messages).toEqual(originalMessages)  // No change
})

it("should truncate if above threshold", async () => {
    const result = await truncateConversationIfNeeded({
        totalTokens: 70_001,  // Above threshold
        // ...
    })
    
    expect(result.messages.length).toBe(3)  // Truncated
})
```

### Condensing Tests

```typescript
it("should use AI condensing when enabled and above threshold", async () => {
    const summarizeSpy = vi.spyOn(condenseModule, "summarizeConversation")
    
    await truncateConversationIfNeeded({
        autoCondenseContext: true,
        autoCondenseContextPercent: 50,
        totalTokens: 60_000,  // 60% of 100k context
        // ...
    })
    
    expect(summarizeSpy).toHaveBeenCalled()
})

it("should fall back to sliding window if condensing fails", async () => {
    const mockError = { 
        messages: originalMessages, 
        error: "Condensing failed" 
    }
    vi.spyOn(condenseModule, "summarizeConversation")
        .mockResolvedValue(mockError)
    
    const result = await truncateConversationIfNeeded({
        autoCondenseContext: true,
        totalTokens: 70_001,
        // ...
    })
    
    expect(result.messages.length).toBe(3)  // Used sliding window
})
```

### Profile Threshold Tests

```typescript
it("should use profile-specific threshold", async () => {
    const result = await truncateConversationIfNeeded({
        profileThresholds: { "code-mode": 60 },
        currentProfileId: "code-mode",
        autoCondenseContextPercent: 80,  // Global
        totalTokens: 65_000,  // 65% (above profile 60%, below global 80%)
        // ...
    })
    
    // Should condense because 65% > 60% (profile threshold)
    expect(summarizeSpy).toHaveBeenCalled()
})

it("should fall back to global when profile is -1", async () => {
    await truncateConversationIfNeeded({
        profileThresholds: { "default": -1 },
        currentProfileId: "default",
        autoCondenseContextPercent: 75,
        // ...
    })
    
    // Uses 75% global threshold
})
```

---

## 🔄 INTEGRATION POINTS

### 1. Task Execution Loop

```typescript
// In core/task/TaskExecutor.ts
const { messages, summary, cost } = await truncateConversationIfNeeded({
    messages: conversationHistory,
    totalTokens: currentTokenCount,
    contextWindow: model.info.contextWindow,
    maxTokens: model.info.maxTokens,
    apiHandler: this.apiHandler,
    autoCondenseContext: config.autoCondenseContext,
    autoCondenseContextPercent: config.autoCondensePercent,
    systemPrompt: this.systemPrompt,
    taskId: this.taskId,
    profileThresholds: config.profileThresholds,
    currentProfileId: this.profileId,
})

// Update conversation with potentially truncated messages
this.conversationHistory = messages

// If summarized, notify user
if (summary) {
    this.showNotification(`Context condensed: ${summary.slice(0, 100)}...`)
}
```

### 2. Telemetry Tracking

```typescript
// Automatic tracking in truncateConversation()
TelemetryService.instance.captureSlidingWindowTruncation(taskId)

// Also tracked in condense module
TelemetryService.instance.captureCondensing({
    taskId,
    originalTokens: prevContextTokens,
    newTokens: newContextTokens,
    cost,
})
```

### 3. User Settings

```typescript
// Settings UI
{
    "autoCondenseContext": true,
    "autoCondenseContextPercent": 75,
    "profileThresholds": {
        "code-mode": 60,
        "chat-mode": 85,
        "research-mode": 50
    }
}
```

---

## 🦀 RUST TRANSLATION

```rust
use std::collections::HashMap;

pub const TOKEN_BUFFER_PERCENTAGE: f64 = 0.1;
pub const MIN_CONDENSE_THRESHOLD: u8 = 50;
pub const MAX_CONDENSE_THRESHOLD: u8 = 100;

#[derive(Clone)]
pub struct ApiMessage {
    pub role: String,
    pub content: MessageContent,
    pub is_summary: Option<bool>,
}

pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
}

pub struct TruncateOptions {
    pub messages: Vec<ApiMessage>,
    pub total_tokens: usize,
    pub context_window: usize,
    pub max_tokens: Option<usize>,
    pub auto_condense_context: bool,
    pub auto_condense_context_percent: u8,
    pub system_prompt: String,
    pub task_id: String,
    pub profile_thresholds: HashMap<String, i32>,
    pub current_profile_id: String,
}

pub struct TruncateResponse {
    pub messages: Vec<ApiMessage>,
    pub summary: String,
    pub cost: f64,
    pub prev_context_tokens: usize,
    pub new_context_tokens: Option<usize>,
    pub error: Option<String>,
}

/// Estimate token count for message content
pub async fn estimate_token_count(
    content: &[ContentBlock],
    api_handler: &dyn ApiHandler,
) -> Result<usize> {
    if content.is_empty() {
        return Ok(0);
    }
    api_handler.count_tokens(content).await
}

/// Truncate conversation with sliding window
pub fn truncate_conversation(
    messages: &[ApiMessage],
    frac_to_remove: f64,
    task_id: &str,
) -> Vec<ApiMessage> {
    // Track telemetry
    telemetry::capture_sliding_window_truncation(task_id);
    
    // Always keep first message
    let mut truncated = vec![messages[0].clone()];
    
    // Calculate even number of messages to remove
    let raw_to_remove = ((messages.len() - 1) as f64 * frac_to_remove).floor() as usize;
    let to_remove = raw_to_remove - (raw_to_remove % 2);
    
    // Add remaining messages
    truncated.extend_from_slice(&messages[to_remove + 1..]);
    
    truncated
}

/// Main truncation logic with AI condensing support
pub async fn truncate_conversation_if_needed(
    options: TruncateOptions,
) -> Result<TruncateResponse> {
    let TruncateOptions {
        messages,
        total_tokens,
        context_window,
        max_tokens,
        auto_condense_context,
        auto_condense_context_percent,
        system_prompt,
        task_id,
        profile_thresholds,
        current_profile_id,
        ..
    } = options;
    
    // 1. Calculate reserved tokens
    const ANTHROPIC_DEFAULT_MAX_TOKENS: usize = 8192;
    let reserved_tokens = max_tokens.unwrap_or(ANTHROPIC_DEFAULT_MAX_TOKENS);
    
    // 2. Count last message tokens
    let last_message = messages.last().ok_or("No messages")?;
    let last_message_tokens = estimate_token_count(
        &last_message.content.as_blocks(), 
        &api_handler
    ).await?;
    
    // 3. Calculate total context
    let prev_context_tokens = total_tokens + last_message_tokens;
    
    // 4. Calculate allowed tokens with buffer
    let allowed_tokens = 
        (context_window as f64 * (1.0 - TOKEN_BUFFER_PERCENTAGE)) as usize 
        - reserved_tokens;
    
    // 5. Determine effective threshold
    let effective_threshold = match profile_thresholds.get(&current_profile_id) {
        Some(&-1) => auto_condense_context_percent,
        Some(&threshold) if threshold >= MIN_CONDENSE_THRESHOLD as i32 
                         && threshold <= MAX_CONDENSE_THRESHOLD as i32 => {
            threshold as u8
        }
        Some(&invalid) => {
            log::warn!("Invalid profile threshold {}", invalid);
            auto_condense_context_percent
        }
        None => auto_condense_context_percent,
    };
    
    // 6. Try AI condensing if enabled
    if auto_condense_context {
        let context_percent = (100 * prev_context_tokens) / context_window;
        
        if context_percent >= effective_threshold as usize 
           || prev_context_tokens > allowed_tokens {
            match summarize_conversation(
                &messages,
                &api_handler,
                &system_prompt,
                &task_id,
                prev_context_tokens,
                true,  // automatic trigger
            ).await {
                Ok(result) if result.error.is_none() => {
                    return Ok(TruncateResponse {
                        messages: result.messages,
                        summary: result.summary,
                        cost: result.cost,
                        prev_context_tokens,
                        new_context_tokens: result.new_context_tokens,
                        error: None,
                    });
                }
                _ => {
                    // Fall through to sliding window
                }
            }
        }
    }
    
    // 7. Fall back to sliding window if needed
    if prev_context_tokens > allowed_tokens {
        let truncated = truncate_conversation(&messages, 0.5, &task_id);
        return Ok(TruncateResponse {
            messages: truncated,
            summary: String::new(),
            cost: 0.0,
            prev_context_tokens,
            new_context_tokens: None,
            error: None,
        });
    }
    
    // 8. No truncation needed
    Ok(TruncateResponse {
        messages,
        summary: String::new(),
        cost: 0.0,
        prev_context_tokens,
        new_context_tokens: None,
        error: None,
    })
}
```

### Trait for API Handlers

```rust
#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<usize>;
    async fn create_message(&self, messages: &[ApiMessage]) -> Result<MessageStream>;
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Why 10% Buffer?

**Prevents hard limits**: Token counting is approximate
- Tokenizers vary between models
- Image token estimation is heuristic
- Buffer ensures we never hit hard context limit

### 2. Why Even Number of Messages?

**Maintains conversation structure**:
```
User: Question 1
Assistant: Answer 1    ← If we remove just the user message,
User: Question 2       ← this response loses context
```

**Even removal preserves pairs**: User+Assistant units stay intact

### 3. Why Keep First Message?

**First message usually contains task description**:
```
User: "Refactor this codebase to use async/await..."
[... many turns ...]
User: "Now add error handling"
```

Without first message, AI loses the original task context.

### 4. Why 50% Default Fraction?

**Balance between context and freshness**:
- Too aggressive (90%) → loses too much history
- Too conservative (20%) → doesn't free enough tokens
- 50% → good middle ground

### 5. Profile Thresholds vs Global

**Flexibility for different use cases**:
- Code tasks: Need more context → lower threshold (60%)
- Chat tasks: Less context needed → higher threshold (85%)
- Per-profile control without changing global settings

---

## 📊 PERFORMANCE CHARACTERISTICS

### Sliding Window
- **Speed**: O(n) where n = number of messages
- **Memory**: O(n) for cloning messages
- **Cost**: Free (no API calls)
- **Quality**: Loses context abruptly

### AI Condensing
- **Speed**: API call latency (1-5 seconds)
- **Memory**: Same as sliding window
- **Cost**: ~$0.01-0.05 per condensing (depends on model)
- **Quality**: Preserves semantic meaning

### Comparison

| Metric | Sliding Window | AI Condensing |
|--------|---------------|---------------|
| Speed | Instant | 1-5 seconds |
| Cost | $0 | $0.01-0.05 |
| Context Loss | High | Low |
| Deterministic | Yes | No |
| Requires API | No | Yes |

---

## 🚨 EDGE CASES

### 1. Very Short Conversations

```typescript
// Only 2 messages
const messages = [
    { role: "user", content: "First" },
    { role: "assistant", content: "Second" },
]

// frac = 0.5, 1 message after first, 0.5 * 1 = 0.5 → 0
// Result: No truncation
```

### 2. Empty Last Message

```typescript
const lastMessage = { role: "user", content: "" }
const tokens = await estimateTokenCount([], apiHandler)  // Returns 0
```

### 3. Condensing Fails

```typescript
// Automatic fallback to sliding window
if (result.error) {
    // Use sliding window instead
    const truncated = truncateConversation(messages, 0.5, taskId)
}
```

### 4. Invalid Profile Threshold

```typescript
profileThresholds = { "invalid": 150 }  // > 100
// Logs warning, uses global threshold
```

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `@anthropic-ai/sdk` - Type definitions
- `@clean-code/telemetry` - Usage tracking
- `@clean-code/types` - Shared types

**Internal Modules**:
- `../condense` - AI summarization
- `../../api` - API handlers
- `../task-persistence/apiMessages` - Message types

**Rust Crates**:
- `async-trait` - Async trait support
- `tokio` - Async runtime
- `log` - Logging

---

## 🎓 KEY TAKEAWAYS

✅ **Dual Strategy**: Sliding window (fast) + AI condensing (smart)

✅ **Safety First**: 10% buffer prevents context overflow

✅ **Conversation Integrity**: Always preserve first message and even pairs

✅ **Flexible Thresholds**: Global + per-profile configuration

✅ **Graceful Degradation**: Falls back to sliding window if condensing fails

✅ **Well-Tested**: 1248 lines of tests covering all edge cases

✅ **Production-Ready**: Telemetry, error handling, profile support

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Medium-High
**Estimated Effort**: 4-6 hours
**Lines of Rust**: ~250 lines (more structured than TypeScript)
**Dependencies**: `async-trait`, `tokio`, logging
**Key Challenge**: Async trait methods for API handlers
**Risk**: Medium - async complexity, requires careful testing

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-18 (task-persistence/)
