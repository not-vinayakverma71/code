# Prompt System

**Status:** Production-Ready (Pre-IPC)  
**Codex Parity:** 100% for pre-IPC features  
**Test Coverage:** 145+ comprehensive tests

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Assembly Flow](#assembly-flow)
4. [Modules](#modules)
5. [Feature Toggles](#feature-toggles)
6. [Testing](#testing)
7. [Performance](#performance)
8. [Usage Examples](#usage-examples)
9. [Codex Translation](#codex-translation)

---

## Overview

The prompt system generates context-aware system prompts for the Lapce AI engine. It assembles structured prompts from multiple sections, tool descriptions, and custom instructions, maintaining 1:1 parity with the Codex TypeScript implementation.

### Key Features

- **Mode-Based Prompts:** 5 specialized modes (code, architect, ask, debug, orchestrator)
- **Tool Descriptions:** 15 core tools with feature-gated availability
- **Custom Instructions:** Layered loading from `.kilocode/rules/`, `AGENTS.md`, etc.
- **Security:** Workspace boundary enforcement, symlink cycle prevention, binary file detection
- **Performance:** <10ms prompt builds (5x faster than 50ms target)
- **Observability:** Structured logging with duration, token count, metrics

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      PromptBuilder                          │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Modes      │  │   Settings   │  │  Workspace   │    │
│  │ Config       │  │              │  │    Path      │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Assembly Process                         │
│                                                             │
│  1. Load custom system prompt file (if exists)             │
│  2. Generate structured prompt from sections:              │
│     ├─ Markdown Formatting                                 │
│     ├─ Tool Use (shared)                                   │
│     ├─ Tool Descriptions (mode-specific, feature-gated)    │
│     ├─ Tool Use Guidelines                                 │
│     ├─ Capabilities (diff strategy, workspace path)        │
│     ├─ Objective (mode role definition)                    │
│     ├─ System Info (OS, shell, cwd)                        │
│     └─ Custom Instructions (layered)                       │
│  3. Apply retry & fallback strategies                      │
│  4. Log observability metrics                              │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Final Prompt                             │
│                                                             │
│  - 19K-27K chars (code/architect/debug)                    │
│  - 16K-22K chars (ask)                                     │
│  - 15K-19K chars (orchestrator)                            │
│  - ~4.8K-6.8K tokens estimated                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Assembly Flow

### Complete Build Sequence

```rust
// 1. Initialize Builder
let builder = PromptBuilder::new(workspace, mode, settings, custom_instructions);

// 2. Build Prompt
let prompt = builder.build().await?;

// Steps inside build():
// 2.1. Start timing for observability
// 2.2. Load custom system prompt file (.kilocode/system-prompts/{mode}.md)
// 2.3. If custom file exists:
//      - Use file as base
//      - Add custom instructions only
//      - Return early
// 2.4. Otherwise, generate structured prompt:
//      a. Get mode role definition & base instructions
//      b. Generate tool descriptions (feature-gated)
//      c. Assemble all sections in order
//      d. Load custom instructions (layered)
//      e. Join sections with double newlines
// 2.5. Log metrics (duration, tokens, char count)
// 2.6. Return prompt
```

### Section Order (Matches Codex)

1. **Role Definition** - Mode-specific role (e.g., "You are Kilo Code, a software engineer...")
2. **Markdown Formatting** - Link formatting rules
3. **Tool Use** - Shared tool usage instructions
4. **Tool Descriptions** - Mode-specific, feature-gated tool list
5. **Tool Use Guidelines** - Best practices, trash-put warning
6. **Capabilities** - Problem-solving, editing, diff strategies
7. **Objective** - Mode-specific objective
8. **System Info** - OS, shell, workspace path
9. **Custom Instructions** - Layered rules (if any)

---

## Modules

### Core Modules

```
src/core/prompt/
├── mod.rs                  - Module exports
├── builder.rs              - PromptBuilder (orchestration)
├── modes.rs                - Mode definitions & tool groups
├── settings.rs             - SystemPromptSettings
├── tokenizer.rs            - Token estimation
├── errors.rs               - Error types
│
├── sections/               - Prompt sections
│   ├── mod.rs
│   ├── markdown_formatting.rs
│   ├── tool_use.rs
│   ├── tool_use_guidelines.rs
│   ├── capabilities.rs
│   ├── objective.rs
│   ├── system_info.rs
│   ├── custom_instructions.rs  - Loader (rules, AGENTS.md)
│   └── custom_system_prompt.rs - File-based prompt loader
│
├── tools/                  - Tool descriptions
│   ├── mod.rs              - Coordinator
│   ├── descriptions.rs     - 15 tool description generators
│   ├── registry.rs         - Tool groups & filtering
│   └── tests.rs            - 30+ registry tests
│
└── tests/                  - Integration tests
    ├── mod.rs
    ├── loader_tests.rs     - 30+ loader tests
    ├── section_snapshot_tests.rs  - 40+ snapshot tests
    └── integration_tests.rs       - 30+ integration tests
```

### Module Responsibilities

| Module | Responsibility | Codex Reference |
|--------|----------------|-----------------|
| `builder.rs` | Orchestrate prompt assembly, retry logic | `system.ts` |
| `modes.rs` | Mode configs, tool groups, role definitions | `modes.ts` |
| `settings.rs` | Configuration settings | Implicit from Codex usage |
| `sections/` | Generate individual prompt sections | `sections/*.ts` |
| `tools/` | Tool descriptions & registry | `tools/*.ts` descriptions |
| `tests/` | Comprehensive test suites | Various |

---

## Feature Toggles

### Settings-Based Toggles

```rust
pub struct SystemPromptSettings {
    pub max_concurrent_file_reads: u32,      // Default: 5
    pub todo_list_enabled: bool,             // Default: false
    pub use_agent_rules: bool,               // Default: true
    pub new_task_require_todos: bool,        // Default: false
    pub browser_viewport_size: Option<String>, // Default: None
}
```

### Tool Context Toggles

```rust
pub struct ToolDescriptionContext<'a> {
    supports_browser: bool,                  // DEFAULT: false (IPC)
    codebase_search_available: bool,         // DEFAULT: false (IPC)
    fast_apply_available: bool,              // DEFAULT: false (IPC)
    partial_reads_enabled: bool,             // DEFAULT: false (IPC)
    todo_list_enabled: bool,                 // DEFAULT: false
    image_generation_enabled: bool,          // DEFAULT: false (IPC)
    run_slash_command_enabled: bool,         // DEFAULT: false (IPC)
    // ... other configuration
}
```

### Feature-Gated Tools

| Tool | Gate | Default | IPC Required |
|------|------|---------|--------------|
| browser_action | supports_browser | OFF | Yes |
| codebase_search | codebase_search_available | OFF | Yes |
| edit_file (future) | fast_apply_available | OFF | Yes |
| update_todo_list | todo_list_enabled | OFF | No |
| generate_image (future) | image_generation_enabled | OFF | Yes |
| run_slash_command (future) | run_slash_command_enabled | OFF | Yes |

See `PROMPT_FEATURE_GATES.md` for complete documentation.

---

## Testing

### Test Coverage: 145+ Tests

| Test Suite | Tests | File | Purpose |
|------------|-------|------|---------|
| **Loader Tests (P6)** | 30+ | `tests/loader_tests.rs` | Symlinks, encodings, binaries, ordering |
| **Snapshot Tests (P8)** | 40+ | `tests/section_snapshot_tests.rs` | Exact Codex string matching |
| **Integration Tests (P14)** | 30+ | `tests/integration_tests.rs` | End-to-end prompt builds |
| **Registry Tests (P11)** | 30+ | `tools/tests.rs` | Mode filtering, feature gates |
| **Module Tests** | 15+ | Various `mod.rs` | Unit tests for individual modules |

### Running Tests

```bash
# All prompt tests
cargo test --lib core::prompt

# Specific test suite
cargo test --lib core::prompt::tests::loader_tests
cargo test --lib core::prompt::tests::section_snapshot_tests
cargo test --lib core::prompt::tests::integration_tests

# Tool registry tests
cargo test --lib core::prompt::tools::tests
```

### Test Philosophy

- **No Mocks:** All tests use real file systems (tempdir), real loaders, real builders
- **Production-Grade:** Tests mirror actual usage scenarios
- **Comprehensive:** Cover happy paths, edge cases, error recovery
- **Deterministic:** Reproducible results, alphabetically sorted outputs
- **Fast:** Full suite runs in <5 seconds

---

## Performance

### Benchmarks

```bash
# Run benchmarks
cargo bench --bench prompt_benchmarks

# Individual benchmark groups
cargo bench --bench prompt_benchmarks -- prompt_build
cargo bench --bench prompt_benchmarks -- custom_instructions
cargo bench --bench prompt_benchmarks -- tool_descriptions
```

### Performance Targets (All Exceeded)

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Prompt build (code mode) | <50ms | ~10ms | ✅ 5x faster |
| Custom instructions load | <10ms | ~3ms | ✅ 3x faster |
| Tool descriptions | <5ms | ~1ms | ✅ 5x faster |

### Actual Results (from demo)

| Mode | Build Time | Tokens | Size |
|------|------------|--------|------|
| code | ~10ms | 6,806 | 27KB |
| architect | ~10ms | 6,865 | 27KB |
| ask | ~8ms | 5,447 | 22KB |
| debug | ~10ms | 6,857 | 27KB |
| orchestrator | ~7ms | 4,804 | 19KB |

---

## Usage Examples

### Example 1: Basic Prompt Build

```rust
use lapce_ai_rust::core::prompt::{
    builder::PromptBuilder,
    modes::get_mode_by_slug,
    settings::SystemPromptSettings,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get mode configuration
    let mode = get_mode_by_slug("code").unwrap();
    
    // Configure settings
    let settings = SystemPromptSettings::default();
    
    // Create builder
    let workspace = PathBuf::from("/path/to/workspace");
    let builder = PromptBuilder::new(workspace, mode, settings, None);
    
    // Build prompt
    let prompt = builder.build().await?;
    
    println!("Prompt: {} chars", prompt.len());
    Ok(())
}
```

### Example 2: With Custom Instructions

```rust
let mode = get_mode_by_slug("architect").unwrap();
let settings = SystemPromptSettings::default();

let builder = PromptBuilder::new(
    workspace,
    mode,
    settings,
    Some("Always design for scalability and maintainability".to_string()),
);

let prompt = builder.build().await?;
// Prompt includes global custom instructions
```

### Example 3: Enable TODO List Tool

```rust
let mode = get_mode_by_slug("code").unwrap();

let mut settings = SystemPromptSettings::default();
settings.todo_list_enabled = true;  // Enable TODO list tool

let builder = PromptBuilder::new(workspace, mode, settings, None);
let prompt = builder.build().await?;
// Prompt now includes update_todo_list tool description
```

### Example 4: With Retry & Token Count

```rust
let builder = PromptBuilder::new(workspace, mode, settings, None);

// Build with automatic retry on errors
let prompt = builder.build_with_retry().await?;

// Get prompt with token count
let (prompt, tokens) = builder.build_and_count().await?;
println!("Tokens: ~{}", tokens);
```

### Example 5: Disable AGENTS.md

```rust
let mut settings = SystemPromptSettings::default();
settings.use_agent_rules = false;  // Disable AGENTS.md loading

let builder = PromptBuilder::new(workspace, mode, settings, None);
let prompt = builder.build().await?;
// AGENTS.md content will not be included
```

---

## Codex Translation

### Translation Methodology

All modules are **1:1 translations** from Codex TypeScript sources:

1. **Line-by-Line Comparison:** Each function matches Codex behavior exactly
2. **Reference Comments:** Every file has Codex source references
3. **Function Mapping:** TypeScript functions → Rust functions with same logic
4. **Snapshot Tests:** Ensure exact string output matches Codex

### Codex Source Map

| Rust Module | Codex Source | Lines | Parity |
|-------------|--------------|-------|--------|
| `builder.rs` | `system.ts` | 1-246 | ✅ 100% |
| `modes.rs` | `modes.ts` | 1-127 | ✅ 100% |
| `sections/markdown_formatting.rs` | `markdown-formatting.ts` | 1-8 | ✅ 100% |
| `sections/tool_use.rs` | `tool-use.ts` | 3-18 | ✅ 100% |
| `sections/tool_use_guidelines.rs` | `tool-use-guidelines.ts` | 3-104 | ✅ 100% |
| `sections/capabilities.rs` | `capabilities.ts` | 1-245 | ✅ 100% |
| `sections/objective.rs` | `objective.ts` | 1-18 | ✅ 100% |
| `sections/system_info.rs` | `system-info.ts` | 1-55 | ✅ 100% |
| `sections/custom_instructions.rs` | `custom-instructions.ts` | 1-472 | ✅ 100% |
| `sections/custom_system_prompt.rs` | `custom-system-prompt.ts` | 1-82 | ✅ 100% |
| `tools/descriptions.rs` | Various `tools/*.ts` | N/A | ✅ 100% (15/15) |

See `PROMPT_PARITY_VALIDATION.md` for comprehensive validation.

---

## Error Handling

### Error Types

```rust
pub enum PromptError {
    IoError(std::io::Error),
    RuleLoadError(std::io::Error),
    TokenCountError(String),
    SymlinkCycle(String),
    OutsideWorkspace(String),
}
```

### Retry Strategy

```rust
builder.build_with_retry().await
// Handles:
// 1. Prompt too large → build_condensed()
// 2. Rule load error → build_without_rules()
// 3. Other errors → propagate
```

### Fallback Strategies

1. **Condensed Build:** Skip custom instructions if prompt exceeds MAX_PROMPT_SIZE (400K chars)
2. **Without Rules:** Skip all rule files if loading fails, use only global instructions
3. **Error Recovery:** Graceful degradation for missing files, invalid symlinks, etc.

---

## Observability

### Structured Logging

All builds log structured metrics:

```rust
tracing::info!(
    mode = "code",
    duration_ms = 10,
    char_count = 27224,
    token_estimate = 6806,
    has_custom_instructions = false,
    "Prompt build completed"
);
```

### Retry Logging

```rust
tracing::warn!(
    mode = "code",
    char_count = 450000,
    max_size = 400000,
    "Prompt too large, attempting condensed build"
);

tracing::info!(
    mode = "code",
    retry_count = 1,
    used_fallback = true,
    total_duration_ms = 15,
    "Prompt build with retry completed"
);
```

---

## Security

### Workspace Boundary Enforcement

All file operations are bounded to the workspace:

```rust
ensure_workspace_path(workspace, file_path)?;
// Prevents path traversal attacks
```

### Symlink Safety

- **Cycle Detection:** MAX_DEPTH=5 prevents infinite loops
- **Broken Symlinks:** Skipped gracefully
- **Outside Workspace:** Rejected with error

### Binary File Detection

- **Encoding Detection:** UTF-8, UTF-8 BOM, ASCII, Binary
- **Binary Skip:** Binary files automatically excluded from rules
- **Image Detection:** PNG/JPG headers detected and skipped

### Cache File Filtering

Automatically excludes:
- `.DS_Store`, `Thumbs.db` (OS cache)
- `.log`, `.bak`, `.tmp` (temporary files)
- `.cache`, `.old`, `.swp` (editor artifacts)

---

## Future Work

### Post-IPC Integration

1. **Wire Feature Gates:** Connect all IPC-dependent features
2. **MCP Tools:** Implement `use_mcp_tool`, `access_mcp_resource`
3. **Fast Apply:** Implement `edit_file` (Morph integration)
4. **Image Generation:** Implement `generate_image` tool
5. **Slash Commands:** Implement `run_slash_command` tool

### Optimizations

1. **Token Counting:** Replace approximation with `tiktoken-rs`
2. **Caching:** Cache tool descriptions per mode
3. **Parallel Loading:** Concurrent file reads for custom instructions
4. **Compression:** Implement smart condensing strategies

### Enhancements

1. **Custom Modes:** User-defined modes with custom tool sets
2. **Dynamic Rules:** Hot-reload rules without rebuild
3. **Analytics:** Track tool usage, prompt sizes, build times
4. **Validation:** Schema validation for custom system prompts

---

## Documentation

### Available Documents

1. **PROMPT_SYSTEM_STATUS.md** - Overall status and tracking
2. **PROMPT_PARITY_VALIDATION.md** - Comprehensive Codex parity audit
3. **PROMPT_TESTING_COMPLETE.md** - Test coverage summary
4. **PROMPT_FEATURE_GATES.md** - Feature flags and gates
5. **This README** - Architecture and usage guide

### Examples

- `examples/prompt_builder_demo.rs` - Demo showing all 5 modes

---

## Contributing

### Adding a New Tool Description

1. Add tool function to `tools/descriptions.rs`:
   ```rust
   pub fn my_tool_description(workspace: &Path) -> String {
       format!(r#"## my_tool
   Description: ...
   "#, workspace.display())
   }
   ```

2. Register in `tools/registry.rs`:
   ```rust
   ("my_tool", ExtendedToolGroup::MyGroup),
   ```

3. Wire in `tools/mod.rs`:
   ```rust
   "my_tool" => Some(my_tool_description(context.workspace)),
   ```

4. Add test in `tools/tests.rs`:
   ```rust
   #[test]
   fn test_my_tool_description() { ... }
   ```

### Adding a New Section

1. Create `sections/my_section.rs`:
   ```rust
   pub fn my_section() -> String {
       "====\n\nMY SECTION\n\nContent...".to_string()
   }
   ```

2. Export in `sections/mod.rs`:
   ```rust
   pub use my_section::my_section;
   ```

3. Add to builder in `builder.rs`:
   ```rust
   sections.push(my_section());
   ```

4. Add snapshot test in `tests/section_snapshot_tests.rs`

---

## FAQ

### Q: How do I enable browser support?

**A:** Currently disabled pre-IPC. Post-IPC, set `supports_browser: true` in `ToolDescriptionContext`.

### Q: Why is my custom rule file not loading?

**A:** Check:
1. File is in `.kilocode/rules/` directory
2. File is not binary (check encoding)
3. File is not a cache file (.log, .bak, etc.)
4. `use_agent_rules` is true in settings (for AGENTS.md)

### Q: How do I disable AGENTS.md?

**A:** Set `settings.use_agent_rules = false`.

### Q: Why is the prompt so large?

**A:** Code/architect/debug modes include many tools. Use `orchestrator` mode for minimal prompts, or implement condensing strategy.

### Q: How do I add mode-specific rules?

**A:** Create `.kilocode/rules-{mode}/` directory (e.g., `.kilocode/rules-code/`) and add rule files there.

### Q: What's the token limit?

**A:** No hard limit, but prompts >400K chars trigger condensed build. Most prompts are 16K-27K chars (~4K-7K tokens).

---

## Changelog

### v1.0.0 (2025-10-17) - Production Ready ✅

- ✅ All 9 sections implemented with Codex parity
- ✅ 15/15 core tool descriptions
- ✅ 5 modes (code, architect, ask, debug, orchestrator)
- ✅ Custom instructions loader (rules, AGENTS.md, legacy files)
- ✅ 145+ comprehensive tests
- ✅ Performance benchmarks (5x faster than target)
- ✅ Observability (structured logging)
- ✅ Feature gates documented
- ✅ Security hardened (symlinks, binaries, workspace boundaries)

---

**Status:** Production-ready for IPC integration. All pre-IPC features complete and validated.
