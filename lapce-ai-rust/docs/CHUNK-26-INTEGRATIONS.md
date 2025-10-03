# CHUNK 11: src/integrations/ - EDITOR, TERMINAL, MISC (54 FILES)

## Overview
Integrations handle deep VS Code-specific functionality:
- **Terminal** (15 files, ~1500 lines) - Shell integration, command execution
- **Editor** (5 files, ~800 lines) - Diff view, decorations, code detection
- **Misc** (20+ files) - File extraction (PDF, DOCX), image handling, export
- **Theme** (2 files) - Theme detection and color extraction
- **Notifications** (1 file) - System notifications

## CRITICAL: Terminal Integration (~1500 LINES)

### Architecture
```
Terminal.ts (197 lines)
  ├─ TerminalProcess.ts (468 lines) - Shell integration execution
  ├─ BaseTerminal.ts - Abstract base class
  ├─ BaseTerminalProcess.ts - Event emitter base
  ├─ ExecaTerminal.ts - Fallback execa-based execution
  └─ ShellIntegrationManager.ts - Shell integration lifecycle
```

### Terminal.ts - Main Terminal Class

```typescript
export class Terminal extends BaseTerminal {
    public terminal: vscode.Terminal
    public cmdCounter: number = 0
    
    constructor(id: number, terminal: vscode.Terminal | undefined, cwd: string) {
        super("vscode", id, cwd)
        
        const env = Terminal.getEnv()
        this.terminal = terminal ?? vscode.window.createTerminal({ 
            cwd, 
            name: "Kilo Code", 
            env 
        })
    }
    
    runCommand(command: string, callbacks: RooTerminalCallbacks): Promise<void> {
        this.busy = true
        const process = new TerminalProcess(this)
        process.command = command
        this.process = process
        
        // Set up event handlers
        process.on("line", (line) => callbacks.onLine(line, process))
        process.once("completed", (output) => callbacks.onCompleted(output, process))
        process.once("shell_execution_started", (pid) => 
            callbacks.onShellExecutionStarted(pid, process))
        process.once("shell_execution_complete", (details) => 
            callbacks.onShellExecutionComplete(details, process))
        process.once("no_shell_integration", (msg) => 
            callbacks.onNoShellIntegration?.(msg, process))
        
        // Wait for shell integration
        return pWaitFor(() => this.terminal.shellIntegration !== undefined, {
            timeout: Terminal.getShellIntegrationTimeout()
        })
        .then(() => process.run(command))
        .catch(() => {
            process.emit("no_shell_integration", 
                "Shell integration not available within timeout")
        })
    }
    
    static async getTerminalContents(commands = -1): Promise<string> {
        // Save clipboard
        const tempCopyBuffer = await vscode.env.clipboard.readText()
        
        try {
            // Select terminal content
            if (commands < 0) {
                await vscode.commands.executeCommand("workbench.action.terminal.selectAll")
            } else {
                for (let i = 0; i < commands; i++) {
                    await vscode.commands.executeCommand(
                        "workbench.action.terminal.selectToPreviousCommand")
                }
            }
            
            // Copy and clear selection
            await vscode.commands.executeCommand("workbench.action.terminal.copySelection")
            await vscode.commands.executeCommand("workbench.action.terminal.clearSelection")
            
            const terminalContents = await vscode.env.clipboard.readText()
            
            // Restore clipboard
            await vscode.env.clipboard.writeText(tempCopyBuffer)
            
            return terminalContents.trim()
        } catch (error) {
            await vscode.env.clipboard.writeText(tempCopyBuffer)
            throw error
        }
    }
}
```

**RUST TRANSLATION CHALLENGE:**
- **No VS Code terminal API** in Lapce
- **Solution:** Use tokio::process::Command directly

```rust
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct Terminal {
    id: usize,
    cwd: PathBuf,
    busy: bool,
    process: Option<TerminalProcess>,
}

impl Terminal {
    pub fn new(id: usize, cwd: PathBuf) -> Self {
        Self {
            id,
            cwd,
            busy: false,
            process: None,
        }
    }
    
    pub async fn run_command(
        &mut self,
        command: String,
        callbacks: TerminalCallbacks,
    ) -> Result<()> {
        self.busy = true;
        
        let mut child = Command::new(get_shell_path())
            .arg("-c")
            .arg(&command)
            .current_dir(&self.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        // Stream stdout
        let stdout_reader = BufReader::new(stdout);
        let mut stdout_lines = stdout_reader.lines();
        
        while let Some(line) = stdout_lines.next_line().await? {
            callbacks.on_line(line).await;
        }
        
        // Wait for completion
        let status = child.wait().await?;
        let exit_code = status.code().unwrap_or(-1);
        
        callbacks.on_completed(exit_code).await;
        
        self.busy = false;
        Ok(())
    }
    
    pub async fn get_terminal_contents() -> Result<String> {
        // In Lapce: read from process output buffer
        // No clipboard manipulation needed
        Ok("Terminal output not available in Lapce".to_string())
    }
}

#[async_trait]
pub trait TerminalCallbacks: Send + Sync {
    async fn on_line(&self, line: String);
    async fn on_completed(&self, exit_code: i32);
}
```

### TerminalProcess.ts - Shell Integration (468 LINES)

**Key Features:**
1. **Shell integration detection** - Waits for VS Code shell integration
2. **Output streaming** - Real-time line-by-line output
3. **Exit code detection** - Determines success/failure
4. **PowerShell workarounds** - Special handling for PowerShell quirks

```typescript
export class TerminalProcess extends BaseTerminalProcess {
    async run(command: string) {
        const terminal = this.terminal.terminal
        const isShellIntegrationAvailable = 
            terminal.shellIntegration && terminal.shellIntegration.executeCommand
        
        if (!isShellIntegrationAvailable) {
            // Fallback: send text without output capture
            terminal.sendText(command, true)
            this.emit("no_shell_integration", "Output not available")
            this.emit("completed", "<unknown status>")
            return
        }
        
        // Wait for stream
        const stream = await this.waitForStream()
        
        // Execute command
        terminal.shellIntegration.executeCommand(command)
        
        // Stream output
        for await (const data of stream) {
            const lines = data.split("\n")
            for (const line of lines) {
                this.emit("line", stripAnsi(line))
            }
        }
        
        // Wait for completion
        const exitDetails = await this.waitForCompletion()
        this.emit("completed", this.capturedOutput)
        this.emit("shell_execution_complete", exitDetails)
    }
}
```

**RUST:** Already handled in Terminal implementation above.

## DiffViewProvider.ts (727 LINES!)

**Purpose:** Show side-by-side diff when editing files

### Key Functionality
```typescript
export class DiffViewProvider {
    originalContent: string | undefined
    newContent?: string
    activeDiffEditor?: vscode.TextEditor
    
    async open(relPath: string): Promise<void> {
        const absolutePath = path.resolve(this.cwd, relPath)
        
        // Save original content
        if (await fileExists(absolutePath)) {
            this.originalContent = await fs.readFile(absolutePath, "utf-8")
        } else {
            this.originalContent = ""
        }
        
        // Create directories if needed
        await createDirectoriesForFile(absolutePath)
        
        // Open diff view
        const originalUri = vscode.Uri.parse(
            `${DIFF_VIEW_URI_SCHEME}:${absolutePath}?original`
        )
        const modifiedUri = vscode.Uri.file(absolutePath)
        
        await vscode.commands.executeCommand(
            "vscode.diff",
            originalUri,
            modifiedUri,
            `Original ↔ Kilo Code's Changes`
        )
    }
    
    async update(accumulatedContent: string): Promise<void> {
        // Stream partial content to diff view
        const lines = accumulatedContent.split("\n")
        
        // Update decorations to show streaming
        this.fadedOverlayController?.update(lines.length)
        this.activeLineController?.setActiveLine(lines.length)
    }
    
    async saveChanges(): Promise<void> {
        const absolutePath = path.resolve(this.cwd, this.relPath!)
        
        // Write new content
        await fs.writeFile(absolutePath, this.newContent!)
        
        // Check for new diagnostics (errors/warnings)
        const newDiagnostics = getNewDiagnostics(
            this.preDiagnostics,
            vscode.languages.getDiagnostics()
        )
        
        this.newProblemsMessage = diagnosticsToProblemsString(newDiagnostics)
    }
    
    async revertChanges(): Promise<void> {
        const absolutePath = path.resolve(this.cwd, this.relPath!)
        
        if (this.editType === "create") {
            // Delete created file and directories
            await fs.unlink(absolutePath)
            for (const dir of this.createdDirs.reverse()) {
                await fs.rmdir(dir)
            }
        } else {
            // Restore original content
            await fs.writeFile(absolutePath, this.originalContent!)
        }
    }
}
```

**RUST TRANSLATION:**
```rust
pub struct DiffViewProvider {
    cwd: PathBuf,
    original_content: Option<String>,
    new_content: Option<String>,
    created_dirs: Vec<PathBuf>,
    edit_type: EditType,
}

#[derive(Debug, Clone)]
pub enum EditType {
    Create,
    Modify,
}

impl DiffViewProvider {
    pub async fn open(&mut self, rel_path: &str) -> Result<()> {
        let absolute_path = self.cwd.join(rel_path);
        
        // Save original content
        if absolute_path.exists() {
            self.original_content = Some(fs::read_to_string(&absolute_path).await?);
            self.edit_type = EditType::Modify;
        } else {
            self.original_content = Some(String::new());
            self.edit_type = EditType::Create;
        }
        
        // Create directories
        self.created_dirs = create_directories_for_file(&absolute_path).await?;
        
        // In Lapce: Can't show VS Code-style diff
        // Options:
        // 1. Write temp files and let user view them
        // 2. Return diff string for display in web UI
        // 3. Use external diff tool
        
        Ok(())
    }
    
    pub async fn save_changes(&mut self, new_content: String) -> Result<()> {
        let absolute_path = self.cwd.join(self.rel_path.as_ref().unwrap());
        
        fs::write(&absolute_path, &new_content).await?;
        
        self.new_content = Some(new_content);
        
        Ok(())
    }
    
    pub async fn revert_changes(&mut self) -> Result<()> {
        let absolute_path = self.cwd.join(self.rel_path.as_ref().unwrap());
        
        match self.edit_type {
            EditType::Create => {
                // Delete file
                fs::remove_file(&absolute_path).await?;
                
                // Delete created directories
                for dir in self.created_dirs.iter().rev() {
                    fs::remove_dir(dir).await.ok(); // Ignore errors
                }
            }
            EditType::Modify => {
                // Restore original
                if let Some(original) = &self.original_content {
                    fs::write(&absolute_path, original).await?;
                }
            }
        }
        
        Ok(())
    }
}
```

## extract-text.ts - Binary File Support (493 LINES)

**Purpose:** Extract text from PDF, DOCX, XLSX, IPYNB files

```typescript
const SUPPORTED_BINARY_FORMATS = {
    ".pdf": extractTextFromPDF,
    ".docx": extractTextFromDOCX,
    ".ipynb": extractTextFromIPYNB,
    ".xlsx": extractTextFromXLSX,
}

export async function extractTextFromFile(
    filePath: string,
    maxReadFileLine?: number
): Promise<string> {
    const fileExtension = path.extname(filePath).toLowerCase()
    
    // Check for specific extractor
    const extractor = SUPPORTED_BINARY_FORMATS[fileExtension]
    if (extractor) {
        return extractor(filePath)
    }
    
    // Check if binary
    const isBinary = await isBinaryFile(filePath)
    
    if (!isBinary) {
        // Text file - apply line limit
        if (maxReadFileLine && maxReadFileLine !== -1) {
            const totalLines = await countFileLines(filePath)
            if (totalLines > maxReadFileLine) {
                const content = await readLines(filePath, maxReadFileLine - 1, 0)
                return addLineNumbers(content) + 
                    `\n\n... (Showing ${maxReadFileLine} of ${totalLines} lines)`
            }
        }
        
        const content = await fs.readFile(filePath, "utf-8")
        return addLineNumbers(content)
    }
    
    throw new Error(`Unsupported binary file format: ${fileExtension}`)
}

async function extractTextFromPDF(filePath: string): Promise<string> {
    const dataBuffer = await fs.readFile(filePath)
    const data = await pdf(dataBuffer)
    return addLineNumbers(data.text)
}

async function extractTextFromDOCX(filePath: string): Promise<string> {
    const result = await mammoth.extractRawText({ path: filePath })
    return addLineNumbers(result.value)
}
```

**RUST TRANSLATION:**
```rust
use pdf_extract;
use docx_rs;
use calamine::{Reader, open_workbook, Xlsx};

pub async fn extract_text_from_file(
    file_path: &Path,
    max_read_file_line: Option<i32>
) -> Result<String> {
    let extension = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    match extension {
        "pdf" => extract_text_from_pdf(file_path).await,
        "docx" => extract_text_from_docx(file_path).await,
        "xlsx" => extract_text_from_xlsx(file_path).await,
        "ipynb" => extract_text_from_ipynb(file_path).await,
        _ => {
            // Check if binary
            let is_binary = is_binary_file(file_path).await?;
            
            if !is_binary {
                // Read text file with line limit
                let content = if let Some(limit) = max_read_file_line {
                    if limit == -1 {
                        fs::read_to_string(file_path).await?
                    } else {
                        read_lines_limited(file_path, limit as usize).await?
                    }
                } else {
                    fs::read_to_string(file_path).await?
                };
                
                Ok(add_line_numbers(&content))
            } else {
                Err(anyhow!("Unsupported binary format: {}", extension))
            }
        }
    }
}

async fn extract_text_from_pdf(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path).await?;
    let text = pdf_extract::extract_text_from_mem(&bytes)?;
    Ok(add_line_numbers(&text))
}

async fn extract_text_from_docx(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path).await?;
    let docx = docx_rs::read_docx(&bytes)?;
    let text = extract_text_from_docx_document(&docx);
    Ok(add_line_numbers(&text))
}
```

## Image Handling

```typescript
export async function processImages(
    images: string[]
): Promise<Array<{ type: "image"; source: ImageSource }>> {
    return Promise.all(images.map(async (dataUrl) => {
        if (dataUrl.startsWith("http")) {
            // Download image
            const response = await fetch(dataUrl)
            const buffer = await response.arrayBuffer()
            const base64 = Buffer.from(buffer).toString("base64")
            
            return {
                type: "image" as const,
                source: {
                    type: "base64" as const,
                    media_type: response.headers.get("content-type") || "image/png",
                    data: base64
                }
            }
        } else {
            // Already base64
            const [mediaType, base64Data] = dataUrl.split(",")
            return {
                type: "image" as const,
                source: {
                    type: "base64" as const,
                    media_type: mediaType.split(":")[1].split(";")[0],
                    data: base64Data
                }
            }
        }
    }))
}
```

## Summary: Integration Challenges

| Component | Lines | VS Code Dependency | Lapce Strategy |
|-----------|-------|-------------------|----------------|
| Terminal | 1500 | **Critical** - Shell integration | Use tokio::process directly |
| DiffView | 727 | **High** - vscode.diff command | Web UI diff display |
| Extract Text | 493 | Low - Node.js libraries | Port to Rust crates |
| Image Handling | ~200 | Low | Direct port |
| Theme | ~100 | Medium - VS Code theme API | Lapce theme API |

## Next: CHUNK 12 - activate/
Extension activation, command registration, URI handling.
