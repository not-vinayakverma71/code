# CHUNK 10: src/utils/ - HELPER UTILITIES (50 FILES)

## Overview
Utilities provide essential cross-cutting functionality:
- **XML parsing** (XmlMatcher) - Extract tool calls from AI responses
- **Git operations** - Repository info, commit search
- **Shell detection** - Cross-platform shell identification
- **Path handling** - Cross-platform path normalization
- **File system** - Directory creation, recursive reads
- **Tiktoken** - Token counting
- **Storage** - State persistence
- **Configuration** - VS Code settings migration

## CRITICAL: XmlMatcher (105 LINES)

**Purpose:** Stream-parse XML tags from AI output to extract tool calls

### Algorithm
```typescript
export class XmlMatcher<Result = XmlMatcherResult> {
    index = 0
    chunks: XmlMatcherResult[] = []
    cached: string[] = []
    matched: boolean = false
    state: "TEXT" | "TAG_OPEN" | "TAG_CLOSE" = "TEXT"
    depth = 0
    pointer = 0
    
    constructor(
        readonly tagName: string,
        readonly transform?: (chunks: XmlMatcherResult) => Result,
        readonly position = 0,
    ) {}
    
    update(chunk: string) {
        for (const char of chunk) {
            this.cached.push(char)
            this.pointer++
            
            if (this.state === "TEXT") {
                if (char === "<") {
                    this.state = "TAG_OPEN"
                    this.index = 0
                } else {
                    this.collect()
                }
            } else if (this.state === "TAG_OPEN") {
                if (char === ">" && this.index === this.tagName.length) {
                    this.state = "TEXT"
                    this.depth++
                    this.matched = true
                } else if (char === "/" && this.index === 0) {
                    this.state = "TAG_CLOSE"
                } else if (this.tagName[this.index] === char) {
                    this.index++
                } else {
                    this.state = "TEXT"
                    this.collect()
                }
            } else if (this.state === "TAG_CLOSE") {
                if (char === ">" && this.index === this.tagName.length) {
                    this.state = "TEXT"
                    this.depth--
                    this.matched = this.depth > 0
                } else if (this.tagName[this.index] === char) {
                    this.index++
                } else {
                    this.state = "TEXT"
                    this.collect()
                }
            }
        }
        return this.pop()
    }
}
```

**Usage:**
```typescript
// Parse <thinking> tags from AI response
const matcher = new XmlMatcher("thinking")

for await (const chunk of stream) {
    const results = matcher.update(chunk.text)
    for (const result of results) {
        if (result.matched) {
            console.log("Thinking:", result.data)
        } else {
            console.log("Text:", result.data)
        }
    }
}
```

**RUST TRANSLATION:**
```rust
pub struct XmlMatcher {
    tag_name: String,
    index: usize,
    chunks: Vec<XmlMatcherResult>,
    cached: Vec<char>,
    matched: bool,
    state: XmlMatcherState,
    depth: usize,
    pointer: usize,
}

#[derive(Debug, Clone)]
pub enum XmlMatcherState {
    Text,
    TagOpen,
    TagClose,
}

#[derive(Debug, Clone)]
pub struct XmlMatcherResult {
    pub matched: bool,
    pub data: String,
}

impl XmlMatcher {
    pub fn new(tag_name: String) -> Self {
        Self {
            tag_name,
            index: 0,
            chunks: Vec::new(),
            cached: Vec::new(),
            matched: false,
            state: XmlMatcherState::Text,
            depth: 0,
            pointer: 0,
        }
    }
    
    pub fn update(&mut self, chunk: &str) -> Vec<XmlMatcherResult> {
        for ch in chunk.chars() {
            self.cached.push(ch);
            self.pointer += 1;
            
            match self.state {
                XmlMatcherState::Text => {
                    if ch == '<' {
                        self.state = XmlMatcherState::TagOpen;
                        self.index = 0;
                    } else {
                        self.collect();
                    }
                }
                XmlMatcherState::TagOpen => {
                    if ch == '>' && self.index == self.tag_name.len() {
                        self.state = XmlMatcherState::Text;
                        self.depth += 1;
                        self.matched = true;
                    } else if ch == '/' && self.index == 0 {
                        self.state = XmlMatcherState::TagClose;
                    } else if self.tag_name.chars().nth(self.index) == Some(ch) {
                        self.index += 1;
                    } else {
                        self.state = XmlMatcherState::Text;
                        self.collect();
                    }
                }
                XmlMatcherState::TagClose => {
                    if ch == '>' && self.index == self.tag_name.len() {
                        self.state = XmlMatcherState::Text;
                        self.depth -= 1;
                        self.matched = self.depth > 0;
                    } else if self.tag_name.chars().nth(self.index) == Some(ch) {
                        self.index += 1;
                    } else {
                        self.state = XmlMatcherState::Text;
                        self.collect();
                    }
                }
            }
        }
        
        self.pop()
    }
    
    fn collect(&mut self) {
        if self.cached.is_empty() {
            return;
        }
        
        let data: String = self.cached.iter().collect();
        let matched = self.matched;
        
        if let Some(last) = self.chunks.last_mut() {
            if last.matched == matched {
                last.data.push_str(&data);
                self.cached.clear();
                return;
            }
        }
        
        self.chunks.push(XmlMatcherResult { matched, data });
        self.cached.clear();
    }
    
    fn pop(&mut self) -> Vec<XmlMatcherResult> {
        std::mem::take(&mut self.chunks)
    }
}
```

## Git Operations (358 LINES)

### Repository Info
```typescript
export interface GitRepositoryInfo {
    repositoryUrl?: string
    repositoryName?: string
    defaultBranch?: string
}

export async function getGitRepositoryInfo(workspaceRoot: string): Promise<GitRepositoryInfo> {
    const gitDir = path.join(workspaceRoot, ".git")
    
    // Check if git repo
    if (!await fs.access(gitDir)) {
        return {}
    }
    
    // Read config
    const configPath = path.join(gitDir, "config")
    const configContent = await fs.readFile(configPath, "utf8")
    
    // Extract URL
    const urlMatch = configContent.match(/url\s*=\s*(.+?)(?:\r?\n|$)/m)
    if (urlMatch) {
        const url = convertGitUrlToHttps(sanitizeGitUrl(urlMatch[1]))
        const repoName = extractRepositoryName(url)
        return { repositoryUrl: url, repositoryName: repoName }
    }
    
    return {}
}
```

### Commit Search
```typescript
export interface GitCommit {
    hash: string
    shortHash: string
    subject: string
    author: string
    date: string
}

export async function searchGitCommits(
    workspaceRoot: string,
    query: string,
    limit: number = 10
): Promise<GitCommit[]> {
    const format = "--format=%H|%h|%s|%an|%ai"
    const cmd = `git log ${format} --grep="${query}" -n ${limit}`
    
    const { stdout } = await execAsync(cmd, { cwd: workspaceRoot })
    
    return stdout.trim().split("\n").map(line => {
        const [hash, shortHash, subject, author, date] = line.split("|")
        return { hash, shortHash, subject, author, date }
    })
}
```

**RUST:**
```rust
use git2::{Repository, Oid};

pub struct GitRepositoryInfo {
    pub repository_url: Option<String>,
    pub repository_name: Option<String>,
    pub default_branch: Option<String>,
}

pub fn get_git_repository_info(workspace_root: &Path) -> Result<GitRepositoryInfo> {
    let repo = Repository::open(workspace_root)?;
    
    // Get remote URL
    let remote = repo.find_remote("origin")?;
    let url = remote.url().map(|s| s.to_string());
    
    let repository_name = url.as_ref()
        .and_then(|u| extract_repository_name(u));
    
    // Get default branch
    let head = repo.head()?;
    let default_branch = head.shorthand().map(|s| s.to_string());
    
    Ok(GitRepositoryInfo {
        repository_url: url,
        repository_name,
        default_branch,
    })
}

pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub author: String,
    pub date: String,
}

pub fn search_git_commits(
    workspace_root: &Path,
    query: &str,
    limit: usize
) -> Result<Vec<GitCommit>> {
    let repo = Repository::open(workspace_root)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    
    let mut commits = Vec::new();
    
    for oid in revwalk.take(1000) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("");
        
        if message.contains(query) {
            commits.push(GitCommit {
                hash: oid.to_string(),
                short_hash: format!("{:.7}", oid),
                subject: message.lines().next().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                date: commit.time().seconds().to_string(),
            });
            
            if commits.len() >= limit {
                break;
            }
        }
    }
    
    Ok(commits)
}
```

## Shell Detection (228 LINES)

**Purpose:** Detect user's preferred shell across Windows/Mac/Linux

```typescript
const SHELL_PATHS = {
    POWERSHELL_7: "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
    POWERSHELL_LEGACY: "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
    CMD: "C:\\Windows\\System32\\cmd.exe",
    MAC_DEFAULT: "/bin/zsh",
    LINUX_DEFAULT: "/bin/bash",
    ZSH: "/bin/zsh",
    BASH: "/bin/bash",
}

export function getShellPath(): string {
    const platform = process.platform
    
    if (platform === "win32") {
        return getWindowsShell()
    } else if (platform === "darwin") {
        return getMacShell()
    } else {
        return getLinuxShell()
    }
}

function getWindowsShell(): string {
    // Check VS Code config
    const config = vscode.workspace.getConfiguration("terminal.integrated")
    const profile = config.get<string>("defaultProfile.windows")
    
    if (profile?.includes("PowerShell")) {
        return fs.existsSync(SHELL_PATHS.POWERSHELL_7) 
            ? SHELL_PATHS.POWERSHELL_7 
            : SHELL_PATHS.POWERSHELL_LEGACY
    }
    
    return SHELL_PATHS.CMD
}
```

**RUST:**
```rust
pub fn get_shell_path() -> String {
    if cfg!(target_os = "windows") {
        get_windows_shell()
    } else if cfg!(target_os = "macos") {
        get_mac_shell()
    } else {
        get_linux_shell()
    }
}

fn get_windows_shell() -> String {
    // Check if PowerShell 7 exists
    if Path::new("C:\\Program Files\\PowerShell\\7\\pwsh.exe").exists() {
        return "C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_string();
    }
    
    // Fall back to legacy PowerShell
    "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".to_string()
}

fn get_mac_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

fn get_linux_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
}
```

## Path Utilities (133 LINES)

**Cross-platform path handling:**

```typescript
// Convert Windows paths to POSIX for display
function toPosixPath(p: string) {
    return p.replace(/\\/g, "/")
}

// Safe path comparison (case-insensitive on Windows)
export function arePathsEqual(path1?: string, path2?: string): boolean {
    if (!path1 && !path2) return true
    if (!path1 || !path2) return false
    
    path1 = normalizePath(path1)
    path2 = normalizePath(path2)
    
    if (process.platform === "win32") {
        return path1.toLowerCase() === path2.toLowerCase()
    }
    return path1 === path2
}

export function getReadablePath(cwd: string, relPath?: string): string {
    const absolutePath = path.resolve(cwd, relPath || "")
    
    if (arePathsEqual(absolutePath, cwd)) {
        return path.basename(absolutePath).toPosix()
    } else {
        const normalizedRelPath = path.relative(cwd, absolutePath)
        if (absolutePath.includes(cwd)) {
            return normalizedRelPath.toPosix()
        } else {
            return absolutePath.toPosix()
        }
    }
}
```

**RUST:**
```rust
pub fn to_posix_path(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn are_paths_equal(path1: Option<&str>, path2: Option<&str>) -> bool {
    match (path1, path2) {
        (None, None) => true,
        (None, Some(_)) | (Some(_), None) => false,
        (Some(p1), Some(p2)) => {
            let p1 = normalize_path(p1);
            let p2 = normalize_path(p2);
            
            if cfg!(target_os = "windows") {
                p1.to_lowercase() == p2.to_lowercase()
            } else {
                p1 == p2
            }
        }
    }
}

pub fn get_readable_path(cwd: &Path, rel_path: Option<&str>) -> String {
    let absolute_path = if let Some(rel) = rel_path {
        cwd.join(rel)
    } else {
        cwd.to_path_buf()
    };
    
    if absolute_path == cwd {
        absolute_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    } else if absolute_path.starts_with(cwd) {
        absolute_path.strip_prefix(cwd)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        absolute_path.to_str().unwrap_or("").to_string()
    }
}
```

## File System Operations (110 LINES)

```typescript
export async function createDirectoriesForFile(filePath: string): Promise<string[]> {
    const newDirectories: string[] = []
    const directoryPath = path.dirname(filePath)
    
    let currentPath = directoryPath
    const dirsToCreate: string[] = []
    
    // Traverse up and collect missing directories
    while (!(await fileExistsAtPath(currentPath))) {
        dirsToCreate.push(currentPath)
        currentPath = path.dirname(currentPath)
    }
    
    // Create from top down
    for (let i = dirsToCreate.length - 1; i >= 0; i--) {
        await fs.mkdir(dirsToCreate[i])
        newDirectories.push(dirsToCreate[i])
    }
    
    return newDirectories
}

export async function readDirectory(
    directoryPath: string, 
    excludedPaths: string[][] = []
): Promise<string[]> {
    const entries = await fs.readdir(directoryPath, { 
        withFileTypes: true, 
        recursive: true 
    })
    
    return entries
        .filter(e => !OS_GENERATED_FILES.includes(e.name))
        .filter(e => e.isFile())
        .map(e => path.resolve(e.parentPath, e.name))
        .filter(filePath => !isExcluded(filePath, excludedPaths))
}
```

**RUST:**
```rust
use tokio::fs;

pub async fn create_directories_for_file(file_path: &Path) -> Result<Vec<PathBuf>> {
    let mut new_directories = Vec::new();
    let directory_path = file_path.parent()
        .ok_or_else(|| anyhow!("No parent directory"))?;
    
    let mut current_path = directory_path.to_path_buf();
    let mut dirs_to_create = Vec::new();
    
    // Collect missing directories
    while !current_path.exists() {
        dirs_to_create.push(current_path.clone());
        current_path = current_path.parent()
            .ok_or_else(|| anyhow!("No parent"))?
            .to_path_buf();
    }
    
    // Create from top down
    for dir in dirs_to_create.iter().rev() {
        fs::create_dir(dir).await?;
        new_directories.push(dir.clone());
    }
    
    Ok(new_directories)
}

pub async fn read_directory(
    directory_path: &Path,
    excluded_paths: &[Vec<String>]
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut walker = WalkDir::new(directory_path);
    
    while let Some(entry) = walker.next().await {
        let entry = entry?;
        
        if !entry.file_type().is_file() {
            continue;
        }
        
        let path = entry.path();
        if is_excluded(path, excluded_paths) {
            continue;
        }
        
        files.push(path.to_path_buf());
    }
    
    Ok(files)
}
```

## Tiktoken (Token Counting)

```typescript
import { get_encoding } from "tiktoken"

export function countTokens(text: string): number {
    const encoding = get_encoding("cl100k_base")
    return encoding.encode(text).length
}
```

**RUST:**
```rust
use tiktoken_rs::cl100k_base;

pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    bpe.encode_ordinary(text).len()
}
```

## Summary: Critical Utilities

| Utility | Lines | Priority | Rust Crate |
|---------|-------|----------|-----------|
| XmlMatcher | 105 | **CRITICAL** | Custom implementation |
| Git operations | 358 | High | git2 |
| Shell detection | 228 | High | Custom + std::env |
| Path handling | 133 | High | std::path |
| File system | 110 | High | tokio::fs |
| Tiktoken | ~50 | High | tiktoken-rs |
| Storage | ~100 | Medium | serde_json + fs |

## Next: CHUNK 11 - integrations/
Terminal, editor, theme, misc utilities.
