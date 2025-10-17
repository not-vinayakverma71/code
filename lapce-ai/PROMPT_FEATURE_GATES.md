# Prompt System Feature Gates (P18)

**Date:** 2025-10-17  
**Status:** Pre-IPC (All experimental features disabled by default)

---

## Overview

The prompt system uses feature gates to control access to experimental or IPC-dependent capabilities. All gates default to **OFF** pre-IPC to ensure stability and gradual rollout.

---

## Feature Gate Categories

### 1. **Core Feature Gates** (SystemPromptSettings)

These are runtime settings controlled via `SystemPromptSettings`:

```rust
pub struct SystemPromptSettings {
    /// Maximum concurrent file reads for read_file tool
    pub max_concurrent_file_reads: u32,
    
    /// Enable TODO list management tool
    pub todo_list_enabled: bool,
    
    /// Include AGENTS.md content in custom instructions
    pub use_agent_rules: bool,
    
    /// Require TODOs parameter in new_task tool
    pub new_task_require_todos: bool,
    
    /// Browser viewport size for browser_action tool
    pub browser_viewport_size: Option<String>,
}
```

**Defaults (Pre-IPC):**
- `max_concurrent_file_reads`: 5
- `todo_list_enabled`: **false** (disabled)
- `use_agent_rules`: **true** (enabled - low risk)
- `new_task_require_todos`: **false** (optional)
- `browser_viewport_size`: None

---

### 2. **Tool Feature Gates** (ToolDescriptionContext)

These control which tools appear in the prompt:

```rust
pub struct ToolDescriptionContext<'a> {
    workspace: &'a Path,
    
    // === Browser Support ===
    supports_browser: bool,                // DEFAULT: false (requires IPC)
    browser_viewport_size: &'a str,        // DEFAULT: "900x600"
    
    // === Search & Code Intelligence ===
    codebase_search_available: bool,       // DEFAULT: false (requires IPC)
    
    // === Fast Apply (Morph Integration) ===
    fast_apply_available: bool,            // DEFAULT: false (requires IPC)
    
    // === File Operations ===
    max_concurrent_file_reads: usize,      // DEFAULT: 5
    partial_reads_enabled: bool,           // DEFAULT: false (requires IPC)
    
    // === Workflow Tools ===
    todo_list_enabled: bool,               // DEFAULT: false
    image_generation_enabled: bool,        // DEFAULT: false (requires IPC)
    run_slash_command_enabled: bool,       // DEFAULT: false (requires IPC)
    
    // === Task Management ===
    new_task_require_todos: bool,          // DEFAULT: false
}
```

---

### 3. **Cargo Features** (Cargo.toml)

Currently defined features in `Cargo.toml`:

```toml
[features]
default = []
local_models = ["candle-core", "candle-nn", "candle-transformers", "ort"]
jemalloc = ["jemallocator"]
nuclear-tests = []
unix-bins = []  # Platform-specific binaries
```

**Planned Prompt-Specific Features:**

```toml
# Future additions (post-IPC)
experimental-mcp = []        # Enable MCP tool integration
experimental-browser = []     # Enable browser_action tool
experimental-search = []      # Enable codebase_search tool
experimental-fast-apply = []  # Enable edit_file (Morph) tool
```

---

## Feature Gate Reference

### **Pre-IPC Status** (Current)

| Feature | Type | Default | Status | IPC Required |
|---------|------|---------|--------|--------------|
| **Browser Support** |
| `supports_browser` | Tool Context | false | ❌ Disabled | Yes |
| `browser_action` tool | Tool | Gated | ❌ Hidden | Yes |
| **Search & Intelligence** |
| `codebase_search_available` | Tool Context | false | ❌ Disabled | Yes |
| `codebase_search` tool | Tool | Gated | ❌ Hidden | Yes |
| **Fast Apply** |
| `fast_apply_available` | Tool Context | false | ❌ Disabled | Yes |
| `edit_file` tool | Tool | Not Impl | ❌ Future | Yes |
| **File Operations** |
| `max_concurrent_file_reads` | Setting | 5 | ✅ Enabled | No |
| `partial_reads_enabled` | Tool Context | false | ❌ Disabled | Yes |
| **Workflow** |
| `todo_list_enabled` | Setting | false | ❌ Disabled | No |
| `update_todo_list` tool | Tool | Gated | ❌ Hidden | No |
| `image_generation_enabled` | Tool Context | false | ❌ Disabled | Yes |
| `generate_image` tool | Tool | Not Impl | ❌ Future | Yes |
| `run_slash_command_enabled` | Tool Context | false | ❌ Disabled | Yes |
| `run_slash_command` tool | Tool | Not Impl | ❌ Future | Yes |
| **Task Management** |
| `new_task_require_todos` | Setting | false | ✅ Optional | No |
| **MCP Integration** |
| MCP tools | Tool Group | Not Impl | ❌ Future | Yes |
| `use_mcp_tool` | Tool | Not Impl | ❌ Future | Yes |
| `access_mcp_resource` | Tool | Not Impl | ❌ Future | Yes |
| **Custom Instructions** |
| `use_agent_rules` | Setting | true | ✅ Enabled | No |

---

## Usage Examples

### Example 1: Enable TODO List Tool

```rust
let mut settings = SystemPromptSettings::default();
settings.todo_list_enabled = true;

let builder = PromptBuilder::new(workspace, mode, settings, None);
let prompt = builder.build().await?;

// Prompt now includes update_todo_list tool description
```

### Example 2: Browser Support (Post-IPC)

```rust
// After IPC is wired
let context = ToolDescriptionContext {
    workspace: &workspace,
    supports_browser: true,  // Enable browser
    browser_viewport_size: "1920x1080",
    // ... other fields
};

let tools = get_tool_descriptions_for_mode(&mode, &context);
// tools now includes browser_action description
```

### Example 3: Disable AGENTS.md

```rust
let mut settings = SystemPromptSettings::default();
settings.use_agent_rules = false;  // Disable AGENTS.md loading

let builder = PromptBuilder::new(workspace, mode, settings, None);
let prompt = builder.build().await?;

// AGENTS.md content will not be included
```

---

## Feature Gate Implementation

### In PromptBuilder

```rust
// builder.rs (lines 96-102)
let codebase_search_available = false; // TODO: Wire after IPC
let supports_browser = false; // TODO: Wire after IPC
let has_mcp = false; // TODO: Wire after IPC
let diff_strategy = None; // TODO: Wire after IPC
let fast_apply_available = false; // TODO: Wire after IPC (Morph)
```

### In Tool Registry

```rust
// tools/mod.rs (lines 75-79)
pub fn get_tool_description(tool_name: &str, context: &ToolDescriptionContext) -> Option<String> {
    match tool_name {
        "browser_action" => {
            if context.supports_browser {
                Some(browser_action_description(...))
            } else {
                None  // Tool hidden when gate is off
            }
        }
        "codebase_search" => {
            if context.codebase_search_available {
                Some(codebase_search_description(...))
            } else {
                None
            }
        }
        // ... other tools
    }
}
```

### In Custom Instructions Loader

```rust
// sections/custom_instructions.rs (lines 392-397)
if settings.use_agent_rules {
    let agent_rules = load_agent_rules_file(workspace).await?;
    if !agent_rules.trim().is_empty() {
        rules.push(agent_rules.trim().to_string());
    }
}
```

---

## Testing Feature Gates

### Test: Tool Visibility

```rust
// tests/tools/tests.rs
#[test]
fn test_browser_action_gated() {
    let context_disabled = ToolDescriptionContext {
        supports_browser: false,
        // ...
    };
    
    let desc = get_tool_description("browser_action", &context_disabled);
    assert!(desc.is_none(), "browser_action should be hidden when disabled");
    
    let context_enabled = ToolDescriptionContext {
        supports_browser: true,
        // ...
    };
    
    let desc = get_tool_description("browser_action", &context_enabled);
    assert!(desc.is_some(), "browser_action should be visible when enabled");
}
```

### Test: Settings Impact

```rust
// tests/integration_tests.rs
#[tokio::test]
async fn test_agents_md_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    fs::write(workspace.join("AGENTS.md"), "Should not appear").await.unwrap();
    
    let mut settings = SystemPromptSettings::default();
    settings.use_agent_rules = false;  // Disable
    
    let builder = PromptBuilder::new(workspace, mode, settings, None);
    let prompt = builder.build().await.unwrap();
    
    assert!(!prompt.contains("Should not appear"));
}
```

---

## Post-IPC Rollout Plan

### Phase 1: Core IPC Tools (P0)
- ✅ `execute_command` (already implemented)
- ✅ `read_file`, `write_to_file` (already implemented)
- ✅ `list_files`, `search_files` (already implemented)

### Phase 2: Workflow Tools (P1)
- 🔜 Enable `todo_list_enabled` by default
- 🔜 Enable `partial_reads_enabled`
- 🔜 Wire `max_concurrent_file_reads` from UI settings

### Phase 3: Browser Integration (P1)
- 🔜 Add `experimental-browser` cargo feature
- 🔜 Wire `supports_browser` from IPC
- 🔜 Implement browser launcher in IPC layer

### Phase 4: Search & Code Intelligence (P1)
- 🔜 Add `experimental-search` cargo feature
- 🔜 Wire `codebase_search_available` from IPC
- 🔜 Implement semantic search backend

### Phase 5: Fast Apply (Morph) (P2)
- 🔜 Add `experimental-fast-apply` cargo feature
- 🔜 Implement `edit_file` tool description
- 🔜 Wire to Morph diff engine

### Phase 6: MCP Integration (P2)
- 🔜 Add `experimental-mcp` cargo feature
- 🔜 Implement `use_mcp_tool` description
- 🔜 Implement `access_mcp_resource` description
- 🔜 Wire to MCP server manager

---

## Configuration Best Practices

### 1. **Gradual Rollout**
- Enable one feature at a time
- Test thoroughly before enabling next feature
- Monitor logs for errors

### 2. **User Control**
- Provide UI toggles for all feature gates
- Allow per-workspace overrides
- Respect user preferences

### 3. **Default Safety**
- All new features default to OFF
- Require explicit opt-in
- Document breaking changes

### 4. **Observability**
- Log feature gate states on prompt build
- Track feature usage metrics
- Monitor error rates per feature

---

## Environment Variable Overrides

Future support for environment variables:

```bash
# Enable experimental features (post-IPC)
LAPCE_AI_ENABLE_BROWSER=1
LAPCE_AI_ENABLE_CODEBASE_SEARCH=1
LAPCE_AI_ENABLE_MCP=1
LAPCE_AI_MAX_FILE_READS=100
```

---

## Security Considerations

### Browser Support
- **Risk:** High (arbitrary web navigation)
- **Mitigation:** Sandboxed browser, URL allowlist, viewport size limits
- **Gated:** Yes, disabled by default

### MCP Integration
- **Risk:** Medium (external tool execution)
- **Mitigation:** Tool allowlist, rate limiting, audit logging
- **Gated:** Yes, experimental feature flag

### Codebase Search
- **Risk:** Low (read-only operations)
- **Mitigation:** Workspace boundary enforcement
- **Gated:** Yes, IPC-dependent

### File Operations
- **Risk:** Low (workspace-bound)
- **Mitigation:** MAX_CONCURRENT_FILE_READS limit, size limits
- **Gated:** Configurable limit (default: 5)

---

## Monitoring & Metrics

### Recommended Metrics (Post-IPC)

```rust
// Track feature usage
tracing::info!(
    feature = "browser_action",
    enabled = context.supports_browser,
    "Feature gate check"
);

// Track tool invocations
tracing::info!(
    tool = "codebase_search",
    gated = true,
    available = context.codebase_search_available,
    "Tool availability check"
);
```

---

## Summary

### Current Status (Pre-IPC)

**Enabled by Default:**
- ✅ Core file operations (read, write, list, search)
- ✅ AGENTS.md loading (`use_agent_rules: true`)
- ✅ max_concurrent_file_reads (limit: 5)

**Disabled by Default:**
- ❌ Browser support
- ❌ Codebase search
- ❌ Fast apply (Morph)
- ❌ TODO list tool
- ❌ Image generation
- ❌ Slash commands
- ❌ MCP tools

**Not Yet Implemented:**
- ⏳ MCP integration
- ⏳ Image generation
- ⏳ Slash command execution
- ⏳ edit_file (fast apply)

### Post-IPC Goals

1. Wire all feature gates to IPC configuration
2. Add UI toggles for each feature
3. Implement cargo features for experiments
4. Enable gradual rollout per feature
5. Monitor usage and stability

---

**Status:** All feature gates documented and ready for IPC integration.
