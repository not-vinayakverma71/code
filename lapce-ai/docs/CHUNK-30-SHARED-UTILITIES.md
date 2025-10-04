# CHUNK 09: src/shared/ - MESSAGE TYPES & UTILITIES (47 FILES)

## Overview
The shared directory contains **the most critical types and utilities** used throughout the codebase:
- **WebviewMessage** (436 lines!) - ALL UI → Extension messages
- **ExtensionMessage** (502 lines!) - ALL Extension → UI messages  
- **API utilities** - Cost calculation, provider configs
- **Experiments** - Feature flags
- **Modes** - Custom AI modes
- **MCP** - Model Context Protocol types

## CRITICAL FILE: WebviewMessage.ts (436 LINES)

**Purpose:** Defines EVERY message type the webview can send to the extension

### Complete Message Type Enumeration (150+ types!)

```typescript
export interface WebviewMessage {
    type:
        // Task Management
        | "webviewDidLaunch"
        | "newTask"
        | "askResponse"
        | "clearTask"
        | "cancelTask"
        | "showTaskWithId"
        | "deleteTaskWithId"
        | "exportTaskWithId"
        | "exportCurrentTask"
        | "shareCurrentTask"
        | "deleteMultipleTasksWithIds"
        
        // API Configuration
        | "apiConfiguration"
        | "saveApiConfiguration"
        | "upsertApiConfiguration"
        | "deleteApiConfiguration"
        | "loadApiConfiguration"
        | "loadApiConfigurationById"
        | "renameApiConfiguration"
        | "getListApiConfiguration"
        | "currentApiConfigName"
        | "setApiConfigPassword"
        
        // Settings
        | "customInstructions"
        | "allowedCommands"
        | "deniedCommands"
        | "alwaysAllowReadOnly"
        | "alwaysAllowReadOnlyOutsideWorkspace"
        | "alwaysAllowWrite"
        | "alwaysAllowWriteOutsideWorkspace"
        | "alwaysAllowWriteProtected"
        | "alwaysAllowExecute"
        | "alwaysAllowFollowupQuestions"
        | "alwaysAllowUpdateTodoList"
        | "alwaysAllowBrowser"
        | "alwaysAllowMcp"
        | "alwaysAllowModeSwitch"
        | "alwaysAllowSubtasks"
        | "alwaysApproveResubmit"
        | "followupAutoApproveTimeoutMs"
        | "allowedMaxRequests"
        | "allowedMaxCost"
        | "autoCondenseContext"
        | "autoCondenseContextPercent"
        | "condensingApiConfigId"
        | "updateCondensingPrompt"
        
        // Terminal Settings
        | "terminalOperation"
        | "terminalOutputLineLimit"
        | "terminalOutputCharacterLimit"
        | "terminalShellIntegrationTimeout"
        | "terminalShellIntegrationDisabled"
        | "terminalCommandDelay"
        | "terminalPowershellCounter"
        | "terminalZshClearEolMark"
        | "terminalZshOhMy"
        | "terminalZshP10k"
        | "terminalZdotdir"
        | "terminalCompressProgressBar"
        
        // UI Features
        | "selectImages"
        | "draggedImages"
        | "openImage"
        | "saveImage"
        | "openFile"
        | "openMention"
        | "openInBrowser"
        | "fetchOpenGraphData"
        | "checkIsImageUrl"
        | "didShowAnnouncement"
        | "deleteMessage"
        | "deleteMessageConfirm"
        | "submitEditedMessage"
        | "editMessageConfirm"
        
        // TODO List
        | "updateTodoList"
        
        // Audio/TTS
        | "playSound"
        | "playTts"
        | "stopTts"
        | "soundEnabled"
        | "ttsEnabled"
        | "ttsSpeed"
        | "soundVolume"
        
        // Diff/Checkpoints
        | "diffEnabled"
        | "enableCheckpoints"
        
        // Browser
        | "browserViewportSize"
        | "screenshotQuality"
        | "remoteBrowserHost"
        
        // MCP (Model Context Protocol)
        | "openMcpSettings"
        | "openProjectMcpSettings"
        | "restartMcpServer"
        | "refreshAllMcpServers"
        | "toggleMcpServer"
        | "updateMcpTimeout"
        | "mcpEnabled"
        | "enableMcpServerCreation"
        
        // Tool Settings
        | "toggleToolAutoApprove"
        | "toggleToolAlwaysAllow"
        | "toggleToolEnabledForPrompt"
        | "fuzzyMatchThreshold"
        | "writeDelayMs"
        | "diagnosticsEnabled"
        
        // Model Discovery
        | "requestRouterModels"
        | "flushRouterModels"
        | "requestOpenAiModels"
        | "requestOllamaModels"
        | "requestLmStudioModels"
        | "requestVsCodeLmModels"
        | "requestHuggingFaceModels"
        
        // Import/Export
        | "importSettings"
        | "exportSettings"
        | "resetState"
        
        // VS Code Integration
        | "openExtensionSettings"
        | "updateVSCodeSetting"
        | "getVSCodeSetting"
        | "vsCodeSetting"
        
        // Prompt Enhancement
        | "enhancePrompt"
        | "enhancedPrompt"
        
        // Advanced
        | "mode"
        | "remoteControlEnabled"
        | "searchCommits"
        | "requestDelaySeconds"
        | "morphApiKey"
        
    // Additional fields based on type
    text?: string
    images?: string[]
    askResponse?: ClineAskResponse
    apiConfiguration?: ProviderSettings
    // ... many more fields
}

export type ClineAskResponse =
    | "yesButtonClicked"
    | "noButtonClicked"
    | "messageResponse"
    | "objectResponse"
    | "retry_clicked"
```

**RUST TRANSLATION:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    // Task Management
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
    
    // ... 150+ variants total
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClineAskResponse {
    #[serde(rename = "yesButtonClicked")]
    YesButtonClicked,
    #[serde(rename = "noButtonClicked")]
    NoButtonClicked,
    #[serde(rename = "messageResponse")]
    MessageResponse,
    #[serde(rename = "objectResponse")]
    ObjectResponse,
    #[serde(rename = "retry_clicked")]
    RetryClicked,
}
```

**CRITICAL:** Every single variant must be ported exactly for UI compatibility.

## CRITICAL FILE: ExtensionMessage.ts (502 LINES)

**Purpose:** Defines EVERY message type the extension can send to the webview

### Complete Message Type Enumeration (80+ types!)

```typescript
export interface ExtensionMessage {
    type:
        | "action"              // UI button clicks
        | "state"               // Full state sync
        | "selectedImages"      // Image selection result
        | "theme"               // Theme update
        | "workspaceUpdated"    // Workspace changed
        | "invoke"              // Invoke task action
        | "messageUpdated"      // Chat message updated
        | "mcpServers"          // MCP server list
        | "enhancedPrompt"      // Enhanced prompt result
        | "commitSearchResults" // Git commit search
        | "listApiConfig"       // API config list
        | "routerModels"        // Router model list
        | "openAiModels"        // OpenAI models
        | "ollamaModels"        // Ollama models
        | "lmStudioModels"      // LM Studio models
        | "vsCodeLmModels"      // VS Code LM models
        | "huggingFaceModels"   // HuggingFace models
        | "vsCodeLmApiAvailable" // LM API status
        | "updatePrompt"        // Prompt update
        | "systemPrompt"        // System prompt
        | "autoApprovalEnabled" // Auto-approval status
        | "updateCustomMode"    // Custom mode update
        | "deleteCustomMode"    // Custom mode delete
        | "exportModeResult"    // Mode export result
        | "importModeResult"    // Mode import result
        | "checkRulesDirectoryResult" // Rules check
        | "deleteCustomModeCheck" // Delete check
        | "currentCheckpointUpdated" // Checkpoint update
        | "showHumanRelayDialog" // Human relay dialog
        | "humanRelayResponse"   // Human relay response
        | "humanRelayCancel"     // Human relay cancel
        | "insertTextToChatArea" // Insert text
        | "browserToolEnabled"   // Browser status
        | "browserConnectionResult" // Browser connection
        | "remoteBrowserEnabled" // Remote browser
        | "ttsStart"            // TTS started
        | "ttsStop"             // TTS stopped
        | "maxReadFileLine"     // Max file lines
        | "fileSearchResults"   // File search results
        | "toggleApiConfigPin"  // Pin API config
        | "mcpMarketplaceCatalog" // MCP marketplace
        | "mcpDownloadDetails"  // MCP download
        | "showSystemNotification" // System notification
        | "openInBrowser"       // Open browser
        | "acceptInput"         // Accept input
        | "focusChatInput"      // Focus chat
        | "setHistoryPreviewCollapsed" // History preview
        | "commandExecutionStatus" // Command status
        | "mcpExecutionStatus"  // MCP status
        | "vsCodeSetting"       // VS Code setting
        | "profileDataResponse" // User profile
        | "balanceDataResponse" // Account balance
        | "updateProfileData"   // Profile update
        | "authenticatedUser"   // Auth status
        | "condenseTaskContextResponse" // Context condensing
        | "singleRouterModelFetchResponse" // Single model
        | "indexingStatusUpdate" // Code indexing
        | "indexCleared"        // Index cleared
        | "codebaseIndexConfig" // Index config
        | "rulesData"           // Rules data
        | "marketplaceInstallResult" // Marketplace install
        | "marketplaceRemoveResult" // Marketplace remove
        | "marketplaceData"     // Marketplace data
        | "mermaidFixResponse"  // Mermaid diagram
        | "shareTaskSuccess"    // Task share success
        | "codeIndexSettingsSaved" // Index settings
        | "codeIndexSecretStatus" // Index secrets
        | "showDeleteMessageDialog" // Delete dialog
        | "showEditMessageDialog" // Edit dialog
        | "kilocodeNotificationsResponse" // Notifications
        | "usageDataResponse"   // Usage data
        | "commands"            // Command list
        | "insertTextIntoTextarea" // Insert text
        
    text?: string
    action?: "chatButtonClicked" | "mcpButtonClicked" | ...
    // ... many payload fields
}
```

**RUST:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionMessage {
    #[serde(rename = "action")]
    Action { action: Action },
    
    #[serde(rename = "state")]
    State { state: GlobalState },
    
    #[serde(rename = "selectedImages")]
    SelectedImages { images: Vec<String> },
    
    #[serde(rename = "theme")]
    Theme { 
        #[serde(rename = "isDark")]
        is_dark: bool 
    },
    
    // ... 80+ variants
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "chatButtonClicked")]
    ChatButtonClicked,
    #[serde(rename = "settingsButtonClicked")]
    SettingsButtonClicked,
    // ...
}
```

## api.ts - Model Definitions (263 LINES)

**Purpose:** Model configuration for ALL 40+ providers

### Model Info Structure

```typescript
export interface ModelRecord {
    id: string
    info: ModelInfo
}

export interface ModelInfo {
    maxTokens?: number
    contextWindow: number
    supportsImages: boolean
    supportsPromptCache: boolean
    inputPrice: number    // per 1M tokens
    outputPrice: number   // per 1M tokens
    cacheWritesPrice?: number
    cacheReadsPrice?: number
    description?: string
}

// Example: Cerebras models
export const cerebrasModels = {
    "llama3.1-8b": {
        maxTokens: 8192,
        contextWindow: 8192,
        supportsImages: false,
        supportsPromptCache: false,
        inputPrice: 0.1,
        outputPrice: 0.1,
        description: "Fast model ~2200 tokens/s"
    },
    // ... 10+ Cerebras models
}

// Similar definitions for:
// - anthropicModels (Claude)
// - openAiModels (GPT)
// - geminiModels (Gemini)
// - bedrockModels (AWS)
// - groqModels
// - etc. (40+ providers)
```

**RUST:**
```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub max_tokens: Option<u32>,
    pub context_window: u32,
    pub supports_images: bool,
    pub supports_prompt_cache: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub cache_writes_price: Option<f64>,
    pub cache_reads_price: Option<f64>,
    pub description: Option<String>,
}

// Model registry
pub fn get_cerebras_models() -> HashMap<String, ModelInfo> {
    let mut models = HashMap::new();
    models.insert(
        "llama3.1-8b".to_string(),
        ModelInfo {
            max_tokens: Some(8192),
            context_window: 8192,
            supports_images: false,
            supports_prompt_cache: false,
            input_price: 0.1,
            output_price: 0.1,
            cache_writes_price: None,
            cache_reads_price: None,
            description: Some("Fast model ~2200 tokens/s".to_string()),
        }
    );
    // ... add remaining models
    models
}

// Builder for model registry
pub struct ModelRegistry {
    models: HashMap<String, HashMap<String, ModelInfo>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
        };
        registry.models.insert("cerebras".to_string(), get_cerebras_models());
        registry.models.insert("anthropic".to_string(), get_anthropic_models());
        // ... register all providers
        registry
    }
    
    pub fn get_model(&self, provider: &str, model_id: &str) -> Option<&ModelInfo> {
        self.models.get(provider)?.get(model_id)
    }
}
```

## cost.ts - Cost Calculation

```typescript
export function calculateApiCostAnthropic(
    modelInfo: ModelInfo,
    inputTokens: number,
    outputTokens: number,
    cacheCreationInputTokens?: number,
    cacheReadInputTokens?: number,
): number {
    const cacheWritesCost = ((modelInfo.cacheWritesPrice || 0) / 1_000_000) * (cacheCreationInputTokens || 0)
    const cacheReadsCost = ((modelInfo.cacheReadsPrice || 0) / 1_000_000) * (cacheReadInputTokens || 0)
    const baseInputCost = ((modelInfo.inputPrice || 0) / 1_000_000) * inputTokens
    const outputCost = ((modelInfo.outputPrice || 0) / 1_000_000) * outputTokens
    return cacheWritesCost + cacheReadsCost + baseInputCost + outputCost
}

// OpenAI: input tokens INCLUDE cached tokens
export function calculateApiCostOpenAI(
    modelInfo: ModelInfo,
    inputTokens: number,
    outputTokens: number,
    cacheCreationInputTokens?: number,
    cacheReadInputTokens?: number,
): number {
    const nonCachedInputTokens = Math.max(0, 
        inputTokens - (cacheCreationInputTokens || 0) - (cacheReadInputTokens || 0))
    
    return calculateApiCostInternal(
        modelInfo, 
        nonCachedInputTokens, 
        outputTokens,
        cacheCreationInputTokens || 0,
        cacheReadInputTokens || 0
    )
}
```

**RUST:**
```rust
pub fn calculate_api_cost_anthropic(
    model_info: &ModelInfo,
    input_tokens: u32,
    output_tokens: u32,
    cache_creation_tokens: Option<u32>,
    cache_read_tokens: Option<u32>,
) -> f64 {
    let cache_writes_cost = (model_info.cache_writes_price.unwrap_or(0.0) / 1_000_000.0) 
        * cache_creation_tokens.unwrap_or(0) as f64;
    let cache_reads_cost = (model_info.cache_reads_price.unwrap_or(0.0) / 1_000_000.0) 
        * cache_read_tokens.unwrap_or(0) as f64;
    let base_input_cost = (model_info.input_price / 1_000_000.0) * input_tokens as f64;
    let output_cost = (model_info.output_price / 1_000_000.0) * output_tokens as f64;
    
    cache_writes_cost + cache_reads_cost + base_input_cost + output_cost
}
```

## experiments.ts - Feature Flags

```typescript
export interface Experiments {
    preventFocusDisruption?: boolean
    parallelBrowserSessions?: boolean
    directoryTreeCacheEnabled?: boolean
    // ... 20+ experimental features
}

export function getExperiments(): Experiments {
    return {
        preventFocusDisruption: true,
        parallelBrowserSessions: false,
        directoryTreeCacheEnabled: true,
        // ...
    }
}
```

## modes.ts - Custom AI Modes

```typescript
export enum Mode {
    Code = "code",
    Architect = "architect",
    Ask = "ask",
    // ...
}

export interface ModeConfig {
    slug: string
    name: string
    systemPrompt?: string
    enabledTools?: string[]
    // ...
}
```

## mcp.ts - Model Context Protocol

```typescript
export interface McpServer {
    name: string
    command: string
    args?: string[]
    env?: Record<string, string>
    disabled?: boolean
}
```

## CRITICAL UTILITIES

### array.ts - Array Helpers

```typescript
export function findLast<T>(arr: T[], predicate: (item: T) => boolean): T | undefined {
    for (let i = arr.length - 1; i >= 0; i--) {
        if (predicate(arr[i])) return arr[i]
    }
    return undefined
}
```

### tools.ts - Tool Definitions

```typescript
export const TOOL_NAMES = [
    "write_to_file",
    "read_file",
    "list_files",
    "execute_command",
    // ... all 20+ tools
] as const

export type ToolName = typeof TOOL_NAMES[number]
```

## Translation Priority

| File | Lines | Priority | Complexity |
|------|-------|----------|-----------|
| WebviewMessage.ts | 436 | **CRITICAL** | High - 150+ variants |
| ExtensionMessage.ts | 502 | **CRITICAL** | High - 80+ variants |
| api.ts | 263 | High | Medium - 40+ model defs |
| cost.ts | 58 | High | Low - Simple math |
| experiments.ts | ~100 | Medium | Low - Feature flags |
| modes.ts | ~150 | Medium | Medium - Mode system |
| mcp.ts | ~80 | Medium | Medium - Protocol types |

## Next: CHUNK 10 - utils/ Directory
Helper functions, XML parsing, git utilities, etc.
