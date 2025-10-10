## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED: 1:1 TYPESCRIPT TO RUST PORT ONLY from `/home/verma/lapce/Codex` to `/home/verma/lapce/lapce-ai`

**THIS IS NOT A REWRITE - IT'S A TRANSLATION OF TYPESCRIPT (VS CODE EXTENSION) TO LAPCE IDE ( NATIVE INTEGRATION WITH SHARED MEMORY IPC LAYER) WITH EXACT AI CORE LOGIC**

# CHUNK 01: src/core/prompts/ - 60 FILES DEEP ANALYSIS

## Critical Discovery: This is the HEART of the entire system

### File Inventory
```
prompts/
├── system.ts (246 lines) - SYSTEM PROMPT BUILDER
├── responses.ts - Response formatting
├── commands.ts - Command handling
├── types.ts - Type definitions
├── sections/ (10 files)
│   ├── rules.ts (110 lines) - CRITICAL RULES
│   ├── capabilities.ts - Model capabilities
│   ├── objective.ts - Task objective
│   ├── system-info.ts - System information
│   ├── tool-use.ts - Tool usage instructions
│   ├── tool-use-guidelines.ts - Tool guidelines
│   ├── mcp-servers.ts - MCP integration
│   ├── modes.ts - Mode system
│   ├── custom-instructions.ts - User instructions
│   └── markdown-formatting.ts - Output formatting
├── tools/ (20 files) - TOOL DESCRIPTIONS
│   ├── read-file.ts (86 lines)
│   ├── write-to-file.ts (41 lines)
│   ├── execute-command.ts (26 lines)
│   ├── edit-file.ts (Morph editing)
│   ├── apply-diff.ts
│   ├── search-files.ts
│   ├── codebase-search.ts
│   ├── list-files.ts
│   ├── browser-action.ts
│   ├── use-mcp-tool.ts
│   ├── ask-followup-question.ts
│   ├── attempt-completion.ts
│   ├── insert-content.ts
│   ├── search-and-replace.ts
│   ├── update-todo-list.ts
│   ├── switch-mode.ts
│   ├── new-task.ts
│   ├── list-code-definition-names.ts
│   ├── access-mcp-resource.ts
│   └── fetch-instructions.ts
├── instructions/ (3 files)
│   ├── create-mcp-server.ts
│   ├── create-mode.ts
│   └── instructions.ts
└── utilities/ (2 files)
    └── mermaid.ts
```

## CRITICAL FINDINGS

### 1. Dynamic Prompt Assembly
The system builds prompts dynamically by concatenating sections:

```typescript
const basePrompt = `${roleDefinition}
${markdownFormattingSection()}
${getSharedToolUseSection()}
${getToolDescriptionsForMode(...)}  // 40+ tools
${getToolUseGuidelinesSection(...)}
${mcpServersSection}
${getCapabilitiesSection(...)}
${modesSection}
${getRulesSection(...)}
${getSystemInfoSection(...)}
${getObjectiveSection(...)}
${addCustomInstructions(...)}`
```

**RUST REQUIREMENT:** Must preserve EXACT character-for-character strings. No whitespace changes allowed.

### 2. Tool Descriptions - 20 Individual Files

Each tool has a dedicated file with:
- Description text (must be exact)
- Parameter definitions
- XML usage examples
- Important notes and warnings

**Example: read_file.ts**
```typescript
return `## read_file
Description: Request to read the contents of ${isMultipleReadsEnabled ? "one or more files" : "a file"}...
Parameters:
- args: Contains one or more file elements...
Usage:
<read_file>
<args>
  <file>
    <path>path/to/file</path>
  </file>
</args>
</read_file>`
```

**RUST TRANSLATION:**
```rust
pub fn get_read_file_description(args: &ToolArgs) -> String {
    let max_concurrent = args.settings
        .as_ref()
        .and_then(|s| s.max_concurrent_file_reads)
        .unwrap_or(5);
    let is_multiple_enabled = max_concurrent > 1;
    
    format!(r#"## read_file
Description: Request to read the contents of {}. The tool outputs line-numbered content...
"#, if is_multiple_enabled { "one or more files" } else { "a file" })
}
```

### 3. Rules Section - 110 Lines of Critical Instructions

**Key Rules (must preserve exactly all):**
- Project base directory: `${cwd.toPosix()}`
- Cannot `cd` into different directories
- Must use relative paths
- No `~` or `$HOME` references
- System-aware command execution
- Codebase search MUST be used first
- File editing tool selection logic
- Mode restrictions on file editing
- No conversational responses ("Great", "Certainly", "Okay" forbidden)
- Never end with questions
- Direct and technical communication only

**CRITICAL:** These rules shape the AI's behavior. Any change breaks the UX contract.

### 4. Mode System Integration

Modes control which tools are available:
- `architect` mode: Only edit `\.md$` files
- `code` mode: Full file access
- `ask` mode: No file editing
- Custom modes: User-defined restrictions

**RUST REQUIREMENT:** Implement regex-based file pattern matching for mode restrictions.

### 5. VS Code Dependencies

**CRITICAL DEPENDENCIES TO REPLACE:**
```typescript
// VS Code APIs used in prompts system:
vscode.ExtensionContext  → Lapce context 
vscode.env.language      → System locale detection
context.workspaceState   → Lapce workspace state      // ALl communication through IPC 
context.globalState      → Lapce global state
path.toPosix()          → Path normalization
```

**LAPCE EQUIVALENT:**
```rust
use lapce_plugin_api::{Context, WorkspaceState, GlobalState};

// Lapce context
let context: Context;
let language = std::env::var("LANG").unwrap_or("en".to_string());
let workspace_state = context.get_workspace_state();
let global_state = context.get_global_state();

// Path normalization (cross-platform)
fn to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', '/')
}
```

### 6. Conditional Logic in Prompts

**Features enabled/disabled based on:**
- `partialReadsEnabled` - Line range support
- `maxConcurrentFileReads` - Batch reading (1-5 files)
- `diffEnabled` - apply_diff tool availability
- `supportsComputerUse` - Browser automation
- `experiments` - Feature flags
- `codeIndexManager.isInitialized` - Semantic search
- `mcpHub.getServers().length > 0` - MCP tools

**RUST:** Must implement feature flag system identical to TypeScript.

### 7. MCP (Model Context Protocol) Integration

When MCP servers are active:
```typescript
const shouldIncludeMcp = hasMcpGroup && hasMcpServers
const mcpServersSection = await getMcpServersSection(mcpHub, ...)
```

Adds dynamic tool descriptions from external servers.

**RUST CHALLENGE:** MCP integration requires async tool discovery and dynamic prompt injection.

## Translation Requirements

### Phase 1: Extract All Prompt Strings
**Task:** Read all 60 files and extract every string literal into a Rust constants file.

### Phase 2: Implement Prompt Builder
```rust
pub struct SystemPromptBuilder {
    context: Arc<LapceContext>,
    cwd: PathBuf,
    mode: Mode,
    settings: PromptSettings,
    feature_flags: FeatureFlags,
}

impl SystemPromptBuilder {
    pub async fn build(&self) -> Result<String, Error> {
        let role_definition = self.get_role_definition().await?;
        let markdown_section = markdown_formatting_section();
        let tool_use = get_shared_tool_use_section();
        let tools = self.get_tool_descriptions().await?;
        let guidelines = get_tool_use_guidelines(self.code_index_manager);
        let mcp = self.get_mcp_section().await?;
        let capabilities = self.get_capabilities_section()?;
        let modes = get_modes_section()?;
        let rules = self.get_rules_section()?;
        let system_info = get_system_info_section(&self.cwd)?;
        let objective = get_objective_section(self.code_index_manager)?;
        let custom = self.add_custom_instructions().await?;
        
        Ok(format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
            role_definition,
            markdown_section,
            tool_use,
            tools,
            guidelines,
            mcp,
            capabilities,
            modes,
            rules,
            system_info,
            objective,
            custom
        ))
    }
}
```

### Phase 3: Tool Description Registry
```rust
pub struct ToolDescriptionRegistry {
    tools: HashMap<String, Box<dyn ToolDescriptionProvider>>,
}

trait ToolDescriptionProvider: Send + Sync {
    fn get_description(&self, args: &ToolArgs) -> String;
    fn get_name(&self) -> &str;
}

// Each tool gets its own struct
pub struct ReadFileToolDescription;
impl ToolDescriptionProvider for ReadFileToolDescription {
    fn get_description(&self, args: &ToolArgs) -> String {
        // Exact string from TypeScript
        include_str!("../descriptions/read_file.txt")
            // Dynamic replacements
            .replace("${CWD}", &args.cwd.to_string_lossy())
            .replace("${MAX_CONCURRENT}", &args.max_concurrent.to_string())
    }
}
```

## Next Chunk: src/core/tools/ (43 files)
This will analyze the actual tool EXECUTION logic (not just descriptions).

## Acceptance Tests

```rust
#[test]
fn test_prompt_exact_match() {
    let ts_prompt = include_str!("golden/typescript_prompt.txt");
    let rust_prompt = build_system_prompt(...);
    assert_eq!(rust_prompt, ts_prompt);
}

#[test]
fn test_tool_description_read_file() {
    let args = ToolArgs { cwd: PathBuf::from("/workspace"), ... };
    let desc = get_read_file_description(&args);
    let golden = include_str!("golden/read_file_description.txt");
    assert_eq!(desc, golden);
}
```
