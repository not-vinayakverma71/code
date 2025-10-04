# CHUNK-29 Step 5: API Translation Layer Design

**Generated:** 2025-10-02  
**Status:** Complete

## Executive Summary

Comprehensive design of the **IPC Protocol Layer** to bridge VS Code TypeScript APIs to Lapce using Native UI + SharedMemory IPC + Backend architecture.

**Key Change:** No trait-based abstraction needed - using **direct IPC message passing** instead.

**Architecture:**
- UI: Native Lapce panels (lapce-app/)
- IPC: Binary protocol over SharedMemory (5.1μs latency)
- Backend: All AI logic in separate process (lapce-ai-rust/)

---

## 1. ARCHITECTURE OVERVIEW

### 1.1 Layer Stack

```
┌─────────────────────────────────────────┐
│  Lapce IDE (lapce-app/)                 │
│  ┌───────────────────────────────────┐  │
│  │ UI Panels (Floem)                 │  │
│  │ - ai_chat.rs                      │  │
│  │ - ai_diff.rs                      │  │
│  └──────────────┬────────────────────┘  │
│                 │                        │
│  ┌──────────────▼────────────────────┐  │
│  │ ai_bridge.rs (IPC Client)         │  │
│  │ - send_message()                  │  │
│  │ - recv_stream()                   │  │
│  │ ~100 lines                        │  │
│  └──────────────┬────────────────────┘  │
└─────────────────┼───────────────────────┘
                  │
         ═════════▼═════════
         SharedMemory IPC
         5.1μs | 1.38M msg/s
         ═════════│═════════
                  │
┌─────────────────▼────────────────────────┐
│  lapce-ai-rust (Backend Process)         │
│  ┌────────────────────────────────────┐  │
│  │ ipc_server.rs                      │  │
│  │ - Message router                   │  │
│  └──────────┬─────────────────────────┘  │
│             │                             │
│  ┌──────────▼──────────┐                 │
│  │ Handlers:           │                 │
│  │ - terminal.rs       │                 │
│  │ - diff.rs           │                 │
│  │ - workspace.rs      │                 │
│  │ - ai_providers.rs   │                 │
│  └─────────────────────┘                 │
└──────────────────────────────────────────┘
```

### 1.2 Design Principles

1. **IPC Message Passing:** All communication via binary protocol
2. **Process Isolation:** UI and backend in separate processes
3. **Zero-Copy Serialization:** Using rkyv for performance
4. **Async Streaming:** Real-time updates via IPC
5. **Crash Isolation:** Backend crash won't kill IDE

---

## 2. CORE TRAIT DEFINITIONS

### 2.1 Terminal Traits

```rust
// lapce-ai-rust/src/traits/terminal.rs

use async_trait::async_trait;
use tokio::sync::mpsc;

/// Core terminal capability trait
#[async_trait]
pub trait Terminal: Send + Sync {
    type Error: std::error::Error + Send + Sync;
    
    /// Create a new terminal instance
    async fn create(
        profile: TerminalProfile,
        size: TerminalSize,
    ) -> Result<Self, Self::Error>
    where Self: Sized;
    
    /// Send text to the terminal
    async fn send_text(&mut self, text: &str) -> Result<(), Self::Error>;
    
    /// Read output stream
    fn output_stream(&self) -> mpsc::Receiver<TerminalOutput>;
    
    /// Get current working directory
    fn cwd(&self) -> Option<PathBuf>;
    
    /// Dispose of terminal resources
    async fn dispose(self) -> Result<(), Self::Error>;
}

/// Shell integration for command tracking
#[async_trait]
pub trait ShellIntegration: Terminal {
    /// Execute command with tracking
    async fn execute_command(
        &mut self,
        command: &str,
    ) -> Result<CommandExecution, Self::Error>;
    
    /// Parse escape sequences for command markers
    fn parse_markers(&mut self, data: &[u8]) -> Vec<ShellMarker>;
}

/// Command execution tracking
pub struct CommandExecution {
    pub command: String,
    pub start_time: Instant,
    pub output: mpsc::Receiver<String>,
    pub exit_code: Option<i32>,
}

/// Shell integration markers
#[derive(Debug, Clone)]
pub enum ShellMarker {
    PromptStart,           // OSC 633;A
    PromptEnd,            // OSC 633;B  
    CommandOutputStart,    // OSC 633;C
    CommandOutputEnd(i32), // OSC 633;D
    CommandLine(String),   // OSC 633;E
}
```

### 2.2 Editor Traits

```rust
// lapce-ai-rust/src/traits/editor.rs

use lapce_xi_rope::{Rope, RopeDelta};

/// Document manipulation trait
#[async_trait]
pub trait Document: Send + Sync {
    type Error: std::error::Error;
    
    /// Open or create a document
    async fn open(path: PathBuf) -> Result<Self, Self::Error>
    where Self: Sized;
    
    /// Get document content
    fn content(&self) -> &Rope;
    
    /// Apply text changes
    async fn apply_delta(&mut self, delta: RopeDelta) -> Result<(), Self::Error>;
    
    /// Save document
    async fn save(&self) -> Result<(), Self::Error>;
    
    /// Get diagnostics
    fn diagnostics(&self) -> Vec<Diagnostic>;
}

/// Editor view trait
#[async_trait]
pub trait EditorView: Send + Sync {
    type Doc: Document;
    
    /// Set active document
    async fn set_document(&mut self, doc: Self::Doc) -> Result<(), Error>;
    
    /// Apply decorations
    fn set_decorations(&mut self, decorations: Vec<Decoration>);
    
    /// Scroll to position
    fn scroll_to(&mut self, line: usize, column: usize);
    
    /// Get visible range
    fn visible_range(&self) -> Range;
}

/// Diff view trait
#[async_trait]
pub trait DiffView: EditorView {
    /// Initialize diff with original content
    async fn init_diff(
        &mut self,
        original: String,
        modified: String,
    ) -> Result<(), Error>;
    
    /// Stream line updates
    async fn update_line(&mut self, line: usize, content: String);
    
    /// Apply decoration to show current edit position
    fn set_active_line(&mut self, line: usize);
}
```

### 2.3 Workspace Traits

```rust
// lapce-ai-rust/src/traits/workspace.rs

use notify::Event as NotifyEvent;

/// File system operations
#[async_trait]
pub trait FileSystem: Send + Sync {
    type Error: std::error::Error;
    
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, Self::Error>;
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), Self::Error>;
    async fn create_dir(&self, path: &Path) -> Result<(), Self::Error>;
    async fn delete(&self, path: &Path) -> Result<(), Self::Error>;
    async fn metadata(&self, path: &Path) -> Result<Metadata, Self::Error>;
}

/// File watcher trait
#[async_trait]
pub trait FileWatcher: Send + Sync {
    type Error: std::error::Error;
    
    /// Start watching a path
    async fn watch(&mut self, path: &Path) -> Result<(), Self::Error>;
    
    /// Stop watching a path
    async fn unwatch(&mut self, path: &Path) -> Result<(), Self::Error>;
    
    /// Get event stream
    fn events(&self) -> mpsc::Receiver<FileEvent>;
}

/// Workspace information
pub trait Workspace: Send + Sync {
    /// Get workspace root
    fn root(&self) -> &Path;
    
    /// Get all workspace folders
    fn folders(&self) -> Vec<&Path>;
    
    /// Find workspace for a file
    fn workspace_for(&self, path: &Path) -> Option<&Path>;
}
```

### 2.4 UI Traits

```rust
// lapce-ai-rust/src/traits/ui.rs

/// Dialog operations
#[async_trait]
pub trait Dialogs: Send + Sync {
    async fn show_save_dialog(
        &self,
        options: SaveDialogOptions,
    ) -> Option<PathBuf>;
    
    async fn show_open_dialog(
        &self,
        options: OpenDialogOptions,
    ) -> Vec<PathBuf>;
    
    async fn show_message(
        &self,
        level: MessageLevel,
        message: &str,
    );
}

/// Tab management
pub trait TabManager: Send + Sync {
    type Tab: Tab;
    
    /// Get all tab groups
    fn groups(&self) -> Vec<TabGroup<Self::Tab>>;
    
    /// Get active tab
    fn active_tab(&self) -> Option<&Self::Tab>;
    
    /// Close tab
    fn close_tab(&mut self, tab: &Self::Tab) -> Result<(), Error>;
}

/// Command execution
#[async_trait]
pub trait CommandPalette: Send + Sync {
    /// Execute a command by name
    async fn execute_command(
        &self,
        command: &str,
        args: CommandArgs,
    ) -> Result<(), Error>;
    
    /// Register new command
    fn register_command(&mut self, command: Command);
}
```

---

## 3. ADAPTER IMPLEMENTATIONS

### 3.1 Terminal Adapter

```rust
// lapce-ai-rust/src/adapters/terminal.rs

use alacritty_terminal::tty::Pty;
use crate::traits::terminal::*;

pub struct LapceTerminal {
    pty: Pty,
    parser: OscParser,
    output_tx: mpsc::Sender<TerminalOutput>,
    output_rx: mpsc::Receiver<TerminalOutput>,
}

#[async_trait]
impl Terminal for LapceTerminal {
    type Error = TerminalError;
    
    async fn create(
        profile: TerminalProfile,
        size: TerminalSize,
    ) -> Result<Self, Self::Error> {
        let pty = create_pty(profile, size)?;
        let (output_tx, output_rx) = mpsc::channel(1000);
        
        Ok(Self {
            pty,
            parser: OscParser::new(),
            output_tx,
            output_rx,
        })
    }
    
    async fn send_text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.pty.write_all(text.as_bytes())?;
        Ok(())
    }
    
    fn output_stream(&self) -> mpsc::Receiver<TerminalOutput> {
        self.output_rx.clone()
    }
}

#[async_trait]
impl ShellIntegration for LapceTerminal {
    async fn execute_command(
        &mut self,
        command: &str,
    ) -> Result<CommandExecution, Self::Error> {
        // Send command with tracking
        self.send_text(&format!("{}\n", command)).await?;
        
        // Create execution tracker
        let (tx, rx) = mpsc::channel(1000);
        
        Ok(CommandExecution {
            command: command.to_string(),
            start_time: Instant::now(),
            output: rx,
            exit_code: None,
        })
    }
    
    fn parse_markers(&mut self, data: &[u8]) -> Vec<ShellMarker> {
        self.parser.parse(data)
    }
}
```

### 3.2 Editor Adapter

```rust
// lapce-ai-rust/src/adapters/editor.rs

use lapce_app::doc::Doc;
use crate::traits::editor::*;

pub struct LapceDocument {
    inner: Rc<Doc>,
}

#[async_trait]
impl Document for LapceDocument {
    type Error = EditorError;
    
    async fn open(path: PathBuf) -> Result<Self, Self::Error> {
        let doc = Doc::open(path).await?;
        Ok(Self { inner: Rc::new(doc) })
    }
    
    fn content(&self) -> &Rope {
        &self.inner.buffer.text
    }
    
    async fn apply_delta(&mut self, delta: RopeDelta) -> Result<(), Self::Error> {
        self.inner.apply_delta(delta);
        Ok(())
    }
}

pub struct LapceDiffView {
    left: LapceDocument,
    right: LapceDocument,
    decorations: Vec<Decoration>,
    active_line: Option<usize>,
}

#[async_trait]
impl DiffView for LapceDiffView {
    async fn update_line(&mut self, line: usize, content: String) {
        // Apply streaming update
        let delta = create_line_delta(line, content);
        self.right.apply_delta(delta).await.ok();
        
        // Update decoration for active line
        self.set_active_line(line);
    }
}
```

### 3.3 Workspace Adapter

```rust
// lapce-ai-rust/src/adapters/workspace.rs

use notify::{Watcher, RecursiveMode};
use crate::traits::workspace::*;

pub struct LapceFileSystem;

#[async_trait]
impl FileSystem for LapceFileSystem {
    type Error = io::Error;
    
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, Self::Error> {
        tokio::fs::read(path).await
    }
    
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), Self::Error> {
        tokio::fs::write(path, content).await
    }
}

pub struct LapceFileWatcher {
    watcher: notify::RecommendedWatcher,
    event_tx: mpsc::Sender<FileEvent>,
    event_rx: mpsc::Receiver<FileEvent>,
}

#[async_trait]
impl FileWatcher for LapceFileWatcher {
    type Error = WatchError;
    
    async fn watch(&mut self, path: &Path) -> Result<(), Self::Error> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        Ok(())
    }
    
    fn events(&self) -> mpsc::Receiver<FileEvent> {
        self.event_rx.clone()
    }
}
```

---

## 4. ESCAPE SEQUENCE PARSER

### 4.1 OSC Parser Implementation

```rust
// lapce-ai-rust/src/parsers/osc.rs

use nom::{
    IResult,
    bytes::complete::{tag, take_until},
    character::complete::digit1,
    combinator::{map, opt},
    sequence::{preceded, terminated, tuple},
};

pub struct OscParser {
    buffer: Vec<u8>,
    state: ParseState,
}

#[derive(Debug)]
enum ParseState {
    Normal,
    InEscape,
    InOsc(Vec<u8>),
}

impl OscParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            state: ParseState::Normal,
        }
    }
    
    pub fn parse(&mut self, input: &[u8]) -> Vec<ShellMarker> {
        let mut markers = Vec::new();
        
        for &byte in input {
            match self.state {
                ParseState::Normal => {
                    if byte == 0x1B { // ESC
                        self.state = ParseState::InEscape;
                    }
                }
                ParseState::InEscape => {
                    if byte == b']' {
                        self.state = ParseState::InOsc(Vec::new());
                    } else {
                        self.state = ParseState::Normal;
                    }
                }
                ParseState::InOsc(ref mut buffer) => {
                    if byte == 0x07 { // BEL
                        if let Some(marker) = self.parse_osc_sequence(buffer) {
                            markers.push(marker);
                        }
                        self.state = ParseState::Normal;
                    } else {
                        buffer.push(byte);
                    }
                }
            }
        }
        
        markers
    }
    
    fn parse_osc_sequence(&self, data: &[u8]) -> Option<ShellMarker> {
        // Parse "633;X" or "133;X" patterns
        if data.starts_with(b"633;") || data.starts_with(b"133;") {
            match data.get(4)? {
                b'A' => Some(ShellMarker::PromptStart),
                b'B' => Some(ShellMarker::PromptEnd),
                b'C' => Some(ShellMarker::CommandOutputStart),
                b'D' => {
                    // Parse optional exit code
                    let exit_code = if data.len() > 5 && data[5] == b';' {
                        std::str::from_utf8(&data[6..])
                            .ok()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    Some(ShellMarker::CommandOutputEnd(exit_code))
                }
                b'E' => {
                    // Parse command line
                    if data.len() > 5 && data[5] == b';' {
                        let cmd = String::from_utf8_lossy(&data[6..]).to_string();
                        Some(ShellMarker::CommandLine(cmd))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
```

---

## 5. ERROR HANDLING STRATEGY

### 5.1 Error Types

```rust
// lapce-ai-rust/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranslationError {
    #[error("Terminal error: {0}")]
    Terminal(#[from] TerminalError),
    
    #[error("Editor error: {0}")]
    Editor(#[from] EditorError),
    
    #[error("Workspace error: {0}")]
    Workspace(#[from] WorkspaceError),
    
    #[error("IPC error: {0}")]
    Ipc(#[from] IpcError),
    
    #[error("Unsupported API: {api}")]
    UnsupportedApi { api: String },
}

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Failed to create PTY: {0}")]
    PtyCreation(#[source] io::Error),
    
    #[error("Shell integration not available")]
    NoShellIntegration,
    
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
}
```

### 5.2 Error Recovery Patterns

```rust
// Graceful degradation
async fn execute_with_fallback(terminal: &mut impl Terminal) -> Result<String> {
    match terminal.execute_command("ls").await {
        Ok(execution) => Ok(execution.collect_output().await),
        Err(TerminalError::NoShellIntegration) => {
            // Fall back to simple send_text
            terminal.send_text("ls\n").await?;
            // Collect output without markers
            collect_raw_output(terminal).await
        }
        Err(e) => Err(e.into()),
    }
}

// Retry with exponential backoff
async fn retry_operation<F, T>(
    mut operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut delay = Duration::from_millis(100);
    
    for attempt in 0..max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_retries - 1 => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    
    unreachable!()
}
```

---

## 6. EVENT SYSTEM BRIDGE

### 6.1 Event Translation

```rust
// lapce-ai-rust/src/events/bridge.rs

use tokio::sync::broadcast;

/// Bridge VS Code-style events to Rust channels
pub struct EventBridge {
    subscribers: HashMap<String, Vec<broadcast::Sender<Event>>>,
}

impl EventBridge {
    /// Register event listener (VS Code style)
    pub fn on(&mut self, event: &str) -> broadcast::Receiver<Event> {
        let (tx, rx) = broadcast::channel(100);
        self.subscribers
            .entry(event.to_string())
            .or_default()
            .push(tx);
        rx
    }
    
    /// Emit event to all listeners
    pub fn emit(&self, event: &str, data: Event) {
        if let Some(senders) = self.subscribers.get(event) {
            for tx in senders {
                let _ = tx.send(data.clone());
            }
        }
    }
}

/// Typed event wrapper
#[derive(Clone, Debug)]
pub enum Event {
    Terminal(TerminalEvent),
    Editor(EditorEvent),
    Workspace(WorkspaceEvent),
}

#[derive(Clone, Debug)]
pub enum TerminalEvent {
    OutputLine(String),
    CommandStart(String),
    CommandEnd { exit_code: i32 },
    Closed,
}
```

### 6.2 Async Event Streams

```rust
use futures::stream::{Stream, StreamExt};

/// Convert event receiver to async stream
pub fn event_stream(
    mut rx: broadcast::Receiver<Event>
) -> impl Stream<Item = Event> {
    async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            yield event;
        }
    }
}

/// Process events with async handler
pub async fn process_events<F>(
    stream: impl Stream<Item = Event>,
    mut handler: F,
)
where
    F: FnMut(Event) -> BoxFuture<'static, ()>,
{
    tokio::pin!(stream);
    
    while let Some(event) = stream.next().await {
        handler(event).await;
    }
}
```

---

## 7. PERFORMANCE OPTIMIZATIONS

### 7.1 Zero-Copy Operations

```rust
// Use bytes::Bytes for zero-copy
pub struct TerminalOutput {
    data: Bytes,
    markers: Vec<(usize, ShellMarker)>,
}

impl TerminalOutput {
    pub fn slice(&self, start: usize, end: usize) -> Bytes {
        self.data.slice(start..end) // Zero-copy slice
    }
}
```

### 7.2 Compile-Time Dispatch

```rust
// Static dispatch for known types
pub fn process_terminal<T: Terminal>(terminal: T) {
    // Monomorphized at compile time
}

// Dynamic dispatch only when necessary
pub fn process_any_terminal(terminal: Box<dyn Terminal>) {
    // Virtual dispatch at runtime
}
```

### 7.3 Buffer Pooling

```rust
use bytes::BytesMut;

pub struct BufferPool {
    buffers: Vec<BytesMut>,
}

impl BufferPool {
    pub fn acquire(&mut self) -> BytesMut {
        self.buffers.pop().unwrap_or_else(|| BytesMut::with_capacity(8192))
    }
    
    pub fn release(&mut self, mut buffer: BytesMut) {
        buffer.clear();
        if self.buffers.len() < 10 {
            self.buffers.push(buffer);
        }
    }
}
```

---

## 8. PLUGIN INTERFACE

### 8.1 Extension Points

```rust
// lapce-ai-rust/src/plugin/api.rs

/// Plugin capability trait
pub trait Plugin: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Initialize plugin
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;
    
    /// Handle custom commands
    async fn handle_command(
        &mut self,
        command: &str,
        args: Value,
    ) -> Result<Value>;
}

/// Plugin context for accessing host APIs
pub struct PluginContext {
    pub terminal: Arc<dyn Terminal>,
    pub editor: Arc<dyn EditorView>,
    pub workspace: Arc<dyn Workspace>,
    pub events: Arc<EventBridge>,
}

/// Plugin registration
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.metadata().id.clone();
        self.plugins.insert(id, plugin);
    }
}
```

---

## 9. MIGRATION PATTERNS

### 9.1 TypeScript to Rust Patterns

```rust
// TypeScript: emitter.on('line', (data) => { ... })
// Rust equivalent:
let mut rx = event_bridge.on("line");
tokio::spawn(async move {
    while let Ok(Event::Terminal(TerminalEvent::OutputLine(data))) = rx.recv().await {
        // Process line
    }
});

// TypeScript: await vscode.workspace.fs.readFile(uri)
// Rust equivalent:
let content = file_system.read_file(&path).await?;

// TypeScript: const edit = new vscode.WorkspaceEdit()
// Rust equivalent:
let mut delta = RopeDelta::default();
```

### 9.2 API Wrapper Macros

```rust
/// Macro to simplify API translation
macro_rules! vscode_api {
    (workspace.fs.$method:ident($($arg:expr),*)) => {
        file_system.$method($($arg),*).await
    };
    
    (window.createTerminal($($arg:expr),*)) => {
        Terminal::create($($arg),*).await
    };
}

// Usage:
let content = vscode_api!(workspace.fs.read_file(&path))?;
```

---

## 10. TESTING STRATEGY

### 10.1 Mock Implementations

```rust
#[cfg(test)]
mod mocks {
    use super::*;
    
    pub struct MockTerminal {
        output: Vec<String>,
        commands: Vec<String>,
    }
    
    #[async_trait]
    impl Terminal for MockTerminal {
        type Error = std::io::Error;
        
        async fn send_text(&mut self, text: &str) -> Result<(), Self::Error> {
            self.commands.push(text.to_string());
            Ok(())
        }
        
        // Mock implementation
    }
}
```

### 10.2 Integration Tests

```rust
#[tokio::test]
async fn test_shell_integration() {
    let mut terminal = LapceTerminal::create(Default::default(), Default::default())
        .await
        .unwrap();
    
    let execution = terminal.execute_command("echo hello").await.unwrap();
    let output = execution.collect_output().await;
    
    assert_eq!(output.trim(), "hello");
}
```

---

## 11. KEY DESIGN DECISIONS

### 11.1 Trait Boundaries

1. **Send + Sync:** Required for async runtime
2. **'static lifetimes:** For tokio spawning
3. **Error types:** Associated types for flexibility
4. **Async trait:** Using async-trait crate

### 11.2 Abstraction Levels

1. **High-level traits:** Match VS Code concepts
2. **Low-level adapters:** Map to Lapce internals
3. **Bridge utilities:** Handle impedance mismatch
4. **Extension points:** Allow custom implementations

### 11.3 Performance Trade-offs

1. **Dynamic dispatch:** Only where necessary
2. **Channel capacity:** Bounded to prevent memory growth
3. **Buffer reuse:** Pool for frequent allocations
4. **Lazy evaluation:** Defer expensive operations

---

## 12. IMPLEMENTATION ROADMAP

### Phase 1: Core Infrastructure (Week 1)
- [ ] Trait definitions
- [ ] Error types
- [ ] Event bridge
- [ ] Basic adapters

### Phase 2: Terminal Integration (Week 2)
- [ ] OSC parser
- [ ] Shell integration
- [ ] Command tracking
- [ ] Output streaming

### Phase 3: Editor Integration (Week 3)
- [ ] Document adapter
- [ ] Diff view
- [ ] Decorations
- [ ] Diagnostics

### Phase 4: Testing & Polish (Week 4)
- [ ] Integration tests
- [ ] Performance benchmarks
- [ ] Documentation
- [ ] Example migrations

---

## 13. NEXT STEPS (Step 6 Preview)

Based on this design, **Step 6: Error Recovery & Benchmarks** will:

1. Define comprehensive error recovery strategies
2. Specify performance benchmarks
3. Create fallback mechanisms
4. Design monitoring/telemetry
5. Document reliability patterns

---

**Step 5 Status:** ✅ **COMPLETE**  
**Design Elements:** Traits, Adapters, Parser, Events, Plugins  
**Implementation Estimate:** 4 weeks  
**Next:** Step 6 - Error Recovery & Benchmarks
