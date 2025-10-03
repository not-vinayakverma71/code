# CHUNK-14: MENTIONS SYSTEM (@file, @folder, @url Context Injection)

## 📁 Complete System Analysis

```
Mentions System:
├── Codex/src/core/mentions/
│   ├── index.ts                           (419 lines) - Core parsing & fetching
│   ├── processUserContentMentions.ts      (121 lines) - Generic processor
│   ├── processKiloUserContentMentions.ts  (159 lines) - Kilo-specific processor
│   └── __tests__/
└── Related:
    ├── shared/context-mentions.ts          - Regex patterns
    ├── services/browser/UrlContentFetcher  - Web scraping
    └── utils/git.ts                        - Git operations

TOTAL: 699+ lines mention parsing system
```

---

## 🎯 PURPOSE

**Dynamic Context Injection via @mentions**: Allow users to reference files, folders, URLs, and other context sources inline using `@` syntax.

**Critical for**:
- Natural language file references ("Fix the bug in @src/main.rs")
- Web research ("Implement auth like @https://docs.rs/actix-web")
- Workspace diagnostics ("Fix all @problems")
- Git context ("What changed in @abc1234")
- Terminal output ("Explain this error @terminal")

---

## 🔧 MENTION TYPES SUPPORTED

| Mention | Syntax | Example | XML Tag |
|---------|--------|---------|---------|
| **File** | `@/path/to/file.rs` | `@src/main.rs` | `<file_content>` |
| **Folder** | `@/path/to/dir/` | `@src/utils/` | `<folder_content>` |
| **URL** | `@https://...` | `@https://docs.rs` | `<url_content>` |
| **Problems** | `@problems` | `@problems` | `<workspace_diagnostics>` |
| **Git Changes** | `@git-changes` | `@git-changes` | `<git_working_state>` |
| **Git Commit** | `@abc1234` | `@7c3f891` | `<git_commit>` |
| **Terminal** | `@terminal` | `@terminal` | `<terminal_output>` |
| **Command** | `@command_name` | `@test` | `<command>` |

---

## 🔧 CORE ALGORITHM: parseMentions() (Lines 80-267)

### Step 1: Parse & Replace Mentions

```typescript
// Extract mentions using regex
parsedText = text.replace(mentionRegexGlobal, (match, mention) => {
    mentions.add(mention)
    
    if (mention.startsWith("http")) {
        return `'${mention}' (see below for site content)`
    } else if (mention.startsWith("/")) {
        return mention.endsWith("/")
            ? `'${mentionPath}' (see below for folder content)`
            : `'${mentionPath}' (see below for file content)`
    } else if (mention === "problems") {
        return `Workspace Problems (see below for diagnostics)`
    }
    // ... other mention types
})
```

---

### Step 2: Fetch Content & Append XML

```typescript
for (const mention of mentions) {
    if (mention.startsWith("http")) {
        const markdown = await urlContentFetcher.urlToMarkdown(mention)
        parsedText += `\n\n<url_content url="${mention}">\n${markdown}\n</url_content>`
    } else if (mention.startsWith("/")) {
        const content = await getFileOrFolderContent(mentionPath, cwd, ...)
        parsedText += `\n\n<file_content path="${mentionPath}">\n${content}\n</file_content>`
    } else if (mention === "problems") {
        const problems = await getWorkspaceProblems(cwd, ...)
        parsedText += `\n\n<workspace_diagnostics>\n${problems}\n</workspace_diagnostics>`
    }
    // ... other mention types
}
```

---

## 🔧 FILE/FOLDER CONTENT (Lines 269-349)

### File Case:
```typescript
if (stats.isFile()) {
    if (rooIgnoreController && !rooIgnoreController.validateAccess(absPath)) {
        return `(File ${mentionPath} is ignored by .kilocodeignore)`
    }
    const content = await extractTextFromFile(absPath, maxReadFileLine)
    return content
}
```

### Folder Case:
```typescript
else if (stats.isDirectory()) {
    const entries = await fs.readdir(absPath, { withFileTypes: true })
    let folderContent = ""
    const fileContentPromises: Promise<string | undefined>[] = []
    
    for (const entry of entries) {
        const linePrefix = isLast ? "└── " : "├── "
        const displayName = isIgnored ? `🔒 ${entry.name}` : entry.name
        
        if (entry.isFile()) {
            folderContent += `${linePrefix}${displayName}\n`
            if (!isIgnored) {
                fileContentPromises.push(
                    extractTextFromFile(absoluteFilePath, maxReadFileLine).then(
                        content => `<file_content path="${filePath}">\n${content}\n</file_content>`
                    )
                )
            }
        } else if (entry.isDirectory()) {
            folderContent += `${linePrefix}${displayName}/\n`
        }
    }
    
    const fileContents = (await Promise.all(fileContentPromises)).filter(c => c)
    return `${folderContent}\n${fileContents.join("\n\n")}`.trim()
}
```

**Output**:
```
src/
├── main.rs
└── lib.rs

<file_content path="src/main.rs">
fn main() { println!("Hello"); }
</file_content>

<file_content path="src/lib.rs">
pub fn add(a: i32, b: i32) -> i32 { a + b }
</file_content>
```

---

## 🔧 TERMINAL OUTPUT (Lines 374-414)

```typescript
export async function getLatestTerminalOutput(): Promise<string> {
    const originalClipboard = await vscode.env.clipboard.readText()
    
    try {
        await vscode.commands.executeCommand("workbench.action.terminal.selectAll")
        await vscode.commands.executeCommand("workbench.action.terminal.copySelection")
        await vscode.commands.executeCommand("workbench.action.terminal.clearSelection")
        
        let terminalContents = (await vscode.env.clipboard.readText()).trim()
        
        // Clean up duplicate prompt lines
        const lines = terminalContents.split("\n")
        const lastLine = lines.pop()?.trim()
        if (lastLine) {
            let i = lines.length - 1
            while (i >= 0 && !lines[i].trim().startsWith(lastLine)) {
                i--
            }
            terminalContents = lines.slice(Math.max(i, 0)).join("\n")
        }
        
        return terminalContents
    } finally {
        await vscode.env.clipboard.writeText(originalClipboard)
    }
}
```

**Clipboard hack**: VSCode doesn't provide terminal content API → Use copy/paste.

---

## 🎯 COMPLETE EXAMPLE

### Input:
```
Fix bug in @src/auth.rs using @https://example.com/auth
Check @problems and @git-changes
```

### Output:
```
Fix bug in 'src/auth.rs' (see below for file content) using 'https://example.com/auth' (see below for site content)
Check Workspace Problems (see below for diagnostics) and Working directory changes (see below for details)

<file_content path="src/auth.rs">
pub fn verify_token(token: &str) -> Result<User, AuthError> {
    let claims = decode_jwt(token)?;
    Ok(User::from_claims(claims))
}
</file_content>

<url_content url="https://example.com/auth">
# JWT Auth Best Practices
Always validate token expiration...
</url_content>

<workspace_diagnostics>
File: src/auth.rs
  Line 15: warning: unused variable `expiry`
</workspace_diagnostics>

<git_working_state>
Modified: src/auth.rs
Diff: +let expiry = claims.exp;
</git_working_state>
```

---

## 🎯 RUST TRANSLATION

```rust
use regex::Regex;
use std::path::Path;

pub struct MentionParser {
    cwd: PathBuf,
    url_fetcher: UrlContentFetcher,
    file_tracker: FileContextTracker,
    ignore_controller: RooIgnoreController,
}

impl MentionParser {
    pub async fn parse_mentions(&self, text: &str) -> Result<String, Error> {
        let mut mentions = HashSet::new();
        let mention_regex = Regex::new(r"@(\S+)").unwrap();
        
        let mut parsed_text = text.to_string();
        for cap in mention_regex.captures_iter(text) {
            let mention = &cap[1];
            mentions.insert(mention.to_string());
            
            let replacement = if mention.starts_with("http") {
                format!("'{}' (see below for site content)", mention)
            } else if mention.starts_with('/') {
                if mention.ends_with('/') {
                    format!("'{}' (see below for folder content)", mention)
                } else {
                    format!("'{}' (see below for file content)", mention)
                }
            } else if mention == "problems" {
                "Workspace Problems (see below)".to_string()
            } else {
                continue;
            };
            
            parsed_text = parsed_text.replace(&format!("@{}", mention), &replacement);
        }
        
        for mention in mentions {
            if mention.starts_with("http") {
                let markdown = self.url_fetcher.url_to_markdown(&mention).await?;
                parsed_text.push_str(&format!("\n\n<url_content url=\"{}\">\n{}\n</url_content>", mention, markdown));
            } else if mention.starts_with('/') {
                let content = self.get_file_or_folder_content(&mention[1..]).await?;
                if mention.ends_with('/') {
                    parsed_text.push_str(&format!("\n\n<folder_content path=\"{}\">\n{}\n</folder_content>", &mention[1..], content));
                } else {
                    parsed_text.push_str(&format!("\n\n<file_content path=\"{}\">\n{}\n</file_content>", &mention[1..], content));
                    self.file_tracker.track_file_context(&mention[1..], "file_mentioned").await?;
                }
            } else if mention == "problems" {
                let problems = self.get_workspace_problems().await?;
                parsed_text.push_str(&format!("\n\n<workspace_diagnostics>\n{}\n</workspace_diagnostics>", problems));
            }
        }
        
        Ok(parsed_text)
    }
    
    async fn get_file_or_folder_content(&self, path: &str) -> Result<String, Error> {
        let abs_path = self.cwd.join(path);
        let metadata = tokio::fs::metadata(&abs_path).await?;
        
        if metadata.is_file() {
            if !self.ignore_controller.validate_access(&abs_path) {
                return Ok(format!("(File {} is ignored by .kilocodeignore)", path));
            }
            let content = tokio::fs::read_to_string(&abs_path).await?;
            Ok(content)
        } else if metadata.is_dir() {
            let mut folder_content = String::new();
            let mut entries = tokio::fs::read_dir(&abs_path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().await?.is_dir();
                folder_content.push_str(&format!("├── {}{}\n", name, if is_dir { "/" } else { "" }));
            }
            
            Ok(folder_content)
        } else {
            Err(Error::InvalidPath(path.to_string()))
        }
    }
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] All mention types documented
- [x] Parsing algorithm explained
- [x] Content fetching detailed
- [x] XML wrapping format shown
- [x] File/folder handling covered
- [x] Terminal clipboard hack explained
- [x] Complete example provided
- [x] Rust translation patterns defined

**STATUS**: CHUNK-14 COMPLETE (comprehensive mentions system analysis)
