# CHUNK-30: SHARED/ - MESSAGE TYPES & SHARED LOGIC (47 FILES)

## 📁 MODULE STRUCTURE

```
Codex/src/shared/
├── ExtensionMessage.ts    (502 lines) - Extension → Webview messages
├── WebviewMessage.ts      (436 lines) - Webview → Extension messages
├── api.ts                 (263 lines) - Model definitions & API types
├── modes.ts               (384 lines) - Mode system & tool groups
├── tools.ts               (8872 lines) - Tool definitions
├── support-prompt.ts      (10075 lines) - Support prompts
├── embeddingModels.ts     (5521 lines) - Embedding model configs
├── context-mentions.ts    (4685 lines) - @mention system
├── combineCommandSequences.ts (4613 lines) - Command merging
├── getApiMetrics.ts       (4116 lines) - API cost tracking
├── cost.ts                (1875 lines) - Cost calculations
├── ProfileValidator.ts    (2601 lines) - Profile validation
├── combineApiRequests.ts  (2370 lines) - Request combining
├── experiments.ts         (1328 lines) - Feature flags
├── mcp.ts                 (1394 lines) - MCP server types
├── kilocode/              (5 files) - Kilocode-specific
│   ├── errorUtils.ts
│   ├── kiloLanguages.ts
│   ├── mcp.ts
│   ├── rules.ts
│   └── token.ts
└── utils/                 (2 files) - Shared utilities
    └── requesty.ts
```

**Total**: 47 TypeScript files, ~55,000+ lines

---

## 🎯 PURPOSE

Central repository for **shared types and business logic** used by both Extension and Webview:

1. **Message Protocols**: ExtensionMessage ↔ WebviewMessage communication
2. **API Configurations**: 40+ AI provider model definitions
3. **Mode System**: Tool groups, custom modes, prompt engineering
4. **Cost Tracking**: Token usage and API cost calculations
5. **Context System**: @mentions, file tracking
6. **Feature Flags**: Experimental features management

---

## 🔄 MESSAGE PROTOCOLS

### 1. ExtensionMessage (Extension → Webview)

```typescript
export interface ExtensionMessage {
    type:
        | "action"              // Task state update
        | "state"               // Full state sync
        | "selectedImages"      // Image selection result
        | "theme"               // VS Code theme update
        | "workspaceUpdated"    // Workspace change
        | "invoke"              // Tool invocation
        | "messageUpdated"      // Message stream update
        | "mcpServers"          // MCP server list
        | "enhancedPrompt"      // AI-enhanced prompt
        | "commitSearchResults" // Git commit search
        | "listApiConfig"       // API configurations
        | "routerModels"        // OpenRouter models
        | "openAiModels"        // OpenAI models
        | "ollamaModels"        // Ollama models
        | "lmStudioModels"      // LM Studio models
        | "vsCodeLmModels"      // VS Code LM models
        | "systemPrompt"        // System prompt update
        | "autoApprovalEnabled" // Auto-approve status
        | "currentCheckpointUpdated" // Checkpoint status
        | "browserToolEnabled"  // Browser tool status
        // ... 30+ more message types
}
```

**Key Message Types**:

```typescript
// Task action update
{
    type: "action",
    action: "api_req_started" | "api_req_finished" | "error" | "completed",
    text?: string,
    images?: string[]
}

// Full state sync
{
    type: "state",
    state: {
        version: string,
        taskHistory: HistoryItem[],
        currentTaskId?: string,
        shouldShowAnnouncement: boolean,
        apiConfiguration: ProviderSettings,
        customInstructions?: string,
        // ... 50+ state fields
    }
}

// MCP server update
{
    type: "mcpServers",
    mcpServers: McpServer[]
}

// Indexing status
{
    type: "indexingStatusUpdate",
    values: {
        systemStatus: "idle" | "processing" | "error",
        processedItems: number,
        totalItems: number,
        message?: string
    }
}
```

### 2. WebviewMessage (Webview → Extension)

```typescript
export interface WebviewMessage {
    type:
        | "webviewDidLaunch"        // Webview ready
        | "newTask"                 // Start new task
        | "askResponse"             // User response to ask
        | "clearTask"               // Clear current task
        | "exportCurrentTask"       // Export task
        | "shareCurrentTask"        // Share to cloud
        | "deleteTaskWithId"        // Delete task
        | "selectImages"            // Open image selector
        | "openFile"                // Open file in editor
        | "openMention"             // Open @mentioned file
        | "cancelTask"              // Cancel running task
        | "saveApiConfiguration"    // Save API config
        | "customInstructions"      // Update custom instructions
        | "toggleToolAutoApprove"   // Toggle auto-approve
        | "requestRouterModels"     // Fetch router models
        | "alwaysAllowReadOnly"     // Permission toggle
        | "alwaysAllowWrite"        // Permission toggle
        | "alwaysAllowExecute"      // Permission toggle
        // ... 100+ message types
}
```

**Key Message Payloads**:

```typescript
// New task request
{
    type: "newTask",
    text: string,
    images?: string[],
    mentions?: ContextMention[],
    mode?: Mode
}

// Ask response
{
    type: "askResponse",
    askResponse: "yesButtonClicked" | "noButtonClicked" | "messageResponse",
    text?: string
}

// API configuration
{
    type: "saveApiConfiguration",
    apiConfiguration: ProviderSettings
}

// Custom mode update
{
    type: "updateCustomMode",
    mode: ModeConfig
}
```

---

## 🤖 API CONFIGURATIONS: api.ts (263 lines)

### Model Definitions for 40+ Providers

```typescript
export const cerebrasModels = {
    "gpt-oss-120b": {
        maxTokens: 65536,
        contextWindow: 65536,
        supportsImages: false,
        supportsPromptCache: false,
        inputPrice: 0.25,   // per 1M tokens
        outputPrice: 0.69,  // per 1M tokens
        description: "OpenAI's GPT-OSS model with ~3000 tokens/s",
    },
    "llama-4-scout-17b-16e-instruct": {
        maxTokens: 8192,
        contextWindow: 8192,
        supportsImages: false,
        supportsPromptCache: false,
        inputPrice: 0.65,
        outputPrice: 0.85,
        description: "Llama 4 Scout with ~2600 tokens/s",
    },
    // ... 10+ Cerebras models
}

export const openaiModels = {
    "gpt-4o": {
        maxTokens: 16384,
        contextWindow: 128000,
        supportsImages: true,
        supportsPromptCache: true,
        inputPrice: 2.50,
        outputPrice: 10.00,
        description: "Most capable GPT-4 model",
    },
    "gpt-4o-mini": {
        maxTokens: 16384,
        contextWindow: 128000,
        supportsImages: true,
        supportsPromptCache: true,
        inputPrice: 0.15,
        outputPrice: 0.60,
        description: "Affordable and intelligent small model",
    },
    // ... 15+ OpenAI models
}

export const anthropicModels = {
    "claude-sonnet-4-20250514": {
        maxTokens: 8192,
        contextWindow: 200000,
        supportsImages: true,
        supportsPromptCache: true,
        supportsPromptCachingBeta: true,
        inputPrice: 3.00,
        outputPrice: 15.00,
        cacheWritesPrice: 3.75,
        cacheReadsPrice: 0.30,
        description: "Most intelligent model",
    },
    // ... 10+ Anthropic models
}

// Similar for: Gemini, Bedrock, DeepSeek, Mistral, Groq, etc.
```

### Model Record Type

```typescript
export interface ModelRecord {
    [modelId: string]: ModelInfo
}

export interface ModelInfo {
    maxTokens: number
    contextWindow: number
    supportsImages: boolean
    supportsPromptCache: boolean
    inputPrice: number
    outputPrice: number
    cacheWritesPrice?: number
    cacheReadsPrice?: number
    description?: string
    supportsComputerUse?: boolean
    betas?: string[]
}
```

### Router Models (Dynamic Discovery)

```typescript
export interface RouterModels {
    [modelId: string]: {
        id: string
        name: string
        context_length: number
        pricing: {
            prompt: string    // e.g., "0.000002"
            completion: string
            image?: string
            request?: string
        }
        architecture?: {
            modality: "text" | "multimodal"
        }
        top_provider?: {
            context_length: number
            throughput: number
            is_moderated: boolean
        }
    }
}
```

---

## 🎨 MODE SYSTEM: modes.ts (384 lines)

### Mode Configuration

```typescript
export interface ModeConfig {
    slug: string                      // Unique identifier
    name: string                      // Display name
    description: string               // Description
    groups: readonly GroupEntry[]     // Tool groups
    roleDefinition?: string           // System prompt override
    customPrompts?: CustomModePrompts // Prompt overrides
    experiments?: ExperimentId[]      // Feature flags
}

export type GroupEntry = ToolGroup | [ToolGroup, GroupOptions]

export interface GroupOptions {
    enabledTools?: string[]  // Subset of tools
    files?: string[]         // File patterns
}

export const DEFAULT_MODES: ModeConfig[] = [
    {
        slug: "code",
        name: "Code",
        description: "Full-stack development with all tools",
        groups: ["core", "code", "browser", "mcp"],
        roleDefinition: "You are Kilo Code, an expert software engineer...",
    },
    {
        slug: "architect",
        name: "Architect",
        description: "High-level system design",
        groups: ["core", "read-only"],
        roleDefinition: "You are a software architect...",
    },
    {
        slug: "ask",
        name: "Ask",
        description: "Question answering without code changes",
        groups: ["core"],
        roleDefinition: "You are a helpful assistant...",
    },
    // ... 10+ default modes
]
```

### Tool Groups

```typescript
export const TOOL_GROUPS: Record<ToolGroup, ToolGroupConfig> = {
    "core": {
        label: "Core",
        description: "Essential tools",
        tools: ["write_to_file", "read_file", "list_files"],
    },
    "code": {
        label: "Code",
        description: "Code manipulation",
        tools: ["search_files", "list_code_definition_names", "execute_command"],
    },
    "browser": {
        label: "Browser",
        description: "Web automation",
        tools: ["url_screenshot", "ask_followup_question"],
    },
    "mcp": {
        label: "MCP",
        description: "Model Context Protocol tools",
        tools: [], // Dynamically loaded from MCP servers
    },
}

export const ALWAYS_AVAILABLE_TOOLS = [
    "attempt_completion",
    "ask_followup_question",
]
```

### Mode Selection Logic

```typescript
export function getModeBySlug(
    slug: string, 
    customModes?: ModeConfig[]
): ModeConfig | undefined {
    // Check custom modes first
    const customMode = customModes?.find((mode) => mode.slug === slug)
    if (customMode) {
        return customMode
    }
    // Then check built-in modes
    return DEFAULT_MODES.find((mode) => mode.slug === slug)
}

export function getToolsForMode(groups: readonly GroupEntry[]): string[] {
    const tools = new Set<string>()
    
    // Add tools from each group
    groups.forEach((group) => {
        const groupName = getGroupName(group)
        const groupConfig = TOOL_GROUPS[groupName]
        groupConfig.tools.forEach((tool: string) => tools.add(tool))
    })
    
    // Always add required tools
    ALWAYS_AVAILABLE_TOOLS.forEach((tool) => tools.add(tool))
    
    return Array.from(tools)
}
```

---

## 🛠️ TOOL DEFINITIONS: tools.ts (8872 lines!)

### Tool Schema

```typescript
export interface ToolDefinition {
    name: string
    description: string
    input_schema: {
        type: "object"
        properties: Record<string, any>
        required?: string[]
    }
}

export const TOOL_DEFINITIONS: Record<string, ToolDefinition> = {
    "write_to_file": {
        name: "write_to_file",
        description: "Write content to a file...",
        input_schema: {
            type: "object",
            properties: {
                path: {
                    type: "string",
                    description: "Path to file (relative to workspace)",
                },
                content: {
                    type: "string",
                    description: "Content to write",
                },
            },
            required: ["path", "content"],
        },
    },
    
    "read_file": {
        name: "read_file",
        description: "Read file contents...",
        input_schema: {
            type: "object",
            properties: {
                path: { type: "string" },
            },
            required: ["path"],
        },
    },
    
    "execute_command": {
        name: "execute_command",
        description: "Execute shell command...",
        input_schema: {
            type: "object",
            properties: {
                command: { type: "string" },
            },
            required: ["command"],
        },
    },
    
    // ... 30+ tool definitions (8872 lines of detailed schemas)
}
```

---

## 💰 COST TRACKING: cost.ts (1875 lines)

### Cost Calculation

```typescript
export function calculateApiCostAnthropic(
    inputTokens: number,
    outputTokens: number,
    cacheWriteTokens: number,
    cacheReadTokens: number,
    modelInfo: ModelInfo
): number {
    const inputCost = (inputTokens / 1_000_000) * modelInfo.inputPrice
    const outputCost = (outputTokens / 1_000_000) * modelInfo.outputPrice
    const cacheWriteCost = cacheWriteTokens && modelInfo.cacheWritesPrice
        ? (cacheWriteTokens / 1_000_000) * modelInfo.cacheWritesPrice
        : 0
    const cacheReadCost = cacheReadTokens && modelInfo.cacheReadsPrice
        ? (cacheReadTokens / 1_000_000) * modelInfo.cacheReadsPrice
        : 0
    
    return inputCost + outputCost + cacheWriteCost + cacheReadCost
}
```

### API Metrics: getApiMetrics.ts (4116 lines)

```typescript
export interface ApiMetrics {
    totalCost: number
    totalTokensIn: number
    totalTokensOut: number
    totalCacheWrites: number
    totalCacheReads: number
    requestCount: number
}

export function getApiMetrics(messages: ClineMessage[]): ApiMetrics {
    return messages.reduce((acc, msg) => {
        if (msg.type === "say" && msg.say === "api_req_started") {
            const cost = calculateCost(msg.cost)
            return {
                totalCost: acc.totalCost + cost,
                totalTokensIn: acc.totalTokensIn + (msg.tokensIn || 0),
                totalTokensOut: acc.totalTokensOut + (msg.tokensOut || 0),
                totalCacheWrites: acc.totalCacheWrites + (msg.cacheWrites || 0),
                totalCacheReads: acc.totalCacheReads + (msg.cacheReads || 0),
                requestCount: acc.requestCount + 1,
            }
        }
        return acc
    }, {
        totalCost: 0,
        totalTokensIn: 0,
        totalTokensOut: 0,
        totalCacheWrites: 0,
        totalCacheReads: 0,
        requestCount: 0,
    })
}
```

---

## 📝 CONTEXT MENTIONS: context-mentions.ts (4685 lines)

### Mention Types

```typescript
export type ContextMention = 
    | { type: "file"; path: string }
    | { type: "folder"; path: string }
    | { type: "url"; url: string }
    | { type: "problem"; path: string; line: number }
    | { type: "code"; language: string; code: string }

export function parseContextMentions(text: string): ContextMention[] {
    const mentions: ContextMention[] = []
    
    // @file:path/to/file.ts
    const fileRegex = /@file:([^\s]+)/g
    for (const match of text.matchAll(fileRegex)) {
        mentions.push({ type: "file", path: match[1] })
    }
    
    // @folder:path/to/dir
    const folderRegex = /@folder:([^\s]+)/g
    for (const match of text.matchAll(folderRegex)) {
        mentions.push({ type: "folder", path: match[1] })
    }
    
    // @url:https://example.com
    const urlRegex = /@url:(https?:\/\/[^\s]+)/g
    for (const match of text.matchAll(urlRegex)) {
        mentions.push({ type: "url", url: match[1] })
    }
    
    return mentions
}
```

---

## 🧪 EXPERIMENTS: experiments.ts (1328 lines)

### Feature Flags

```typescript
export type ExperimentId =
    | "codebase-search"
    | "prompt-caching"
    | "ghost-autocomplete"
    | "browser-tool"
    | "mcp-tools"
    | "checkpoint-system"
    | "commit-message-generator"

export const EXPERIMENT_IDS: Record<ExperimentId, boolean> = {
    "codebase-search": true,
    "prompt-caching": true,
    "ghost-autocomplete": false, // Disabled by default
    "browser-tool": true,
    "mcp-tools": true,
    "checkpoint-system": true,
    "commit-message-generator": false,
}

export function isExperimentEnabled(
    experimentId: ExperimentId,
    experiments?: Experiments
): boolean {
    return experiments?.[experimentId] ?? EXPERIMENT_IDS[experimentId]
}
```

---

## 🔗 COMMAND COMBINING: combineCommandSequences.ts (4613 lines)

### Command Merging Logic

```typescript
export function combineCommandSequences(
    commands: string[]
): string[] {
    // Combine cd commands
    // cd dir1 && cd dir2 && command → cd dir2 && command
    
    // Combine package installs
    // npm install pkg1 && npm install pkg2 → npm install pkg1 pkg2
    
    // Remove redundant commands
    // pwd && pwd → pwd
    
    const combined: string[] = []
    let currentDir: string | null = null
    let npmPackages: string[] = []
    
    for (const cmd of commands) {
        if (cmd.startsWith("cd ")) {
            currentDir = cmd.slice(3).trim()
        } else if (cmd.startsWith("npm install ")) {
            npmPackages.push(cmd.slice(12).trim())
        } else {
            // Flush accumulated state
            if (currentDir) {
                combined.push(`cd ${currentDir}`)
                currentDir = null
            }
            if (npmPackages.length > 0) {
                combined.push(`npm install ${npmPackages.join(" ")}`)
                npmPackages = []
            }
            combined.push(cmd)
        }
    }
    
    return combined
}
```

---

## 📚 SUPPORT PROMPTS: support-prompt.ts (10075 lines!)

### Massive Prompt Templates

```typescript
export const SYSTEM_PROMPT = `You are Kilo Code, an expert software engineer with deep knowledge of...
[10,000+ lines of detailed system prompts, examples, best practices]
`

export const ERROR_RECOVERY_PROMPT = `When an error occurs, follow these steps...
[Detailed error recovery strategies]
`

export const CODE_REVIEW_PROMPT = `When reviewing code, check for...
[Comprehensive code review guidelines]
`

// ... 50+ different prompt templates
```

---

## 🦀 RUST TRANSLATION STRATEGY

### Message Protocol Translation

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExtensionMessage {
    Action {
        action: String,
        text: Option<String>,
        images: Option<Vec<String>>,
    },
    State {
        state: TaskState,
    },
    McpServers {
        mcp_servers: Vec<McpServer>,
    },
    // ... 50+ variants
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WebviewMessage {
    WebviewDidLaunch,
    NewTask {
        text: String,
        images: Option<Vec<String>>,
        mentions: Option<Vec<ContextMention>>,
        mode: Option<String>,
    },
    AskResponse {
        ask_response: AskResponse,
        text: Option<String>,
    },
    // ... 100+ variants
}
```

### Model Definitions

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub max_tokens: u32,
    pub context_window: u32,
    pub supports_images: bool,
    pub supports_prompt_cache: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub cache_writes_price: Option<f64>,
    pub cache_reads_price: Option<f64>,
    pub description: Option<String>,
}

pub static CEREBRAS_MODELS: Lazy<HashMap<&'static str, ModelInfo>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("gpt-oss-120b", ModelInfo {
        max_tokens: 65536,
        context_window: 65536,
        supports_images: false,
        supports_prompt_cache: false,
        input_price: 0.25,
        output_price: 0.69,
        cache_writes_price: None,
        cache_reads_price: None,
        description: Some("OpenAI's GPT-OSS model".to_string()),
    });
    // ... all models
    m
});
```

### Cost Calculation

```rust
pub fn calculate_api_cost_anthropic(
    input_tokens: u64,
    output_tokens: u64,
    cache_write_tokens: u64,
    cache_read_tokens: u64,
    model_info: &ModelInfo,
) -> f64 {
    let input_cost = (input_tokens as f64 / 1_000_000.0) * model_info.input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * model_info.output_price;
    
    let cache_write_cost = if cache_write_tokens > 0 {
        model_info.cache_writes_price
            .map(|price| (cache_write_tokens as f64 / 1_000_000.0) * price)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    
    let cache_read_cost = if cache_read_tokens > 0 {
        model_info.cache_reads_price
            .map(|price| (cache_read_tokens as f64 / 1_000_000.0) * price)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    
    input_cost + output_cost + cache_write_cost + cache_read_cost
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Enum-Based Message Types

**Why discriminated unions?**
- Type safety: Each message type has specific payload
- Pattern matching: Easy to handle in TypeScript/Rust
- Extensibility: Add new message types without breaking old code

### 2. Centralized Model Definitions

**Why in shared/?**
- Single source of truth for pricing
- Both extension and webview need model info
- Easy to update when providers change pricing

### 3. Mode System Architecture

**Why tool groups?**
- Composability: Mix and match tool groups
- Flexibility: Custom modes can override
- Permission control: Fine-grained tool access

### 4. Static Prompts (10K+ lines)

**Why such large prompt files?**
- Few-shot learning: Many examples improve quality
- Consistency: Standardized responses
- Knowledge: Encode best practices

---

## 📊 TRANSLATION COMPLEXITY

| File | Lines | Complexity | Effort |
|------|-------|------------|--------|
| support-prompt.ts | 10075 | Low | 2h (just string constants) |
| tools.ts | 8872 | Low | 2h (JSON schemas) |
| context-mentions.ts | 4685 | Medium | 4h (regex parsing) |
| combineCommandSequences.ts | 4613 | Medium | 4h (logic) |
| getApiMetrics.ts | 4116 | Low | 2h (calculations) |
| embeddingModels.ts | 5521 | Low | 1h (data) |
| WebviewMessage.ts | 436 | Low | 2h (enums) |
| ExtensionMessage.ts | 502 | Low | 2h (enums) |
| modes.ts | 384 | Medium | 3h (logic) |
| api.ts | 263 | Low | 1h (data) |
| **TOTAL** | **~55,000** | **Medium** | **25-30 hours** |

---

## 🎓 KEY TAKEAWAYS

✅ **Message Protocol**: 150+ message types for bidirectional communication

✅ **40+ AI Providers**: Comprehensive model definitions with pricing

✅ **Mode System**: Flexible tool group composition

✅ **Cost Tracking**: Detailed token usage and cost calculations

✅ **Context Mentions**: @file, @folder, @url parsing

✅ **10K+ Line Prompts**: Extensive prompt engineering

✅ **Rust-Friendly**: All data structures map cleanly to Rust enums/structs

✅ **Type-Safe**: Zod schemas + TypeScript for validation

---

**Status**: ✅ Complete analysis of shared/ module
**Next**: CHUNK-31 (utils/), CHUNK-32 (activate/)
