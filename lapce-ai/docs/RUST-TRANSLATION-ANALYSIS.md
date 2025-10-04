# Deep Analysis: Codex → Rust Translation Requirements

## Executive Summary

This document provides an ultra-deep analysis of four critical Codex directories (`src/api`, `src/assets`, `src/activate`, `src/i18n`) with exact Rust translation requirements. Every TypeScript pattern, API contract, and architectural decision is documented for 1:1 behavioral parity.

---

## 1. src/api Directory (60 files)

### Core Architecture

#### 1.1 Provider System (`src/api/index.ts`)

**TypeScript Pattern:**
```typescript
export interface ApiHandler {
    createMessage(systemPrompt: string, messages: Anthropic.Messages.MessageParam[], 
                  metadata?: ApiHandlerCreateMessageMetadata): ApiStream
    getModel(): { id: string; info: ModelInfo }
    countTokens(content: Array<Anthropic.Messages.ContentBlockParam>): Promise<number>
}

export function buildApiHandler(configuration: ProviderSettings): ApiHandler {
    switch (apiProvider) {
        case "anthropic": return new AnthropicHandler(options)
        case "openai": return new OpenAiHandler(options)
        // ... 40+ providers
    }
}
```

**Rust Translation Requirements:**
```rust
// Trait-based polymorphism instead of interface
pub trait ApiHandler: Send + Sync {
    fn create_message<'a>(
        &'a self,
        system_prompt: &str,
        messages: &[AnthropicMessage],
        metadata: Option<&ApiHandlerCreateMessageMetadata>
    ) -> Pin<Box<dyn Stream<Item = Result<ApiStreamChunk, Error>> + Send + 'a>>;
    
    fn get_model(&self) -> ModelInfo;
    
    async fn count_tokens(&self, content: &[ContentBlock]) -> Result<usize, Error>;
}

// Factory pattern with enum dispatch
pub enum ProviderType {
    Anthropic(AnthropicHandler),
    OpenAI(OpenAIHandler),
    Bedrock(BedrockHandler),
    // ... all 40+ providers
}

impl ApiHandler for ProviderType {
    // Delegate to inner handler
}

pub fn build_api_handler(config: &ProviderSettings) -> Result<Box<dyn ApiHandler>, Error> {
    match config.api_provider.as_str() {
        "anthropic" => Ok(Box::new(AnthropicHandler::new(config)?)),
        // ...
    }
}
```

**Critical Details:**
- Must preserve exact provider string IDs ("anthropic", "openai", etc.)
- Stream must be lazy, async, and cancellable
- Metadata must preserve all fields including taskId, previousResponseId, suppressPreviousResponseId
- Error types must map to exact TypeScript error messages

#### 1.2 Base Provider (`src/api/providers/base-provider.ts`)

**TypeScript Pattern:**
```typescript
export abstract class BaseProvider implements ApiHandler {
    abstract createMessage(...): ApiStream
    abstract getModel(): { id: string; info: ModelInfo }
    
    async countTokens(content: ContentBlockParam[]): Promise<number> {
        return countTokens(content, { useWorker: true })
    }
}
```

**Rust Translation:**
```rust
pub struct BaseProvider {
    // Common fields if any
}

impl BaseProvider {
    // Default implementation using tiktoken-rs
    pub async fn count_tokens_default(content: &[ContentBlock]) -> Result<usize, Error> {
        // Use tiktoken-rs with cl100k_base encoding
        let bpe = tiktoken_rs::cl100k_base()?;
        let text = content_to_text(content);
        Ok(bpe.encode_with_special_tokens(&text).len())
    }
}

// Each provider inherits default or overrides
impl AnthropicHandler {
    pub async fn count_tokens(&self, content: &[ContentBlock]) -> Result<usize, Error> {
        // Try native API first
        match self.client.count_tokens(content).await {
            Ok(count) => Ok(count),
            Err(_) => BaseProvider::count_tokens_default(content).await
        }
    }
}
```

#### 1.3 Stream Types (`src/api/transform/stream.ts`)

**TypeScript:**
```typescript
export type ApiStreamChunk = 
    | { type: "text"; text: string }
    | { type: "reasoning"; text: string }
    | { type: "usage"; inputTokens: number; outputTokens: number; 
        cacheWriteTokens?: number; cacheReadTokens?: number; 
        reasoningTokens?: number; totalCost?: number }
    | { type: "error"; error: string; message: string }
```

**Rust Translation:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ApiStreamChunk {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "reasoning")]
    Reasoning { text: String },
    
    #[serde(rename = "usage")]
    Usage {
        #[serde(rename = "inputTokens")]
        input_tokens: u32,
        #[serde(rename = "outputTokens")]
        output_tokens: u32,
        #[serde(rename = "cacheWriteTokens", skip_serializing_if = "Option::is_none")]
        cache_write_tokens: Option<u32>,
        #[serde(rename = "cacheReadTokens", skip_serializing_if = "Option::is_none")]
        cache_read_tokens: Option<u32>,
        #[serde(rename = "reasoningTokens", skip_serializing_if = "Option::is_none")]
        reasoning_tokens: Option<u32>,
        #[serde(rename = "totalCost", skip_serializing_if = "Option::is_none")]
        total_cost: Option<f64>,
    },
    
    #[serde(rename = "error")]
    Error { error: String, message: String }
}
```

### Provider-Specific Requirements

#### 1.4 OpenAI Provider (`src/api/providers/openai.ts`)

**Critical Implementation Details:**
- Azure OpenAI uses `AzureOpenAI` client class
- Azure AI Inference uses special path: `OPENAI_AZURE_AI_INFERENCE_PATH`
- O1/O3/O4 models use "developer" role instead of "system"
- DeepSeek Reasoner uses R1 format conversion
- Prompt caching adds `cache_control` to last 2 user messages
- XmlMatcher processes `<think>` tags for reasoning

**Rust Requirements:**
```rust
pub struct OpenAIHandler {
    client: OpenAIClient,
    options: ApiHandlerOptions,
    xml_matcher: XmlMatcher,
}

impl OpenAIHandler {
    async fn create_message_stream(&self, ...) -> ApiStream {
        // Special case O1/O3/O4
        if model_id.starts_with("o1") || model_id.starts_with("o3") {
            messages[0].role = "developer";
            messages[0].content = format!("Formatting re-enabled\n{}", system_prompt);
        }
        
        // DeepSeek R1 format
        if model_id.contains("deepseek-r1") {
            messages = convert_to_r1_format(messages);
        }
        
        // Stream with XmlMatcher for reasoning
        let stream = client.chat_completions_stream(request).await?;
        for chunk in stream {
            if let Some(content) = chunk.delta.content {
                for matched in xml_matcher.update(&content) {
                    yield matched;
                }
            }
        }
    }
}
```

#### 1.5 Anthropic Provider (`src/api/providers/anthropic.ts`)

**Critical Details:**
- Prompt caching via `anthropic-beta: prompt-caching-2024-07-31` header
- 1M context via `anthropic-beta: context-1m-2025-08-07` header
- Cache control on system prompt and last 2 user messages
- Thinking blocks emit reasoning chunks
- Usage accumulates across message_start, message_delta events

#### 1.6 Bedrock Provider (`src/api/providers/bedrock.ts`)

**Critical Details:**
- ARN parsing for cross-region inference profiles
- Prompt Router updates pricing model via `trace.promptRouter.invokedModelId`
- Thinking enabled via `additionalModelRequestFields.thinking`
- Multi-point cache strategy for optimal cache placement
- Both AWS SDK field naming conventions supported

### Transform Functions

#### 1.7 Message Format Conversions

**OpenAI Format (`src/api/transform/openai-format.ts`):**
- Tool results become separate messages with role "tool"
- Images convert to base64 data URIs or URL references
- Assistant tool calls preserve exact JSON arguments

**Gemini Format (`src/api/transform/gemini-format.ts`):**
- Tool results become functionResponse parts
- Images become inlineData with mimeType
- Content blocks flatten to parts array

**Bedrock Format (`src/api/transform/bedrock-converse-format.ts`):**
- Images convert to Uint8Array bytes
- Tool use becomes XML format
- System messages separate from conversation

**Rust Requirements:**
- Each format needs exact field preservation
- JSON serialization must match TypeScript exactly
- No field reordering or case changes allowed

### Model Parameters (`src/api/transform/model-params.ts`)

**Critical Logic:**
```typescript
// Reasoning budget calculation
if (shouldUseReasoningBudget({ model, settings })) {
    reasoningBudget = customMaxThinkingTokens ?? 
        (isGemini25Pro ? 128 : 8192);
    
    // Cannot exceed 80% of maxTokens
    if (maxTokens && reasoningBudget > Math.floor(maxTokens * 0.8)) {
        reasoningBudget = Math.floor(maxTokens * 0.8);
    }
    
    // Minimum tokens enforced
    const minThinkingTokens = isGemini25Pro ? 128 : 1024;
    if (reasoningBudget < minThinkingTokens) {
        reasoningBudget = minThinkingTokens;
    }
    
    temperature = 1.0; // Force for reasoning models
}
```

**Rust Must Preserve:**
- Exact calculation logic
- Model-specific defaults
- Temperature overrides
- Min/max constraints

---

## 2. src/assets Directory (57 files)

### Structure
```
assets/
├── codicons/         # VS Code icon font
├── docs/             # Documentation assets
├── icons/            # App icons and branding
└── vscode-material-icons/  # File type icons
```

### Rust Translation Strategy

**Embed at Compile Time:**
```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

impl Assets {
    pub fn get_icon(name: &str) -> Option<Vec<u8>> {
        Self::get(&format!("icons/{}", name))
            .map(|f| f.data.to_vec())
    }
    
    pub fn get_material_icon_mapping() -> HashMap<String, String> {
        let json_data = Self::get("vscode-material-icons/icon-map.json").unwrap();
        serde_json::from_slice(&json_data.data).unwrap()
    }
}
```

**Critical Files:**
- `codicon.ttf`: Must serve with correct MIME type
- `icon-map.json`: Maps file extensions to icon names
- All SVGs: Must preserve viewBox and path data exactly

---

## 3. src/activate Directory (10 files)

### VS Code Extension Activation

#### 3.1 Command Registration (`registerCommands.ts`)

**TypeScript Pattern:**
```typescript
context.subscriptions.push(
    vscode.commands.registerCommand(command, callback)
)

// Native panel handling
let sidebarPanel: any | undefined
let tabPanel: any | undefined

export function setPanel(newPanel: any, type: "sidebar" | "tab") {
    if (type === "sidebar") {
        sidebarPanel = newPanel
        tabPanel = undefined
    } else {
        tabPanel = newPanel
        sidebarPanel = undefined
    }
}
```

**Rust Translation for Lapce:**
```rust
pub struct ActivationManager {
    commands: HashMap<String, Box<dyn Fn(&mut Context) + Send + Sync>>,
    active_panel: Option<PanelRef>,
    panel_type: PanelType,
}

impl ActivationManager {
    pub fn register_commands(&mut self, ctx: &mut Context) {
        // Map VS Code commands to Lapce actions
        self.commands.insert("kilo-code.newTask", Box::new(|ctx| {
            Self::handle_new_task(ctx);
        }));
        
        // Register with Lapce command palette
        for (id, handler) in &self.commands {
            ctx.register_command(id, handler.clone());
        }
    }
    
    pub fn set_panel(&mut self, panel: PanelRef, panel_type: PanelType) {
        self.active_panel = Some(panel);
        self.panel_type = panel_type;
    }
}
```

#### 3.2 Code Actions (`CodeActionProvider.ts`)

**TypeScript:**
```typescript
export class CodeActionProvider implements vscode.CodeActionProvider {
    async provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range | vscode.Selection,
        context: vscode.CodeActionContext
    ): Promise<vscode.CodeAction[]> {
        const actions: vscode.CodeAction[] = []
        // Smart selection, error fixing, etc.
        return actions
    }
}
```

**Rust for Lapce:**
```rust
pub struct CodeActionProvider {
    // state
}

impl LapceCodeActionProvider for CodeActionProvider {
    fn provide_code_actions(
        &self,
        doc: &Document,
        range: Range,
        context: &CodeActionContext
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        // Port exact action logic
        if context.diagnostics.len() > 0 {
            actions.push(CodeAction {
                title: "Fix with AI".to_string(),
                kind: Some(CodeActionKind::QuickFix),
                command: Some(Command {
                    id: "kilo-code.fixError",
                    arguments: vec![/* ... */],
                }),
            });
        }
        
        actions
    }
}
```

#### 3.3 Human Relay (`humanRelay.ts`)

**TypeScript:**
```typescript
const humanRelayCallbacks = new Map<string, (response: string | undefined) => void>()

export const registerHumanRelayCallback = (
    requestId: string, 
    callback: (response: string | undefined) => void
) => humanRelayCallbacks.set(requestId, callback)
```

**Rust Translation:**
```rust
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub struct HumanRelayManager {
    callbacks: Arc<Mutex<HashMap<String, oneshot::Sender<Option<String>>>>>,
}

impl HumanRelayManager {
    pub fn register_callback(&self, request_id: String) -> oneshot::Receiver<Option<String>> {
        let (tx, rx) = oneshot::channel();
        self.callbacks.lock().unwrap().insert(request_id, tx);
        rx
    }
    
    pub fn handle_response(&self, request_id: &str, response: Option<String>) {
        if let Some(tx) = self.callbacks.lock().unwrap().remove(request_id) {
            let _ = tx.send(response);
        }
    }
}
```

---

## 4. src/i18n Directory (72 files)

### Structure
```
i18n/
├── index.ts          # Public API
├── setup.ts          # i18next configuration
└── locales/
    ├── en/           # English (base)
    ├── es/           # Spanish
    ├── fr/           # French
    ├── de/           # German
    ├── zh/           # Chinese
    ├── ja/           # Japanese
    └── ...           # 12 total languages
```

### Translation System

**TypeScript (`setup.ts`):**
```typescript
const translations: Record<string, Record<string, any>> = {}

languages.forEach((language: string) => {
    files.forEach((file: string) => {
        const namespace = path.basename(file, ".json")
        translations[language][namespace] = JSON.parse(content)
    })
})

i18next.init({
    lng: "en",
    fallbackLng: "en",
    resources: translations,
    interpolation: { escapeValue: false }
})
```

**Rust Translation:**
```rust
use fluent::{FluentBundle, FluentResource};
use serde_json::Value;
use std::collections::HashMap;

pub struct I18nManager {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
    current_language: String,
    translations: HashMap<String, HashMap<String, Value>>,
}

impl I18nManager {
    pub fn new() -> Result<Self, Error> {
        let mut manager = Self {
            bundles: HashMap::new(),
            current_language: "en".to_string(),
            translations: HashMap::new(),
        };
        
        // Load all JSON files
        for lang in ["en", "es", "fr", "de", "zh", "ja", /* ... */] {
            for namespace in ["common", "kilocode", "tools", "embeddings", "mcp", "marketplace"] {
                let path = format!("i18n/locales/{}/{}.json", lang, namespace);
                let content = std::fs::read_to_string(&path)?;
                let json: Value = serde_json::from_str(&content)?;
                
                manager.translations
                    .entry(lang.to_string())
                    .or_default()
                    .insert(namespace.to_string(), json);
            }
        }
        
        Ok(manager)
    }
    
    pub fn t(&self, key: &str, args: Option<&HashMap<String, String>>) -> String {
        // Parse key like "common:errors.invalid_data_uri"
        let parts: Vec<&str> = key.split(':').collect();
        let (namespace, key_path) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("common", key)
        };
        
        // Navigate JSON path
        if let Some(lang_data) = self.translations.get(&self.current_language) {
            if let Some(namespace_data) = lang_data.get(namespace) {
                if let Some(value) = self.get_nested_value(namespace_data, key_path) {
                    return self.interpolate(value, args);
                }
            }
        }
        
        // Fallback to English
        if self.current_language != "en" {
            // Recursive call with "en"
        }
        
        key.to_string() // Last resort
    }
    
    fn interpolate(&self, template: &str, args: Option<&HashMap<String, String>>) -> String {
        let mut result = template.to_string();
        if let Some(args) = args {
            for (key, value) in args {
                result = result.replace(&format!("{{{{{}}}}}", key), value);
            }
        }
        result
    }
}
```

### Critical JSON Structure

**Each locale has 6 namespaces:**
1. `common.json`: Shared UI strings, errors, confirmations
2. `kilocode.json`: Main extension strings
3. `tools.json`: Tool descriptions and prompts
4. `embeddings.json`: Indexing and search strings
5. `mcp.json`: Model Context Protocol strings
6. `marketplace.json`: Extension marketplace strings

**Key patterns to preserve:**
- Pluralization: `"items": { "zero": "No items", "one": "One item", "other": "{{count}} items" }`
- Interpolation: `"Welcome, {{name}}! You have {{count}} notifications."`
- Nested paths: `"errors.invalid_data_uri"`

---

## Critical Migration Constraints

### 1. Exact String Preservation
- ALL prompt strings must be character-for-character identical
- Tool descriptions cannot change even whitespace
- Error messages must match exactly for debugging

### 2. API Compatibility
- Request/response schemas cannot change field names or structure
- Streaming event names must be identical
- Header names and values preserved exactly

### 3. Behavioral Parity
- Reasoning extraction logic must be identical
- Token counting must produce same results
- Cost calculations must match to 6 decimal places

### 4. Testing Requirements
- Golden test files from TypeScript must pass
- Stream replay tests with captured responses
- Provider-specific edge cases (timeouts, rate limits, auth failures)

### 5. Performance Targets
- Stream latency < 10ms per chunk
- Memory usage < 2x TypeScript version
- Concurrent provider limit: 10 simultaneous streams

---

## 5. src/services Directory (55 files)

### Browser Automation (`browser/` - 5 files)

#### BrowserSession.ts
**TypeScript:**
```typescript
private browser?: Browser
private isUsingRemoteBrowser: boolean = false
async ensureChromiumExists(): Promise<PCRStats>
async launchLocalBrowser(): Promise<void>
async connectWithChromeHostUrl(chromeHostUrl: string): Promise<boolean>
```

**Rust Translation:**
```rust
use headless_chrome::{Browser, LaunchOptions};
pub struct BrowserSession {
    browser: Arc<RwLock<Option<Browser>>>,
    is_using_remote: bool,
}
```

### Code Indexing (`code-index/` - 30+ files)

#### Manager Singleton Pattern
```rust
static INSTANCES: Lazy<DashMap<String, Arc<CodeIndexManager>>> = Lazy::new(DashMap::new);
```

#### Embedders (OpenAI, Gemini, Ollama)
```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn create_embeddings(&self, texts: &[String], model: Option<&str>) -> Result<EmbeddingResponse, Error>;
}
```

#### Parser with Tree-sitter
```rust
use tree_sitter::{Parser, Tree, Query, QueryCursor};
pub async fn parse_file(&self, file_path: &Path) -> Result<Vec<CodeBlock>, Error>
```

### Checkpoints (`checkpoints/` - 6 files)
```rust
use git2::{Repository, Signature, Oid};
pub trait CheckpointService: Send + Sync {
    async fn create_checkpoint(&mut self, message: &str) -> Result<CheckpointResult, Error>;
    async fn restore_checkpoint(&self, hash: &str) -> Result<(), Error>;
}
```

### MDM Service
```rust
static INSTANCE: OnceCell<Arc<RwLock<MdmService>>> = OnceCell::new();
pub fn is_compliant(&self) -> ComplianceResult
```

---

## 6. src/integrations Directory (54 files)

### Terminal Integration (`terminal/` - 20+ files)

#### Process Management
```rust
use tokio::process::{Command, Child};
pub async fn run_command(&mut self, command: &str) -> Result<TerminalProcessResult, Error>
```

#### Shell Integration
```rust
pub struct ShellIntegrationManager {
    terminal_tmp_dirs: Arc<DashMap<u32, PathBuf>>,
}
```

### Editor Integration (`editor/` - 6 files)

#### DiffViewProvider
```rust
use similar::{ChangeTag, TextDiff};
pub async fn open(&mut self, rel_path: &str) -> Result<(), Error>
pub async fn stream_changes(&mut self, content: &str)
```

### Misc (`misc/` - 15 files)
- Text extraction from XLSX, PDF, DOCX
- Image handling with base64 encoding
- Line counting and file reading utilities

---

## 7. src/extension Directory (1 file)

### Extension API (`api.ts`)

**TypeScript:**
```typescript
export class API extends EventEmitter<RooCodeEvents> implements RooCodeAPI {
    private readonly taskMap = new Map<string, ClineProvider>()
    private readonly ipc?: IpcServer
    
    async startNewTask(config: RooCodeSettings): Promise<string>
    async resumeTask(taskId: string): Promise<void>
    async cancelTask(taskId: string): Promise<void>
}
```

**Rust Translation:**
```rust
use tokio::sync::{broadcast, RwLock};
use std::collections::HashMap;

pub struct ExtensionAPI {
    task_map: Arc<RwLock<HashMap<String, Arc<TaskProvider>>>>,
    ipc_server: Option<Arc<IpcServer>>,
    event_tx: broadcast::Sender<RooCodeEvent>,
}

impl ExtensionAPI {
    pub async fn start_new_task(&self, config: RooCodeSettings) -> Result<String, Error> {
        let task_id = Uuid::new_v4().to_string();
        let provider = Arc::new(TaskProvider::new(config));
        self.task_map.write().await.insert(task_id.clone(), provider);
        self.event_tx.send(RooCodeEvent::TaskCreated { task_id: task_id.clone() })?;
        Ok(task_id)
    }
}
```

---

## 8. src/shared Directory (47 files)

### Core Types

#### WebviewMessage
```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    UpdateTodoList { todos: Vec<TodoItem> },
    SaveApiConfiguration { config: ProviderSettings },
    AskResponse { response: ClineAskResponse },
    // ... 100+ variants
}
```

#### ExtensionMessage
```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionMessage {
    Action { action: String },
    State { state: GlobalState },
    MessageUpdated { message: ClineMessage },
    // ... 50+ variants
}
```

### API Models (`api.ts`)

#### Cerebras Models
```rust
pub const CEREBRAS_MODELS: &[(&str, ModelInfo)] = &[
    ("llama-3.3-70b", ModelInfo {
        max_tokens: 65536,
        context_window: 65536,
        supports_images: false,
        input_price: 0.85,
        output_price: 1.2,
    }),
    // ... 10+ models
];
```

### Modes System
```rust
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub groups: Vec<ToolGroup>,
    pub prompts: Option<CustomModePrompts>,
}

pub fn get_tools_for_mode(groups: &[GroupEntry]) -> Vec<String>
```

### Utility Functions

#### Combine API Requests
```rust
pub fn combine_api_requests(requests: Vec<ApiRequest>) -> CombinedRequest
```

#### Context Mentions
```rust
pub fn parse_mentions(text: &str) -> Vec<ContextMention>
```

#### Cost Calculation
```rust
pub fn calculate_cost(model: &ModelInfo, input_tokens: u32, output_tokens: u32) -> f64
```

---

## Complete File Statistics

### Total Files to Translate: 216
- `src/api/`: 60 files
- `src/assets/`: 57 files (mostly static, embed at compile time)
- `src/activate/`: 10 files
- `src/i18n/`: 72 files (66 JSON + 6 TS)
- `src/services/`: 55 files
- `src/integrations/`: 54 files
- `src/extension/`: 1 file
- `src/shared/`: 47 files
- `src/core/`: ~100 files (not analyzed yet)
- `src/utils/`: ~50 files (not analyzed yet)

### Translation Complexity
1. **High Complexity** (requires architecture changes):
   - Provider system (trait-based instead of class)
   - Streaming (async iterators → tokio streams)
   - Terminal integration (VSCode → Lapce APIs)
   - Extension activation (VSCode → Lapce plugin)

2. **Medium Complexity** (direct translation with adaptations):
   - Code indexing (tree-sitter already in Rust)
   - Git operations (git2 crate)
   - File operations (tokio::fs)
   - IPC (Unix sockets)

3. **Low Complexity** (straightforward ports):
   - Type definitions
   - Utility functions
   - Constants and configurations
   - JSON handling

---

## Implementation Priority

### Phase 1: Core API (Week 1)
1. Base provider trait and streaming infrastructure
2. ApiStreamChunk enum with exact JSON serialization
3. Provider factory with all 40+ providers stubbed

### Phase 2: Critical Providers (Week 2)
1. OpenAI provider with Azure variants
2. Anthropic provider with caching
3. Bedrock provider with ARN parsing
4. Transform functions with golden tests

### Phase 3: I18n and Assets (Week 3)
1. I18n manager with JSON loading
2. Asset embedding at compile time
3. Translation key resolution with fallbacks

### Phase 4: Lapce Integration (Week 4)
1. Command registration mapping
2. Panel management adaptation
3. Code action provider interface
4. Human relay with async callbacks

---

## 9. src/core Directory (~100 files)

### Task System (`task/` - 5 files)

**Task.ts - Core orchestrator (2859 lines!)**
```typescript
export class Task extends EventEmitter<TaskEvents> implements TaskLike {
    private api?: ApiHandler
    private terminalManager: TerminalRegistry
    private urlContentFetcher?: UrlContentFetcher
    private browserSession?: BrowserSession
    private checkpointService?: RepoPerTaskCheckpointService
    
    async handleApiRequest(request: ApiRequest): Promise<void> {
        // 1000+ lines of stream processing, tool execution, error handling
    }
}
```

**Rust Translation:**
```rust
use tokio::sync::{mpsc, RwLock, broadcast};
use futures::stream::{Stream, StreamExt};

pub struct Task {
    api: Arc<RwLock<Option<Box<dyn ApiHandler>>>>,
    terminal_manager: Arc<TerminalRegistry>,
    browser_session: Arc<RwLock<Option<BrowserSession>>>,
    checkpoint_service: Arc<RwLock<Option<CheckpointService>>>,
    state: Arc<RwLock<TaskState>>,
    event_tx: broadcast::Sender<TaskEvent>,
}

impl Task {
    pub async fn handle_api_request(&mut self, request: ApiRequest) -> Result<(), Error> {
        let stream = self.create_message_stream(&request).await?;
        tokio::pin!(stream);
        
        while let Some(chunk) = stream.next().await {
            match chunk? {
                ApiStreamChunk::Text { text } => self.handle_text(&text).await?,
                ApiStreamChunk::ToolUse { tool, input } => {
                    self.execute_tool(&tool, &input).await?
                },
                ApiStreamChunk::Usage { .. } => self.update_usage(usage).await?,
                _ => {}
            }
        }
        Ok(())
    }
}
```

### Webview Provider (`webview/` - 11 files)

**ClineProvider.ts (2831 lines!) - Main extension interface**
```typescript
export class ClineProvider implements vscode.WebviewViewProvider, TaskProviderLike {
    private task?: Task
    private taskMap = new Map<string, Task>()
    private globalState: GlobalState
    
    async resolveWebviewView(webviewView: vscode.WebviewView): Promise<void> {
        // Initialize webview, set HTML, handle messages
    }
    
    async handleMessage(message: WebviewMessage): Promise<void> {
        // 500+ message type handlers
    }
}
```

**Rust for Lapce:**
```rust
use lapce_plugin_api::{ViewId, WebviewHost};

pub struct ClineProvider {
    task: Arc<RwLock<Option<Task>>>,
    task_map: Arc<DashMap<String, Arc<Task>>>,
    global_state: Arc<RwLock<GlobalState>>,
    webview: Arc<WebviewHost>,
}

impl ClineProvider {
    pub async fn handle_message(&self, msg: WebviewMessage) -> Result<(), Error> {
        use WebviewMessage::*;
        match msg {
            UpdateTodoList { todos } => self.update_todos(todos).await,
            SaveApiConfiguration { config } => self.save_config(config).await,
            NewTask { text, images } => self.create_task(text, images).await,
            // ... 100+ message handlers
        }
    }
}
```

### Prompts System (`prompts/` - 87 files!)

#### System Prompt Builder
```rust
pub struct SystemPromptBuilder {
    sections: Vec<PromptSection>,
    variables: HashMap<String, String>,
}

impl SystemPromptBuilder {
    pub fn build(&self) -> String {
        let mut prompt = String::new();
        
        // Add sections in order
        prompt.push_str(&self.get_rules_section());
        prompt.push_str(&self.get_capabilities_section());
        prompt.push_str(&self.get_tools_section());
        prompt.push_str(&self.get_mode_specific_section());
        prompt.push_str(&self.get_custom_instructions());
        
        // Variable substitution
        for (key, value) in &self.variables {
            prompt = prompt.replace(&format!("{{{{ {} }}}}", key), value);
        }
        
        prompt
    }
}
```

#### Tool Descriptions
```rust
pub fn get_tool_descriptions(mode: &Mode) -> Vec<ToolDescription> {
    let mut descriptions = Vec::new();
    
    for tool_name in mode.get_enabled_tools() {
        descriptions.push(match tool_name {
            "read_file" => include_str!("tools/read_file.md"),
            "write_file" => include_str!("tools/write_file.md"),
            "execute_command" => include_str!("tools/execute_command.md"),
            // ... 40+ tools
        });
    }
    
    descriptions
}
```

### Tools System (`tools/` - 43 files)

**Tool Executor Architecture:**
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: &ToolInput) -> Result<ToolOutput, Error>;
    fn validate_input(&self, input: &ToolInput) -> Result<(), ValidationError>;
}

pub struct ToolExecutor {
    tools: HashMap<String, Box<dyn Tool>>,
    permissions: PermissionManager,
    rate_limiter: RateLimiter,
}

impl ToolExecutor {
    pub async fn execute_tool(
        &self,
        name: &str,
        input: &ToolInput,
        ask_approval: bool
    ) -> Result<ToolOutput, Error> {
        // Check permissions
        if !self.permissions.is_allowed(name, input).await? {
            if ask_approval {
                let approved = self.ask_user_approval(name, input).await?;
                if !approved {
                    return Err(Error::PermissionDenied);
                }
            } else {
                return Err(Error::PermissionDenied);
            }
        }
        
        // Rate limiting
        self.rate_limiter.check_limit(name).await?;
        
        // Execute
        let tool = self.tools.get(name).ok_or(Error::ToolNotFound)?;
        tool.validate_input(input)?;
        tool.execute(input).await
    }
}
```

### Context Management (`context/` - 5 files)

```rust
pub struct FileContextTracker {
    mentioned_files: Arc<RwLock<HashSet<PathBuf>>>,
    edited_files: Arc<RwLock<HashMap<PathBuf, FileState>>>,
    max_context_size: usize,
}

impl FileContextTracker {
    pub async fn add_file(&self, path: &Path) -> Result<(), Error> {
        let mut mentioned = self.mentioned_files.write().await;
        mentioned.insert(path.to_path_buf());
        
        // Check total context size
        let total_size = self.calculate_context_size().await?;
        if total_size > self.max_context_size {
            self.trigger_condensation().await?;
        }
        
        Ok(())
    }
}
```

### Sliding Window (`sliding-window/` - 2 files)

```rust
pub struct SlidingWindowManager {
    messages: VecDeque<ClineMessage>,
    max_window_tokens: usize,
    current_tokens: usize,
}

impl SlidingWindowManager {
    pub fn add_message(&mut self, msg: ClineMessage) -> Result<(), Error> {
        let tokens = count_tokens(&msg)?;
        
        while self.current_tokens + tokens > self.max_window_tokens {
            if let Some(old_msg) = self.messages.pop_front() {
                self.current_tokens -= count_tokens(&old_msg)?;
            } else {
                break;
            }
        }
        
        self.messages.push_back(msg);
        self.current_tokens += tokens;
        Ok(())
    }
}
```

---

## 10. src/utils Directory (50 files)

### Token Counting (`countTokens.ts`, `tiktoken.ts`)

**TypeScript:**
```typescript
import { encoding_for_model, Tiktoken } from "tiktoken"
export async function countTokens(
    content: ContentBlockParam[],
    { useWorker = true }: CountTokensOptions = {}
): Promise<number>
```

**Rust:**
```rust
use tiktoken_rs::{cl100k_base, CoreBPE};
use rayon::prelude::*;

pub fn count_tokens(content: &[ContentBlock]) -> Result<usize, Error> {
    let bpe = cl100k_base()?;
    
    let total: usize = content
        .par_iter()
        .map(|block| {
            match block {
                ContentBlock::Text { text } => bpe.encode(text, HashSet::new()).len(),
                ContentBlock::Image { .. } => 765, // Fixed cost for images
                ContentBlock::ToolUse { input, .. } => {
                    bpe.encode(&serde_json::to_string(input)?, HashSet::new()).len()
                },
                _ => 0,
            }
        })
        .sum();
        
    Ok(total)
}
```

### Git Operations (`git.ts`)

```rust
use git2::{Repository, StatusOptions, DiffOptions};

pub struct GitInfo {
    pub branch: String,
    pub remote_url: Option<String>,
    pub root_dir: PathBuf,
    pub modified_files: Vec<PathBuf>,
}

pub async fn get_workspace_git_info(workspace: &Path) -> Result<GitInfo, Error> {
    let repo = Repository::discover(workspace)?;
    
    let head = repo.head()?;
    let branch = head.shorthand().unwrap_or("HEAD").to_string();
    
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().map(String::from);
    
    let mut status_opts = StatusOptions::new();
    status_opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut status_opts))?;
    
    let modified_files = statuses
        .iter()
        .filter(|s| !s.status().is_empty())
        .filter_map(|s| s.path().map(PathBuf::from))
        .collect();
        
    Ok(GitInfo {
        branch,
        remote_url,
        root_dir: repo.workdir().unwrap().to_path_buf(),
        modified_files,
    })
}
```

### XML Matcher (`xml-matcher.ts`)

```rust
pub struct XmlMatcher {
    stack: Vec<String>,
    buffer: String,
    in_tag: bool,
}

impl XmlMatcher {
    pub fn process_chunk(&mut self, chunk: &str) -> Vec<XmlMatch> {
        let mut matches = Vec::new();
        
        for ch in chunk.chars() {
            self.buffer.push(ch);
            
            if ch == '<' {
                self.in_tag = true;
            } else if ch == '>' && self.in_tag {
                self.in_tag = false;
                
                if let Some(tag) = self.parse_tag(&self.buffer) {
                    if tag.starts_with('/') {
                        // Closing tag
                        if let Some(open_tag) = self.stack.pop() {
                            if tag[1..] == open_tag {
                                matches.push(XmlMatch::Element {
                                    tag: open_tag,
                                    content: self.extract_content(),
                                });
                            }
                        }
                    } else {
                        // Opening tag
                        self.stack.push(tag);
                    }
                }
                
                self.buffer.clear();
            }
        }
        
        matches
    }
}
```

### Path Utilities (`path.ts`, `pathUtils.ts`)

```rust
use std::path::{Path, PathBuf, Component};
use pathdiff::diff_paths;

pub fn get_relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    diff_paths(to, from)
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    
    for component in path.components() {
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {}
            c => components.push(c),
        }
    }
    
    components.iter().collect()
}

pub fn is_path_within_workspace(path: &Path, workspace: &Path) -> bool {
    path.canonicalize()
        .ok()
        .and_then(|p| workspace.canonicalize().ok().map(|w| p.starts_with(w)))
        .unwrap_or(false)
}
```

### Logging System (`logging/` - 5 files)

```rust
use tracing::{info, warn, error, span, Level};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct CompactLogger {
    file_appender: RollingFileAppender,
    console_writer: ConsoleWriter,
}

impl CompactLogger {
    pub fn init() -> Result<(), Error> {
        let file_appender = RollingFileAppender::new(
            Rotation::Daily,
            "logs",
            "kilo-code.log"
        );
        
        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(true)
            .compact();
            
        let filter = EnvFilter::from_default_env()
            .add_directive("kilo_code=debug".parse()?);
            
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(filter)
            .init();
            
        Ok(())
    }
}

#[tracing::instrument]
pub async fn log_api_request(request: &ApiRequest) -> Result<(), Error> {
    info!("API request: {:?}", request);
    // Log details
    Ok(())
}
```

---

## Critical Translation Requirements

### 1. Exact Behavioral Preservation
- **Prompt Strings**: Character-for-character identical
- **Tool Descriptions**: No changes to whitespace or formatting
- **Error Messages**: Exact match for debugging compatibility
- **API Schemas**: Field names, types, optionality preserved

### 2. Architecture Adaptations

#### VS Code → Lapce
```rust
// VS Code
vscode.window.showInformationMessage("Hello")
vscode.workspace.getConfiguration("kilo-code")
vscode.commands.registerCommand("kilo.newTask", handler)

// Lapce equivalent
ctx.notify("Hello")
ctx.get_config::<KiloConfig>()
ctx.register_command("kilo.newTask", handler)
```

#### Class → Trait
```rust
// TypeScript class hierarchy
class BaseProvider implements ApiHandler
class OpenAIProvider extends BaseProvider

// Rust trait system
trait ApiHandler: Send + Sync
struct OpenAIProvider { base: BaseProvider }
impl ApiHandler for OpenAIProvider
```

#### Async Patterns
```rust
// TypeScript async iterators
for await (const chunk of stream)

// Rust tokio streams
while let Some(chunk) = stream.next().await
```

### 3. Performance Requirements

| Metric | Target | Implementation |
|--------|--------|----------------|
| Stream Latency | <10ms/chunk | Use tokio unbounded channels |
| Memory Usage | <100MB base | Arc<str> for shared strings |
| Token Counting | <1ms/1K tokens | Parallel with rayon |
| File Operations | <5ms/file | Memory-mapped files for large reads |
| Startup Time | <500ms | Lazy initialization |

### 4. Testing Strategy

#### Golden Tests
```rust
#[test]
fn test_openai_request_format() {
    let request = create_openai_request();
    let json = serde_json::to_string_pretty(&request).unwrap();
    let golden = include_str!("golden/openai_request.json");
    assert_eq!(json, golden);
}
```

#### Stream Replay
```rust
#[tokio::test]
async fn test_anthropic_stream() {
    let events = include_str!("fixtures/anthropic_stream.jsonl");
    let mut stream = replay_stream(events);
    let mut chunks = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk?);
    }
    
    assert_eq!(chunks.len(), 42);
    assert_eq!(chunks[0], ApiStreamChunk::Text { text: "Hello" });
}
```

#### Property Tests
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_token_counting_consistency(content in any::<String>()) {
        let ts_count = typescript_count_tokens(&content);
        let rs_count = rust_count_tokens(&content);
        prop_assert_eq!(ts_count, rs_count);
    }
}
```

---

## Complete Statistics

### Final Count: ~500 TypeScript files
- Core logic: 216 files analyzed
- UI components: ~50 files (React → Lapce UI)
- Tests: ~100 files
- Build/Config: ~30 files

### Lines of Code
- TypeScript: ~150,000 lines
- Estimated Rust: ~200,000 lines (more verbose)

### Timeline (Realistic)
- **Month 1**: Core API + Providers
- **Month 2**: Tools + Task System
- **Month 3**: Lapce Integration
- **Month 4**: Testing + Optimization
- **Month 5**: Production Hardening
- **Month 6**: Documentation + Release

---

## Acceptance Criteria

Each phase must pass:
1. **Unit tests**: 100% coverage of public APIs
2. **Golden tests**: Exact output match with TypeScript
3. **Integration tests**: End-to-end provider streams
4. **Performance tests**: Meet latency/memory targets
5. **Behavioral tests**: Identical error handling and edge cases

This document serves as the authoritative reference for the Rust port. Any deviation requires explicit documentation and justification.
