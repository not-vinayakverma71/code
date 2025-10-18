# DEEP ANALYSIS 07: KILOCODE FEATURES - MODES, RULES, WORKFLOWS

## 📁 Analyzed Files

```
Codex/
├── packages/types/src/
│   └── mode.ts                       (212 lines, mode system)
│       ├── ModeConfig (5 default modes)
│       ├── GroupEntry (tool groups)
│       ├── GroupOptions (file restrictions)
│       └── DEFAULT_MODES array
│           ├── Architect (plan/design)
│           ├── Code (write/refactor)
│           ├── Ask (Q&A)
│           ├── Debug (troubleshoot)
│           └── Orchestrator (coordinate)
│
├── src/shared/modes.ts               (384 lines, mode logic)
│   ├── getAllModes()                 (merge custom + built-in)
│   ├── getModeSelection()            (prompt generation)
│   ├── isToolAllowedForMode()        (permission validation)
│   └── Tool group configuration
│
├── webview-ui/src/components/kilocode/
│   ├── KiloModeSelector.tsx          (84 lines, UI dropdown)
│   ├── rules/
│   │   ├── RulesWorkflowsSection.tsx (42 lines, rules/workflows UI)
│   │   ├── RulesToggleList.tsx       (25 lines, list display)
│   │   ├── RuleRow.tsx               (81 lines, toggle/edit/delete)
│   │   └── NewRuleRow.tsx            (create new rule)
│   └── helpers.ts
│
└── Storage Structure
    ├── .roo/rules/                   (Global rules)
    ├── .roo/workflows/               (Global workflows)
    ├── .roo-local/rules/             (Workspace rules)
    └── .roo-local/workflows/         (Workspace workflows)

Total: 38 Kilocode components → Rust mode system + rule injection
```

---

## Overview
Kilocode extends the base Roo/Cline functionality with **custom modes**, **rules**, **workflows**, and **enhanced UI**. These features allow specialized AI behavior and project-specific automation.

---

## 1. Modes System

### Mode Architecture

```
Mode = Specialized AI Persona with:
├── Role Definition ────► AI's identity and expertise
├── Custom Instructions ► Behavior and constraints
├── Tool Groups ────────► Available tools (read, edit, command, browser, mcp)
├── File Restrictions ──► Regex patterns limiting file access
└── Icon/Description ───► UI representation
```

### ModeConfig Type

```typescript
interface ModeConfig {
    slug: string                    // Unique identifier: "architect", "code", "ask"
    name: string                    // Display name: "Architect", "Code", "Ask"
    iconName?: string               // VSCode codicon: "codicon-type-hierarchy-sub"
    roleDefinition: string          // AI persona prompt
    whenToUse?: string              // Guidance for when to use this mode
    description?: string            // Short description
    customInstructions?: string     // Additional behavior instructions
    groups: GroupEntry[]            // Tool groups with optional restrictions
    source?: "global" | "project"   // Origin of custom mode
}

type GroupEntry = 
    | ToolGroup                     // Simple: "read"
    | [ToolGroup, GroupOptions]     // With restrictions: ["edit", { fileRegex: "\\.md$" }]

interface GroupOptions {
    fileRegex?: string              // File path regex restriction
    description?: string            // Human-readable restriction description
}

type ToolGroup = 
    | "read"                        // File reading tools
    | "edit"                        // File editing tools
    | "command"                     // Terminal execution
    | "browser"                     // Web browsing
    | "mcp"                         // MCP tool access
```

**Rust Translation:**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub icon_name: Option<String>,
    pub role_definition: String,
    pub when_to_use: Option<String>,
    pub description: Option<String>,
    pub custom_instructions: Option<String>,
    pub groups: Vec<GroupEntry>,
    pub source: Option<ModeSource>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum GroupEntry {
    Simple(ToolGroup),
    WithOptions(ToolGroup, GroupOptions),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupOptions {
    #[serde(rename = "fileRegex")]
    pub file_regex: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolGroup {
    Read,
    Edit,
    Command,
    Browser,
    Mcp,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ModeSource {
    Global,
    Project,
}
```

---

## 2. Default Modes

### Mode 1: Architect

```typescript
{
    slug: "architect",
    name: "Architect",
    iconName: "codicon-type-hierarchy-sub",
    roleDefinition: 
        "You are Kilo Code, an experienced technical leader who is inquisitive and an excellent planner. Your goal is to gather information and get context to create a detailed plan for accomplishing the user's task, which the user will review and approve before they switch into another mode to implement the solution.",
    whenToUse:
        "Use this mode when you need to plan, design, or strategize before implementation. Perfect for breaking down complex problems, creating technical specifications, designing system architecture, or brainstorming solutions before coding.",
    description: "Plan and design before implementation",
    groups: [
        "read",
        ["edit", { fileRegex: "\\.md$", description: "Markdown files only" }],
        "browser",
        "mcp"
    ],
    customInstructions:
        "1. Do some information gathering (using provided tools) to get more context about the task.\n
        2. You should also ask the user clarifying questions to get a better understanding of the task.\n
        3. Once you've gained more context about the user's request, break down the task into clear, actionable steps and create a todo list using the `update_todo_list` tool.\n
        4. As you gather more information or discover new requirements, update the todo list to reflect the current understanding of what needs to be accomplished.\n
        5. Ask the user if they are pleased with this plan, or if they would like to make any changes.\n
        6. Include Mermaid diagrams if they help clarify complex workflows or system architecture.\n
        7. Use the switch_mode tool to request that the user switch to another mode to implement the solution."
}
```

**Tool Restrictions:**
- ✅ Can read any file
- ⚠️ Can only edit `.md` files (markdown documents)
- ✅ Can browse web
- ✅ Can use MCP tools
- ❌ Cannot execute commands

### Mode 2: Code

```typescript
{
    slug: "code",
    name: "Code",
    iconName: "codicon-code",
    roleDefinition:
        "You are Kilo Code, a highly skilled software engineer with extensive knowledge in many programming languages, frameworks, design patterns, and best practices.",
    whenToUse:
        "Use this mode when you need to write, modify, or refactor code. Ideal for implementing features, fixing bugs, creating new files, or making code improvements across any programming language or framework.",
    description: "Write, modify, and refactor code",
    groups: ["read", "edit", "browser", "command", "mcp"]
}
```

**Tool Restrictions:**
- ✅ Can read any file
- ✅ Can edit any file (no restrictions)
- ✅ Can browse web
- ✅ Can execute commands
- ✅ Can use MCP tools

### Mode 3: Ask

```typescript
{
    slug: "ask",
    name: "Ask",
    iconName: "codicon-question",
    roleDefinition:
        "You are Kilo Code, a knowledgeable technical assistant focused on answering questions and providing information about software development, technology, and related topics.",
    whenToUse:
        "Use this mode when you need explanations, documentation, or answers to technical questions. Best for understanding concepts, analyzing existing code, getting recommendations, or learning about technologies without making changes.",
    description: "Get answers and explanations",
    groups: ["read", "browser", "mcp"],
    customInstructions:
        "You can analyze code, explain concepts, and access external resources. Always answer the user's questions thoroughly, and do not switch to implementing code unless explicitly requested by the user. Include Mermaid diagrams when they clarify your response."
}
```

**Tool Restrictions:**
- ✅ Can read any file
- ❌ Cannot edit files
- ✅ Can browse web
- ❌ Cannot execute commands
- ✅ Can use MCP tools

### Mode 4: Debug

```typescript
{
    slug: "debug",
    name: "Debug",
    iconName: "codicon-bug",
    roleDefinition:
        "You are Kilo Code, an expert software debugger specializing in systematic problem diagnosis and resolution.",
    whenToUse:
        "Use this mode when you're troubleshooting issues, investigating errors, or diagnosing problems. Specialized in systematic debugging, adding logging, analyzing stack traces, and identifying root causes before applying fixes.",
    description: "Diagnose and fix software issues",
    groups: ["read", "edit", "browser", "command", "mcp"],
    customInstructions:
        "Reflect on 5-7 different possible sources of the problem, distill those down to 1-2 most likely sources, and then add logs to validate your assumptions. Explicitly ask the user to confirm the diagnosis before fixing the problem."
}
```

### Mode 5: Orchestrator

```typescript
{
    slug: "orchestrator",
    name: "Orchestrator",
    iconName: "codicon-run-all",
    roleDefinition:
        "You are Kilo Code, a strategic workflow orchestrator who coordinates complex tasks by delegating them to appropriate specialized modes. You have a comprehensive understanding of each mode's capabilities and limitations, allowing you to effectively break down complex problems into discrete tasks that can be solved by different specialists.",
    whenToUse:
        "Use this mode for complex, multi-step projects that require coordination across different specialties. Ideal when you need to break down large tasks into subtasks, manage workflows, or coordinate work that spans multiple domains or expertise areas.",
    description: "Coordinate tasks across multiple modes",
    groups: [],  // No tools - orchestrates only
    customInstructions:
        "Your role is to coordinate complex workflows by delegating tasks to specialized modes. As an orchestrator, you should:\n
        1. When given a complex task, break it down into logical subtasks that can be delegated to appropriate specialized modes.\n
        2. For each subtask, use the `new_task` tool to delegate. Choose the most appropriate mode for the subtask's specific goal.\n
        3. Track and manage the progress of all subtasks. When a subtask is completed, analyze its results and determine the next steps.\n
        4. Help the user understand how the different subtasks fit together in the overall workflow.\n
        5. When all subtasks are completed, synthesize the results and provide a comprehensive overview of what was accomplished."
}
```

---

## 3. Mode Selection Logic

```typescript
// Get all available modes (built-in + custom)
export function getAllModes(customModes?: ModeConfig[]): ModeConfig[] {
    if (!customModes?.length) {
        return [...DEFAULT_MODES]
    }
    
    // Start with built-in modes
    const allModes = [...DEFAULT_MODES]
    
    // Process custom modes (override or add)
    customModes.forEach((customMode) => {
        const index = allModes.findIndex((mode) => mode.slug === customMode.slug)
        if (index !== -1) {
            // Override existing mode
            allModes[index] = customMode
        } else {
            // Add new mode
            allModes.push(customMode)
        }
    })
    
    return allModes
}

// Get mode configuration for AI prompt
export function getModeSelection(
    modeSlug: string,
    promptComponent?: PromptComponent,
    customModes?: ModeConfig[]
) {
    const customMode = customModes?.find(m => m.slug === modeSlug)
    const builtInMode = DEFAULT_MODES.find(m => m.slug === modeSlug)
    
    // Custom mode takes full precedence
    if (customMode) {
        return {
            roleDefinition: customMode.roleDefinition || "",
            baseInstructions: customMode.customInstructions || "",
            description: customMode.description || "",
        }
    }
    
    // Built-in mode with optional prompt component override
    const baseMode = builtInMode || DEFAULT_MODES[0]
    return {
        roleDefinition: promptComponent?.roleDefinition || baseMode.roleDefinition || "",
        baseInstructions: promptComponent?.customInstructions || baseMode.customInstructions || "",
        description: baseMode.description || "",
    }
}
```

**Rust Translation:**

```rust
pub fn get_all_modes(custom_modes: Option<&[ModeConfig]>) -> Vec<ModeConfig> {
    let mut all_modes = DEFAULT_MODES.to_vec();
    
    if let Some(custom) = custom_modes {
        for custom_mode in custom {
            if let Some(index) = all_modes.iter().position(|m| m.slug == custom_mode.slug) {
                // Override existing
                all_modes[index] = custom_mode.clone();
            } else {
                // Add new
                all_modes.push(custom_mode.clone());
            }
        }
    }
    
    all_modes
}

pub fn get_mode_selection(
    mode_slug: &str,
    prompt_component: Option<&PromptComponent>,
    custom_modes: Option<&[ModeConfig]>,
) -> ModeSelection {
    // Check custom modes first
    if let Some(custom) = custom_modes {
        if let Some(mode) = custom.iter().find(|m| m.slug == mode_slug) {
            return ModeSelection {
                role_definition: mode.role_definition.clone(),
                base_instructions: mode.custom_instructions.clone().unwrap_or_default(),
                description: mode.description.clone().unwrap_or_default(),
            };
        }
    }
    
    // Check built-in modes
    let base_mode = DEFAULT_MODES.iter()
        .find(|m| m.slug == mode_slug)
        .unwrap_or(&DEFAULT_MODES[0]);
    
    ModeSelection {
        role_definition: prompt_component
            .and_then(|p| p.role_definition.clone())
            .unwrap_or_else(|| base_mode.role_definition.clone()),
        base_instructions: prompt_component
            .and_then(|p| p.custom_instructions.clone())
            .or_else(|| base_mode.custom_instructions.clone())
            .unwrap_or_default(),
        description: base_mode.description.clone().unwrap_or_default(),
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ModeSelection {
    pub role_definition: String,
    pub base_instructions: String,
    pub description: String,
}

lazy_static! {
    static ref DEFAULT_MODES: Vec<ModeConfig> = vec![
        ModeConfig {
            slug: "architect".to_string(),
            name: "Architect".to_string(),
            icon_name: Some("codicon-type-hierarchy-sub".to_string()),
            role_definition: "You are Kilo Code, an experienced technical leader...".to_string(),
            when_to_use: Some("Use this mode when you need to plan, design...".to_string()),
            description: Some("Plan and design before implementation".to_string()),
            groups: vec![
                GroupEntry::Simple(ToolGroup::Read),
                GroupEntry::WithOptions(
                    ToolGroup::Edit,
                    GroupOptions {
                        file_regex: Some(r"\.md$".to_string()),
                        description: Some("Markdown files only".to_string()),
                    }
                ),
                GroupEntry::Simple(ToolGroup::Browser),
                GroupEntry::Simple(ToolGroup::Mcp),
            ],
            custom_instructions: Some("1. Do some information gathering...".to_string()),
            source: None,
        },
        // ... other 4 modes
    ];
}
```

---

## 4. Tool Permission Validation

```typescript
export function isToolAllowedForMode(
    tool: string,
    modeSlug: string,
    customModes: ModeConfig[],
    toolParams?: Record<string, any>,
): boolean {
    // Always allow core tools
    if (ALWAYS_AVAILABLE_TOOLS.includes(tool)) {
        return true
    }
    
    const mode = getModeBySlug(modeSlug, customModes)
    if (!mode) {
        return false
    }
    
    // Check if tool is in any of the mode's groups
    for (const groupEntry of mode.groups) {
        const groupName = Array.isArray(groupEntry) ? groupEntry[0] : groupEntry
        const groupOptions = Array.isArray(groupEntry) ? groupEntry[1] : undefined
        
        const groupConfig = TOOL_GROUPS[groupName]
        if (!groupConfig.tools.includes(tool)) {
            continue  // Tool not in this group
        }
        
        // Tool is in group, check file restrictions
        if (groupOptions?.fileRegex && toolParams?.path) {
            const regex = new RegExp(groupOptions.fileRegex)
            if (!regex.test(toolParams.path)) {
                throw new FileRestrictionError(
                    modeSlug,
                    groupOptions.fileRegex,
                    groupOptions.description,
                    toolParams.path,
                    tool
                )
            }
        }
        
        return true  // Tool allowed
    }
    
    return false  // Tool not in any group
}

// Example tool groups
const TOOL_GROUPS = {
    read: {
        tools: ["readFile", "listFiles", "searchFiles", "listCodeDefinitionNames"]
    },
    edit: {
        tools: ["writeToFile", "editedExistingFile", "insertCodeBlock", "replaceCodeBlock"]
    },
    command: {
        tools: ["executeCommand"]
    },
    browser: {
        tools: ["browserAction"]
    },
    mcp: {
        tools: ["useMcpTool"]
    }
}
```

**Rust Translation:**

```rust
use regex::Regex;

pub fn is_tool_allowed_for_mode(
    tool: &str,
    mode_slug: &str,
    custom_modes: &[ModeConfig],
    tool_params: Option<&ToolParams>,
) -> Result<bool> {
    // Always allow core tools
    if ALWAYS_AVAILABLE_TOOLS.contains(&tool) {
        return Ok(true);
    }
    
    let mode = get_mode_by_slug(mode_slug, Some(custom_modes))
        .ok_or_else(|| anyhow!("Mode not found: {}", mode_slug))?;
    
    // Check each group
    for group_entry in &mode.groups {
        let (group_name, group_options) = match group_entry {
            GroupEntry::Simple(g) => (g, None),
            GroupEntry::WithOptions(g, opts) => (g, Some(opts)),
        };
        
        let group_config = TOOL_GROUPS.get(group_name)
            .ok_or_else(|| anyhow!("Unknown tool group: {:?}", group_name))?;
        
        if !group_config.tools.contains(&tool.to_string()) {
            continue;  // Tool not in this group
        }
        
        // Tool is in group, check file restrictions
        if let Some(opts) = group_options {
            if let Some(file_regex) = &opts.file_regex {
                if let Some(params) = tool_params {
                    if let Some(path) = &params.path {
                        let regex = Regex::new(file_regex)?;
                        if !regex.is_match(path) {
                            return Err(anyhow!(
                                "Tool '{}' in mode '{}' can only access files matching pattern: {} ({}). Got: {}",
                                tool,
                                mode_slug,
                                file_regex,
                                opts.description.as_deref().unwrap_or(""),
                                path
                            ));
                        }
                    }
                }
            }
        }
        
        return Ok(true);  // Tool allowed
    }
    
    Ok(false)  // Tool not in any group
}

#[derive(Deserialize, Clone, Debug)]
pub struct ToolParams {
    pub path: Option<String>,
    // ... other tool parameters
}

lazy_static! {
    static ref TOOL_GROUPS: HashMap<ToolGroup, ToolGroupConfig> = {
        let mut map = HashMap::new();
        map.insert(ToolGroup::Read, ToolGroupConfig {
            tools: vec!["readFile", "listFiles", "searchFiles", "listCodeDefinitionNames"]
                .into_iter().map(String::from).collect(),
        });
        map.insert(ToolGroup::Edit, ToolGroupConfig {
            tools: vec!["writeToFile", "editedExistingFile", "insertCodeBlock", "replaceCodeBlock"]
                .into_iter().map(String::from).collect(),
        });
        map.insert(ToolGroup::Command, ToolGroupConfig {
            tools: vec!["executeCommand"].into_iter().map(String::from).collect(),
        });
        map.insert(ToolGroup::Browser, ToolGroupConfig {
            tools: vec!["browserAction"].into_iter().map(String::from).collect(),
        });
        map.insert(ToolGroup::Mcp, ToolGroupConfig {
            tools: vec!["useMcpTool"].into_iter().map(String::from).collect(),
        });
        map
    };
    
    static ref ALWAYS_AVAILABLE_TOOLS: Vec<&'static str> = vec![
        "attemptCompletion",
        "askFollowupQuestion",
        "updateTodoList",
        "switchMode",
        "newTask",
    ];
}

struct ToolGroupConfig {
    tools: Vec<String>,
}
```

---

## 5. Rules & Workflows System

### Concept

**Rules** and **Workflows** are markdown files that inject custom instructions into the AI prompt:
- **Rules:** General guidelines (e.g., "Always use TypeScript strict mode")
- **Workflows:** Step-by-step procedures (e.g., "Git commit workflow")

### Storage

```
.roo/
├── rules/              # Global rules (user-wide)
│   ├── typescript-rules.md
│   └── naming-conventions.md
└── workflows/          # Global workflows
    └── release-process.md

.roo-local/             # Workspace-specific (project-level)
├── rules/
│   └── project-style.md
└── workflows/
    └── deployment.md
```

### Rule/Workflow State

```typescript
interface RuleState {
    globalRules: [string, boolean][]      // [filepath, enabled]
    localRules: [string, boolean][]       // [filepath, enabled]
    globalWorkflows: [string, boolean][]
    localWorkflows: [string, boolean][]
}

// Example:
{
    globalRules: [
        ["/home/user/.roo/rules/typescript-rules.md", true],
        ["/home/user/.roo/rules/naming-conventions.md", false],
    ],
    localRules: [
        ["/project/.roo-local/rules/project-style.md", true],
    ],
    globalWorkflows: [],
    localWorkflows: [
        ["/project/.roo-local/workflows/deployment.md", true],
    ]
}
```

**Rust Translation:**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuleState {
    pub global_rules: Vec<(String, bool)>,
    pub local_rules: Vec<(String, bool)>,
    pub global_workflows: Vec<(String, bool)>,
    pub local_workflows: Vec<(String, bool)>,
}

impl RuleState {
    pub fn get_enabled_rules(&self) -> Vec<String> {
        self.global_rules.iter()
            .chain(self.local_rules.iter())
            .filter(|(_, enabled)| *enabled)
            .map(|(path, _)| path.clone())
            .collect()
    }
    
    pub fn get_enabled_workflows(&self) -> Vec<String> {
        self.global_workflows.iter()
            .chain(self.local_workflows.iter())
            .filter(|(_, enabled)| *enabled)
            .map(|(path, _)| path.clone())
            .collect()
    }
}

// Load rule/workflow content
pub async fn load_rule_content(path: &str) -> Result<String> {
    tokio::fs::read_to_string(path).await
        .context(format!("Failed to read rule: {}", path))
}

// Inject rules into AI prompt
pub async fn inject_rules_into_prompt(
    base_prompt: &str,
    rule_state: &RuleState,
) -> Result<String> {
    let mut prompt = base_prompt.to_string();
    
    // Load all enabled rules
    for rule_path in rule_state.get_enabled_rules() {
        let content = load_rule_content(&rule_path).await?;
        prompt.push_str(&format!("\n\n## Custom Rule\n{}", content));
    }
    
    // Load all enabled workflows
    for workflow_path in rule_state.get_enabled_workflows() {
        let content = load_rule_content(&workflow_path).await?;
        prompt.push_str(&format!("\n\n## Workflow\n{}", content));
    }
    
    Ok(prompt)
}
```

### UI Components

```typescript
// RulesWorkflowsSection - shows global and workspace rules
<RulesWorkflowsSection
    type="rule"  // or "workflow"
    globalItems={globalRules}
    localItems={localRules}
    toggleGlobal={(path, enabled) => {
        vscode.postMessage({ 
            type: "toggleGlobalRule", 
            rulePath: path, 
            enabled 
        })
    }}
    toggleLocal={(path, enabled) => {
        vscode.postMessage({ 
            type: "toggleLocalRule", 
            rulePath: path, 
            enabled 
        })
    }}
/>

// RuleRow - individual rule toggle
<RuleRow
    rulePath="/home/user/.roo/rules/typescript-rules.md"
    enabled={true}
    toggleRule={(path, enabled) => {
        vscode.postMessage({ 
            type: "toggleGlobalRule", 
            rulePath: path, 
            enabled 
        })
    }}
/>

// Actions:
// - Toggle: Enable/disable rule
// - Edit: Open file in editor
// - Delete: Remove file
```

**Backend Messages:**

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum WebviewMessage {
    #[serde(rename = "toggleGlobalRule")]
    ToggleGlobalRule { rule_path: String, enabled: bool },
    
    #[serde(rename = "toggleLocalRule")]
    ToggleLocalRule { rule_path: String, enabled: bool },
    
    #[serde(rename = "deleteRuleFile")]
    DeleteRuleFile { rule_path: String },
    
    #[serde(rename = "openFile")]
    OpenFile { text: String },  // file path
}

pub async fn handle_toggle_rule(
    rule_path: &str,
    enabled: bool,
    is_global: bool,
    state: Arc<RwLock<AppState>>,
) -> Result<()> {
    let mut state = state.write().await;
    
    let rules = if is_global {
        &mut state.rule_state.global_rules
    } else {
        &mut state.rule_state.local_rules
    };
    
    // Update or add rule
    if let Some(entry) = rules.iter_mut().find(|(path, _)| path == rule_path) {
        entry.1 = enabled;
    } else {
        rules.push((rule_path.to_string(), enabled));
    }
    
    // Persist to database
    save_rule_state(&state.rule_state).await?;
    
    Ok(())
}
```

---

**STATUS:** Complete Kilocode features analysis (modes, rules, workflows)
**NEXT:** DEEP-08-MCP-COMPONENTS.md - MCP integration
