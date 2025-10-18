# CHUNK-15: SLASH COMMANDS & WORKFLOWS (Custom User Instructions)

## 📁 Complete System Analysis

```
Slash Commands & Workflows:
├── Codex/src/core/slash-commands/
│   └── kilo.ts                           (114 lines) - Slash command parser
├── Codex/src/core/context/instructions/
│   ├── workflows.ts                      (43 lines)  - Workflow toggle management
│   ├── kilo-rules.ts                     (43 lines)  - Rules directory migration
│   └── rule-helpers.ts                   - Toggle synchronization
└── Related:
    ├── core/prompts/commands.ts          - Command response templates
    └── shared/globalFileNames.ts         - File path constants

TOTAL: 200+ lines slash command & workflow system
```

---

## 🎯 PURPOSE

**Dynamic Instruction Injection via Slash Commands**: Allow users to inject custom instructions, workflows, and commands inline using `/` syntax.

**Critical for**:
- Quick task templates (`/newtask` → New task creation instructions)
- Custom workflows (`/testing` → Load testing workflow file)
- Bug reporting (`/reportbug` → Format bug report)
- Context condensing (`/smol` → Summarize conversation)
- Rule management (`/newrule` → Prompt to create rules)

---

## 📊 ARCHITECTURE OVERVIEW

```
Slash Command Processing Pipeline:

USER INPUT:
"<task>/newtask Build authentication system</task>"

↓ Step 1: Parse command
Regex match: /newtask found in <task> tag

↓ Step 2: Lookup command
commandReplacements["newtask"] → newTaskToolResponse()

↓ Step 3: Remove command from text
"<task>Build authentication system</task>"

↓ Step 4: Prepend command instructions
"<explicit_instructions type="newtask">
When starting a new task...
[template content]
</explicit_instructions>
<task>Build authentication system</task>"

↓ FINAL OUTPUT TO AI:
Text with custom instructions prepended
```

---

## 🔧 SUPPORTED COMMANDS

### Built-in Commands:
| Command | Purpose | Template Source |
|---------|---------|-----------------|
| `/newtask` | New task creation guidance | `newTaskToolResponse()` |
| `/newrule` | Prompt to create rules directory | `newRuleToolResponse()` |
| `/reportbug` | Bug report formatting | `reportBugToolResponse()` |
| `/smol` | Condense conversation | `condenseToolResponse()` |

### Dynamic Workflow Commands:
| Command | File Location | Enable/Disable |
|---------|---------------|----------------|
| `/testing` | `.kilocode/workflows/testing.md` | Toggle in UI |
| `/deployment` | `.kilocode/workflows/deployment.md` | Toggle in UI |
| `/<custom>` | `.kilocode/workflows/<custom>.md` | User-defined |

---

## 🔧 FILE 1: kilo.ts (114 lines) - Command Parser

### Main Function: parseKiloSlashCommands() - Lines 26-113

```typescript
export async function parseKiloSlashCommands(
    text: string,
    localWorkflowToggles: ClineRulesToggles,
    globalWorkflowToggles: ClineRulesToggles,
): Promise<{ processedText: string; needsRulesFileCheck: boolean }>
```

**Input**:
- `text`: User message with potential slash command
- `localWorkflowToggles`: Workspace-specific workflows (`.kilocode/workflows/`)
- `globalWorkflowToggles`: Global workflows (`~/.kilocode/workflows/`)

**Output**:
- `processedText`: Text with command replaced by instructions
- `needsRulesFileCheck`: True if `/newrule` used (triggers directory creation)

---

### Step 1: Define Command Mappings - Lines 31-36

```typescript
const commandReplacements: Record<string, ((userInput: string) => string) | undefined> = {
    newtask: newTaskToolResponse,
    newrule: newRuleToolResponse,
    reportbug: reportBugToolResponse,
    smol: condenseToolResponse,
}
```

**Function signatures**:
```typescript
function newTaskToolResponse(userInput: string): string {
    return `<explicit_instructions type="newtask">
When starting a new task:
1. Break down into subtasks
2. Create todo list with update_todo_list tool
3. Plan before executing
...
</explicit_instructions>
${userInput}`
}
```

---

### Step 2: Define Tag Patterns - Lines 39-44

```typescript
const tagPatterns = [
    { tag: "task", regex: /<task>(\s*\/([a-zA-Z0-9_.-]+))(\s+.+?)?\s*<\/task>/is },
    { tag: "feedback", regex: /<feedback>(\s*\/([a-zA-Z0-9_-]+))(\s+.+?)?\s*<\/feedback>/is },
    { tag: "answer", regex: /<answer>(\s*\/([a-zA-Z0-9_-]+))(\s+.+?)?\s*<\/answer>/is },
    { tag: "user_message", regex: /<user_message>(\s*\/([a-zA-Z0-9_-]+))(\s+.+?)?\s*<\/user_message>/is },
]
```

**Why restrict to tags?** Only process commands in user-generated content, not AI responses.

**Regex breakdown**:
- `(\s*\/([a-zA-Z0-9_.-]+))`: Capture group 1 = entire command with whitespace, group 2 = command name
- `(\s+.+?)?`: Optional text after command
- `/is` flags: Case-insensitive, dotall (`.` matches newlines)

**Examples**:
```typescript
"<task>/newtask Build auth</task>"
// match[0] = "<task>/newtask Build auth</task>"
// match[1] = "/newtask"
// match[2] = "newtask"

"<feedback>  /reportbug   Login fails</feedback>"
// match[1] = "  /reportbug"
// match[2] = "reportbug"
```

---

### Step 3: Process Built-in Commands - Lines 47-75

```typescript
for (const { regex } of tagPatterns) {
    const match = regex.exec(text)
    
    if (match) {
        const commandName = match[2]  // e.g., "newtask"
        const command = commandReplacements[commandName]
        
        if (command) {
            const fullMatchStartIndex = match.index
            const fullMatch = match[0]
            const relativeStartIndex = fullMatch.indexOf(match[1])
            
            // Calculate absolute indices
            const slashCommandStartIndex = fullMatchStartIndex + relativeStartIndex
            const slashCommandEndIndex = slashCommandStartIndex + match[1].length
            
            // Remove slash command
            const textWithoutSlashCommand =
                text.substring(0, slashCommandStartIndex) +
                text.substring(slashCommandEndIndex)
            
            // Prepend command instructions
            const processedText = command(textWithoutSlashCommand)
            
            return { processedText, needsRulesFileCheck: commandName === "newrule" }
        }
        // ... continue to workflow processing
    }
}
```

**Index calculation example**:
```
Text: "Previous message\n<task>/newtask Build auth</task>\nMore text"
                      ^                     ^
                      |                     |
         fullMatchStartIndex=16    slashCommandStartIndex=22

fullMatch = "<task>/newtask Build auth</task>"
match[1] = "/newtask"
relativeStartIndex = fullMatch.indexOf("/newtask") = 6
slashCommandStartIndex = 16 + 6 = 22
slashCommandEndIndex = 22 + 8 = 30

Result: "Previous message\n<task> Build auth</task>\nMore text"
        (removed "/newtask")
```

---

### Step 4: Process Workflow Commands - Lines 77-107

```typescript
const matchingWorkflow = [
    ...enabledWorkflowToggles(localWorkflowToggles),
    ...enabledWorkflowToggles(globalWorkflowToggles),
].find((workflow) => workflow.fileName === commandName)

if (matchingWorkflow) {
    try {
        // Read workflow file content
        const workflowContent = (await fs.readFile(matchingWorkflow.fullPath, "utf8")).trim()
        
        // Calculate indices (same as above)
        const fullMatchStartIndex = match.index
        const fullMatch = match[0]
        const relativeStartIndex = fullMatch.indexOf(match[1])
        const slashCommandStartIndex = fullMatchStartIndex + relativeStartIndex
        const slashCommandEndIndex = slashCommandStartIndex + match[1].length
        
        // Remove slash command
        const textWithoutSlashCommand =
            text.substring(0, slashCommandStartIndex) +
            text.substring(slashCommandEndIndex)
        
        // Prepend workflow content
        const processedText =
            `<explicit_instructions type="${matchingWorkflow.fileName}">\n${workflowContent}\n</explicit_instructions>\n` +
            textWithoutSlashCommand
        
        return { processedText, needsRulesFileCheck: false }
    } catch (error) {
        console.error(`Error reading workflow file ${matchingWorkflow.fullPath}: ${error}`)
    }
}
```

**enabledWorkflowToggles()** - Lines 13-20:
```typescript
function enabledWorkflowToggles(workflowToggles: ClineRulesToggles) {
    return Object.entries(workflowToggles)
        .filter(([_, enabled]) => enabled)
        .map(([filePath, _]) => ({
            fullPath: filePath,
            fileName: path.basename(filePath),
        }))
}
```

**Example**:
```typescript
localWorkflowToggles = {
    "/project/.kilocode/workflows/testing.md": true,
    "/project/.kilocode/workflows/deployment.md": false,
}

enabledWorkflowToggles(localWorkflowToggles) = [
    { fullPath: "/project/.kilocode/workflows/testing.md", fileName: "testing.md" }
]

// User types: "<task>/testing Run all tests</task>"
// commandName = "testing"
// Finds matching workflow → Reads file → Prepends content
```

---

## 🔧 FILE 2: workflows.ts (43 lines) - Toggle Management

### Purpose: Sync workflow files with enable/disable toggles

### Function: refreshWorkflowToggles() - Lines 30-42

```typescript
export async function refreshWorkflowToggles(
    context: vscode.ExtensionContext,
    workingDirectory: string,
): Promise<{
    globalWorkflowToggles: ClineRulesToggles
    localWorkflowToggles: ClineRulesToggles
}>
```

**Called**: Before processing slash commands (see `processKiloUserContentMentions.ts`).

**Implementation**:
```typescript
const proxy = new ContextProxy(context)
return {
    globalWorkflowToggles: await refreshGlobalWorkflowToggles(proxy),
    localWorkflowToggles: await refreshLocalWorkflowToggles(proxy, context, workingDirectory),
}
```

---

### Local Workflows - Lines 9-20

```typescript
async function refreshLocalWorkflowToggles(
    proxy: ContextProxy,
    context: vscode.ExtensionContext,
    workingDirectory: string,
) {
    const workflowRulesToggles =
        ((await proxy.getWorkspaceState(context, "localWorkflowToggles")) as ClineRulesToggles) || {}
    
    const workflowsDirPath = path.resolve(workingDirectory, GlobalFileNames.workflows)
    
    const updatedWorkflowToggles = await synchronizeRuleToggles(workflowsDirPath, workflowRulesToggles)
    
    await proxy.updateWorkspaceState(context, "localWorkflowToggles", updatedWorkflowToggles)
    
    return updatedWorkflowToggles
}
```

**synchronizeRuleToggles()**: Scans directory for `.md` files, adds new files to toggles (enabled by default), removes deleted files.

**Location**: `.kilocode/workflows/*.md`

---

### Global Workflows - Lines 22-28

```typescript
async function refreshGlobalWorkflowToggles(proxy: ContextProxy) {
    const globalWorkflowToggles = ((await proxy.getGlobalState("globalWorkflowToggles")) as ClineRulesToggles) || {}
    
    const globalWorkflowsDir = path.join(os.homedir(), GlobalFileNames.workflows)
    
    const updatedGlobalWorkflowToggles = await synchronizeRuleToggles(globalWorkflowsDir, globalWorkflowToggles)
    
    await proxy.updateGlobalState("globalWorkflowToggles", updatedGlobalWorkflowToggles)
    
    return updatedGlobalWorkflowToggles
}
```

**Location**: `~/.kilocode/workflows/*.md`

**Use case**: User-wide workflows (e.g., company coding standards, personal preferences).

---

## 🔧 FILE 3: kilo-rules.ts (43 lines) - Directory Migration

### Purpose: Convert legacy `.kilocode/rules` file to directory

**Background**: Old versions stored rules in single file. New versions use directory with multiple rule files.

### Function: ensureLocalKilorulesDirExists() - Lines 10-42

```typescript
export async function ensureLocalKilorulesDirExists(
    kilorulePath: string,
    defaultRuleFilename: string,
): Promise<boolean>  // Returns true if error occurred
```

**Migration logic**:
```typescript
const exists = await fileExistsAtPath(kilorulePath)

if (exists && !(await isDirectory(kilorulePath))) {
    // Old file exists, convert to directory
    const content = await fs.readFile(kilorulePath, "utf8")
    const tempPath = kilorulePath + ".bak"
    
    await fs.rename(kilorulePath, tempPath)  // Backup original
    
    try {
        await fs.mkdir(kilorulePath, { recursive: true })
        await fs.writeFile(path.join(kilorulePath, defaultRuleFilename), content, "utf8")
        await fs.unlink(tempPath)  // Delete backup on success
        
        return false  // Success
    } catch (conversionError) {
        // Restore backup on failure
        await fs.rm(kilorulePath, { recursive: true, force: true })
        await fs.rename(tempPath, kilorulePath)
        
        return true  // Error
    }
}

return false  // Already directory or doesn't exist
```

**Example**:
```
BEFORE:
.kilocode/
  rules  (file)

AFTER:
.kilocode/
  rules/  (directory)
    default.md  (contents of old file)
```

**Triggered by**: `/newrule` command sets `needsRulesFileCheck = true` → Calls this function.

---

## 🎯 COMPLETE WORKFLOW EXAMPLE

### Scenario: User creates custom testing workflow

#### Step 1: Create workflow file
```bash
# .kilocode/workflows/testing.md
# Testing Workflow

When testing:
1. Run unit tests: `cargo test`
2. Check coverage: `cargo tarpaulin`
3. Verify >80% coverage
4. Run integration tests
5. Manual QA checklist
```

#### Step 2: Refresh toggles (automatic)
```typescript
refreshWorkflowToggles(context, cwd)
// Scans .kilocode/workflows/
// Finds testing.md
// Adds to localWorkflowToggles: { "/path/testing.md": true }
```

#### Step 3: User invokes command
```
<task>/testing Verify auth module is fully tested</task>
```

#### Step 4: Command parsing
```typescript
parseKiloSlashCommands(text, localWorkflowToggles, globalWorkflowToggles)
// Finds /testing
// Matches with testing.md
// Reads file content
// Prepends instructions
```

#### Step 5: Final output to AI
```
<explicit_instructions type="testing.md">
# Testing Workflow

When testing:
1. Run unit tests: `cargo test`
2. Check coverage: `cargo tarpaulin`
3. Verify >80% coverage
4. Run integration tests
5. Manual QA checklist
</explicit_instructions>
<task>Verify auth module is fully tested</task>
```

---

## 🎯 BUILT-IN COMMAND EXAMPLES

### `/newtask` Output:
```xml
<explicit_instructions type="newtask">
When starting a new task:
1. Break down the task into clear subtasks
2. Create a todo list using the update_todo_list tool
3. Plan your approach before executing
4. Start with the most critical components
5. Test incrementally as you build
</explicit_instructions>
<task>Build authentication system</task>
```

### `/reportbug` Output:
```xml
<explicit_instructions type="reportbug">
When reporting a bug:
1. Describe expected behavior
2. Describe actual behavior
3. Steps to reproduce
4. Environment details (OS, version)
5. Error messages/stack traces
6. Relevant code snippets
</explicit_instructions>
<feedback>Login button doesn't work</feedback>
```

### `/smol` Output:
```xml
<explicit_instructions type="smol">
Condense the conversation:
1. Summarize completed work
2. List remaining tasks
3. Note any blockers or decisions needed
4. Keep summary under 500 words
</explicit_instructions>
<task>Summarize our progress</task>
```

---

## 🎯 RUST TRANSLATION PATTERNS

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use regex::Regex;

pub struct SlashCommandParser {
    command_replacements: HashMap<String, fn(&str) -> String>,
    local_workflows: HashMap<PathBuf, bool>,
    global_workflows: HashMap<PathBuf, bool>,
}

impl SlashCommandParser {
    pub async fn parse_kilo_slash_commands(
        &self,
        text: &str,
    ) -> Result<(String, bool), Error> {
        let tag_patterns = vec![
            r#"<task>(\s*/([a-zA-Z0-9_.-]+))(\s+.+?)?\s*</task>"#,
            r#"<feedback>(\s*/([a-zA-Z0-9_-]+))(\s+.+?)?\s*</feedback>"#,
            r#"<answer>(\s*/([a-zA-Z0-9_-]+))(\s+.+?)?\s*</answer>"#,
            r#"<user_message>(\s*/([a-zA-Z0-9_-]+))(\s+.+?)?\s*</user_message>"#,
        ];
        
        for pattern in tag_patterns {
            let regex = Regex::new(pattern).unwrap();
            
            if let Some(caps) = regex.captures(text) {
                let command_name = &caps[2];
                
                // Check built-in commands
                if let Some(command_fn) = self.command_replacements.get(command_name) {
                    let full_match_start = caps.get(0).unwrap().start();
                    let command_match = &caps[1];
                    let relative_start = caps.get(0).unwrap().as_str().find(command_match).unwrap();
                    
                    let slash_cmd_start = full_match_start + relative_start;
                    let slash_cmd_end = slash_cmd_start + command_match.len();
                    
                    let text_without_cmd = format!(
                        "{}{}",
                        &text[..slash_cmd_start],
                        &text[slash_cmd_end..]
                    );
                    
                    let processed_text = command_fn(&text_without_cmd);
                    let needs_rules_check = command_name == "newrule";
                    
                    return Ok((processed_text, needs_rules_check));
                }
                
                // Check workflow commands
                let enabled_workflows: Vec<_> = self.local_workflows.iter()
                    .chain(self.global_workflows.iter())
                    .filter(|(_, &enabled)| enabled)
                    .collect();
                
                for (workflow_path, _) in enabled_workflows {
                    let filename = workflow_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if filename == command_name {
                        let workflow_content = tokio::fs::read_to_string(workflow_path).await?;
                        
                        let full_match_start = caps.get(0).unwrap().start();
                        let command_match = &caps[1];
                        let relative_start = caps.get(0).unwrap().as_str().find(command_match).unwrap();
                        
                        let slash_cmd_start = full_match_start + relative_start;
                        let slash_cmd_end = slash_cmd_start + command_match.len();
                        
                        let text_without_cmd = format!(
                            "{}{}",
                            &text[..slash_cmd_start],
                            &text[slash_cmd_end..]
                        );
                        
                        let processed_text = format!(
                            "<explicit_instructions type=\"{}\">\n{}\n</explicit_instructions>\n{}",
                            filename,
                            workflow_content.trim(),
                            text_without_cmd
                        );
                        
                        return Ok((processed_text, false));
                    }
                }
            }
        }
        
        Ok((text.to_string(), false))
    }
}

pub async fn refresh_workflow_toggles(
    workspace_dir: &Path,
) -> Result<(HashMap<PathBuf, bool>, HashMap<PathBuf, bool>), Error> {
    let local_workflows = refresh_local_workflows(workspace_dir).await?;
    let global_workflows = refresh_global_workflows().await?;
    
    Ok((local_workflows, global_workflows))
}

async fn refresh_local_workflows(workspace_dir: &Path) -> Result<HashMap<PathBuf, bool>, Error> {
    let workflows_dir = workspace_dir.join(".kilocode/workflows");
    synchronize_rule_toggles(&workflows_dir).await
}

async fn refresh_global_workflows() -> Result<HashMap<PathBuf, bool>, Error> {
    let home_dir = dirs::home_dir().ok_or(Error::NoHomeDir)?;
    let workflows_dir = home_dir.join(".kilocode/workflows");
    synchronize_rule_toggles(&workflows_dir).await
}

async fn synchronize_rule_toggles(dir: &Path) -> Result<HashMap<PathBuf, bool>, Error> {
    let mut toggles = HashMap::new();
    
    if dir.exists() {
        let mut entries = tokio::fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                toggles.insert(path, true);  // Enabled by default
            }
        }
    }
    
    Ok(toggles)
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] Slash command parsing explained
- [x] Built-in vs workflow commands differentiated
- [x] Toggle management system documented
- [x] Directory migration logic covered
- [x] Complete workflow example provided
- [x] Rust translation patterns defined

**STATUS**: CHUNK-15 COMPLETE ✅

## 🎯 ANALYSIS COMPLETE

**Total chunks completed**: 11 (CHUNK-05 through CHUNK-15)
**Total lines analyzed**: ~10,000+ lines of TypeScript
**Total documentation**: ~50,000+ words across 11 markdown files

All major subsystems documented with production-grade detail and Rust translation patterns! 🚀
