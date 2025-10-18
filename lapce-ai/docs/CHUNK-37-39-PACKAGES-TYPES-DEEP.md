# CHUNK-37-39: PACKAGES/TYPES - CORE TYPE DEFINITIONS (DEEP ANALYSIS)

## 🎯 OVERVIEW

**Purpose**: Central type system for entire Codex codebase - shared between extension, webview, and all packages.

**Scale**: 
- **63 TypeScript files**
- **~8,000+ lines** of type definitions
- **40+ AI providers** with model definitions
- **Zod schemas** for runtime validation
- **Critical dependency** for all other packages

---

## 📂 FILE STRUCTURE (63 FILES)

```
packages/types/src/
├── index.ts                        - Main exports (27 lines)
├── api.ts                          - API handler types
├── codebase-index.ts               - Semantic search types
├── events.ts                       - Event system types
├── experiment.ts                   - Feature flags
├── followup.ts                     - Followup question types
├── global-settings.ts              - User settings
├── history.ts                      - Task history types
├── ipc.ts                          - IPC message protocol
├── kilocode.ts                     - Kilocode-specific types
├── kiloLanguages.ts                - Language configurations
├── marketplace.ts                  - Marketplace item types
├── mcp.ts                          - Model Context Protocol
├── message.ts                      - Chat message types (266 lines)
├── mode.ts                         - Mode configuration (212 lines)
├── model.ts                        - Model metadata types
├── provider-settings.ts            - Provider configs (633 lines!)
├── single-file-read-models.ts      - Single-file models
├── task.ts                         - Task state types
├── telemetry.ts                    - Analytics types
├── terminal.ts                     - Terminal types
├── todo.ts                         - Todo list types
├── tool.ts                         - Tool definitions (62 lines)
├── type-fu.ts                      - TypeScript utilities
├── usage-tracker.ts                - Usage tracking
├── vscode.ts                       - VS Code types
└── providers/                      - 40+ provider definitions
    ├── index.ts                    - Provider exports
    ├── anthropic.ts                - Claude models
    ├── bedrock.ts                  - AWS Bedrock
    ├── cerebras.ts                 - Cerebras models
    ├── chutes.ts                   - Chutes models
    ├── claude-code.ts              - Claude Code
    ├── deepinfra.ts                - DeepInfra
    ├── deepseek.ts                 - DeepSeek
    ├── doubao.ts                   - ByteDance Doubao
    ├── featherless.ts              - Featherless
    ├── fireworks.ts                - Fireworks AI
    ├── gemini.ts                   - Google Gemini
    ├── gemini-cli.ts               - Gemini CLI
    ├── glama.ts                    - Glama
    ├── groq.ts                     - Groq
    ├── huggingface.ts              - HuggingFace
    ├── io-intelligence.ts          - IO Intelligence
    ├── lite-llm.ts                 - LiteLLM
    ├── lm-studio.ts                - LM Studio
    ├── mistral.ts                  - Mistral AI
    ├── moonshot.ts                 - Moonshot (Chinese)
    ├── ollama.ts                   - Ollama (local)
    ├── openai.ts                   - OpenAI
    ├── openrouter.ts               - OpenRouter
    ├── qwen-code.ts                - Qwen Code
    ├── requesty.ts                 - Requesty
    ├── roo.ts                      - Roo
    ├── sambanova.ts                - SambaNova
    ├── unbound.ts                  - Unbound
    ├── vertex.ts                   - Google Vertex
    ├── virtual-quota-fallback.ts   - Quota fallback
    ├── vscode-lm.ts                - VS Code LM
    ├── xai.ts                      - X.AI (Grok)
    └── zai.ts                      - ZAI
```

---

## 🔑 CRITICAL TYPE DEFINITIONS

### 1. PROVIDER-SETTINGS.TS (633 LINES!)

**The most complex type file** - defines all provider configurations.

#### Provider Names (40+ providers)

```typescript
export const providerNames = [
    "anthropic",           // Claude
    "claude-code",         // Claude Code-specific
    "glama",              // Glama
    "openrouter",         // OpenRouter aggregator
    "bedrock",            // AWS Bedrock
    "vertex",             // Google Vertex AI
    "openai",             // OpenAI
    "ollama",             // Local Ollama
    "vscode-lm",          // VS Code built-in
    "lmstudio",           // LM Studio local
    "gemini",             // Google Gemini
    "openai-native",      // OpenAI native integration
    "mistral",            // Mistral AI
    "moonshot",           // Moonshot (Chinese)
    "deepseek",           // DeepSeek (Chinese)
    "doubao",             // ByteDance Doubao
    "unbound",            // Unbound
    "requesty",           // Requesty
    "human-relay",        // Human in the loop
    "fake-ai",            // Testing/development
    "xai",                // X.AI (Grok)
    "groq",               // Groq (ultra-fast)
    "chutes",             // Chutes
    "litellm",            // LiteLLM proxy
    "kilocode",           // Kilocode custom
    "deepinfra",          // DeepInfra
    "gemini-cli",         // Gemini CLI
    "virtual-quota-fallback", // Quota management
    "qwen-code",          // Alibaba Qwen
    "huggingface",        // HuggingFace
    "cerebras",           // Cerebras
    "sambanova",          // SambaNova
    "zai",                // ZAI
    "fireworks",          // Fireworks AI
    "featherless",        // Featherless
    "io-intelligence",    // IO Intelligence
    "roo",                // Roo
] as const

export type ProviderName = typeof providerNames[number]
```

#### Provider Settings Schema

```typescript
const baseProviderSettingsSchema = z.object({
    // Provider selection
    apiProvider: providerNamesSchema.optional(),
    
    // API credentials
    apiKey: z.string().optional(),
    anthropicBaseUrl: z.string().optional(),
    openRouterApiKey: z.string().optional(),
    awsRegion: z.string().optional(),
    
    // Model selection
    anthropicModelId: anthropicModelIdSchema.optional(),
    openAiModelId: openAiModelIdSchema.optional(),
    bedrockModelId: bedrockModelIdSchema.optional(),
    
    // Model parameters
    apiModelId: z.string().optional(),
    modelMaxTokens: z.number().optional(),
    modelTemperature: z.number().optional(),
    modelTopP: z.number().optional(),
    modelTopK: z.number().optional(),
    
    // Advanced features
    enablePromptCache: z.boolean().default(true),
    enableVerboseLogging: z.boolean().default(false),
    enableComputerUse: z.boolean().default(false),
    
    // Codebase search
    codebaseIndexProvider: codebaseIndexProviderSchema.optional(),
    codebaseIndexApiKey: z.string().optional(),
    
    // Auto-approval settings
    alwaysAllowReadOnly: z.boolean().default(false),
    alwaysAllowWrite: z.boolean().default(false),
    alwaysAllowExecute: z.boolean().default(false),
    alwaysAllowBrowser: z.boolean().default(false),
    alwaysAllowMcp: z.boolean().default(false),
    
    // Safety limits
    allowedMaxRequests: z.number().optional(),
    allowedMaxCost: z.number().optional(),
    consecutiveMistakeLimit: z.number().default(3),
    
    // ... 50+ more settings
})

export type ProviderSettings = z.infer<typeof baseProviderSettingsSchema>
```

---

### 2. MODE.TS (212 LINES) - MODE SYSTEM

#### Mode Configuration

```typescript
export const modeConfigSchema = z.object({
    slug: z.string().regex(/^[a-zA-Z0-9-]+$/),
    name: z.string().min(1),
    roleDefinition: z.string().min(1),
    whenToUse: z.string().optional(),
    description: z.string().optional(),
    customInstructions: z.string().optional(),
    groups: groupEntryArraySchema,
    source: z.enum(["global", "project"]).optional(),
    iconName: z.string().optional(),
})

export type ModeConfig = z.infer<typeof modeConfigSchema>
```

#### Tool Group System

```typescript
export const groupOptionsSchema = z.object({
    fileRegex: z.string()
        .optional()
        .refine((pattern) => {
            if (!pattern) return true
            try {
                new RegExp(pattern)
                return true
            } catch {
                return false
            }
        }),
    description: z.string().optional(),
})

export type GroupOptions = z.infer<typeof groupOptionsSchema>

// Group can be just name OR [name, options]
export const groupEntrySchema = z.union([
    toolGroupsSchema,
    z.tuple([toolGroupsSchema, groupOptionsSchema])
])

export type GroupEntry = z.infer<typeof groupEntrySchema>
```

#### Example Mode

```typescript
const codeMode: ModeConfig = {
    slug: "code",
    name: "Code",
    roleDefinition: "You are an expert software engineer...",
    whenToUse: "Use for implementing features and fixing bugs",
    description: "Full-featured coding assistant",
    groups: [
        "read",
        "edit", 
        "command",
        ["browser", { fileRegex: "\\.(html|css|jsx?)$" }],
        "mcp"
    ],
    iconName: "code",
}
```

---

### 3. MESSAGE.TS (266 LINES) - CHAT MESSAGES

#### ClineAsk - User Interaction Types

```typescript
export const clineAsks = [
    "followup",                     // Ask clarifying question
    "command",                      // Execute terminal command
    "command_output",               // Read command output
    "completion_result",            // Task completed
    "tool",                         // File operation permission
    "api_req_failed",               // API failed, retry?
    "resume_task",                  // Resume paused task
    "resume_completed_task",        // Resume finished task
    "mistake_limit_reached",        // Too many errors
    "browser_action_launch",        // Browser permission
    "use_mcp_server",               // MCP tool permission
    "auto_approval_max_req_reached", // Hit auto-approve limit
    "payment_required_prompt",      // Low credits (kilocode)
    "report_bug",                   // Bug report (kilocode)
    "condense",                     // Condense context (kilocode)
] as const

export type ClineAsk = typeof clineAsks[number]
```

#### Ask Classifications

```typescript
// Asks that pause the task
export const idleAsks = [
    "completion_result",
    "api_req_failed",
    "resume_completed_task",
    "mistake_limit_reached",
    "auto_approval_max_req_reached",
] as const

// Asks that can resume
export const resumableAsks = ["resume_task"] as const

// Asks requiring immediate user action
export const interactiveAsks = [
    "command",
    "tool",
    "browser_action_launch",
    "use_mcp_server",
] as const
```

#### Message Types

```typescript
export interface ClineMessage {
    ts: number              // Timestamp
    type: "say" | "ask"     // Direction
    say?: ClineSay          // AI → User
    ask?: ClineAsk          // AI asks for permission
    text?: string           // Message content
    images?: string[]       // Base64 images
    partial?: boolean       // Streaming chunk
}

export const clineSays = [
    "text",                 // Regular message
    "user_feedback",        // User response
    "user_feedback_diff",   // User edit suggestion
    "api_req_started",      // API call started
    "api_req_finished",     // API call finished
    "error",                // Error occurred
    "tool",                 // Tool execution
    "completion_result",    // Final result
    "shell_integration_warning", // Shell issue
] as const
```

---

### 4. TOOL.TS (62 LINES) - TOOL DEFINITIONS

```typescript
export const toolGroups = [
    "read",      // File reading tools
    "edit",      // File editing tools
    "browser",   // Browser automation
    "command",   // Terminal commands
    "mcp",       // MCP tools
    "modes"      // Mode switching
] as const

export type ToolGroup = typeof toolGroups[number]

export const toolNames = [
    // File operations
    "read_file",
    "write_to_file",
    "apply_diff",
    "insert_content",
    "search_and_replace",
    "edit_file",                // kilocode
    
    // Search & navigation
    "search_files",
    "list_files",
    "list_code_definition_names",
    "codebase_search",
    
    // Execution
    "execute_command",
    
    // Browser
    "browser_action",
    
    // MCP
    "use_mcp_tool",
    "access_mcp_resource",
    
    // Meta tools
    "ask_followup_question",
    "attempt_completion",
    "switch_mode",
    "new_task",
    "fetch_instructions",
    "update_todo_list",
    
    // Kilocode additions
    "new_rule",
    "report_bug",
    "condense",
] as const

export type ToolName = typeof toolNames[number]
```

---

### 5. PROVIDER MODEL DEFINITIONS (40+ FILES)

Each provider file defines available models with pricing and capabilities.

#### Example: anthropic.ts

```typescript
export const anthropicModels = {
    "claude-sonnet-4-20250514": {
        maxTokens: 8192,
        contextWindow: 200000,
        supportsImages: true,
        supportsPromptCache: true,
        supportsComputerUse: true,
        inputPrice: 3.00,           // per 1M tokens
        outputPrice: 15.00,
        cacheWritesPrice: 3.75,
        cacheReadsPrice: 0.30,
        description: "Most intelligent Claude model",
        betas: ["pdfs-2025-03-04"],
    },
    
    "claude-opus-4-20250514": {
        maxTokens: 16384,
        contextWindow: 200000,
        supportsImages: true,
        supportsPromptCache: true,
        inputPrice: 15.00,
        outputPrice: 75.00,
        cacheWritesPrice: 18.75,
        cacheReadsPrice: 1.50,
        description: "Most capable model for complex tasks",
    },
    
    "claude-3-5-haiku-20241022": {
        maxTokens: 8192,
        contextWindow: 200000,
        supportsImages: false,
        supportsPromptCache: true,
        inputPrice: 1.00,
        outputPrice: 5.00,
        cacheWritesPrice: 1.25,
        cacheReadsPrice: 0.10,
        description: "Fast and affordable",
    },
    
    // ... 10+ Claude models
}

export type AnthropicModelId = keyof typeof anthropicModels
```

#### Example: openai.ts

```typescript
export const openAiModels = {
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
        description: "Affordable and intelligent",
    },
    
    "o1": {
        maxTokens: 100000,
        contextWindow: 200000,
        supportsImages: true,
        reasoning: true,
        inputPrice: 15.00,
        outputPrice: 60.00,
        description: "Advanced reasoning model",
    },
    
    // ... 15+ OpenAI models
}
```

#### Model Info Schema

```typescript
export const modelInfoSchema = z.object({
    maxTokens: z.number(),
    contextWindow: z.number(),
    supportsImages: z.boolean().default(false),
    supportsPromptCache: z.boolean().default(false),
    supportsComputerUse: z.boolean().default(false),
    inputPrice: z.number(),
    outputPrice: z.number(),
    cacheWritesPrice: z.number().optional(),
    cacheReadsPrice: z.number().optional(),
    description: z.string().optional(),
    reasoning: z.boolean().optional(),
    betas: z.array(z.string()).optional(),
})

export type ModelInfo = z.infer<typeof modelInfoSchema>
```

---

### 6. IPC.TS - IPC MESSAGE PROTOCOL

```typescript
export interface IpcRequest {
    id: string
    method: string
    params?: any
}

export interface IpcResponse {
    id: string
    result?: any
    error?: {
        code: number
        message: string
        data?: any
    }
}

export interface IpcNotification {
    method: string
    params?: any
}

// Specific IPC methods
export type IpcMethod =
    | "task/start"
    | "task/stop"
    | "task/cancel"
    | "task/respond"
    | "state/get"
    | "state/update"
    | "settings/get"
    | "settings/update"
    | "mcp/list"
    | "mcp/call"
```

---

### 7. TASK.TS - TASK STATE

```typescript
export interface TaskState {
    taskId: string
    status: TaskStatus
    mode: string
    apiConfiguration: ProviderSettings
    messages: ClineMessage[]
    tokensIn: number
    tokensOut: number
    cacheWrites: number
    cacheReads: number
    totalCost: number
    createdAt: number
    updatedAt: number
}

export type TaskStatus = 
    | "idle"
    | "running"
    | "waiting"
    | "completed"
    | "error"
    | "cancelled"
```

---

### 8. GLOBAL-SETTINGS.TS - USER PREFERENCES

```typescript
export interface GlobalSettings {
    // UI preferences
    theme: "light" | "dark" | "auto"
    fontSize: number
    fontFamily: string
    
    // Behavior
    autoSave: boolean
    autoFormat: boolean
    showLineNumbers: boolean
    
    // Telemetry
    telemetryEnabled: boolean
    analyticsUserId?: string
    
    // Custom paths
    customStoragePath?: string
    rulesDirectory?: string
    
    // Feature flags
    experiments: Record<string, boolean>
}
```

---

## 🦀 RUST TRANSLATION STRATEGY

### Phase 1: Core Type System

```rust
// src/types/mod.rs
pub mod provider;
pub mod mode;
pub mod message;
pub mod tool;
pub mod task;
pub mod ipc;

// Use serde for serialization
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderName {
    Anthropic,
    ClaudeCode,
    Glama,
    OpenRouter,
    Bedrock,
    Vertex,
    OpenAi,
    Ollama,
    VsCodeLm,
    // ... 40+ variants
}
```

### Phase 2: Provider Definitions

```rust
// src/types/provider/anthropic.rs
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

pub static ANTHROPIC_MODELS: Lazy<HashMap<&'static str, ModelInfo>> = 
    Lazy::new(|| {
        let mut m = HashMap::new();
        
        m.insert("claude-sonnet-4-20250514", ModelInfo {
            max_tokens: 8192,
            context_window: 200000,
            supports_images: true,
            supports_prompt_cache: true,
            input_price: 3.00,
            output_price: 15.00,
            cache_writes_price: Some(3.75),
            cache_reads_price: Some(0.30),
            description: Some("Most intelligent".to_string()),
        });
        
        // ... all models
        
        m
    });
```

### Phase 3: Message Protocol

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "ask", rename_all = "snake_case")]
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
        matches!(self,
            ClineAsk::CompletionResult
            | ClineAsk::ApiReqFailed
            | ClineAsk::ResumeCompletedTask
            | ClineAsk::MistakeLimitReached
            | ClineAsk::AutoApprovalMaxReqReached
        )
    }
    
    pub fn is_interactive(&self) -> bool {
        matches!(self,
            ClineAsk::Command
            | ClineAsk::Tool
            | ClineAsk::BrowserActionLaunch
            | ClineAsk::UseMcpServer
        )
    }
}
```

### Phase 4: Mode System

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub role_definition: String,
    pub when_to_use: Option<String>,
    pub description: Option<String>,
    pub custom_instructions: Option<String>,
    pub groups: Vec<GroupEntry>,
    pub source: Option<ModeSource>,
    pub icon_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroupEntry {
    Simple(ToolGroup),
    WithOptions(ToolGroup, GroupOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupOptions {
    pub file_regex: Option<String>,
    pub description: Option<String>,
}
```

### Phase 5: Validation with `validator` Crate

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
pub struct ModeConfig {
    #[validate(regex = "SLUG_REGEX")]
    pub slug: String,
    
    #[validate(length(min = 1))]
    pub name: String,
    
    #[validate(length(min = 1))]
    pub role_definition: String,
    
    #[validate(custom = "validate_unique_groups")]
    pub groups: Vec<GroupEntry>,
}

fn validate_unique_groups(groups: &[GroupEntry]) -> Result<(), ValidationError> {
    let mut seen = std::collections::HashSet::new();
    for group in groups {
        let name = group.name();
        if !seen.insert(name) {
            return Err(ValidationError::new("duplicate_group"));
        }
    }
    Ok(())
}
```

---

## 📊 TRANSLATION METRICS

| Category | Files | Lines | Complexity | Effort |
|----------|-------|-------|------------|--------|
| Provider models | 40 | ~4,000 | Low | 8-10h |
| Core types | 10 | ~2,000 | Medium | 10-12h |
| Message protocol | 3 | ~500 | Medium | 6-8h |
| Mode system | 2 | ~300 | Medium | 5-6h |
| Tool definitions | 1 | ~100 | Low | 2-3h |
| Validation | All | - | High | 8-10h |
| **TOTAL** | **63** | **~8,000** | **Medium-High** | **40-50 hours** |

---

## 🎯 KEY DESIGN DECISIONS

### 1. Zod → Rust Validation

**TypeScript**: Runtime validation with Zod schemas
**Rust**: Compile-time validation + `validator` crate for runtime

```rust
// Compile-time type safety
pub enum ProviderName {
    Anthropic,
    OpenAi,
    // ...
}

// Runtime validation
#[derive(Validate)]
pub struct ProviderSettings {
    #[validate(length(min = 1))]
    pub api_key: Option<String>,
    
    #[validate(range(min = 0.0, max = 2.0))]
    pub temperature: Option<f64>,
}
```

### 2. Model Registry Pattern

**Challenge**: 40+ providers × 5-15 models each = 200+ models

**Solution**: Static lazy initialization

```rust
pub struct ModelRegistry {
    models: HashMap<(ProviderName, &'static str), &'static ModelInfo>,
}

impl ModelRegistry {
    pub fn get_model(&self, provider: ProviderName, id: &str) -> Option<&ModelInfo> {
        self.models.get(&(provider, id))
    }
}

pub static REGISTRY: Lazy<ModelRegistry> = Lazy::new(|| {
    ModelRegistry::new()
});
```

### 3. Const Generics for Type Safety

```rust
// Ensure tool groups are valid at compile time
pub struct Mode<const N: usize> {
    slug: String,
    groups: [ToolGroup; N],
}

// Usage
let code_mode = Mode {
    slug: "code".to_string(),
    groups: [
        ToolGroup::Read,
        ToolGroup::Edit,
        ToolGroup::Command,
    ],
};
```

---

## 🎓 CRITICAL INSIGHTS

### Why This Package Matters

**1. Single Source of Truth**
- All types defined once, used everywhere
- Changes propagate automatically
- Type safety across boundaries

**2. Runtime Validation**
- Zod schemas validate at runtime
- Prevents invalid configurations
- User-friendly error messages

**3. Provider Extensibility**
- Easy to add new providers
- Model definitions centralized
- Pricing updates in one place

### Translation Challenges

**1. Zod → Rust**
- Zod: Runtime schema validation
- Rust: Compile-time + validator crate
- Need both for full parity

**2. Type Unions**
- TypeScript: `type X = A | B | C`
- Rust: `enum X { A, B, C }`
- Similar but different ergonomics

**3. Optional Fields**
- TypeScript: `field?: type`
- Rust: `field: Option<Type>`
- Same concept, different syntax

---

## ✅ TRANSLATION CHECKLIST

### High Priority
- [x] Document all 63 files
- [ ] Translate core enums (ProviderName, ToolName, ClineAsk)
- [ ] Translate model definitions (40+ providers)
- [ ] Implement validation (validator crate)
- [ ] Create ModelRegistry pattern
- [ ] IPC protocol types

### Medium Priority
- [ ] Mode system validation
- [ ] Provider settings struct
- [ ] Global settings
- [ ] Task state management

### Low Priority
- [ ] Experiment types
- [ ] Telemetry types
- [ ] Marketplace types

---

**Status**: ✅ Deep analysis complete for packages/types (63 files, ~8,000 lines)
**Priority**: **CRITICAL** - Foundation for entire type system
**Effort**: 40-50 hours for full translation
**Next**: CHUNK-40 (packages/telemetry)
