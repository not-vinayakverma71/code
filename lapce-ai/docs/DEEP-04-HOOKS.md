# DEEP ANALYSIS 04: CUSTOM HOOKS - REACT TO RUST PATTERNS

## 📁 Analyzed Files

```
Codex/webview-ui/src/
├── hooks/
│   ├── useAutoApproval.ts            (Auto-approval permission logic)
│   ├── useSelectedModel.ts           (Model selection state)
│   ├── useRouterModels.ts            (Router model fetching)
│   ├── useCopyToClipboard.ts         (Copy feedback)
│   └── useTaskSearch.ts              (Fuzzy search + filters)
│
├── components/kilocode/hooks/
│   ├── useProviderModels.ts          (Provider-specific models)
│   └── useSelectedModel.ts           (Kilocode model selection)
│
└── context/
    └── ExtensionStateContext.tsx     (State management hooks)
        ├── useState patterns          (179 state fields)
        ├── useEffect patterns         (Lifecycle hooks)
        ├── useMemo patterns           (Computed values)
        └── useCallback patterns       (Memoized functions)

Total: 26 custom hooks → Rust state management & caching patterns
```

---

## Overview
The frontend uses **26 custom React hooks** for state management, API queries, and UI logic. These must be translated to Rust backend functions or WebSocket state streams.

---

## 1. Model Selection Hooks

### useSelectedModel (448 lines)

**Purpose:** Resolves the active AI model based on provider and configuration.

```typescript
// React hook
const { provider, id, info, isLoading, isError } = useSelectedModel(apiConfiguration)

// Returns:
// - provider: "anthropic" | "openai" | "gemini" | ... (40 providers)
// - id: "claude-sonnet-4-20250514"
// - info: { maxTokens, contextWindow, supportsImages, inputPrice, outputPrice }
// - isLoading: boolean (fetching router models)
// - isError: boolean

// Internal logic:
function getSelectedModel(provider, apiConfiguration, routerModels) {
    switch (provider) {
        case "anthropic":
            const id = apiConfiguration.apiModelId ?? "claude-sonnet-4-20250514"
            const info = anthropicModels[id]
            
            // Handle 1M context beta
            if (apiConfiguration.anthropicBeta1MContext && id === "claude-sonnet-4") {
                return { id, info: { ...info, contextWindow: 1_000_000 } }
            }
            return { id, info }
            
        case "openrouter":
            const id = apiConfiguration.openRouterModelId ?? "anthropic/claude-3.5-sonnet"
            let info = routerModels.openrouter[id]
            
            // Override with specific provider pricing
            if (apiConfiguration.openRouterSpecificProvider) {
                info = { ...info, ...openRouterModelProviders[specificProvider] }
            }
            return { id, info }
            
        case "ollama":
            // Query local Ollama server for models
            const id = apiConfiguration.ollamaModelId ?? ""
            const info = routerModels.ollama[id]  // From API query
            return { id, info }
            
        // ... 37 more providers
    }
}
```

**Rust Translation:**

```rust
// Backend function (synchronous, no React)
pub fn get_selected_model(
    provider: &str,
    api_configuration: &ProviderSettings,
    router_models: &RouterModels,
) -> Result<SelectedModel> {
    match provider {
        "anthropic" => {
            let id = api_configuration.api_model_id.as_deref()
                .unwrap_or("claude-sonnet-4-20250514");
            let mut info = ANTHROPIC_MODELS.get(id)
                .ok_or_else(|| anyhow!("Unknown model: {}", id))?;
            
            // Handle 1M context beta
            if api_configuration.anthropic_beta_1m_context.unwrap_or(false) 
                && id == "claude-sonnet-4-20250514" {
                info.context_window = 1_000_000;
            }
            
            Ok(SelectedModel { id: id.to_string(), info })
        }
        
        "openrouter" => {
            let id = api_configuration.open_router_model_id.as_deref()
                .unwrap_or("anthropic/claude-3.5-sonnet");
            let mut info = router_models.openrouter.get(id)
                .ok_or_else(|| anyhow!("Model not found: {}", id))?;
            
            // Override with specific provider
            if let Some(provider) = &api_configuration.open_router_specific_provider {
                if let Some(provider_info) = get_openrouter_provider_info(provider).await? {
                    info = merge_model_info(info, provider_info);
                }
            }
            
            Ok(SelectedModel { id: id.to_string(), info })
        }
        
        "ollama" => {
            let id = api_configuration.ollama_model_id.as_deref()
                .unwrap_or("");
            let info = router_models.ollama.get(id)
                .ok_or_else(|| anyhow!("Ollama model not found: {}", id))?;
            
            Ok(SelectedModel { id: id.to_string(), info })
        }
        
        _ => Err(anyhow!("Unknown provider: {}", provider))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SelectedModel {
    pub id: String,
    pub info: ModelInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelInfo {
    pub max_tokens: i32,
    pub context_window: i32,
    pub supports_images: bool,
    pub supports_prompt_cache: bool,
    pub input_price: f64,      // per million tokens
    pub output_price: f64,     // per million tokens
    pub cache_writes_price: Option<f64>,
    pub cache_reads_price: Option<f64>,
}

// Static model definitions (no runtime queries)
lazy_static! {
    static ref ANTHROPIC_MODELS: HashMap<&'static str, ModelInfo> = {
        let mut map = HashMap::new();
        map.insert("claude-sonnet-4-20250514", ModelInfo {
            max_tokens: 8192,
            context_window: 200_000,
            supports_images: true,
            supports_prompt_cache: true,
            input_price: 3.0,
            output_price: 15.0,
            cache_writes_price: Some(3.75),
            cache_reads_price: Some(0.30),
        });
        // ... more models
        map
    };
}
```

---

### useRouterModels (52 lines)

**Purpose:** Fetches available models from OpenRouter, Ollama, LM Studio APIs.

```typescript
// React Query hook (async, cached)
const { data, isLoading, isError } = useRouterModels({
    openRouterBaseUrl: "https://openrouter.ai/api/v1",
    openRouterApiKey: "sk-or-...",
    ollamaBaseUrl: "http://localhost:11434",
    kilocodeOrganizationId: "org_123"
})

// Returns: RouterModels
type RouterModels = {
    openrouter: Record<string, ModelInfo>
    ollama: Record<string, ModelInfo>
    lmstudio: Record<string, ModelInfo>
    requesty: Record<string, ModelInfo>
    glama: Record<string, ModelInfo>
    // ... more
}

// Implementation: sends message, waits for response
const getRouterModels = async () =>
    new Promise<RouterModels>((resolve, reject) => {
        const handler = (event: MessageEvent) => {
            if (event.data.type === "routerModels") {
                resolve(event.data.routerModels)
            }
        }
        
        window.addEventListener("message", handler)
        vscode.postMessage({ type: "requestRouterModels" })
        
        setTimeout(() => reject(new Error("Timeout")), 10000)
    })
```

**Rust Translation:**

```rust
// Cached API queries (executed on startup and refresh)
pub async fn fetch_router_models(
    config: &ProviderSettings
) -> Result<RouterModels> {
    let mut models = RouterModels::default();
    
    // Query OpenRouter API
    if let Some(api_key) = &config.open_router_api_key {
        let base_url = config.open_router_base_url.as_deref()
            .unwrap_or("https://openrouter.ai/api/v1");
        
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/models", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;
        
        let openrouter_models: Vec<OpenRouterModel> = response.json().await?;
        
        for model in openrouter_models {
            models.openrouter.insert(model.id.clone(), ModelInfo {
                max_tokens: model.top_provider.max_completion_tokens,
                context_window: model.top_provider.context_length,
                supports_images: model.architecture.modality.contains("image"),
                supports_prompt_cache: false,
                input_price: model.pricing.prompt * 1_000_000.0,
                output_price: model.pricing.completion * 1_000_000.0,
                cache_writes_price: None,
                cache_reads_price: None,
            });
        }
    }
    
    // Query Ollama API (local)
    if let Some(base_url) = &config.ollama_base_url {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/tags", base_url))
            .send()
            .await?;
        
        let ollama_response: OllamaTagsResponse = response.json().await?;
        
        for model in ollama_response.models {
            models.ollama.insert(model.name.clone(), ModelInfo {
                max_tokens: 4096,  // Default, Ollama doesn't report this
                context_window: model.details.parameter_size * 1000,
                supports_images: false,
                supports_prompt_cache: false,
                input_price: 0.0,  // Local, free
                output_price: 0.0,
                cache_writes_price: None,
                cache_reads_price: None,
            });
        }
    }
    
    // Query LM Studio API (local)
    if let Some(base_url) = &config.lm_studio_base_url {
        // Similar to Ollama query
    }
    
    Ok(models)
}

// Cache in AppState, refresh every 1 hour
pub async fn cached_router_models(
    state: Arc<RwLock<AppState>>
) -> Result<RouterModels> {
    let cached = {
        let state = state.read().await;
        if let Some((models, timestamp)) = &state.cached_router_models {
            if timestamp.elapsed()? < Duration::from_secs(3600) {
                return Ok(models.clone());
            }
        }
        None
    };
    
    // Fetch fresh
    let models = fetch_router_models(&state.read().await.api_configuration).await?;
    
    // Update cache
    {
        let mut state = state.write().await;
        state.cached_router_models = Some((models.clone(), SystemTime::now()));
    }
    
    Ok(models)
}
```

---

## 2. Auto-Approval Hooks

### useAutoApprovalState (30 lines)

**Purpose:** Determines if auto-approval UI should be shown.

```typescript
const { hasEnabledOptions, effectiveAutoApprovalEnabled } = useAutoApprovalState({
    alwaysAllowReadOnly,
    alwaysAllowWrite,
    alwaysAllowExecute,
    alwaysAllowBrowser,
    alwaysAllowMcp,
    alwaysAllowModeSwitch,
    alwaysAllowSubtasks,
    alwaysApproveResubmit,
    alwaysAllowFollowupQuestions,
    alwaysAllowUpdateTodoList,
}, autoApprovalEnabled)

// Returns:
// - hasEnabledOptions: true if any permission is enabled
// - effectiveAutoApprovalEnabled: hasEnabledOptions && autoApprovalEnabled
```

**Rust Translation:**

```rust
// Simple computed property
pub fn get_auto_approval_state(permissions: &Permissions) -> AutoApprovalState {
    let has_enabled_options = 
        permissions.always_allow_read_only ||
        permissions.always_allow_write ||
        permissions.always_allow_execute ||
        permissions.always_allow_browser ||
        permissions.always_allow_mcp ||
        permissions.always_allow_mode_switch ||
        permissions.always_allow_subtasks ||
        permissions.always_approve_resubmit ||
        permissions.always_allow_followup_questions ||
        permissions.always_allow_update_todo_list;
    
    AutoApprovalState {
        has_enabled_options,
        effective_auto_approval_enabled: 
            has_enabled_options && permissions.auto_approval_enabled,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutoApprovalState {
    pub has_enabled_options: bool,
    pub effective_auto_approval_enabled: bool,
}
```

---

## 3. History & Search Hooks

### useTaskSearch (101 lines)

**Purpose:** Fuzzy search and sort task history.

```typescript
const {
    tasks,           // Filtered & sorted results
    searchQuery,
    setSearchQuery,
    sortOption,      // "newest" | "oldest" | "mostExpensive" | "mostTokens" | "mostRelevant"
    setSortOption,
    showAllWorkspaces,
    setShowAllWorkspaces,
    showFavoritesOnly,
    setShowFavoritesOnly,
} = useTaskSearch()

// Implementation uses Fzf (fuzzy finder) library
const fzf = new Fzf(taskHistory, {
    selector: (item) => item.task
})

const results = fzf.find(searchQuery)  // Fuzzy search
    .map(result => ({
        ...result.item,
        highlight: highlightFzfMatch(result.item.task, result.positions)
    }))
    .sort((a, b) => {
        switch (sortOption) {
            case "newest": return b.ts - a.ts
            case "oldest": return a.ts - b.ts
            case "mostExpensive": return b.totalCost - a.totalCost
            case "mostTokens": 
                return (b.tokensIn + b.tokensOut) - (a.tokensIn + a.tokensOut)
        }
    })
```

**Rust Translation:**

```rust
// Use fuzzy-matcher crate
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub fn search_task_history(
    task_history: &[HistoryItem],
    search_query: &str,
    sort_option: SortOption,
    show_all_workspaces: bool,
    show_favorites_only: bool,
    current_workspace: &str,
) -> Vec<TaskSearchResult> {
    // Filter
    let mut tasks: Vec<_> = task_history
        .iter()
        .filter(|item| {
            if !show_all_workspaces && item.workspace.as_deref() != Some(current_workspace) {
                return false;
            }
            if show_favorites_only && !item.is_favorited.unwrap_or(false) {
                return false;
            }
            true
        })
        .collect();
    
    // Fuzzy search
    let results = if !search_query.is_empty() {
        let matcher = SkimMatcherV2::default();
        
        tasks
            .iter()
            .filter_map(|item| {
                matcher.fuzzy_match(&item.task, search_query)
                    .map(|score| (item, score))
            })
            .map(|(item, score)| TaskSearchResult {
                item: (*item).clone(),
                score: Some(score),
                highlight: None,  // TODO: extract match positions
            })
            .collect()
    } else {
        tasks.iter()
            .map(|item| TaskSearchResult {
                item: (*item).clone(),
                score: None,
                highlight: None,
            })
            .collect()
    };
    
    // Sort
    let mut results = results;
    results.sort_by(|a, b| {
        match sort_option {
            SortOption::Newest => b.item.ts.cmp(&a.item.ts),
            SortOption::Oldest => a.item.ts.cmp(&b.item.ts),
            SortOption::MostExpensive => {
                b.item.total_cost.partial_cmp(&a.item.total_cost)
                    .unwrap_or(Ordering::Equal)
            }
            SortOption::MostTokens => {
                let a_tokens = a.item.tokens_in + a.item.tokens_out;
                let b_tokens = b.item.tokens_in + b.item.tokens_out;
                b_tokens.cmp(&a_tokens)
            }
            SortOption::MostRelevant => {
                if !search_query.is_empty() {
                    b.score.cmp(&a.score)  // Higher score first
                } else {
                    b.item.ts.cmp(&a.item.ts)  // Fallback to newest
                }
            }
        }
    });
    
    results
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SortOption {
    Newest,
    Oldest,
    MostExpensive,
    MostTokens,
    MostRelevant,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskSearchResult {
    pub item: HistoryItem,
    pub score: Option<i64>,
    pub highlight: Option<String>,  // HTML with <mark> tags
}
```

---

## 4. Input Management Hooks

### usePromptHistory (189 lines)

**Purpose:** Navigate previous prompts with Up/Down arrows.

```typescript
const {
    handleHistoryNavigation,  // Arrow key handler
    resetHistoryNavigation,   // Clear on send
    resetOnInputChange,       // Clear on type
} = usePromptHistory({
    clineMessages,
    taskHistory,
    cwd,
    inputValue,
    setInputValue,
})

// Extracts user prompts from two sources:
// 1. Current conversation: user_feedback messages
// 2. Task history: task field from completed tasks

const conversationPrompts = clineMessages
    ?.filter(msg => msg.say === "user_feedback" && msg.text?.trim())
    .map(msg => msg.text)
    .reverse()  // Newest first

// Navigation logic
const handleHistoryNavigation = (event) => {
    if (event.key === "ArrowUp" && cursorAtBeginning) {
        if (historyIndex === -1) {
            tempInput = inputValue  // Save current
        }
        historyIndex++
        inputValue = promptHistory[historyIndex]
        return true  // Handled
    }
    
    if (event.key === "ArrowDown" && historyIndex >= 0) {
        historyIndex--
        if (historyIndex === -1) {
            inputValue = tempInput  // Restore
        } else {
            inputValue = promptHistory[historyIndex]
        }
        return true
    }
    
    return false  // Not handled
}
```

**Rust Translation:**

```rust
// This is pure UI logic, stays in frontend
// Backend only needs to provide the data:

// 1. Stream user_feedback messages as they happen
pub async fn broadcast_user_feedback(
    text: &str,
    clients: &HashMap<Uuid, WebSocket>
) {
    let msg = ClineMessage {
        ts: SystemTime::now().timestamp_millis(),
        r#type: "say",
        say: Some("user_feedback"),
        text: Some(text.to_string()),
        ..Default::default()
    };
    
    broadcast_message_update(msg, clients).await;
}

// 2. Provide task history via state message
// (Already handled in DEEP-02-STATE-MANAGEMENT.md)
```

---

## 5. Clipboard Hook

### useCopyToClipboard (76 lines)

**Purpose:** Copy text with feedback animation.

```typescript
const { showCopyFeedback, copyWithFeedback } = useCopyToClipboard(2000)

// Usage in UI
<button onClick={() => copyWithFeedback(text)}>
    {showCopyFeedback ? "Copied!" : "Copy"}
</button>

// Implementation
const copyWithFeedback = async (text: string) => {
    await navigator.clipboard.writeText(text)
    setShowCopyFeedback(true)
    
    setTimeout(() => {
        setShowCopyFeedback(false)
    }, 2000)
}
```

**Rust Translation:**

```rust
// Pure frontend logic, no backend needed
// Browser Clipboard API handles everything
```

---

## Hook Translation Summary

| Hook Category | React Hooks | Rust Translation |
|---------------|-------------|------------------|
| **Model Selection** | `useSelectedModel`, `useRouterModels`, `useModelProviders` | Backend functions with static model data + cached API queries |
| **Auto-Approval** | `useAutoApprovalState`, `useAutoApprovalToggles` | Simple computed properties in backend |
| **History Search** | `useTaskSearch` | Backend search function with fuzzy-matcher crate |
| **Input Management** | `usePromptHistory` | Frontend only, backend provides data via WebSocket |
| **UI Utilities** | `useCopyToClipboard`, `useEscapeKey`, `useTooltip` | Frontend only, pure browser APIs |
| **API Queries** | `useOpenRouterKeyInfo`, `useRequestyKeyInfo` | Backend async functions |
| **Theme** | `useVSCodeTheme` | Frontend only, reads CSS variables |

---

## Critical Translation Patterns

### Pattern 1: React Query → Rust Cache

```typescript
// React Query (automatic caching, refetching)
const { data, isLoading, isError } = useQuery({
    queryKey: ["routerModels", apiKey],
    queryFn: fetchRouterModels,
})
```

```rust
// Rust manual cache with TTL
pub struct CachedData<T> {
    data: T,
    timestamp: SystemTime,
    ttl: Duration,
}

impl<T: Clone> CachedData<T> {
    pub fn get(&self) -> Option<T> {
        if self.timestamp.elapsed().ok()? < self.ttl {
            Some(self.data.clone())
        } else {
            None
        }
    }
}

// In AppState
pub struct AppState {
    router_models_cache: Option<CachedData<RouterModels>>,
}
```

### Pattern 2: useMemo → Rust Pure Function

```typescript
// React memoized computation
const tasks = useMemo(() => {
    return taskHistory
        .filter(item => item.workspace === cwd)
        .sort((a, b) => b.ts - a.ts)
}, [taskHistory, cwd])
```

```rust
// Rust pure function (called on-demand)
pub fn get_filtered_tasks(
    task_history: &[HistoryItem],
    cwd: &str
) -> Vec<HistoryItem> {
    let mut tasks: Vec<_> = task_history
        .iter()
        .filter(|item| item.workspace.as_deref() == Some(cwd))
        .cloned()
        .collect();
    
    tasks.sort_by(|a, b| b.ts.cmp(&a.ts));
    tasks
}
```

### Pattern 3: useCallback → Rust Method

```typescript
// React callback with closure
const handleSearch = useCallback((query: string) => {
    setSearchQuery(query)
    const results = fzf.find(query)
    setResults(results)
}, [fzf])
```

```rust
// Rust method (no closures needed)
impl TaskSearch {
    pub fn search(&mut self, query: &str) -> Vec<TaskSearchResult> {
        self.search_query = query.to_string();
        let results = self.fuzzy_search(query);
        self.results = results.clone();
        results
    }
}
```

---

**STATUS:** Complete hooks analysis (26 hooks → Rust patterns)
**NEXT:** DEEP-05-SERVICES.md - Service layer and utility functions
