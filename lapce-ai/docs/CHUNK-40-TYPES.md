# CHUNK-40: PACKAGES/TYPES - SHARED TYPE DEFINITIONS

## 📁 MODULE STRUCTURE

```
Codex/packages/types/
├── src/
│   ├── __tests__/                    - Unit tests
│   ├── providers/                    - Provider configurations (50+ providers)
│   ├── index.ts                      (27 lines)  - Main exports
│   ├── api.ts                        (145 lines) - API interface definitions
│   ├── ipc.ts                        (128 lines) - IPC protocol types
│   ├── message.ts                    (266 lines) - Message schemas
│   ├── model.ts                      (85 lines)  - Model metadata
│   ├── global-settings.ts            (329 lines) - Configuration schema
│   ├── provider-settings.ts          - Provider configs
│   ├── events.ts                     - Event types
│   ├── history.ts                    - Task history
│   ├── task.ts                       - Task types
│   ├── tool.ts                       - Tool types
│   ├── terminal.ts                   - Terminal types
│   ├── mcp.ts                        - MCP protocol
│   ├── kilocode.ts                   - Kilocode types
│   ├── telemetry.ts                  - Telemetry types
│   └── ...                           (50+ more files)
├── package.json                      (37 lines)
└── tsconfig.json
```

**Total**: ~3,000+ lines of TypeScript type definitions

---

## 🎯 PURPOSE

Central package for **shared type definitions** across the entire Codex codebase:
1. **Type Safety**: Runtime validation with Zod schemas
2. **Code Sharing**: Single source of truth for types
3. **API Contracts**: Enforce interfaces between packages
4. **Provider Configs**: Model metadata for 50+ AI providers
5. **Message Protocol**: Standardized communication types
6. **Documentation**: Self-documenting types with JSDoc

**Key Library**: `zod` for runtime validation and type inference

---

## 🏗️ CORE TYPE CATEGORIES

### 1. API Interface (`api.ts` - 145 lines)

**Purpose**: Define the public API contract for external integrations

```typescript
export interface RooCodeAPI extends EventEmitter<RooCodeAPIEvents> {
    // Task Management
    startNewTask(params: {
        configuration?: RooCodeSettings
        text?: string
        images?: string[]
        newTab?: boolean
    }): Promise<string>
    
    resumeTask(taskId: string): Promise<void>
    isTaskInHistory(taskId: string): Promise<boolean>
    getCurrentTaskStack(): string[]
    
    // Task Control
    clearCurrentTask(lastMessage?: string): Promise<void>
    cancelCurrentTask(): Promise<void>
    sendMessage(message?: string, images?: string[]): Promise<void>
    
    // UI Actions
    pressPrimaryButton(): Promise<void>
    pressSecondaryButton(): Promise<void>
    
    // Configuration
    isReady(): boolean
    getConfiguration(): RooCodeSettings
    setConfiguration(values: RooCodeSettings): Promise<void>
    
    // Profile Management
    getProfiles(): string[]
    getProfileEntry(name: string): ProviderSettingsEntry | undefined
    createProfile(name: string, profile?: ProviderSettings): Promise<string>
    deleteProfile(id: string): Promise<void>
    renameProfile(id: string, newName: string): Promise<void>
}
```

**IPC Server Interface**:
```typescript
export interface RooCodeIpcServer extends EventEmitter<IpcServerEvents> {
    listen(): void
    broadcast(message: IpcMessage): void
    send(client: string | Socket, message: IpcMessage): void
    readonly socketPath: string
    readonly isListening: boolean
}
```

---

### 2. IPC Protocol (`ipc.ts` - 128 lines)

**Message Types**:
```typescript
export enum IpcMessageType {
    Connect = "Connect",
    Disconnect = "Disconnect",
    Ack = "Ack",
    TaskCommand = "TaskCommand",
    TaskEvent = "TaskEvent",
}

export enum IpcOrigin {
    Client = "client",
    Server = "server",
}
```

**Task Commands**:
```typescript
export enum TaskCommandName {
    StartNewTask = "StartNewTask",
    CancelTask = "CancelTask",
    CloseTask = "CloseTask",
    ResumeTask = "ResumeTask",
}

export const taskCommandSchema = z.discriminatedUnion("commandName", [
    z.object({
        commandName: z.literal(TaskCommandName.StartNewTask),
        data: z.object({
            configuration: rooCodeSettingsSchema,
            text: z.string(),
            images: z.array(z.string()).optional(),
            newTab: z.boolean().optional(),
        }),
    }),
    z.object({
        commandName: z.literal(TaskCommandName.CancelTask),
        data: z.string(),  // Task ID
    }),
    z.object({
        commandName: z.literal(TaskCommandName.CloseTask),
        data: z.string(),  // Task ID
    }),
    z.object({
        commandName: z.literal(TaskCommandName.ResumeTask),
        data: z.string(),  // Task ID
    }),
])
```

**IPC Message Schema**:
```typescript
export const ipcMessageSchema = z.discriminatedUnion("type", [
    z.object({
        type: z.literal(IpcMessageType.Ack),
        origin: z.literal(IpcOrigin.Server),
        data: ackSchema,
    }),
    z.object({
        type: z.literal(IpcMessageType.TaskCommand),
        origin: z.literal(IpcOrigin.Client),
        clientId: z.string(),
        data: taskCommandSchema,
    }),
    z.object({
        type: z.literal(IpcMessageType.TaskEvent),
        origin: z.literal(IpcOrigin.Server),
        relayClientId: z.string().optional(),
        data: taskEventSchema,
    }),
])
```

---

### 3. Message Types (`message.ts` - 266 lines)

**Ask Types** (User interaction required):
```typescript
export const clineAsks = [
    "followup",                    // Clarifying question
    "command",                     // Execute terminal command
    "command_output",              // Read command output
    "completion_result",           // Task completed
    "tool",                        // File operation
    "api_req_failed",             // API request failed
    "resume_task",                // Resume paused task
    "resume_completed_task",      // Resume completed task
    "mistake_limit_reached",      // Too many errors
    "browser_action_launch",      // Browser interaction
    "use_mcp_server",             // MCP protocol
    "auto_approval_max_req_reached",  // Approval limit
    "payment_required_prompt",    // Low credits (Kilocode)
    "report_bug",                 // Bug report (Kilocode)
    "condense",                   // Context condensing
] as const
```

**Ask Classification**:
```typescript
// Idle state (awaiting user input)
export const idleAsks = [
    "completion_result",
    "api_req_failed",
    "resume_completed_task",
    "mistake_limit_reached",
    "auto_approval_max_req_reached",
] as const

// Resumable state (can be resumed later)
export const resumableAsks = ["resume_task"] as const

// Interactive state (approval required)
export const interactiveAsks = [
    "command",
    "tool",
    "browser_action_launch",
    "use_mcp_server",
] as const
```

**Say Types** (System messages):
```typescript
export const clineSays = [
    "error",
    "api_req_started",
    "api_req_finished",
    "api_req_retried",
    "api_req_retry_delayed",
    "api_req_deleted",
    "text",
    "reasoning",
    "completion_result",
    "user_feedback",
    "user_feedback_diff",
    "command_output",
    "shell_integration_warning",
    "browser_action",
    "browser_action_result",
    "mcp_server_request_started",
    "mcp_server_response",
    "subtask_result",
    "checkpoint_saved",
    "rooignore_error",
    "diff_error",
    "condense_context",
    "condense_context_error",
    "codebase_search_result",
    "user_edit_todos",
] as const
```

**Message Schema**:
```typescript
export const clineMessageSchema = z.object({
    ts: z.number(),                                    // Timestamp
    type: z.union([z.literal("ask"), z.literal("say")]),
    ask: clineAskSchema.optional(),
    say: clineSaySchema.optional(),
    text: z.string().optional(),
    images: z.array(z.string()).optional(),
    partial: z.boolean().optional(),
    reasoning: z.string().optional(),
    conversationHistoryIndex: z.number().optional(),
    checkpoint: z.record(z.string(), z.unknown()).optional(),
    progressStatus: toolProgressStatusSchema.optional(),
    contextCondense: contextCondenseSchema.optional(),
    isProtected: z.boolean().optional(),
    apiProtocol: z.union([z.literal("openai"), z.literal("anthropic")]).optional(),
    metadata: z.object({
        gpt5: z.object({
            previous_response_id: z.string().optional(),
            instructions: z.string().optional(),
            reasoning_summary: z.string().optional(),
        }).optional(),
        kiloCode: kiloCodeMetaDataSchema.optional(),
    }).optional(),
})
```

---

### 4. Model Metadata (`model.ts` - 85 lines)

**Model Information**:
```typescript
export const modelInfoSchema = z.object({
    maxTokens: z.number().nullish(),              // Max output tokens
    maxThinkingTokens: z.number().nullish(),      // Max reasoning tokens
    contextWindow: z.number(),                     // Context window size
    
    // Capabilities
    supportsImages: z.boolean().optional(),
    supportsComputerUse: z.boolean().optional(),
    supportsPromptCache: z.boolean(),
    supportsVerbosity: z.boolean().optional(),
    supportsReasoningBudget: z.boolean().optional(),
    requiredReasoningBudget: z.boolean().optional(),
    supportsReasoningEffort: z.boolean().optional(),
    
    // Pricing (per million tokens)
    inputPrice: z.number().optional(),
    outputPrice: z.number().optional(),
    cacheWritesPrice: z.number().optional(),
    cacheReadsPrice: z.number().optional(),
    
    // Caching
    minTokensPerCachePoint: z.number().optional(),
    maxCachePoints: z.number().optional(),
    cachableFields: z.array(z.string()).optional(),
    
    // Metadata
    description: z.string().optional(),
    supportedParameters: z.array(modelParametersSchema).optional(),
    preferredIndex: z.number().nullish(),
    
    // Tiered pricing
    tiers: z.array(z.object({
        contextWindow: z.number(),
        inputPrice: z.number().optional(),
        outputPrice: z.number().optional(),
        cacheWritesPrice: z.number().optional(),
        cacheReadsPrice: z.number().optional(),
    })).optional(),
})
```

**Reasoning Effort**:
```typescript
export const reasoningEfforts = ["low", "medium", "high"] as const
export type ReasoningEffort = "low" | "medium" | "high"

// With minimal option
export type ReasoningEffortWithMinimal = ReasoningEffort | "minimal"
```

---

### 5. Global Settings (`global-settings.ts` - 329 lines)

**Configuration Schema** (partial):
```typescript
export const globalSettingsSchema = z.object({
    // API Configuration
    currentApiConfigName: z.string().optional(),
    listApiConfigMeta: z.array(providerSettingsEntrySchema).optional(),
    pinnedApiConfigs: z.record(z.string(), z.boolean()).optional(),
    
    // Task History
    taskHistory: z.array(historyItemSchema).optional(),
    
    // Context Condensing
    condensingApiConfigId: z.string().optional(),
    customCondensingPrompt: z.string().optional(),
    autoCondenseContext: z.boolean().optional(),
    autoCondenseContextPercent: z.number().optional(),
    
    // Auto-Approval Settings
    autoApprovalEnabled: z.boolean().optional(),
    alwaysAllowReadOnly: z.boolean().optional(),
    alwaysAllowWrite: z.boolean().optional(),
    alwaysAllowBrowser: z.boolean().optional(),
    alwaysAllowMcp: z.boolean().optional(),
    alwaysAllowExecute: z.boolean().optional(),
    
    // Command Control
    allowedCommands: z.array(z.string()).optional(),
    deniedCommands: z.array(z.string()).optional(),
    commandExecutionTimeout: z.number().optional(),
    
    // Limits
    allowedMaxRequests: z.number().nullish(),
    allowedMaxCost: z.number().nullish(),
    maxConcurrentFileReads: z.number().optional(),
    
    // Diagnostics
    includeDiagnosticMessages: z.boolean().optional(),
    maxDiagnosticMessages: z.number().optional(),
    
    // Browser
    browserToolEnabled: z.boolean().optional(),
    browserViewportSize: z.string().optional(),
    remoteBrowserEnabled: z.boolean().optional(),
    
    // Checkpoints
    checkpointsEnabled: z.boolean().optional(),
    checkpointsAutoSave: z.boolean().optional(),
    
    // MCP
    mcpServers: z.record(z.string(), mcpServerSchema).optional(),
    
    // Experimental
    experiments: experimentsSchema.optional(),
    
    // ... 100+ more settings
})
```

---

## 🔌 PROVIDER SYSTEM

### Provider Architecture

**50+ Providers Supported**:
- **Cloud**: Anthropic, OpenAI, Google Gemini, AWS Bedrock
- **Specialized**: Claude Code CLI, Gemini CLI, Qwen Code
- **Local**: Ollama, LM Studio, LiteLLM
- **Proxies**: OpenRouter, Featherless, Glama
- **Regional**: Doubao (ByteDance), Moonshot (China)
- **Open Source**: DeepSeek, Mistral, Groq, Fireworks
- **Custom**: Requesty, Unbound, Chutes

### Provider Definition Structure

**Example: Anthropic Provider** (`providers/anthropic.ts`):
```typescript
export const anthropicModels = {
    "claude-3-5-sonnet-20241022": {
        maxTokens: 8192,
        contextWindow: 200000,
        supportsImages: true,
        supportsPromptCache: true,
        inputPrice: 3.0,
        outputPrice: 15.0,
        cacheWritesPrice: 3.75,
        cacheReadsPrice: 0.3,
        description: "Claude 3.5 Sonnet (Oct 2024)",
    },
    "claude-3-5-haiku-20241022": {
        maxTokens: 8192,
        contextWindow: 200000,
        supportsImages: false,
        supportsPromptCache: true,
        inputPrice: 0.8,
        outputPrice: 4.0,
        cacheWritesPrice: 1.0,
        cacheReadsPrice: 0.08,
        description: "Claude 3.5 Haiku (Oct 2024)",
    },
    "claude-opus-4-20250514": {
        maxTokens: 16384,
        contextWindow: 200000,
        supportsImages: true,
        supportsPromptCache: true,
        inputPrice: 15.0,
        outputPrice: 75.0,
        cacheWritesPrice: 18.75,
        cacheReadsPrice: 1.5,
        description: "Claude Opus 4 (May 2025)",
    },
}

export const anthropicProviderSettings = {
    apiKey: "",
    baseUrl: "https://api.anthropic.com",
    models: anthropicModels,
}
```

**Example: OpenAI Provider** (`providers/openai.ts`):
```typescript
export const openaiModels = {
    "gpt-4o": {
        maxTokens: 16384,
        contextWindow: 128000,
        supportsImages: true,
        supportsPromptCache: false,
        inputPrice: 2.5,
        outputPrice: 10.0,
        description: "GPT-4o",
    },
    "o1-preview": {
        maxTokens: 32768,
        contextWindow: 128000,
        supportsImages: false,
        supportsPromptCache: false,
        supportsReasoningBudget: true,
        inputPrice: 15.0,
        outputPrice: 60.0,
        description: "O1 Preview (Reasoning)",
    },
    "o1-mini": {
        maxTokens: 65536,
        contextWindow: 128000,
        supportsImages: false,
        supportsPromptCache: false,
        supportsReasoningBudget: true,
        inputPrice: 3.0,
        outputPrice: 12.0,
        description: "O1 Mini (Reasoning)",
    },
}
```

---

## 📊 TOKEN USAGE TRACKING

```typescript
export const tokenUsageSchema = z.object({
    totalTokensIn: z.number(),
    totalTokensOut: z.number(),
    totalCacheWrites: z.number().optional(),
    totalCacheReads: z.number().optional(),
    totalCost: z.number(),
    contextTokens: z.number(),
})

export type TokenUsage = z.infer<typeof tokenUsageSchema>
```

**Cost Calculation**:
```typescript
// Pricing is per million tokens
const inputCost = (inputTokens / 1_000_000) * model.inputPrice
const outputCost = (outputTokens / 1_000_000) * model.outputPrice
const cacheWriteCost = (cacheWrites / 1_000_000) * model.cacheWritesPrice
const cacheReadCost = (cacheReads / 1_000_000) * model.cacheReadsPrice

const totalCost = inputCost + outputCost + cacheWriteCost + cacheReadCost
```

---

## 🦀 RUST TRANSLATION

### Strategy

**Option 1: Serde-based (Recommended)**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub max_tokens: Option<u32>,
    pub max_thinking_tokens: Option<u32>,
    pub context_window: u32,
    pub supports_images: Option<bool>,
    pub supports_computer_use: Option<bool>,
    pub supports_prompt_cache: bool,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_writes_price: Option<f64>,
    pub cache_reads_price: Option<f64>,
    pub description: Option<String>,
    pub preferred_index: Option<i32>,
    pub tiers: Option<Vec<PricingTier>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingTier {
    pub context_window: u32,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_writes_price: Option<f64>,
    pub cache_reads_price: Option<f64>,
}
```

**Option 2: Validation with garde or validator**
```rust
use garde::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
pub struct GlobalSettings {
    #[garde(length(min = 1))]
    pub current_api_config_name: Option<String>,
    
    #[garde(range(min = 0, max = 100))]
    pub auto_condense_context_percent: Option<u8>,
    
    #[garde(range(min = 0))]
    pub command_execution_timeout: Option<u32>,
}
```

### Message Enums

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    BrowserActionLaunch,
    UseMcpServer,
    AutoApprovalMaxReqReached,
    PaymentRequiredPrompt,
    ReportBug,
    Condense,
}

impl ClineAsk {
    pub fn is_idle(&self) -> bool {
        matches!(
            self,
            ClineAsk::CompletionResult
                | ClineAsk::ApiReqFailed
                | ClineAsk::ResumeCompletedTask
                | ClineAsk::MistakeLimitReached
                | ClineAsk::AutoApprovalMaxReqReached
        )
    }
    
    pub fn is_resumable(&self) -> bool {
        matches!(self, ClineAsk::ResumeTask)
    }
    
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            ClineAsk::Command
                | ClineAsk::Tool
                | ClineAsk::BrowserActionLaunch
                | ClineAsk::UseMcpServer
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
    pub ts: u64,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub ask: Option<ClineAsk>,
    pub say: Option<ClineSay>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub partial: Option<bool>,
    pub reasoning: Option<String>,
    pub conversation_history_index: Option<usize>,
    pub is_protected: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Ask,
    Say,
}
```

### IPC Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum IpcMessage {
    Ack {
        origin: IpcOrigin,
        data: AckData,
    },
    TaskCommand {
        origin: IpcOrigin,
        client_id: String,
        data: TaskCommand,
    },
    TaskEvent {
        origin: IpcOrigin,
        relay_client_id: Option<String>,
        data: TaskEvent,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpcOrigin {
    Client,
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "commandName", content = "data")]
pub enum TaskCommand {
    StartNewTask {
        configuration: Option<RooCodeSettings>,
        text: String,
        images: Option<Vec<String>>,
        new_tab: Option<bool>,
    },
    CancelTask(String),
    CloseTask(String),
    ResumeTask(String),
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Zod for Runtime Validation

**Why Zod?**
- Type inference from schemas
- Runtime validation
- Detailed error messages
- No code generation needed

**Example**:
```typescript
const schema = z.object({ name: z.string() })
type Type = z.infer<typeof schema>  // { name: string }

const result = schema.safeParse(data)
if (result.success) {
    const validated = result.data  // Type-safe
}
```

### 2. Discriminated Unions

**For type-safe message handling**:
```typescript
export const taskCommandSchema = z.discriminatedUnion("commandName", [
    z.object({ commandName: z.literal("StartNewTask"), ... }),
    z.object({ commandName: z.literal("CancelTask"), ... }),
])
```

**Benefits**:
- TypeScript narrows types correctly
- Exhaustive pattern matching
- Clear intent

### 3. Centralized Provider Configs

**Single source of truth for model metadata**:
- Pricing updates in one place
- Consistent model naming
- Easy to add new providers
- Type-safe model selection

### 4. Optional vs Nullable

**Zod convention**:
- `.optional()`: Field may be missing
- `.nullable()`: Field may be null
- `.nullish()`: Field may be null or undefined

### 5. Separate Package

**Why standalone package?**
- Shared across extension and IPC
- Can be published to npm
- No circular dependencies
- Fast compilation (types only)

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `zod` (^3.25.61) - Schema validation

**Dev Dependencies**:
- `@clean-code/config-eslint` (workspace)
- `@clean-code/config-typescript` (workspace)
- `vitest` (^3.2.3) - Testing

**Rust Crates**:
- `serde` (1.0) - Serialization framework
- `serde_json` (1.0) - JSON support
- `garde` (0.20) or `validator` (0.18) - Optional validation

---

## 📊 PACKAGE STATISTICS

### By Category

| Category | Files | Lines (est) | Purpose |
|----------|-------|-------------|---------|
| Providers | 50+ | ~1500 | Model configurations |
| Core Types | 15 | ~1000 | Messages, models, settings |
| Tests | 3 | ~200 | Unit tests |
| Infrastructure | 5 | ~300 | Build, config |

### Provider Count by Type

| Type | Count | Examples |
|------|-------|----------|
| Cloud | 8 | Anthropic, OpenAI, Gemini, Bedrock |
| Local | 3 | Ollama, LM Studio, LiteLLM |
| Proxy | 5 | OpenRouter, Featherless, Requesty |
| Specialized | 4 | Claude Code, Gemini CLI, Qwen Code |
| Regional | 2 | Doubao, Moonshot |
| Open Source | 10+ | DeepSeek, Mistral, Groq, etc. |

---

## 🎓 KEY TAKEAWAYS

✅ **Type Safety**: Zod schemas provide runtime validation

✅ **Centralized**: Single package for all shared types

✅ **50+ Providers**: Comprehensive AI model support

✅ **IPC Protocol**: Type-safe inter-process communication

✅ **Message System**: Standardized ask/say types

✅ **Configuration**: Extensive settings schema

✅ **Cost Tracking**: Built-in token usage and pricing

✅ **Publishable**: Can be distributed as npm package

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Medium-High
**Estimated Effort**: 15-20 hours
**Lines of Rust**: ~2,500-3,000 lines
**Dependencies**: `serde`, `serde_json`, optional validation
**Key Challenge**: Mapping Zod schemas to Rust types
**Risk**: Medium - large surface area, many types

**Recommendation**: 
- Start with core types (messages, IPC, model)
- Use serde for serialization
- Add validation later if needed
- Generate provider configs from JSON

---

**Status**: ✅ Deep analysis complete
**Progress**: 7 of 11 high-priority CHUNKs completed
**Next**: Continue with remaining medium-priority packages (evals, telemetry) or fix CHUNK-44 (statistics)
