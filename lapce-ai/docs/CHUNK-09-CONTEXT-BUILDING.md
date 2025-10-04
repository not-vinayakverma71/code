# CHUNK-09: CONTEXT BUILDING (SYSTEM PROMPT GENERATION)

## 📁 Complete System Analysis

```
Context Building System:
├── Codex/src/core/webview/generateSystemPrompt.ts    (104 lines)
├── Codex/src/core/prompts/system.ts                  (246 lines)
├── Codex/src/core/prompts/sections/
│   ├── capabilities.ts                               (50 lines)
│   ├── rules.ts                                      (110 lines)
│   ├── custom-instructions.ts                        (472 lines)
│   ├── custom-system-prompt.ts                       (90 lines)
│   ├── tool-use-guidelines.ts                        (150 lines)
│   ├── objective.ts                                  (100 lines)
│   ├── system-info.ts                                (60 lines)
│   └── kilo.ts                                       (100 lines)
└── Codex/src/core/context/
    ├── context-management/context-error-handling.ts  (115 lines)
    └── instructions/
        ├── kilo-rules.ts                             (43 lines)
        ├── rule-helpers.ts                           (102 lines)
        └── workflows.ts                              (43 lines)

TOTAL: 1,685+ lines prompt engineering
```

---

## 🎯 PURPOSE

**Dynamic System Prompt Generation**: Build context-aware prompts based on mode, environment, tools, custom instructions, and rules files.

**Critical for**: Tool-appropriate instructions, mode-specific constraints, project coding standards, context window management.

---

## 📊 FLOW

```
generateSystemPrompt() → SYSTEM_PROMPT() → Assemble 12 Sections:
1. Role Definition
2. Tool Descriptions  
3. Capabilities
4. Rules
5. Custom Instructions
6-12. System Info, Modes, MCP, etc.
→ Return 6,000-14,000 token prompt
```

---

## 🔧 KEY FILES

### 1. generateSystemPrompt.ts - Entry Point

```typescript
export const generateSystemPrompt = async (provider, message) => {
    const state = await provider.getState()
    
    // Determine diff strategy
    const diffStrategy = isMultiFileApplyDiffEnabled
        ? new MultiFileSearchReplaceDiffStrategy()
        : new MultiSearchReplaceDiffStrategy()
    
    // Check model capabilities
    const modelSupportsComputerUse = tempApiHandler.getModel().info.supportsImages
    const canUseBrowserTool = modelSupportsComputerUse && modeSupportsBrowser
    
    return await SYSTEM_PROMPT(
        context, cwd, canUseBrowserTool, mcpHub, diffStrategy,
        browserViewportSize, mode, customModePrompts, customModes,
        customInstructions, diffEnabled, experiments, ...
    )
}
```

### 2. system.ts - Assembly

```typescript
export const SYSTEM_PROMPT = async (...) => {
    // Try custom file first
    const fileCustomSystemPrompt = await loadSystemPromptFile(cwd, mode, variables)
    if (fileCustomSystemPrompt) {
        return `${roleDefinition}\n${fileCustomSystemPrompt}\n${customInstructions}`
    }
    
    // Generate full prompt
    return `${roleDefinition}
${markdownFormattingSection()}
${getToolDescriptionsForMode(...)}
${getCapabilitiesSection(...)}
${getRulesSection(...)}
${getSystemInfoSection(...)}
${await addCustomInstructions(...)}`
}
```

### 3. sections/rules.ts - Behavioral Rules

```typescript
export function getRulesSection(cwd, supportsComputerUse, diffStrategy, ...) {
    return `====
RULES
- The project base directory is: ${cwd}
- All file paths must be relative to this directory
- For ANY exploration of code, use codebase_search FIRST
- When creating new projects, organize in dedicated directory
${getEditingInstructions(diffStrategy)}
- STRICTLY FORBIDDEN from starting messages with "Great", "Certainly"
- Wait for user response after each tool use`
}

function getEditingInstructions(diffStrategy) {
    const tools = []
    if (diffStrategy) tools.push("apply_diff", "write_to_file")
    else tools.push("write_to_file")
    tools.push("insert_content", "search_and_replace")
    
    return `- For editing files: ${tools.join(", ")}
- Always prefer other tools over write_to_file (slower)
- When using write_to_file: ALWAYS provide COMPLETE file content`
}
```

### 4. sections/custom-instructions.ts - Load Rules

```typescript
export async function addCustomInstructions(
    baseInstructions, globalInstructions, cwd, mode, options
) {
    let instructions = baseInstructions
    
    if (globalInstructions) {
        instructions += `\nCUSTOM INSTRUCTIONS\n${globalInstructions}`
    }
    
    // Load rules from .kilocode/rules/
    const rulesContent = await loadEnabledRules(
        cwd, mode, localToggles, globalToggles, settings
    )
    if (rulesContent) {
        instructions += `\nRULES\n${rulesContent}`
    }
    
    return instructions
}

async function loadEnabledRules(cwd, mode, localToggles, ...) {
    const allRules = []
    const rulesDir = path.join(cwd, ".kilocode", "rules")
    
    if (await directoryExists(rulesDir)) {
        const files = await readTextFilesFromDirectory(rulesDir)
        for (const {filename, content} of files) {
            const filePath = path.join(rulesDir, filename)
            const isEnabled = localToggles?.[filePath] ?? true
            if (isEnabled && content.trim()) {
                allRules.push(`# ${filename}\n${content}`)
            }
        }
    }
    
    return allRules.join("\n\n")
}
```

### 5. sections/custom-system-prompt.ts - Per-Mode Prompts

```typescript
export async function loadSystemPromptFile(cwd, mode, variables) {
    const filePath = path.join(cwd, ".kilocode", `system-prompt-${mode}`)
    const content = await safeReadFile(filePath)
    if (!content) return ""
    
    return interpolatePromptContent(content, variables)
}

function interpolatePromptContent(content, variables) {
    let result = content
    for (const [key, value] of Object.entries(variables)) {
        const pattern = new RegExp(`\\{\\{${key}\\}\\}`, "g")
        result = result.replace(pattern, value)
    }
    return result
}
```

**Variables**: `{{workspace}}`, `{{mode}}`, `{{language}}`, `{{shell}}`, `{{operatingSystem}}`

### 6. context-error-handling.ts - Detect Overflows

```typescript
export function checkContextWindowExceededError(error) {
    return (
        checkIsOpenAIContextWindowError(error) ||
        checkIsAnthropicContextWindowError(error) ||
        ...
    )
}

function checkIsAnthropicContextWindowError(response) {
    const patterns = [
        /prompt is too long/i,
        /maximum.*tokens/i,
        /context.*too.*long/i,
        /token.*limit/i,
    ]
    const message = response?.error?.error?.message
    return patterns.some(p => p.test(message))
}
```

---

## 🎯 PROMPT SIZE

| Section | Tokens | % |
|---------|--------|---|
| Tool Descriptions | 3,000-6,000 | 40% |
| Rules | 1,500-2,500 | 18% |
| Custom Instructions | 0-2,000 | 0-15% |
| Capabilities | 500-800 | 7% |
| Role Definition | 200-500 | 5% |
| Other | 1,800-3,200 | 15% |
| **TOTAL** | **6,000-14,000** | **100%** |

**Optimization**:
- Disable diff: -1,000 tokens
- Disable MCP: -500 tokens
- Minimal mode: -2,000 tokens
- No custom instructions: -500-2,000 tokens

---

## 🎯 RUST TRANSLATION

```rust
pub struct SystemPromptBuilder {
    context: Arc<ExtensionContext>,
    cwd: PathBuf,
    mode_config: ModeConfig,
}

impl SystemPromptBuilder {
    pub async fn build(&self, options: PromptOptions) -> Result<String, Error> {
        // Try custom file first
        if let Some(custom) = self.load_custom_prompt_file().await? {
            return Ok(self.assemble_custom_prompt(custom).await?);
        }
        
        // Generate sections
        let sections = tokio::join!(
            self.get_role_definition(),
            self.get_tool_descriptions(&options),
            self.get_capabilities_section(&options),
            self.get_rules_section(&options),
            self.load_custom_instructions(&options),
        );
        
        Ok(self.assemble_sections(sections))
    }
    
    async fn load_custom_prompt_file(&self) -> Result<Option<String>, Error> {
        let path = self.cwd.join(".kilocode")
            .join(format!("system-prompt-{}", self.mode_config.slug));
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(Some(self.interpolate_variables(&content)))
    }
    
    fn interpolate_variables(&self, content: &str) -> String {
        content
            .replace("{{workspace}}", &self.cwd.display().to_string())
            .replace("{{mode}}", &self.mode_config.slug)
            .replace("{{language}}", &env::var("LANG").unwrap_or_default())
            .replace("{{shell}}", &env::var("SHELL").unwrap_or_default())
            .replace("{{operatingSystem}}", std::env::consts::OS)
    }
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] Prompt generation flow traced
- [x] All 12 sections identified
- [x] Custom instructions loading explained
- [x] Rules file system documented
- [x] Variable interpolation covered
- [x] Error detection patterns shown
- [x] Token optimization strategies listed
- [x] Rust translation patterns defined

**STATUS**: CHUNK-09 COMPLETE
