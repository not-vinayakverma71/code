# MCP & TOOLS IMPLEMENTATION  - MCP FOR EXTERNAL COMMUNICATION & TOOLS FOR INTERNAL SYSTEM COMMUNICATION

**Status:** 25% Complete - 0 COMPILATION ERRORS ✅
## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED: 1:1 TRANSLATION OF YEARS OF TOOL DEVELOPMENT
**THIS IS NOT A REWRITE - IT'S A TYPESCRIPT → RUST PORT**

**MUST TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex/` - ALL 29 tool implementations
- Each tool took MONTHS to perfect - preserve EVERYTHING
- Same XML format, same parameters, same error messages
- Same validation, same edge cases, same quirks
- ONLY change syntax from TypeScript to Rust

### Example Tool Format (MUST MATCH):
```xml
<tool_use>
<tool_name>readFile</tool_name>
<path>/path/to/file.rs</path>
<view_range>1-50</view_range>
</tool_use>
```

## ✅ Success Criteria 
- [ ] **Memory Usage**: < 3MB total footprint
- [ ] **Tool Execution**: < 10ms dispatch overhead 
- [ ] **Sandboxing**: Process isolation 
- [ ] **Resource Limits**: Enforce memory/CPU caps 
- [ ] **Permission System**: Fine-grained access control 
- [ ] **Rate Limiting**: Adaptive throttling per user 
- [ ] **Tool Registry**: Support 29 tool types 
- [ ] **Test Coverage**: Execute 10K tool calls safely 

## Overview
Our MCP (Model Context Protocol) tools implementation provides sandboxed execution of IDE operations with minimal memory overhead, replacing the 25MB Node.js implementation with a 3MB Rust solution.

## Core MCP Architecture

### Tool System Design
```rust
use tokio::process::Command;
use nix::unistd::{chroot, setuid, setgid};
use nix::sys::resource::{setrlimit, Resource};

pub struct McpToolSystem {
    // Tool registry
    tools: DashMap<String, Arc<dyn Tool>>,
    
    // Execution sandbox
    sandbox: Arc<ProcessSandbox>,
    
    // Rate limiting
    rate_limiter: Arc<RateLimiter>,
    
    // Permission system
    permissions: Arc<PermissionManager>,
    
    // Metrics
    metrics: Arc<ToolMetrics>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> &JsonSchema;
    
    async fn validate(&self, args: &Value) -> Result<()>;
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult>;
    
    fn required_permissions(&self) -> Vec<Permission>;
    fn resource_limits(&self) -> ResourceLimits;
}

pub struct ToolContext {
    workspace: PathBuf,
    user: UserId,
    session: SessionId,
    cancellation_token: CancellationToken,
}
```

## File System Tools

### 1. File Operations
```rust
pub struct FileSystemTool {
    fs_guard: Arc<FileSystemGuard>,
    cache: Arc<FileCache>,
}

#[async_trait]
impl Tool for FileSystemTool {
    fn name(&self) -> &str {
        "fs_operations"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let operation = args["operation"].as_str()
            .ok_or(Error::InvalidArgs("operation required"))?;
            
        match operation {
            "read" => self.read_file(args, context).await,
            "write" => self.write_file(args, context).await,
            "list" => self.list_directory(args, context).await,
            "search" => self.search_files(args, context).await,
            "watch" => self.watch_changes(args, context).await,
            _ => Err(Error::UnknownOperation(operation.to_string())),
        }
    }
    
    async fn read_file(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Check permissions
        self.fs_guard.check_read_permission(&path, &context.user)?;
        
        // Try cache first
        if let Some(cached) = self.cache.get(&path).await {
            return Ok(ToolResult::success(json!({
                "content": cached.content,
                "cached": true
            })));
        }
        
        // Read file efficiently
        let content = if path.metadata()?.len() > 1_000_000 {
            // Use memory-mapped I/O for large files
            let file = File::open(&path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            String::from_utf8_lossy(&mmap).into_owned()
        } else {
            tokio::fs::read_to_string(&path).await?
        };
        
        // Cache the content
        self.cache.put(path.clone(), FileContent {
            content: content.clone(),
            modified: SystemTime::now(),
        }).await;
        
        Ok(ToolResult::success(json!({
            "content": content,
            "path": path.display().to_string()
        })))
    }
    
    async fn search_files(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str()
            .ok_or(Error::InvalidArgs("pattern required"))?;
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Use ripgrep for fast searching
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\0'))
            .build();
            
        let matcher = RegexMatcher::new(pattern)?;
        let mut matches = Vec::new();
        
        // Walk directory with ignore rules
        let walker = WalkBuilder::new(&path)
            .standard_filters(true)
            .build();
            
        for entry in walker {
            let entry = entry?;
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                let mut sink = UTF8(|line_num, line| {
                    matches.push(json!({
                        "file": entry.path().display().to_string(),
                        "line": line_num,
                        "content": line
                    }));
                    Ok(true)
                });
                
                let _ = searcher.search_path(&matcher, entry.path(), sink);
            }
        }
        
        Ok(ToolResult::success(json!({ "matches": matches })))
    }
}
```

### 2. File Watcher
```rust
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    events: Arc<Mutex<VecDeque<FileEvent>>>,
}

impl FileWatcher {
    pub async fn watch(&self, paths: Vec<PathBuf>) -> Result<BoxStream<FileEvent>> {
        let (tx, rx) = mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.blocking_send(event);
        })?;
        
        for path in paths {
            watcher.watch(&path, RecursiveMode::Recursive)?;
        }
        
        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}
```

## Git Operations

### 1. Git Tool
```rust
pub struct GitTool {
    git2: Arc<git2::Repository>,
    operations: Arc<GitOperations>,
}

#[async_trait]
impl Tool for GitTool {
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let operation = args["operation"].as_str()
            .ok_or(Error::InvalidArgs("operation required"))?;
            
        match operation {
            "status" => self.git_status(context).await,
            "diff" => self.git_diff(args, context).await,
            "commit" => self.git_commit(args, context).await,
            "branch" => self.git_branch(args, context).await,
            "log" => self.git_log(args, context).await,
            _ => Err(Error::UnknownOperation(operation.to_string())),
        }
    }
    
    async fn git_status(&self, context: ToolContext) -> Result<ToolResult> {
        let repo = git2::Repository::open(&context.workspace)?;
        let statuses = repo.statuses(None)?;
        
        let mut files = Vec::new();
        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("");
            
            files.push(json!({
                "path": path,
                "status": Self::status_to_string(status),
                "staged": status.contains(git2::Status::INDEX_NEW | 
                                         git2::Status::INDEX_MODIFIED | 
                                         git2::Status::INDEX_DELETED),
            }));
        }
        
        Ok(ToolResult::success(json!({ "files": files })))
    }
    
    async fn git_diff(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let repo = git2::Repository::open(&context.workspace)?;
        let mut diff_options = git2::DiffOptions::new();
        
        if let Some(path) = args["path"].as_str() {
            diff_options.pathspec(path);
        }
        
        let diff = if args["staged"].as_bool().unwrap_or(false) {
            repo.diff_index_to_workdir(None, Some(&mut diff_options))?
        } else {
            repo.diff_tree_to_workdir(None, Some(&mut diff_options))?
        };
        
        let mut patches = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                patches.push(json!({
                    "old_path": delta.old_file().path().map(|p| p.to_string_lossy()),
                    "new_path": delta.new_file().path().map(|p| p.to_string_lossy()),
                    "status": format!("{:?}", delta.status()),
                }));
                true
            },
            None,
            None,
            None,
        )?;
        
        Ok(ToolResult::success(json!({ "patches": patches })))
    }
}
```

## Code Search Tools

### 1. Semantic Code Search
```rust
pub struct CodeSearchTool {
    semantic_engine: Arc<SemanticSearchEngine>,
    syntax_searcher: Arc<SyntaxSearcher>,
}

#[async_trait]
impl Tool for CodeSearchTool {
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let query = args["query"].as_str()
            .ok_or(Error::InvalidArgs("query required"))?;
        let search_type = args["type"].as_str().unwrap_or("hybrid");
        
        let results = match search_type {
            "semantic" => self.semantic_search(query, args).await?,
            "syntax" => self.syntax_search(query, args).await?,
            "hybrid" => self.hybrid_search(query, args).await?,
            _ => return Err(Error::InvalidSearchType),
        };
        
        Ok(ToolResult::success(json!({ "results": results })))
    }
    
    async fn semantic_search(&self, query: &str, args: Value) -> Result<Vec<Value>> {
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        let filters = self.parse_filters(&args["filters"])?;
        
        let results = self.semantic_engine
            .search(query, limit, filters)
            .await?;
            
        Ok(results.into_iter()
            .map(|r| json!({
                "path": r.path,
                "content": r.content,
                "score": r.score,
                "line_range": [r.start_line, r.end_line],
            }))
            .collect())
    }
}
```

## Terminal Operations

### 1. Terminal Tool
```rust
pub struct TerminalTool {
    sessions: DashMap<SessionId, TerminalSession>,
    sandbox: Arc<ProcessSandbox>,
}

#[async_trait]
impl Tool for TerminalTool {
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let operation = args["operation"].as_str()
            .ok_or(Error::InvalidArgs("operation required"))?;
            
        match operation {
            "create" => self.create_session(args, context).await,
            "execute" => self.execute_command(args, context).await,
            "read" => self.read_output(args, context).await,
            "close" => self.close_session(args, context).await,
            _ => Err(Error::UnknownOperation(operation.to_string())),
        }
    }
    
    async fn execute_command(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let session_id = SessionId::from_str(args["session_id"].as_str()
            .ok_or(Error::InvalidArgs("session_id required"))?)?;
        let command = args["command"].as_str()
            .ok_or(Error::InvalidArgs("command required"))?;
        
        let session = self.sessions.get(&session_id)
            .ok_or(Error::SessionNotFound)?;
            
        // Sandbox the command execution
        let output = self.sandbox.execute_sandboxed(
            command,
            SandboxConfig {
                working_dir: context.workspace.clone(),
                env_vars: session.env_vars.clone(),
                timeout: Duration::from_secs(30),
                memory_limit: 100 * 1024 * 1024, // 100MB
                cpu_limit: Duration::from_secs(10),
            },
        ).await?;
        
        Ok(ToolResult::success(json!({
            "output": output.stdout,
            "error": output.stderr,
            "exit_code": output.exit_code,
        })))
    }
}
```

## Process Sandboxing

### 1. Sandbox Implementation
```rust
pub struct ProcessSandbox {
    cgroup_manager: Arc<CGroupManager>,
    namespace_manager: Arc<NamespaceManager>,
}

impl ProcessSandbox {
    pub async fn execute_sandboxed(
        &self,
        command: &str,
        config: SandboxConfig,
    ) -> Result<ProcessOutput> {
        // Create new process with restrictions
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(&config.working_dir);
        
        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        // Apply resource limits
        unsafe {
            cmd.pre_exec(move || {
                // Set memory limit
                setrlimit(
                    Resource::RLIMIT_AS,
                    config.memory_limit,
                    config.memory_limit,
                )?;
                
                // Set CPU time limit
                setrlimit(
                    Resource::RLIMIT_CPU,
                    config.cpu_limit.as_secs(),
                    config.cpu_limit.as_secs(),
                )?;
                
                // Drop privileges if running as root
                if nix::unistd::geteuid().is_root() {
                    setgid(Gid::from_raw(1000))?;
                    setuid(Uid::from_raw(1000))?;
                }
                
                Ok(())
            });
        }
        
        // Execute with timeout
        let output = tokio::time::timeout(
            config.timeout,
            cmd.output()
        ).await??;
        
        Ok(ProcessOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
```

## Tool Permission System

### 1. Permission Manager
```rust
pub struct PermissionManager {
    policies: DashMap<UserId, PermissionPolicy>,
    default_policy: PermissionPolicy,
}

impl PermissionManager {
    pub fn check_permission(
        &self,
        user: &UserId,
        tool: &str,
        operation: &str,
    ) -> Result<()> {
        let policy = self.policies.get(user)
            .map(|p| p.clone())
            .unwrap_or_else(|| self.default_policy.clone());
            
        if !policy.is_allowed(tool, operation) {
            return Err(Error::PermissionDenied {
                user: user.clone(),
                tool: tool.to_string(),
                operation: operation.to_string(),
            });
        }
        
        Ok(())
    }
}

pub struct PermissionPolicy {
    allowed_tools: HashSet<String>,
    denied_operations: HashSet<(String, String)>,
    rate_limits: HashMap<String, RateLimit>,
}
```

## Rate Limiting

### 1. Tool Rate Limiter
```rust
use governor::{Quota, RateLimiter as Gov};

pub struct RateLimiter {
    limiters: DashMap<(UserId, String), Arc<Gov<NotKeyed>>>,
    default_quota: Quota,
}

impl RateLimiter {
    pub async fn check_rate_limit(
        &self,
        user: &UserId,
        tool: &str,
    ) -> Result<()> {
        let key = (user.clone(), tool.to_string());
        
        let limiter = self.limiters.entry(key)
            .or_insert_with(|| {
                Arc::new(Gov::new(
                    self.default_quota,
                    SystemClock::default(),
                ))
            });
            
        limiter.until_ready().await;
        Ok(())
    }
}
```

## Memory Profile
- **Tool registry**: 100KB
- **Sandbox overhead**: 500KB
- **Permission system**: 200KB
- **Rate limiters**: 300KB
- **Session management**: 1MB
- **Cache**: 1MB
- **Total**: ~3MB (vs 25MB Node.js)
