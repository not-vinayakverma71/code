# Phase 3: Advanced Features & UI (2 weeks)
## Achieving 90% Total Memory Reduction & Production Quality

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Tool Execution**: All 29 tools work EXACTLY like Codex (100% pass rate)
- [ ] **Memory Target**: < 3MB for entire MCP tool system
- [ ] **Tool Latency**: < 10ms dispatch overhead per tool
- [ ] **Sandboxing**: 100% process isolation for all tools
- [ ] **Error Messages**: CHARACTER-FOR-CHARACTER match with Codex
- [ ] **XML Format**: Exact schema validation for all tool calls
- [ ] **Resource Limits**: Enforce memory/CPU caps without failure
- [ ] **Stress Test**: Execute 10K tool calls without memory growth

⚠️ **GATE**: Phase 4 starts ONLY when AI can use tools identically to Codex.

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : DIRECT TRANSLATION OF YEARS OF AI WORK
**THIS IS NOT A REWRITE - IT'S A LANGUAGE PORT**

**MANDATORY**: Translate ALL 29 tools from `/home/verma/lapce/Codex/tools/`

**TRANSLATION REQUIREMENTS**:
- Copy EVERY function, line by line
- TypeScript syntax → Rust syntax ONLY
- Same XML format: `<tool_use><tool_name>...</tool_name>...</tool_use>`
- Same parameter names (snake_case in Rust)
- Same error messages (CHARACTER-FOR-CHARACTER)
- Same output format (EXACT structure)
- Same validation logic
- Same edge cases
- Same EVERYTHING except language

**29 BATTLE-TESTED TOOLS - TRANSLATE, DON'T REINVENT**:
Each tool took months to perfect - preserve ALL logic

### Week 1: MCP for External System Communication & Tools for Internal System Communication in Pure Rust 
**Current Issue:** Node.js MCP implementation using 25MB+ for tool execution
**Rust Solution:** Native MCP protocol with sandboxed execution

```rust
use tokio::process::Command;
use nix::unistd::{Uid, Gid};
use nix::sys::resource::{setrlimit, Resource};

pub struct McpToolExecutor {
    tools: DashMap<String, Arc<dyn Tool>>,
    sandbox: ProcessSandbox,
    rate_limiter: Governor<NotKeyed, InMemoryState>,
}

#[async_trait]
trait Tool: Send + Sync {
    async fn execute(&self, args: Value) -> Result<Value>;
    fn validate(&self, args: &Value) -> Result<()>;
    fn resource_limits(&self) -> ResourceLimits;
}

pub struct ProcessSandbox {
    memory_limit: usize,
    cpu_limit: Duration,
    allowed_paths: Vec<PathBuf>,
}

impl ProcessSandbox {
    pub async fn execute_sandboxed<F, Fut>(&self, f: F) -> Result<Value>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Value>> + Send,
    {
        // Fork process with restricted permissions
        match unsafe { nix::unistd::fork() }? {
            ForkResult::Child => {
                // Drop privileges
                setrlimit(Resource::RLIMIT_AS, self.memory_limit, self.memory_limit)?;
                setrlimit(Resource::RLIMIT_CPU, self.cpu_limit.as_secs(), self.cpu_limit.as_secs())?;
                
                // Chroot to sandbox directory
                nix::unistd::chroot(&self.sandbox_dir)?;
                
                // Execute tool
                let result = f().await;
                std::process::exit(result.is_ok() as i32);
            }
            ForkResult::Parent { child } => {
                // Monitor child process
                tokio::time::timeout(self.cpu_limit, async {
                    waitpid(child, None)?
                }).await??
            }
        }
    }
}

// Efficient file system tool
pub struct FileSystemTool {
    fs_cache: Arc<DashMap<PathBuf, FileMetadata>>,
}

impl Tool for FileSystemTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        match args["operation"].as_str() {
            Some("read") => {
                let path = Path::new(args["path"].as_str()?);
                
                // Use memory-mapped I/O for large files
                if std::fs::metadata(path)?.len() > 1_000_000 {
                    let mmap = unsafe { Mmap::map(&File::open(path)?)? };
                    Ok(json!({ "content": String::from_utf8_lossy(&mmap) }))
                } else {
                    Ok(json!({ "content": tokio::fs::read_to_string(path).await? }))
                }
            }
            Some("search") => {
                // Use ripgrep's Rust library directly
                let matcher = RegexMatcher::new(args["pattern"].as_str()?)?;
                let mut searcher = SearcherBuilder::new()
                    .binary_detection(BinaryDetection::quit(b'\0'))
                    .build();
                    
                let mut matches = Vec::new();
                searcher.search_path(
                    &matcher,
                    Path::new(args["path"].as_str()?),
                    UTF8(|_, line| {
                        matches.push(line.to_string());
                        Ok(true)
                    }),
                )?;
                
                Ok(json!({ "matches": matches }))
            }
            _ => Err(anyhow!("Unknown operation"))
        }
    }
}
```

**Memory Savings:** 25MB → 3MB

### Week 1.5: Native UI with Immediate Mode GUI
**Current Issue:** WebView using 40MB+ for UI rendering
**Rust Solution:** egui immediate mode GUI or direct terminal UI

```rust
use ratatui::{Terminal, Frame};
use crossterm::event::{Event, KeyCode};

pub struct NativeUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: Arc<RwLock<AppState>>,
    syntax_highlighter: SyntaxHighlighter,
}

impl NativeUI {
    pub async fn render_chat(&mut self) -> Result<()> {
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Chat history
                    Constraint::Length(3), // Input
                    Constraint::Length(1), // Status bar
                ])
                .split(f.size());
                
            // Render chat history with syntax highlighting
            let messages = self.state.read().unwrap().messages.clone();
            let chat_widget = ChatWidget::new(messages)
                .syntax_highlighter(&self.syntax_highlighter)
                .virtual_scroll(true); // Only render visible messages
                
            f.render_stateful_widget(chat_widget, chunks[0], &mut self.chat_state);
            
            // Render input with auto-complete
            let input_widget = InputWidget::new()
                .autocomplete(&self.autocomplete_engine)
                .syntax_aware(true);
                
            f.render_widget(input_widget, chunks[1]);
            
            // Status bar with metrics
            let status = format!(
                "Memory: {}MB | Latency: {}ms | Model: {}",
                self.get_memory_usage(),
                self.get_avg_latency(),
                self.get_current_model()
            );
            f.render_widget(Paragraph::new(status), chunks[2]);
        })?;
        
        Ok(())
    }
}

// Alternative: egui for graphical UI
pub struct EguiUI {
    context: egui::Context,
    state: Arc<RwLock<AppState>>,
    texture_cache: TextureCache, // Reuse textures
}

impl EguiUI {
    pub fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Virtual scrolling for messages
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, viewport| {
                    let messages = self.state.read().unwrap().messages.clone();
                    
                    // Only render visible messages
                    let start = (viewport.min.y / 50.0) as usize;
                    let end = ((viewport.max.y / 50.0) as usize).min(messages.len());
                    
                    for message in &messages[start..end] {
                        self.render_message(ui, message);
                    }
                });
                
            // Input area with syntax highlighting
            ui.add(
                egui::TextEdit::multiline(&mut self.input)
                    .code_editor()
                    .desired_rows(4)
            );
        });
    }
}
```

**Memory Savings:** 40MB → 5MB (terminal) or 10MB (egui)

### Week 2: Final Optimizations & Integration
**Current Issue:** Scattered optimizations, not fully integrated
**Rust Solution:** Unified architecture with all optimizations

```rust
pub struct LapceAI {
    // Core components
    ipc_server: Arc<IpcServer>,
    provider_pool: Arc<AiProviderPool>,
    
    // Semantic search
    semantic_index: Arc<SemanticIndexer>,
    
    // Performance
    cache: Arc<UnifiedCache>,
    stats: Arc<PerformanceStats>,
    
    // UI
    ui: Box<dyn UI>,
}

impl LapceAI {
    pub async fn initialize() -> Result<Self> {
        // Single initialization with all optimizations
        
        // Pre-allocate all memory pools
        let arena = Bump::with_capacity(10 * 1024 * 1024); // 10MB arena
        
        // Initialize LanceDB with optimized settings
        let db = lancedb::connect("./lance_data")
            .memory_pool_size(5 * 1024 * 1024) // 5MB max
            .cache_size(1000) // Cache 1000 vectors
            .build()
            .await?;
            
        // Create unified thread pool
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .max_blocking_threads(2)
            .thread_stack_size(2 * 1024 * 1024) // 2MB stacks
            .build()?;
            
        Ok(Self {
            // ... initialize all components
        })
    }
    
    pub async fn run(&self) -> Result<()> {
        // Main event loop with zero allocations
        loop {
            tokio::select! {
                Some(request) = self.ipc_server.next_request() => {
                    self.handle_request(request).await?;
                }
                Some(event) = self.ui.next_event() => {
                    self.handle_ui_event(event).await?;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    self.run_maintenance().await?;
                }
            }
        }
    }
}
```

## Memory Profile Comparison

### Node.js Implementation
```
Component                | Memory (MB)
------------------------|------------
Node.js Runtime         | 50-100
Extension Host          | 30-50
AI Providers            | 20-30
Tree-sitter WASM        | 50
Code Index (Qdrant)     | 40
Streaming Buffers       | 20
MCP Tools               | 25
WebView UI              | 40
Cache & State           | 15
------------------------|------------
TOTAL                   | 290-340 MB
```

### Rust Implementation
```
Component               | Memory (MB)
------------------------|------------
Rust Runtime           | 2-3
IPC Server             | 3
AI Providers           | 8
Tree-sitter Native     | 5
LanceDB Index          | 10
Streaming Pipeline     | 2
MCP Tools              | 3
Native UI              | 5-10
Cache & State          | 3
------------------------|------------
TOTAL                  | 41-46 MB (86-87% reduction)
```

## Production Features

### 1. Hot Reloading for Development
```rust
#[cfg(debug_assertions)]
pub fn watch_for_changes() {
    use notify::{Watcher, RecursiveMode};
    
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1))?;
    watcher.watch("./src", RecursiveMode::Recursive)?;
    
    for event in rx {
        if let Ok(Event::Write(_)) = event {
            // Reload affected modules without restart
            unsafe { reload_module() };
        }
    }
}
```

### 2. Crash Recovery
```rust
pub struct CrashRecovery {
    state_snapshot: Arc<RwLock<StateSnapshot>>,
    wal: WriteAheadLog,
}

impl CrashRecovery {
    pub async fn checkpoint(&self) -> Result<()> {
        let snapshot = self.state_snapshot.read().unwrap().clone();
        self.wal.append(&snapshot).await?;
        
        // Rotate log if too large
        if self.wal.size() > 10 * 1024 * 1024 {
            self.wal.compact().await?;
        }
        
        Ok(())
    }
    
    pub async fn recover(&self) -> Result<StateSnapshot> {
        self.wal.replay_to_latest().await
    }
}
```

## Final Dependencies
```toml
[dependencies]
# UI Options
ratatui = "0.28"  # Terminal UI
crossterm = "0.28"
egui = "0.28"  # Alternative GUI

# MCP & Tools
grep = "0.3"  # Ripgrep library
nix = "0.29"  # Process sandboxing
governor = "0.6"  # Rate limiting

# Performance monitoring
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"

# Hot reload
notify = "6.1"

# Crash recovery
sled = "0.34"  # Embedded database for WAL
```

## Final Results - Complete System
- **Memory Usage:** 290-340MB → 41-46MB (86-87% reduction)
- **Cold Start:** 500ms → 25ms (95% reduction)
- **Request Latency:** 50-200ms → 5-20ms (90% reduction)
- **CPU Usage:** 70% reduction at idle, 60% reduction under load
- **Throughput:** 5-10x improvement
- **Scalability:** Can handle 1M+ files efficiently

## Deployment Strategy
1. **Week 1-4:** Phase 1 - Core components
2. **Week 5-7:** Phase 2 - Performance systems
3. **Week 8-9:** Phase 3 - Advanced features
4. **Week 10:** Integration testing & benchmarking
5. **Week 11:** Beta release with A/B testing
6. **Week 12:** Production release

## Maintenance & Evolution
- Monthly performance audits
- Continuous profiling with `perf` and `flamegraph`
- Memory leak detection with `valgrind`
- Automated benchmarking in CI/CD
- Regular dependency updates with `cargo-audit`
