# CHUNK 05: src/api/providers/ - 40+ LLM PROVIDERS (145 FILES)

## Overview
The API providers directory contains implementations for **40+ different LLM providers**, each with streaming support, error handling, and provider-specific features like prompt caching and token counting.

## Provider List (Alphabetically)
```
1.  Anthropic (Claude) - Official SDK
2.  Anthropic Vertex - GCP Vertex AI
3.  AWS Bedrock - AWS managed models
4.  Azure OpenAI - Microsoft Azure
5.  Cerebras - High-performance inference
6.  Chutes - Custom provider
7.  Claude Code - Code-specific Claude
8.  DeepInfra - Cloud inference
9.  DeepSeek - Chinese LLM provider
10. Doubao - ByteDance LLM
11. Fake AI - Testing/development
12. Featherless - Custom provider
13. Fireworks - Fast inference
14. Gemini - Google Gemini API
15. Gemini CLI - Command-line wrapper
16. Glama - Custom provider
17. Groq - Ultra-fast inference
18. HuggingFace - HF Inference API
19. Human Relay - Human-in-the-loop
20. IO Intelligence - Custom provider
21. Kilocode OpenRouter - Custom routing
22. LiteLLM - Proxy/gateway
23. LM Studio - Local inference
24. Mistral - Mistral AI
25. Moonshot - Chinese LLM
26. Native Ollama - Local Ollama
27. Ollama - Local models
28. OpenAI - Official OpenAI
29. OpenAI Native - VS Code integrated
30. OpenRouter - Multi-provider router
31. Qwen Code - Alibaba code model
32. Requesty - Custom provider
33. Roo - Custom provider
34. SambaNova - Fast inference
35. Unbound - Custom provider
36. Vertex - GCP Vertex AI
37. Virtual Quota Fallback - Quota management
38. VS Code LM - Built-in VS Code
39. XAI - X.AI (Grok)
40. ZAI - Custom provider
```

## Architecture Pattern: ApiHandler Interface

All providers implement the same interface:

```typescript
export interface ApiHandler {
    createMessage(
        systemPrompt: string,
        messages: Anthropic.Messages.MessageParam[],
        metadata?: ApiHandlerCreateMessageMetadata
    ): ApiStream
    
    getModel(): { id: string; info: ModelInfo }
    
    countTokens(content: Array<Anthropic.Messages.ContentBlockParam>): Promise<number>
}
```

**CRITICAL:** All messages use **Anthropic SDK types** as the common format, even for OpenAI, Gemini, etc. Each provider converts internally.

## Factory Pattern: buildApiHandler()

```typescript
export function buildApiHandler(configuration: ProviderSettings): ApiHandler {
    const { apiProvider, ...options } = configuration
    
    switch (apiProvider) {
        case "anthropic":
            return new AnthropicHandler(options)
        case "openai":
            return new OpenAiHandler(options)
        case "bedrock":
            return new AwsBedrockHandler(options)
        case "gemini":
            return new GeminiHandler(options)
        case "openrouter":
            return new OpenRouterHandler(options)
        // ... 35+ more cases
        default:
            throw new Error(`Unknown API provider: ${apiProvider}`)
    }
}
```

**RUST TRANSLATION:**
```rust
pub enum ApiProvider {
    Anthropic,
    OpenAi,
    Bedrock,
    Gemini,
    OpenRouter,
    // ... 35+ variants
}

pub fn build_api_handler(config: ProviderSettings) -> Result<Box<dyn ApiHandler>, Error> {
    match config.api_provider {
        ApiProvider::Anthropic => Ok(Box::new(AnthropicHandler::new(config)?)),
        ApiProvider::OpenAi => Ok(Box::new(OpenAiHandler::new(config)?)),
        ApiProvider::Bedrock => Ok(Box::new(AwsBedrockHandler::new(config)?)),
        ApiProvider::Gemini => Ok(Box::new(GeminiHandler::new(config)?)),
        ApiProvider::OpenRouter => Ok(Box::new(OpenRouterHandler::new(config)?)),
        // ... 35+ arms
    }
}

#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn create_message(
        &self,
        system_prompt: String,
        messages: Vec<MessageParam>,
        metadata: Option<ApiHandlerCreateMessageMetadata>
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ApiStreamChunk, Error>> + Send>>, Error>;
    
    fn get_model(&self) -> ModelInfo;
    
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<u32, Error>;
}
```

## Deep Dive: AnthropicHandler

```typescript
export class AnthropicHandler extends BaseProvider {
    private client: Anthropic
    
    constructor(options: ApiHandlerOptions) {
        super()
        this.client = new Anthropic({
            baseURL: options.anthropicBaseUrl || undefined,
            apiKey: options.apiKey,
        })
    }
    
    async *createMessage(
        systemPrompt: string,
        messages: Anthropic.Messages.MessageParam[],
        metadata?: ApiHandlerCreateMessageMetadata
    ): ApiStream {
        const { id: modelId, betas = [], maxTokens, temperature } = this.getModel()
        
        // Prompt caching for Claude 3.5+
        const cacheControl: CacheControlEphemeral = { type: "ephemeral" }
        
        // Mark last 2 user messages for caching
        const userMsgIndices = messages.reduce(
            (acc, msg, index) => (msg.role === "user" ? [...acc, index] : acc),
            [] as number[]
        )
        
        const stream = await this.client.messages.create({
            model: modelId,
            max_tokens: maxTokens ?? ANTHROPIC_DEFAULT_MAX_TOKENS,
            temperature,
            system: [{ 
                text: systemPrompt, 
                type: "text", 
                cache_control: cacheControl 
            }],
            messages: messages.map((message, index) => {
                // Add cache_control to last 2 user messages
                if (userMsgIndices.includes(index)) {
                    return {
                        ...message,
                        content: /* add cache_control */
                    }
                }
                return message
            }),
            stream: true,
        }, {
            headers: { "anthropic-beta": "prompt-caching-2024-07-31" }
        })
        
        // Parse stream chunks
        for await (const chunk of stream) {
            switch (chunk.type) {
                case "message_start":
                    yield { type: "usage", usage: chunk.message.usage }
                    break
                case "content_block_start":
                    if (chunk.content_block.type === "text") {
                        yield { type: "text", text: "" }
                    } else if (chunk.content_block.type === "tool_use") {
                        yield { 
                            type: "tool_use",
                            id: chunk.content_block.id,
                            name: chunk.content_block.name
                        }
                    }
                    break
                case "content_block_delta":
                    if (chunk.delta.type === "text_delta") {
                        yield { type: "text", text: chunk.delta.text }
                    } else if (chunk.delta.type === "input_json_delta") {
                        yield { type: "tool_use", input: chunk.delta.partial_json }
                    }
                    break
                case "message_delta":
                    yield { type: "usage", usage: chunk.usage }
                    break
            }
        }
    }
}
```

**CRITICAL FEATURES:**
1. **Prompt caching** - Reduces costs by caching system prompt and last 2 user messages
2. **Streaming** - Async generator yields chunks as they arrive
3. **Usage tracking** - Reports token usage for cost calculation

## Deep Dive: OpenAiHandler

```typescript
export class OpenAiHandler extends BaseProvider {
    private client: OpenAI | AzureOpenAI
    
    constructor(options: ApiHandlerOptions) {
        super()
        const isAzure = this.isAzureOpenAi(options.openAiBaseUrl)
        
        if (isAzure) {
            this.client = new AzureOpenAI({
                baseURL: options.openAiBaseUrl,
                apiKey: options.openAiApiKey,
                apiVersion: options.azureApiVersion || "2024-10-21",
            })
        } else {
            this.client = new OpenAI({
                baseURL: options.openAiBaseUrl ?? "https://api.openai.com/v1",
                apiKey: options.openAiApiKey,
            })
        }
    }
    
    async *createMessage(
        systemPrompt: string,
        messages: Anthropic.Messages.MessageParam[],
        metadata?: ApiHandlerCreateMessageMetadata
    ): ApiStream {
        const modelId = this.options.openAiModelId ?? "gpt-4"
        
        // Special handling for O1/O3 models (no system prompt, no streaming)
        if (modelId.includes("o1") || modelId.includes("o3")) {
            yield* this.handleO3FamilyMessage(modelId, systemPrompt, messages)
            return
        }
        
        // Convert Anthropic format to OpenAI format
        const systemMessage = { role: "system", content: systemPrompt }
        const convertedMessages = [
            systemMessage,
            ...convertToOpenAiMessages(messages)
        ]
        
        // Add prompt caching for compatible providers
        if (this.getModel().info.supportsPromptCache) {
            // Mark last 2 user messages with cache_control
            const lastTwoUserMessages = convertedMessages
                .filter(msg => msg.role === "user")
                .slice(-2)
            
            lastTwoUserMessages.forEach(msg => {
                // @ts-ignore
                msg.content[0]["cache_control"] = { type: "ephemeral" }
            })
        }
        
        const stream = await this.client.chat.completions.create({
            model: modelId,
            messages: convertedMessages,
            stream: true,
            stream_options: { include_usage: true }
        })
        
        for await (const chunk of stream) {
            const delta = chunk.choices[0]?.delta
            
            if (delta?.content) {
                yield { type: "text", text: delta.content }
            }
            
            if (delta?.tool_calls) {
                for (const toolCall of delta.tool_calls) {
                    yield {
                        type: "tool_use",
                        id: toolCall.id,
                        name: toolCall.function.name,
                        input: toolCall.function.arguments
                    }
                }
            }
            
            if (chunk.usage) {
                yield { 
                    type: "usage", 
                    usage: {
                        inputTokens: chunk.usage.prompt_tokens,
                        outputTokens: chunk.usage.completion_tokens
                    }
                }
            }
        }
    }
}
```

**KEY DIFFERENCES FROM ANTHROPIC:**
1. **Format conversion** - Must convert Anthropic types to OpenAI types
2. **Tool calling** - Different format than Claude
3. **O1/O3 special handling** - No streaming, no system prompt
4. **Azure support** - Different SDK initialization

## Message Format Conversion

**Anthropic → OpenAI:**
```typescript
function convertToOpenAiMessages(
    messages: Anthropic.Messages.MessageParam[]
): OpenAI.Chat.ChatCompletionMessageParam[] {
    return messages.map(msg => {
        if (msg.role === "user") {
            if (typeof msg.content === "string") {
                return { role: "user", content: msg.content }
            } else {
                return {
                    role: "user",
                    content: msg.content.map(block => {
                        if (block.type === "text") {
                            return { type: "text", text: block.text }
                        } else if (block.type === "image") {
                            return {
                                type: "image_url",
                                image_url: { url: block.source.data }
                            }
                        }
                    })
                }
            }
        } else if (msg.role === "assistant") {
            // Convert tool_use blocks to function calls
            const toolCalls = msg.content
                .filter(block => block.type === "tool_use")
                .map(block => ({
                    id: block.id,
                    type: "function",
                    function: {
                        name: block.name,
                        arguments: JSON.stringify(block.input)
                    }
                }))
            
            return {
                role: "assistant",
                content: msg.content
                    .filter(block => block.type === "text")
                    .map(block => block.text)
                    .join(""),
                tool_calls: toolCalls.length > 0 ? toolCalls : undefined
            }
        }
    })
}
```

**RUST REQUIREMENT:** Must implement exact same conversion logic.

## Streaming Architecture

All providers return an **async generator** (ApiStream):

```typescript
export type ApiStream = AsyncGenerator<
    | ApiStreamTextChunk
    | ApiStreamUsageChunk
    | ApiStreamToolChunk,
    void,
    unknown
>

export interface ApiStreamTextChunk {
    type: "text"
    text: string
}

export interface ApiStreamUsageChunk {
    type: "usage"
    usage: {
        inputTokens: number
        outputTokens: number
        cacheReadTokens?: number
        cacheWriteTokens?: number
    }
}

export interface ApiStreamToolChunk {
    type: "tool_use"
    id?: string
    name?: string
    input?: string
}
```

**RUST TRANSLATION:**
```rust
#[derive(Debug, Clone)]
pub enum ApiStreamChunk {
    Text { text: String },
    Usage { usage: TokenUsage },
    ToolUse { id: Option<String>, name: Option<String>, input: Option<String> },
}

pub type ApiStream = Pin<Box<dyn Stream<Item = Result<ApiStreamChunk, Error>> + Send>>;

// Example implementation
impl AnthropicHandler {
    pub async fn create_message(
        &self,
        system_prompt: String,
        messages: Vec<MessageParam>,
        metadata: Option<ApiHandlerCreateMessageMetadata>
    ) -> Result<ApiStream, Error> {
        let stream = self.client.messages()
            .create(/* ... */)
            .await?;
        
        let mapped_stream = stream.map(|chunk_result| {
            let chunk = chunk_result?;
            match chunk {
                AnthropicChunk::TextDelta { text } => {
                    Ok(ApiStreamChunk::Text { text })
                }
                AnthropicChunk::Usage { usage } => {
                    Ok(ApiStreamChunk::Usage { usage })
                }
                // ... other variants
            }
        });
        
        Ok(Box::pin(mapped_stream))
    }
}
```

## Token Counting

Each provider can override token counting:

```typescript
abstract class BaseProvider {
    // Default: use tiktoken (GPT tokenizer)
    async countTokens(content: Array<ContentBlockParam>): Promise<number> {
        const text = content
            .filter(block => block.type === "text")
            .map(block => block.text)
            .join(" ")
        
        const encoding = get_encoding("cl100k_base")
        return encoding.encode(text).length
    }
}

// Provider-specific override
class AnthropicHandler extends BaseProvider {
    async countTokens(content: Array<ContentBlockParam>): Promise<number> {
        // Use Anthropic's native token counting API
        const response = await this.client.messages.countTokens({
            model: this.getModel().id,
            messages: [{ role: "user", content }]
        })
        return response.input_tokens
    }
}
```

**RUST:**
```rust
#[async_trait]
pub trait ApiHandler {
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<u32, Error> {
        // Default: tiktoken
        let text: String = content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        
        let encoding = tiktoken_rs::cl100k_base()?;
        Ok(encoding.encode_ordinary(&text).len() as u32)
    }
}

pub struct AnthropicHandler {
    client: anthropic::Client,
}

#[async_trait]
impl ApiHandler for AnthropicHandler {
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<u32, Error> {
        // Use Anthropic API
        let response = self.client
            .messages()
            .count_tokens()
            .model(self.get_model().id)
            .messages(vec![Message::user(content)])
            .send()
            .await?;
        
        Ok(response.input_tokens)
    }
}
```

## Critical Dependencies

### Anthropic SDK
```typescript
import { Anthropic } from "@anthropic-ai/sdk"
import { Stream } from "@anthropic-ai/sdk/streaming"
```

**RUST:** Use `anthropic-sdk` crate (if exists) or implement from scratch

### OpenAI SDK
```typescript
import OpenAI, { AzureOpenAI } from "openai"
```

**RUST:** Use `async-openai` crate

### AWS SDK (Bedrock)
```typescript
import { BedrockRuntime } from "@aws-sdk/client-bedrock-runtime"
```

**RUST:** Use `aws-sdk-bedrockruntime` crate

### Google SDK (Gemini/Vertex)
```typescript
import { GoogleGenerativeAI } from "@google/generative-ai"
import { VertexAI } from "@google-cloud/vertexai"
```

**RUST:** Use `google-generativeai` or implement REST API

## Error Handling

All providers must handle:

```typescript
try {
    const stream = await this.client.messages.create(/* ... */)
    for await (const chunk of stream) {
        yield chunk
    }
} catch (error) {
    if (error instanceof Anthropic.APIError) {
        if (error.status === 429) {
            throw new Error("Rate limit exceeded")
        } else if (error.status === 401) {
            throw new Error("Invalid API key")
        } else if (error.status === 400) {
            throw new Error(`Bad request: ${error.message}`)
        }
    }
    throw error
}
```

## Next: CHUNK 06 - Transform Pipeline
Message transformations, streaming, and format conversions.
