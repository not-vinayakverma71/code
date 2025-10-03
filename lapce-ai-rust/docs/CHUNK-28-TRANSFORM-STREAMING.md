# CHUNK 06: src/api/transform/ - STREAMING & FORMAT CONVERSION (32 FILES)

## Overview
The transform layer handles:
- **Message format conversion** (Anthropic ↔ OpenAI ↔ Gemini ↔ etc.)
- **Streaming chunk normalization** (all providers → unified ApiStream)
- **Prompt caching strategies** (cache_control injection)
- **Image handling** (base64 encoding, URL conversion)
- **Special model handling** (O1/O3, R1, reasoning models)

## File Structure
```
transform/
├── stream.ts - Core streaming types
├── openai-format.ts - Anthropic → OpenAI conversion
├── gemini-format.ts - Anthropic → Gemini conversion
├── bedrock-converse-format.ts - Anthropic → AWS Bedrock
├── mistral-format.ts - Anthropic → Mistral
├── vscode-lm-format.ts - VS Code Language Model API
├── simple-format.ts - Legacy simple format
├── r1-format.ts - DeepSeek R1 reasoning format
├── reasoning.ts - Reasoning token handling
├── model-params.ts - Model parameter extraction
├── image-cleaning.ts - Image deduplication
├── cache-strategy/
│   ├── base-strategy.ts - Abstract cache strategy
│   ├── multi-point-strategy.ts - Multi-message caching
│   └── types.ts - Cache strategy types
└── caching/
    ├── anthropic.ts - Claude cache_control
    ├── gemini.ts - Gemini caching
    └── vertex.ts - Vertex AI caching
```

## Core Type: ApiStream

**The Universal Streaming Interface:**
```typescript
export type ApiStream = AsyncGenerator<ApiStreamChunk>

export type ApiStreamChunk = 
    | ApiStreamTextChunk 
    | ApiStreamUsageChunk 
    | ApiStreamReasoningChunk 
    | ApiStreamError

export interface ApiStreamTextChunk {
    type: "text"
    text: string
}

export interface ApiStreamReasoningChunk {
    type: "reasoning"
    text: string
}

export interface ApiStreamUsageChunk {
    type: "usage"
    inputTokens: number
    outputTokens: number
    cacheWriteTokens?: number
    cacheReadTokens?: number
    reasoningTokens?: number
    totalCost?: number
}

export interface ApiStreamError {
    type: "error"
    error: string
    message: string
}
```

**RUST TRANSLATION:**
```rust
use futures::Stream;
use std::pin::Pin;

pub type ApiStream = Pin<Box<dyn Stream<Item = Result<ApiStreamChunk, Error>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiStreamChunk {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "reasoning")]
    Reasoning { text: String },
    
    #[serde(rename = "usage")]
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_write_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_read_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost: Option<f64>,
    },
    
    #[serde(rename = "error")]
    Error { error: String, message: String },
}
```

## Message Format Conversion: Anthropic → OpenAI

**The Core Conversion Algorithm:**
```typescript
export function convertToOpenAiMessages(
    anthropicMessages: Anthropic.Messages.MessageParam[]
): OpenAI.Chat.ChatCompletionMessageParam[] {
    const openAiMessages: OpenAI.Chat.ChatCompletionMessageParam[] = []
    
    for (const anthropicMessage of anthropicMessages) {
        if (typeof anthropicMessage.content === "string") {
            // Simple text message
            openAiMessages.push({ 
                role: anthropicMessage.role, 
                content: anthropicMessage.content 
            })
        } else if (anthropicMessage.role === "user") {
            // Split tool results from regular content
            const { nonToolMessages, toolMessages } = anthropicMessage.content.reduce(
                (acc, part) => {
                    if (part.type === "tool_result") {
                        acc.toolMessages.push(part)
                    } else if (part.type === "text" || part.type === "image") {
                        acc.nonToolMessages.push(part)
                    }
                    return acc
                },
                { nonToolMessages: [], toolMessages: [] }
            )
            
            // Process tool results first (must follow tool use messages)
            toolMessages.forEach(toolMessage => {
                let content: string
                if (typeof toolMessage.content === "string") {
                    content = toolMessage.content
                } else {
                    // Flatten complex tool results to string
                    content = toolMessage.content
                        .map(part => part.type === "image" 
                            ? "(see following user message for image)" 
                            : part.text
                        )
                        .join("\n")
                }
                
                openAiMessages.push({
                    role: "tool",
                    tool_call_id: toolMessage.tool_use_id,
                    content
                })
            })
            
            // Process regular content
            if (nonToolMessages.length > 0) {
                openAiMessages.push({
                    role: "user",
                    content: nonToolMessages.map(part => {
                        if (part.type === "image") {
                            return {
                                type: "image_url",
                                image_url: {
                                    url: `data:${part.source.media_type};base64,${part.source.data}`
                                }
                            }
                        }
                        return { type: "text", text: part.text }
                    })
                })
            }
        } else if (anthropicMessage.role === "assistant") {
            // Convert tool_use blocks to OpenAI function calls
            const textContent = anthropicMessage.content
                .filter(block => block.type === "text")
                .map(block => block.text)
                .join("")
            
            const toolCalls = anthropicMessage.content
                .filter(block => block.type === "tool_use")
                .map(block => ({
                    id: block.id,
                    type: "function" as const,
                    function: {
                        name: block.name,
                        arguments: JSON.stringify(block.input)
                    }
                }))
            
            openAiMessages.push({
                role: "assistant",
                content: textContent || null,
                tool_calls: toolCalls.length > 0 ? toolCalls : undefined
            })
        }
    }
    
    return openAiMessages
}
```

**CRITICAL DETAILS:**
1. **Tool results → role: "tool"** - Anthropic's `tool_result` becomes separate OpenAI message
2. **Image handling** - Base64 data URL format required
3. **Tool calls** - Different structure between providers
4. **Content flattening** - Complex Anthropic content → simple OpenAI strings

**RUST IMPLEMENTATION:**
```rust
pub fn convert_to_openai_messages(
    anthropic_messages: Vec<MessageParam>
) -> Result<Vec<ChatCompletionMessage>, Error> {
    let mut openai_messages = Vec::new();
    
    for msg in anthropic_messages {
        match msg.role {
            Role::User => {
                let (tool_results, regular_content): (Vec<_>, Vec<_>) = msg.content
                    .into_iter()
                    .partition(|c| matches!(c, ContentBlock::ToolResult { .. }));
                
                // Process tool results first
                for tool_result in tool_results {
                    if let ContentBlock::ToolResult { tool_use_id, content, .. } = tool_result {
                        openai_messages.push(ChatCompletionMessage::Tool {
                            tool_call_id: tool_use_id,
                            content: flatten_tool_result_content(content)?,
                        });
                    }
                }
                
                // Process regular content
                if !regular_content.is_empty() {
                    let content = regular_content.into_iter()
                        .map(|block| match block {
                            ContentBlock::Text { text } => {
                                ChatCompletionContent::Text { text }
                            }
                            ContentBlock::Image { source } => {
                                ChatCompletionContent::ImageUrl {
                                    url: format!(
                                        "data:{};base64,{}",
                                        source.media_type,
                                        source.data
                                    )
                                }
                            }
                            _ => unreachable!()
                        })
                        .collect();
                    
                    openai_messages.push(ChatCompletionMessage::User { content });
                }
            }
            Role::Assistant => {
                let text = msg.content.iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                
                let tool_calls = msg.content.iter()
                    .filter_map(|block| match block {
                        ContentBlock::ToolUse { id, name, input } => Some(ToolCall {
                            id: id.clone(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: name.clone(),
                                arguments: serde_json::to_string(input).ok()?,
                            }
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                
                openai_messages.push(ChatCompletionMessage::Assistant {
                    content: if text.is_empty() { None } else { Some(text) },
                    tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                });
            }
        }
    }
    
    Ok(openai_messages)
}
```

## Gemini Format Conversion

Gemini has its own unique format:
```typescript
export function convertToGeminiMessages(
    anthropicMessages: Anthropic.Messages.MessageParam[]
): GeminiMessage[] {
    const geminiMessages: GeminiMessage[] = []
    
    for (const msg of anthropicMessages) {
        if (msg.role === "user") {
            geminiMessages.push({
                role: "user",
                parts: msg.content.map(part => {
                    if (part.type === "text") {
                        return { text: part.text }
                    } else if (part.type === "image") {
                        return {
                            inlineData: {
                                mimeType: part.source.media_type,
                                data: part.source.data
                            }
                        }
                    }
                })
            })
        } else if (msg.role === "assistant") {
            geminiMessages.push({
                role: "model", // Gemini uses "model" not "assistant"
                parts: msg.content.map(part => ({ text: part.text }))
            })
        }
    }
    
    return geminiMessages
}
```

## R1 Format (DeepSeek Reasoning)

DeepSeek R1 requires special formatting:
```typescript
export function convertToR1Format(
    messages: Anthropic.Messages.MessageParam[]
): OpenAI.Chat.ChatCompletionMessageParam[] {
    // R1 models combine system + user into single user message
    return messages.map(msg => {
        if (msg.role === "assistant") {
            // Split reasoning from final answer
            const content = extractReasoningAndAnswer(msg.content)
            return {
                role: "assistant",
                content: content.reasoning 
                    ? `<think>${content.reasoning}</think>\n${content.answer}`
                    : content.answer
            }
        }
        return msg
    })
}
```

## Prompt Caching Strategies

**Multi-Point Cache Strategy:**
```typescript
export class MultiPointCacheStrategy {
    applyCacheControl(
        messages: MessageParam[],
        systemPrompt: string
    ): { system: SystemPrompt, messages: MessageParam[] } {
        // Cache system prompt
        const system = {
            text: systemPrompt,
            cache_control: { type: "ephemeral" }
        }
        
        // Find last 2 user messages and cache them
        const userMsgIndices = messages
            .map((msg, idx) => msg.role === "user" ? idx : -1)
            .filter(idx => idx !== -1)
        
        const indicesToCache = userMsgIndices.slice(-2)
        
        const cachedMessages = messages.map((msg, idx) => {
            if (indicesToCache.includes(idx)) {
                // Add cache_control to last content block
                return {
                    ...msg,
                    content: addCacheControlToLastBlock(msg.content)
                }
            }
            return msg
        })
        
        return { system, messages: cachedMessages }
    }
}
```

**Why Cache Last 2 User Messages?**
- Message N-2: Previous user message (cache hit)
- Message N-1: Previous response cycle (cache hit)
- Message N: New user message (cache write)

This maximizes cache reuse while minimizing writes.

## Image Deduplication

```typescript
export function maybeRemoveImageBlocks(
    messages: MessageParam[]
): MessageParam[] {
    const seenImages = new Set<string>()
    
    return messages.map(msg => {
        if (msg.role !== "user") return msg
        
        const dedupedContent = msg.content.filter(block => {
            if (block.type !== "image") return true
            
            const imageHash = hashImageData(block.source.data)
            if (seenImages.has(imageHash)) {
                return false // Remove duplicate
            }
            seenImages.add(imageHash)
            return true
        })
        
        return { ...msg, content: dedupedContent }
    })
}
```

## Model Parameters Extraction

```typescript
export function getModelParams(modelInfo: ModelInfo) {
    return {
        maxTokens: modelInfo.maxTokens || 8192,
        temperature: modelInfo.temperature || 1.0,
        topP: modelInfo.topP || 1.0,
        topK: modelInfo.topK,
        reasoning: modelInfo.reasoning,
    }
}
```

## RUST Translation Requirements

### 1. Implement All Format Converters
```rust
pub mod format {
    pub fn to_openai(msgs: Vec<MessageParam>) -> Vec<ChatCompletionMessage>;
    pub fn to_gemini(msgs: Vec<MessageParam>) -> Vec<GeminiMessage>;
    pub fn to_bedrock(msgs: Vec<MessageParam>) -> Vec<BedrockMessage>;
    pub fn to_mistral(msgs: Vec<MessageParam>) -> Vec<MistralMessage>;
    pub fn to_r1(msgs: Vec<MessageParam>) -> Vec<ChatCompletionMessage>;
}
```

### 2. Implement Streaming Adapters
```rust
pub trait StreamAdapter {
    type Input;
    type Output = ApiStreamChunk;
    
    fn adapt(input: Self::Input) -> Result<Self::Output, Error>;
}

pub struct AnthropicStreamAdapter;
impl StreamAdapter for AnthropicStreamAdapter {
    type Input = anthropic::MessageStreamEvent;
    
    fn adapt(input: Self::Input) -> Result<ApiStreamChunk, Error> {
        match input {
            MessageStreamEvent::ContentBlockDelta { delta } => {
                Ok(ApiStreamChunk::Text { text: delta.text })
            }
            MessageStreamEvent::MessageDelta { usage } => {
                Ok(ApiStreamChunk::Usage { /* ... */ })
            }
            // ...
        }
    }
}
```

### 3. Implement Cache Strategies
```rust
pub trait CacheStrategy {
    fn apply_cache_control(
        &self,
        system: String,
        messages: Vec<MessageParam>
    ) -> CachedMessages;
}

pub struct MultiPointStrategy;
impl CacheStrategy for MultiPointStrategy {
    fn apply_cache_control(
        &self,
        system: String,
        messages: Vec<MessageParam>
    ) -> CachedMessages {
        // Implementation
    }
}
```

## Next: CHUNK 07 - VS Code → Lapce Mapping
Complete API mapping for all VS Code dependencies.
