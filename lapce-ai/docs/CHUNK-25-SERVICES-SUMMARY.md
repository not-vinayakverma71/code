# CHUNK-25: SERVICES LAYER - COMPLETE DEEP ANALYSIS (13 SUBDIRECTORIES)

## 📁 COMPLETE SERVICE STRUCTURE

```
Codex/src/services/ (200+ files total)
├── browser/           (5 files)   - Puppeteer web scraping
├── checkpoints/       (7 files)   - Git-based task versioning  
├── code-index/        (45 files)  - Semantic code search with embeddings
├── command/           (2 files)   - Custom command system
├── commit-message/    (7 files)   - AI commit message generation
├── glob/              (7 files)   - File pattern matching & listing
├── mdm/               (2 files)   - Mobile Device Management compliance
├── mocking/           (4 files)   - Test mocks for VSCode API
├── ripgrep/           (2 files)   - Fast regex search wrapper
├── roo-config/        (2 files)   - Configuration directory management
├── search/            (1 file)    - Fuzzy file search
├── terminal-welcome/  (1 file)    - Terminal welcome messages
└── tree-sitter/       (124 files) - Syntax parsing (40+ languages)
```

**Total Lines Analyzed**: ~3,000+ lines across 13 services
**Integration Points**: All services used by core modules for AI context injection

---

# 🔧 SERVICE 1: browser/ - WEB SCRAPING (144 lines)

## Purpose
Fetch web content and convert to markdown for AI context (powers @url mentions).

## Core: UrlContentFetcher.ts

```typescript
export class UrlContentFetcher {
    private browser?: Browser
    private page?: Page
    
    async launchBrowser(): Promise<void> {
        const stats = await this.ensureChromiumExists()  // Downloads if needed
        this.browser = await stats.puppeteer.launch({
            args: [
                '--user-agent=Mozilla/5.0...',
                '--disable-dev-shm-usage',
                '--no-sandbox',  // Linux fix for permissions
            ],
            executablePath: stats.executablePath,
        })
        this.page = await this.browser.newPage()
        await this.page.setViewport({ width: 1280, height: 720 })
    }
    
    async urlToMarkdown(url: string): Promise<string> {
        // Try with full page load
        await this.page.goto(url, {
            timeout: 30_000,
            waitUntil: ['domcontentloaded', 'networkidle2'],
        })
        
        const content = await this.page.content()
        
        // Clean HTML
        const $ = cheerio.load(content)
        $('script, style, nav, footer, header').remove()
        
        // Convert to markdown
        const turndownService = new TurndownService()
        return turndownService.turndown($.html())
    }
}
```

**Chromium Storage**: `globalStorageUri/puppeteer/.chromium-browser-snapshots`

**Fallback**: If `networkidle2` times out → Retry with only `domcontentloaded`

### Rust Translation

```rust
use headless_chrome::Browser;
use html2md;

pub struct UrlContentFetcher;

impl UrlContentFetcher {
    pub async fn url_to_markdown(&self, url: &str) -> Result<String> {
        let browser = Browser::default()?;
        let tab = browser.wait_for_initial_tab()?;
        
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;
        
        let html = tab.get_content()?;
        let cleaned = Self::remove_elements(&html, &["script", "style", "nav"]);
        
        Ok(html2md::parse_html(&cleaned))
    }
}
```

**Crates**: `headless_chrome`, `html2md`, `scraper`

---

# 🔧 SERVICE 2: checkpoints/ - GIT VERSIONING (7 files)

## Purpose
Create git checkpoints per AI task for undo/restore functionality.

## Architecture

```typescript
export class RepoPerTaskCheckpointService extends ShadowCheckpointService {
    public static create({ taskId, workspaceDir, shadowDir }: Options) {
        return new RepoPerTaskCheckpointService(
            taskId,
            path.join(shadowDir, 'tasks', taskId, 'checkpoints'),
            workspaceDir,
        )
    }
}
```

**Directory Structure**:
```
shadowDir/
└── tasks/
    └── <taskId>/
        └── checkpoints/
            ├── .git/
            └── [workspace files mirror]
```

**Each task gets isolated checkpoint history**.

### Rust Translation

```rust
use git2::{Repository, Signature, IndexAddOption};

pub struct CheckpointService {
    repo_path: PathBuf,
}

impl CheckpointService {
    pub fn create_checkpoint(&self, message: &str) -> Result<Oid> {
        let repo = Repository::open(&self.repo_path)?;
        let mut index = repo.index()?;
        
        index.add_all(["."], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = Signature::now("AI Assistant", "ai@lapce.dev")?;
        let parent = repo.head()?.peel_to_commit()?;
        
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
    }
}
```

**Crate**: `git2`

---

# 🔧 SERVICE 3: code-index/ - SEMANTIC SEARCH (45 files)

## Purpose
Index codebase with embeddings for semantic search (@codebase feature).

## Architecture

```
code-index/
├── manager.ts              - Singleton coordinator
├── orchestrator.ts         - Indexing workflow
├── search-service.ts       - Vector search
├── scanner.ts              - File discovery
├── cache-manager.ts        - Embedding cache
├── embedders/
│   ├── openai.ts          - OpenAI API
│   ├── ollama.ts          - Local Ollama
│   ├── gemini.ts          - Google Gemini
│   └── mistral.ts         - Mistral AI
├── processors/
│   ├── scanner.ts         - Directory scanning
│   ├── parser.ts          - Code parsing
│   └── file-watcher.ts    - Incremental updates
└── vector-store/
    └── qdrant-client.ts   - Qdrant integration
```

## Workflow

### 1. Manager (Singleton per workspace)
```typescript
export class CodeIndexManager {
    private static instances = new Map<string, CodeIndexManager>()
    
    public static getInstance(context, workspacePath) {
        if (!this.instances.has(workspacePath)) {
            this.instances.set(workspacePath, new CodeIndexManager(workspacePath, context))
        }
        return this.instances.get(workspacePath)!
    }
}
```

### 2. Orchestrator - Indexing
```typescript
public async startIndexing(): Promise<void> {
    this.stateManager.setSystemState('Indexing', 'Scanning...')
    
    const { stats, totalBlockCount } = await this.scanner.scanDirectory(
        this.workspacePath,
        (error) => this.handleError(error),
        (count) => this.updateProgress(count),
    )
    
    await this._startWatcher()  // Incremental updates
    
    this.stateManager.setSystemState('Indexed', 'Complete')
}
```

### 3. Scanner - Embed & Store
```typescript
public async scanDirectory(dir: string): Promise<ScanResult> {
    const [allPaths] = await listFiles(dir, true, MAX_LIMIT)
    const supportedPaths = allPaths.filter(f => 
        scannerExtensions.includes(path.extname(f))
    )
    
    const blocks: CodeBlock[] = []
    for (const file of supportedPaths) {
        blocks.push(...await this.codeParser.parseFile(file))
    }
    
    for (const batch of chunk(blocks, BATCH_SIZE)) {
        const embeddings = await this.embedder.createEmbeddings(
            batch.map(b => b.content)
        )
        await this.vectorStore.upsert(batch, embeddings)
    }
    
    return { stats, totalBlockCount: blocks.length }
}
```

### 4. Search Service
```typescript
public async searchIndex(query: string, directoryPrefix?: string) {
    const embeddingResponse = await this.embedder.createEmbeddings([query])
    const vector = embeddingResponse.embeddings[0]
    
    return await this.vectorStore.search(
        vector,
        directoryPrefix,
        minScore,
        maxResults
    )
}
```

### 5. Qdrant Vector Store
```typescript
export class QdrantVectorStore implements IVectorStore {
    constructor(workspacePath: string, url: string, vectorSize: number) {
        this.client = new QdrantClient({ host: url, port: 6333 })
        
        // Workspace-specific collection
        const hash = createHash('sha256').update(workspacePath).digest('hex')
        this.collectionName = `ws-${hash.substring(0, 16)}`
    }
    
    async search(vector: number[], prefix?: string, minScore, limit) {
        const filter = prefix ? {
            must: [{ key: 'relativePath', match: { value: `${prefix}*` } }]
        } : undefined
        
        return await this.client.search(this.collectionName, {
            vector,
            filter,
            limit,
            scoreThreshold: minScore,
        })
    }
}
```

## Embedding Providers

**OpenAI**:
```typescript
async createEmbeddings(texts: string[], model = 'text-embedding-3-small') {
    const response = await openai.embeddings.create({ model, input: texts })
    return {
        embeddings: response.data.map(d => d.embedding),
        usage: response.usage,
    }
}
```

**Ollama (Local)**:
```typescript
async createEmbeddings(texts: string[], model = 'nomic-embed-text') {
    const embeddings = []
    for (const text of texts) {
        const response = await fetch(`${this.baseUrl}/api/embeddings`, {
            method: 'POST',
            body: JSON.stringify({ model, prompt: text }),
        })
        embeddings.push((await response.json()).embedding)
    }
    return { embeddings }
}
```

### Rust Translation

```rust
use qdrant_client::prelude::*;

pub struct CodeIndexManager {
    qdrant: QdrantClient,
    embedder: Box<dyn Embedder>,
    collection_name: String,
}

impl CodeIndexManager {
    pub async fn index_workspace(&self, path: &Path) -> Result<()> {
        let files = self.scan_files(path).await?;
        
        for chunk in files.chunks(100) {
            let texts: Vec<_> = chunk.iter()
                .map(|f| std::fs::read_to_string(f).unwrap())
                .collect();
            
            let embeddings = self.embedder.embed(&texts).await?;
            
            let points: Vec<PointStruct> = chunk.iter()
                .zip(embeddings)
                .map(|(file, emb)| PointStruct::new(
                    Uuid::new_v4().to_string(),
                    emb,
                    json!({ "path": file.display().to_string() })
                ))
                .collect();
            
            self.qdrant.upsert_points_blocking(&self.collection_name, points, None).await?;
        }
        Ok(())
    }
    
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_emb = self.embedder.embed(&[query.to_string()]).await?;
        
        let results = self.qdrant.search_points(&SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: query_emb[0].clone(),
            limit: limit as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        }).await?;
        
        Ok(results.result.into_iter().map(|r| SearchResult {
            path: r.payload["path"].as_str().unwrap().to_string(),
            score: r.score,
        }).collect())
    }
}
```

**Crates**: `qdrant-client`, `tokio`, `serde_json`

**Embedder Options**:
- ONNX Runtime with downloaded model
- Call Python service via IPC
- Use Ollama locally

---

# 🔧 SERVICE 4: command/ - CUSTOM COMMANDS (207 lines)

## Purpose
User-defined markdown commands from `.kilocode/commands/*.md`.

## Core: commands.ts

```typescript
export interface Command {
    name: string
    content: string
    source: 'global' | 'project'  // Project overrides global
    filePath: string
    description?: string          // From frontmatter
    argumentHint?: string         // From frontmatter
}

export async function getCommand(cwd: string, name: string): Promise<Command> {
    // 1. Check project commands first (override)
    const projectDir = path.join(getProjectRooDirectoryForCwd(cwd), 'commands')
    const projectCommand = await tryLoadCommand(projectDir, name, 'project')
    if (projectCommand) return projectCommand
    
    // 2. Fallback to global commands
    const globalDir = path.join(getGlobalRooDirectory(), 'commands')
    return await tryLoadCommand(globalDir, name, 'global')
}

async function tryLoadCommand(dirPath, name, source) {
    const filePath = path.join(dirPath, `${name}.md`)
    const content = await fs.readFile(filePath, 'utf-8')
    
    const parsed = matter(content)  // Parse frontmatter
    
    return {
        name,
        content: parsed.content.trim(),
        source,
        filePath,
        description: parsed.data.description,
        argumentHint: parsed.data['argument-hint'],
    }
}
```

**Example** (`.kilocode/commands/test.md`):
```markdown
---
description: Run tests with coverage
argument-hint: [pattern]
---

1. Unit tests: `cargo test`
2. Integration tests: `cargo test --test '*'`
3. Check coverage > 80%
```

### Rust Translation

```rust
use gray_matter::Matter;

pub struct Command {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
}

impl Command {
    pub fn load(path: &Path, name: &str) -> Result<Self> {
        let file_path = path.join(format!("{}.md", name));
        let content = std::fs::read_to_string(&file_path)?;
        
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(&content);
        
        let frontmatter: CommandFrontmatter = parsed.data
            .map(|d| serde_yaml::from_str(&d.to_string()).ok())
            .flatten()
            .unwrap_or_default();
        
        Ok(Command {
            name: name.to_string(),
            content: parsed.content.trim().to_string(),
            description: frontmatter.description,
        })
    }
}
```

**Crates**: `gray_matter`, `serde_yaml`

---

# 🔧 SERVICE 5: commit-message/ - AI COMMITS (286 lines)

## Purpose
Generate conventional commit messages from git diffs using AI.

## Core: CommitMessageProvider.ts

```typescript
export class CommitMessageProvider {
    public async generateCommitMessage(repo?: GitRepository): Promise<void> {
        // 1. Gather staged changes
        let staged = true
        let changes = await this.gitService.gatherChanges({ staged })
        
        // 2. Fallback to unstaged if empty
        if (changes.length === 0) {
            staged = false
            changes = await this.gitService.gatherChanges({ staged: false })
        }
        
        // 3. Get diff context
        const gitContextString = await this.gitService.getCommitContext(changes, {
            staged,
            onProgress: (pct) => progress.report({ increment: pct }),
        })
        
        // 4. Generate with AI
        const commitMessage = await singleCompletionHandler({
            systemPrompt: this.buildSystemPrompt(),
            userMessage: `Generate commit for:\n${gitContextString}`,
            provider: this.providerSettingsManager.getSettings(),
        })
        
        // 5. Set in git UI
        this.gitService.setCommitMessage(commitMessage)
    }
}
```

**System Prompt Template**:
```
Generate concise conventional commit messages:
- Format: <type>(<scope>): <subject>
- Types: feat, fix, docs, refactor, test, chore
- Subject: imperative, lowercase, no period
```

### Rust Translation

```rust
use git2::Repository;

pub struct CommitMessageGenerator {
    repo: Repository,
    ai_client: AIClient,
}

impl CommitMessageGenerator {
    pub async fn generate(&self) -> Result<String> {
        let diff = self.get_staged_diff()?;
        
        let prompt = format!(
            "Generate conventional commit for:\n{}",
            diff
        );
        
        Ok(self.ai_client.complete(&prompt).await?)
    }
    
    fn get_staged_diff(&self) -> Result<String> {
        let head = self.repo.head()?.peel_to_tree()?;
        let index = self.repo.index()?;
        
        let diff = self.repo.diff_tree_to_index(
            Some(&head),
            Some(&index),
            None,
        )?;
        
        let mut output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            output.push_str(std::str::from_utf8(line.content()).unwrap());
            true
        })?;
        
        Ok(output)
    }
}
```

**Crate**: `git2`

---

# 🔧 SERVICE 6: glob/ - FILE LISTING (700 lines)

## Purpose
Fast file discovery with gitignore support using ripgrep.

## Core: list-files.ts

```typescript
export async function listFiles(
    dirPath: string,
    recursive: boolean,
    limit: number
): Promise<[string[], boolean]> {
    // 1. Get ripgrep path from VSCode
    const rgPath = await getRipgrepPath()
    
    // 2. Build ripgrep arguments
    const args = [
        '--files',
        '--hidden',
        '--follow',  // Follow symlinks
    ]
    
    if (recursive) {
        // Respect .gitignore
        args.push(...[
            '-g', '!**/.git/**',
            '-g', '!**/node_modules/**',
            '-g', '!**/.*/**',  // Hidden dirs
        ])
    } else {
        args.push('--maxdepth', '1')
    }
    
    // 3. Execute ripgrep
    const files = await execRipgrep(rgPath, args, limit)
    
    // 4. List directories separately
    const ignoreInstance = await createIgnoreInstance(dirPath)
    const directories = await listFilteredDirectories(dirPath, recursive, ignoreInstance)
    
    // 5. Combine and deduplicate
    return formatAndCombineResults(files, directories, limit)
}
```

**Gitignore Handling**:
```typescript
async function createIgnoreInstance(dirPath: string) {
    const ignoreInstance = ignore()
    
    // Find all .gitignore files up the tree
    const gitignoreFiles = await findGitignoreFiles(dirPath)
    
    for (const file of gitignoreFiles) {
        const content = await fs.readFile(file, 'utf8')
        ignoreInstance.add(content)
    }
    
    return ignoreInstance
}
```

**Ripgrep Execution**:
```typescript
async function execRipgrep(rgPath: string, args: string[], limit: number) {
    return new Promise((resolve) => {
        const rgProcess = childProcess.spawn(rgPath, args)
        let output = ''
        let results: string[] = []
        
        rgProcess.stdout.on('data', (data) => {
            output += data.toString()
            // Process line by line
            const lines = output.split('\n')
            output = lines.pop() || ''
            
            for (const line of lines) {
                if (line.trim() && results.length < limit) {
                    results.push(line)
                }
            }
            
            if (results.length >= limit) {
                rgProcess.kill()
            }
        })
        
        rgProcess.on('close', () => resolve(results.slice(0, limit)))
    })
}
```

### Rust Translation

```rust
use ignore::Walk;

pub fn list_files(dir: &Path, recursive: bool, limit: usize) -> Vec<PathBuf> {
    let walker = if recursive {
        Walk::new(dir)
    } else {
        Walk::new(dir).max_depth(Some(1))
    };
    
    walker
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .take(limit)
        .map(|e| e.path().to_path_buf())
        .collect()
}
```

**Crate**: `ignore` (respects .gitignore automatically)

---

# 🔧 SERVICE 7: mdm/ - DEVICE MANAGEMENT (205 lines)

## Purpose
Mobile Device Management compliance checks for enterprise deployments.

## Core: MdmService.ts

```typescript
export class MdmService {
    private mdmConfig: MdmConfig | null = null
    
    public async initialize(): Promise<void> {
        this.mdmConfig = await this.loadMdmConfig()
    }
    
    public requiresCloudAuth(): boolean {
        return this.mdmConfig?.requireCloudAuth ?? false
    }
    
    public isCompliant(): ComplianceResult {
        if (!this.requiresCloudAuth()) {
            return { compliant: true }
        }
        
        if (!CloudService.hasInstance() || !CloudService.instance.hasOrIsAcquiringActiveSession()) {
            return { compliant: false, reason: 'Cloud auth required' }
        }
        
        const requiredOrgId = this.mdmConfig?.organizationId
        if (requiredOrgId) {
            const currentOrgId = CloudService.instance.getOrganizationId()
            if (currentOrgId !== requiredOrgId) {
                return { compliant: false, reason: 'Organization mismatch' }
            }
        }
        
        return { compliant: true }
    }
    
    private getMdmConfigPath(): string {
        const platform = os.platform()
        const configFileName = isProduction ? 'mdm.json' : 'mdm.dev.json'
        
        switch (platform) {
            case 'win32':
                return path.join(process.env.PROGRAMDATA || 'C:\\ProgramData', 'RooCode', configFileName)
            case 'darwin':
                return `/Library/Application Support/RooCode/${configFileName}`
            default:
                return `/etc/roo-code/${configFileName}`
        }
    }
}
```

**Config Schema**:
```json
{
  "requireCloudAuth": true,
  "organizationId": "org_abc123"
}
```

### Rust Translation

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct MdmConfig {
    require_cloud_auth: bool,
    organization_id: Option<String>,
}

pub struct MdmService {
    config: Option<MdmConfig>,
}

impl MdmService {
    pub fn is_compliant(&self) -> Result<(), String> {
        let config = match &self.config {
            Some(c) if c.require_cloud_auth => c,
            _ => return Ok(()),
        };
        
        if !self.has_active_session() {
            return Err("Cloud auth required".to_string());
        }
        
        if let Some(required_org) = &config.organization_id {
            let current_org = self.get_current_org_id()?;
            if &current_org != required_org {
                return Err("Organization mismatch".to_string());
            }
        }
        
        Ok(())
    }
    
    fn config_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\ProgramData\RooCode\mdm.json")
        } else if cfg!(target_os = "macos") {
            PathBuf::from("/Library/Application Support/RooCode/mdm.json")
        } else {
            PathBuf::from("/etc/roo-code/mdm.json")
        }
    }
}
```

**Crate**: `serde_json`

---

# 🔧 SERVICE 8: mocking/ - TEST MOCKS (4 files)

## Purpose
Mock VSCode API objects for unit testing.

## Core Mocks

**MockTextDocument**:
```typescript
export class MockTextDocument implements vscode.TextDocument {
    uri: vscode.Uri
    fileName: string
    languageId: string
    version: number = 1
    lineCount: number
    private lines: string[]
    
    constructor(content: string, fileName: string = 'test.ts') {
        this.lines = content.split('\n')
        this.lineCount = this.lines.length
        this.fileName = fileName
        this.uri = vscode.Uri.file(fileName)
        this.languageId = path.extname(fileName).slice(1)
    }
    
    getText(range?: vscode.Range): string {
        if (!range) return this.lines.join('\n')
        
        const startLine = range.start.line
        const endLine = range.end.line
        return this.lines.slice(startLine, endLine + 1).join('\n')
    }
    
    lineAt(position: number | vscode.Position): vscode.TextLine {
        const lineNum = typeof position === 'number' ? position : position.line
        return new MockTextLine(this.lines[lineNum], lineNum)
    }
}
```

**MockTextEditor**:
```typescript
export class MockTextEditor implements vscode.TextEditor {
    document: MockTextDocument
    selection: vscode.Selection
    selections: vscode.Selection[]
    visibleRanges: vscode.Range[]
    
    constructor(document: MockTextDocument) {
        this.document = document
        this.selection = new vscode.Selection(0, 0, 0, 0)
        this.selections = [this.selection]
        this.visibleRanges = [new vscode.Range(0, 0, document.lineCount - 1, 0)]
    }
    
    async edit(callback: (editBuilder: vscode.TextEditorEdit) => void): Promise<boolean> {
        // Mock implementation
        return true
    }
}
```

### Rust Translation

**Not needed** - Use standard Rust testing with mock traits.

---

# 🔧 SERVICE 9: ripgrep/ - REGEX SEARCH (264 lines)

## Purpose
Fast regex search across files using VSCode's bundled ripgrep.

## Core: index.ts

```typescript
export async function regexSearchFiles(
    cwd: string,
    directoryPath: string,
    regex: string,
    filePattern?: string,
    rooIgnoreController?: RooIgnoreController,
): Promise<string> {
    const vscodeAppRoot = vscode.env.appRoot
    const rgPath = await getBinPath(vscodeAppRoot)
    
    const args = [
        '--json',
        '-e', regex,
        '--glob', filePattern || '*',
        '--context', '1',  // 1 line before/after
        '--no-messages',
        directoryPath,
    ]
    
    const output = await execRipgrep(rgPath, args)
    
    // Parse JSON output
    const results: SearchFileResult[] = []
    let currentFile: SearchFileResult | null = null
    
    output.split('\n').forEach((line) => {
        const parsed = JSON.parse(line)
        
        if (parsed.type === 'begin') {
            currentFile = { file: parsed.data.path.text, searchResults: [] }
        } else if (parsed.type === 'match') {
            currentFile?.searchResults.push({
                line: parsed.data.line_number,
                text: truncateLine(parsed.data.lines.text),
                isMatch: true,
            })
        } else if (parsed.type === 'end') {
            results.push(currentFile!)
            currentFile = null
        }
    })
    
    // Filter using RooIgnoreController
    const filteredResults = rooIgnoreController
        ? results.filter(r => rooIgnoreController.validateAccess(r.file))
        : results
    
    return formatResults(filteredResults, cwd)
}
```

**Output Format**:
```
Found 3 results.

# src/main.rs
 42 | fn process_data(input: &str) -> Result<String> {
 43 |     // TODO: Add error handling
 44 |     Ok(input.to_string())
----

# src/utils.rs
 15 | // TODO: Optimize this function
 16 | fn calculate(x: i32) -> i32 {
----
```

### Rust Translation

```rust
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use grep::searcher::sinks::UTF8;

pub fn regex_search_files(
    dir: &Path,
    pattern: &str,
    file_pattern: Option<&str>,
) -> Result<Vec<SearchResult>> {
    let matcher = RegexMatcher::new(pattern)?;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\0'))
        .line_number(true)
        .build();
    
    let mut results = Vec::new();
    
    for entry in WalkBuilder::new(dir).build() {
        let entry = entry?;
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        
        searcher.search_path(
            &matcher,
            entry.path(),
            UTF8(|lnum, line| {
                results.push(SearchResult {
                    file: entry.path().to_path_buf(),
                    line_num: lnum as usize,
                    text: line.to_string(),
                });
                Ok(true)
            }),
        )?;
    }
    
    Ok(results)
}
```

**Crates**: `grep`, `walkdir`, `regex`

---

# 🔧 SERVICE 10: roo-config/ - CONFIG MANAGEMENT (261 lines)

## Purpose
Manage global and project-local `.kilocode` directories for configuration.

## Core: index.ts

```typescript
export function getGlobalRooDirectory(): string {
    const homeDir = os.homedir()
    return path.join(homeDir, '.kilocode')
}

export function getProjectRooDirectoryForCwd(cwd: string): string {
    const kiloDir = path.join(cwd, '.kilocode')
    const rooDir = path.join(cwd, '.roo')
    
    // Backward compatibility: check if .roo exists but .kilocode doesn't
    if (fs.existsSync(rooDir) && !fs.existsSync(kiloDir)) {
        return rooDir
    }
    return kiloDir
}

export async function loadConfiguration(
    relativePath: string,
    cwd: string,
): Promise<{ global: string | null; project: string | null; merged: string }> {
    const globalDir = getGlobalRooDirectory()
    const projectDir = getProjectRooDirectoryForCwd(cwd)
    
    const globalFilePath = path.join(globalDir, relativePath)
    const projectFilePath = path.join(projectDir, relativePath)
    
    const globalContent = await readFileIfExists(globalFilePath)
    const projectContent = await readFileIfExists(projectFilePath)
    
    // Merge: project overrides global
    let merged = ''
    if (globalContent) merged += globalContent
    if (projectContent) {
        if (merged) merged += '\n\n# Project-specific rules (override global):\n\n'
        merged += projectContent
    }
    
    return { global: globalContent, project: projectContent, merged }
}
```

**Directory Structure**:
```
~/.kilocode/               # Global config
├── commands/
├── rules/rules.md
└── custom-instructions.md

project/.kilocode/         # Project-local (overrides global)
├── commands/
├── rules/rules.md
└── custom-instructions.md
```

### Rust Translation

```rust
use std::path::{Path, PathBuf};

pub fn get_global_roo_directory() -> PathBuf {
    dirs::home_dir()
        .expect("Home directory not found")
        .join(".kilocode")
}

pub fn get_project_roo_directory(cwd: &Path) -> PathBuf {
    cwd.join(".kilocode")
}

pub async fn load_configuration(
    relative_path: &str,
    cwd: &Path,
) -> Result<(Option<String>, Option<String>, String)> {
    let global_path = get_global_roo_directory().join(relative_path);
    let project_path = get_project_roo_directory(cwd).join(relative_path);
    
    let global_content = tokio::fs::read_to_string(&global_path).await.ok();
    let project_content = tokio::fs::read_to_string(&project_path).await.ok();
    
    let mut merged = String::new();
    if let Some(ref g) = global_content {
        merged.push_str(g);
    }
    if let Some(ref p) = project_content {
        if !merged.is_empty() {
            merged.push_str("\n\n# Project-specific rules:\n\n");
        }
        merged.push_str(p);
    }
    
    Ok((global_content, project_content, merged))
}
```

**Crates**: `dirs`, `tokio`

---

# 🔧 SERVICE 11: search/ - FUZZY FILE SEARCH (164 lines)

## Purpose
Fuzzy file/folder search using fzf algorithm.

## Core: file-search.ts

```typescript
export async function searchWorkspaceFiles(
    query: string,
    workspacePath: string,
    limit: number = 20,
): Promise<FileResult[]> {
    // 1. Get all files using ripgrep
    const allItems = await executeRipgrepForFiles(workspacePath, 5000)
    
    if (!query.trim()) {
        return allItems.slice(0, limit)
    }
    
    // 2. Create search items with path + label
    const searchItems = allItems.map((item) => ({
        original: item,
        searchStr: `${item.path} ${item.label || ''}`,
    }))
    
    // 3. Fuzzy search with fzf
    const fzf = new Fzf(searchItems, {
        selector: (item) => item.searchStr,
        tiebreakers: [byLengthAsc],
        limit,
    })
    
    const fzfResults = fzf.find(query).map((result) => result.item.original)
    
    // 4. Verify types (file vs folder)
    const verifiedResults = await Promise.all(
        fzfResults.map(async (result) => {
            const fullPath = path.join(workspacePath, result.path)
            if (fs.existsSync(fullPath)) {
                const isDirectory = fs.lstatSync(fullPath).isDirectory()
                return {
                    ...result,
                    type: isDirectory ? 'folder' : 'file',
                }
            }
            return result
        })
    )
    
    return verifiedResults
}
```

**Example**:
```typescript
const results = await searchWorkspaceFiles('main.rs', '/project', 10)
// Returns: [{ path: 'src/main.rs', type: 'file', label: 'main.rs' }, ...]
```

### Rust Translation

```rust
use nucleo::{Config, Nucleo};

pub struct FileSearcher {
    nucleo: Nucleo<FileItem>,
}

impl FileSearcher {
    pub fn new() -> Self {
        Self {
            nucleo: Nucleo::new(
                Config::DEFAULT,
                Arc::new(|| {}),
                None,
                1,
            ),
        }
    }
    
    pub fn search(&mut self, query: &str, items: Vec<FileItem>) -> Vec<FileItem> {
        self.nucleo.injector().push_batch(items);
        
        self.nucleo.pattern.reparse(
            0,
            query,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
        );
        
        let snapshot = self.nucleo.snapshot();
        snapshot.matched_items(..)
            .map(|item| item.data.clone())
            .collect()
    }
}
```

**Crates**: `nucleo` (used by Helix editor for fuzzy search)

---

# 🔧 SERVICE 12: terminal-welcome/ - TERMINAL TIPS (57 lines)

## Purpose
Show welcome message when terminal opens.

## Core: TerminalWelcomeService.ts

```typescript
export class TerminalWelcomeService {
    private tipShownThisSession = false
    
    public initialize(): void {
        vscode.window.onDidOpenTerminal((terminal) => {
            this.handleTerminalOpened(terminal)
        })
        
        vscode.window.terminals.forEach((terminal) => {
            this.handleTerminalOpened(terminal)
        })
    }
    
    private handleTerminalOpened(terminal: vscode.Terminal): void {
        if (this.tipShownThisSession) return
        
        this.tipShownThisSession = true
        setTimeout(() => this.showWelcomeMessage(terminal), 500)
    }
    
    private showWelcomeMessage(terminal: vscode.Terminal): void {
        const shortcut = this.getKeyboardShortcut()
        const message = `Tip: Use ${shortcut} to generate terminal commands with AI`
        vscode.window.showInformationMessage(message)
    }
    
    private getKeyboardShortcut(): string {
        const isMac = process.platform === 'darwin'
        const modifier = isMac ? 'Cmd' : 'Ctrl'
        return `${modifier}+Shift+G`
    }
}
```

### Rust Translation

**Low priority** - Simple notification feature.

---

# 🔧 SERVICE 13: tree-sitter/ - SYNTAX PARSING (416 lines, 124 files)

## Purpose
Parse code files to extract definitions (functions, classes, etc.) for AI context.

## Architecture

```
tree-sitter/
├── index.ts              - Main parsing logic
├── languageParser.ts     - WASM parser loader
├── markdownParser.ts     - Markdown heading parser
├── queries/              - Tree-sitter query files
│   ├── javascript.ts
│   ├── typescript.ts
│   ├── python.ts
│   ├── rust.ts
│   └── ... (40+ languages)
└── tree-sitter-*.wasm    - WASM binaries (124 files)
```

## Core Flow

```typescript
export async function parseSourceCodeDefinitionsForFile(
    filePath: string,
    rooIgnoreController?: RooIgnoreController,
): Promise<string | undefined> {
    const ext = path.extname(filePath).toLowerCase()
    
    // Special case: Markdown
    if (ext === '.md') {
        const content = await fs.readFile(filePath, 'utf8')
        const captures = parseMarkdown(content)
        return processCaptures(captures, content.split('\n'), 'markdown')
    }
    
    // Load language parser
    const languageParsers = await loadRequiredLanguageParsers([filePath])
    const { parser, query } = languageParsers[ext.slice(1)] || {}
    
    if (!parser || !query) {
        return undefined
    }
    
    const content = await fs.readFile(filePath, 'utf8')
    const tree = parser.parse(content)
    const captures = query.captures(tree.rootNode)
    
    return processCaptures(captures, content.split('\n'), ext.slice(1))
}
```

**Process Captures**:
```typescript
function processCaptures(captures: QueryCapture[], lines: string[], language: string): string | null {
    // Filter HTML elements in JSX/TSX
    const needsHtmlFiltering = ['jsx', 'tsx'].includes(language)
    
    let output = ''
    captures.sort((a, b) => a.node.startPosition.row - b.node.startPosition.row)
    
    for (const capture of captures) {
        if (!capture.name.includes('definition')) continue
        
        const startLine = capture.node.startPosition.row
        const endLine = capture.node.endPosition.row
        const lineCount = endLine - startLine + 1
        
        if (lineCount < MIN_COMPONENT_LINES) continue
        
        output += `${startLine + 1}--${endLine + 1} | ${lines[startLine]}\n`
    }
    
    return output || null
}
```

**Output Example** (`src/main.rs`):
```
# main.rs
1--5 | fn main() {
10--25 | fn process_data(input: &str) -> Result<String> {
30--45 | struct Config {
```

## Language Parser Loading

```typescript
export async function loadRequiredLanguageParsers(filesToParse: string[]) {
    const { Parser, Query } = require('web-tree-sitter')
    await Parser.init()
    
    const extensionsToLoad = new Set(filesToParse.map(f => path.extname(f).slice(1)))
    const parsers: LanguageParser = {}
    
    for (const ext of extensionsToLoad) {
        let language: Language
        let query: Query
        
        switch (ext) {
            case 'js':
            case 'jsx':
                language = await loadLanguage('javascript')
                query = new Query(language, javascriptQuery)
                break
            case 'ts':
                language = await loadLanguage('typescript')
                query = new Query(language, typescriptQuery)
                break
            case 'py':
                language = await loadLanguage('python')
                query = new Query(language, pythonQuery)
                break
            // ... 40+ more languages
        }
        
        const parser = new Parser()
        parser.setLanguage(language)
        parsers[ext] = { parser, query }
    }
    
    return parsers
}
```

**Supported Extensions**: `.js`, `.jsx`, `.ts`, `.tsx`, `.py`, `.rs`, `.go`, `.c`, `.cpp`, `.java`, `.php`, `.rb`, `.swift`, `.kt`, `.sol`, `.ex`, `.html`, `.md`, `.json`, `.css`, `.toml`, `.lua`, `.zig`, ... (40+ total)

### Rust Translation

**GOOD NEWS: Lapce already has tree-sitter!**

```rust
use tree_sitter::{Parser, Language, Query, QueryCursor};

pub struct CodeParser {
    parsers: HashMap<String, (Parser, Query)>,
}

impl CodeParser {
    pub fn parse_file(&mut self, path: &Path) -> Result<Vec<Definition>> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or("No extension")?;
        
        let (parser, query) = self.parsers.get_mut(ext)
            .ok_or("Unsupported language")?;
        
        let content = std::fs::read_to_string(path)?;
        let tree = parser.parse(&content, None)
            .ok_or("Parse failed")?;
        
        let mut cursor = QueryCursor::new();
        let captures = cursor.captures(
            query,
            tree.root_node(),
            content.as_bytes(),
        );
        
        let mut definitions = Vec::new();
        for (match_, _) in captures {
            for capture in match_.captures {
                let node = capture.node;
                definitions.push(Definition {
                    start_line: node.start_position().row,
                    end_line: node.end_position().row,
                    text: node.utf8_text(content.as_bytes())?.to_string(),
                });
            }
        }
        
        Ok(definitions)
    }
}
```

**Implementation Notes**:
1. Lapce already has tree-sitter parsers for syntax highlighting
2. Reuse existing parser instances
3. Add custom queries for definition extraction
4. Use `lapce-core` tree-sitter integration

**Crates**: Already in Lapce! (`tree-sitter`, `tree-sitter-*` for each language)

---

# 📊 RUST TRANSLATION SUMMARY

## Priority Matrix

### 🔴 HIGH PRIORITY (Must Have)

**1. code-index/** - Semantic search
- **Complexity**: Very High (45 files)
- **Strategy**: Simplify initial implementation
- **Phase 1**: Text-based search with `tantivy`
- **Phase 2**: Add embeddings with ONNX Runtime or Ollama
- **Crates**: `qdrant-client`, `tantivy`, `tokio`

**2. tree-sitter/** - Code parsing
- **Complexity**: Low (already in Lapce!)
- **Strategy**: Reuse existing Lapce parsers
- **Action**: Add custom queries for definition extraction
- **Crates**: Already available in `lapce-core`

**3. glob/** - File listing
- **Complexity**: Low
- **Strategy**: Use `ignore` crate
- **Crates**: `ignore`, `walkdir`

**4. checkpoints/** - Git versioning
- **Complexity**: Medium
- **Strategy**: Direct port with `git2`
- **Crates**: `git2`

**5. ripgrep/** - Regex search
- **Complexity**: Low
- **Strategy**: Use `grep` crate (BurntSushi's library)
- **Crates**: `grep`, `regex`

### 🟡 MEDIUM PRIORITY (Should Have)

**6. command/** - Custom commands
- **Complexity**: Low
- **Crates**: `gray_matter`, `serde_yaml`

**7. commit-message/** - AI commits
- **Complexity**: Low
- **Crates**: `git2`

**8. roo-config/** - Config management
- **Complexity**: Low
- **Crates**: `dirs`, `tokio`

**9. search/** - Fuzzy search
- **Complexity**: Low
- **Crates**: `nucleo` (used by Helix)

### 🟢 LOW PRIORITY (Nice to Have)

**10. browser/** - Web scraping
- **Complexity**: High
- **Decision**: Defer to future release
- **Alternative**: External tool or Python script

**11. mdm/** - Device management
- **Complexity**: Low
- **Decision**: Enterprise feature, low user impact

**12. terminal-welcome/** - Tips
- **Complexity**: Very Low
- **Decision**: Simple notification

**13. mocking/** - Test mocks
- **Complexity**: N/A
- **Decision**: Use Rust native testing

---

## Key Architectural Decisions

### 1. Semantic Search Strategy

**Option A: Full Embedding Pipeline**
```rust
// Pros: Feature parity, semantic understanding
// Cons: Complex, requires ML model, higher resource usage
pub struct CodeIndexManager {
    qdrant: QdrantClient,
    embedder: OnnxEmbedder,  // or OllamaEmbedder
}
```

**Option B: Hybrid Approach (RECOMMENDED)**
```rust
// Phase 1: Fast text search with tantivy
pub struct CodeIndexManager {
    tantivy_index: Index,
    // Phase 2: Add embeddings later
    embedder: Option<Box<dyn Embedder>>,
}
```

### 2. Tree-Sitter Integration

**Leverage Lapce's existing implementation:**
```rust
use lapce_core::syntax::Syntax;

pub fn parse_definitions(path: &Path) -> Vec<Definition> {
    // Use Lapce's existing tree-sitter infrastructure
    let syntax = Syntax::init(path)?;
    // Add custom query for definitions
    extract_definitions(&syntax)
}
```

### 3. Configuration Hierarchy

```
Global: ~/.kilocode/
  ↓ (merge)
Project: {workspace}/.kilocode/
  ↓ (final config)
Runtime
```

---

## Implementation Roadmap

### Week 1: Core Infrastructure
- [x] File listing (glob/)
- [x] Tree-sitter integration
- [ ] Config management (roo-config/)
- [ ] Git checkpoints

### Week 2: Search & Discovery
- [ ] Text-based search (tantivy)
- [ ] Regex search (ripgrep/)
- [ ] Fuzzy file search
- [ ] Command system

### Week 3: AI Features
- [ ] Basic code indexing (text)
- [ ] Commit message generation
- [ ] Definition extraction

### Week 4: Advanced Features
- [ ] Embedding support (optional)
- [ ] Incremental indexing
- [ ] Performance optimization

---

## Crate Dependencies

```toml
[dependencies]
# Already in Lapce
tree-sitter = "0.20"
ignore = "0.4"
walkdir = "2.4"
git2 = "0.18"
regex = "1.10"

# New additions
qdrant-client = "1.7"          # Vector store
tantivy = "0.21"               # Text search
grep = "0.3"                    # Regex search (BurntSushi)
nucleo = "0.2"                  # Fuzzy search (Helix)
gray_matter = "0.2"             # Frontmatter parsing
serde_yaml = "0.9"
dirs = "5.0"

# Optional (for embeddings)
ort = "1.16"                    # ONNX Runtime
reqwest = "0.11"                # Ollama API
```

---

## Performance Targets

### File Listing (glob/)
- **Target**: <100ms for 10k files
- **Strategy**: Use `ignore` crate (respects .gitignore automatically)

### Code Parsing (tree-sitter/)
- **Target**: <50ms per file
- **Strategy**: Parallel processing with `rayon`

### Text Search (tantivy)
- **Target**: <200ms for 100k files
- **Strategy**: Build index incrementally

### Semantic Search (embeddings - optional)
- **Target**: <500ms query latency
- **Strategy**: Local Qdrant instance

---

## Migration Path from TypeScript

### Phase 1: Core Services (2 weeks)
1. Implement file operations (glob, ripgrep)
2. Integrate tree-sitter for code parsing
3. Add git checkpoint system
4. Implement config management

### Phase 2: Search & Discovery (2 weeks)
5. Text-based code search with tantivy
6. Fuzzy file search with nucleo
7. Custom command system
8. Definition extraction

### Phase 3: AI Features (2 weeks)
9. Commit message generation
10. Basic code indexing
11. Incremental indexing

### Phase 4: Advanced (Optional)
12. Embedding support for semantic search
13. Browser automation (external tool)
14. MDM compliance checks

---

## Key Takeaways

✅ **Reuse Lapce Assets**: tree-sitter, ignore, git2 already available

✅ **Simplify Initial Release**: Start with text search, add embeddings later

✅ **Leverage Rust Ecosystem**: Use proven crates (tantivy, nucleo, grep)

✅ **Defer Complex Features**: Browser automation can be external tool

✅ **Focus on Performance**: Rust's speed advantage for file operations

---

**Total Estimated Effort**: 6-8 weeks for core services

**Lines of Code**: ~5,000-7,000 lines (vs 3,000+ in TypeScript)

**Performance Gain**: 5-10x faster file operations, 2-3x faster search
