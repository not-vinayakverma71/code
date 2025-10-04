# Phase 6: Git Integration & Multi-file Operations (1.5 weeks)
## Production-Grade Version Control with Batch Operations

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **File Operations**: All multi-file edits work EXACTLY like Codex
- [ ] **Memory Target**: < 3MB for entire Git/diff system
- [ ] **Diff Processing**: < 100ms for large diffs (1K+ lines)
- [ ] **Batch Operations**: Handle 100+ file changes simultaneously
- [ ] **Git Performance**: < 50ms for status/diff operations
- [ ] **Error Messages**: EXACT same text as Codex tools
- [ ] **Line Numbers**: Preserve 1-based/0-based handling exactly
- [ ] **Stress Test**: Process 10K file operations without memory growth

**GATE**: Phase 7 starts ONLY when file operations are INDISTINGUISHABLE from Codex.

## ⚠️ TYPESCRIPT → RUST SYNTAX CHANGE ONLYMUST BE FOLLOWED : TYPESCRIPT → RUST SYNTAX CHANGE ONLY
**TRANSLATE LINE-BY-LINE - DO NOT REDESIGN**

**MANDATORY TRANSLATION FROM**:
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/editFileTool.ts` → `edit_file_tool.rs`
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/applyDiffTool.ts` → `apply_diff_tool.rs`
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/searchAndReplaceTool.ts` → `search_and_replace_tool.rs`

**COPY EXACTLY**:
- Same line number handling (keep 1-based if that's what it uses)
- Same diff format (unified diff, etc.)
- Same error messages (CHARACTER-FOR-CHARACTER)
- Same validation logic
- Same file handling
- Years of edge cases handled - preserve ALL

### Week 1: Git Integration & Diff Management
**Goal:** Native git operations with minimal overhead
**Memory Target:** < 3MB

### Native Git Integration
```rust
use git2::{Repository, Diff, DiffOptions, Status, Oid};
use tokio::sync::RwLock;

pub struct GitIntegration {
    repo: Arc<RwLock<Repository>>,
    diff_cache: Arc<DashMap<PathBuf, CachedDiff>>,
    status_cache: Arc<RwLock<StatusCache>>,
    commit_analyzer: Arc<CommitAnalyzer>,
}

impl GitIntegration {
    pub fn new(workspace: &Path) -> Result<Self> {
        let repo = Repository::discover(workspace)?;
        
        Ok(Self {
            repo: Arc::new(RwLock::new(repo)),
            diff_cache: Arc::new(DashMap::new()),
            status_cache: Arc::new(RwLock::new(StatusCache::new())),
            commit_analyzer: Arc::new(CommitAnalyzer::new()),
        })
    }
    
    pub async fn get_file_diff(&self, path: &Path) -> Result<FileDiff> {
        // Check cache first
        if let Some(cached) = self.diff_cache.get(path) {
            if cached.is_valid() {
                return Ok(cached.diff.clone());
            }
        }
        
        let repo = self.repo.read().await;
        let mut diff_options = DiffOptions::new();
        diff_options.pathspec(path);
        
        // Get diff between index and working directory
        let diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;
        
        let mut file_diff = FileDiff::default();
        diff.foreach(
            &mut |delta, _| {
                file_diff.status = delta.status();
                true
            },
            None,
            Some(&mut |_delta, _hunk, line| {
                file_diff.lines.push(DiffLine {
                    origin: line.origin(),
                    content: String::from_utf8_lossy(line.content()).to_string(),
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                });
                true
            }),
            None,
        )?;
        
        // Cache the diff
        self.diff_cache.insert(path.to_owned(), CachedDiff {
            diff: file_diff.clone(),
            timestamp: Instant::now(),
        });
        
        Ok(file_diff)
    }
    
    pub async fn stage_files(&self, paths: &[PathBuf]) -> Result<()> {
        let repo = self.repo.write().await;
        let mut index = repo.index()?;
        
        for path in paths {
            index.add_path(path)?;
        }
        
        index.write()?;
        self.invalidate_cache(paths).await;
        Ok(())
    }
}
```

### AI-Powered Commit Messages
```rust
pub struct CommitAnalyzer {
    ai_provider: Arc<dyn AIProvider>,
    template_engine: Arc<TemplateEngine>,
    history_analyzer: Arc<HistoryAnalyzer>,
}

impl CommitAnalyzer {
    pub async fn generate_commit_message(&self, diff: &Diff) -> Result<String> {
        // Analyze diff to understand changes
        let changes = self.analyze_changes(diff)?;
        
        // Build context from recent commits
        let context = self.history_analyzer.get_commit_context().await?;
        
        // Generate message using AI
        let prompt = self.template_engine.render("commit_message", json!({
            "changes": changes,
            "context": context,
            "guidelines": self.get_commit_guidelines(),
        }))?;
        
        let response = self.ai_provider.complete(CompletionRequest {
            messages: vec![Message {
                role: "system",
                content: "Generate a conventional commit message",
            }, Message {
                role: "user",
                content: prompt,
            }],
            temperature: 0.3,
            max_tokens: 200,
            ..Default::default()
        }).await?;
        
        // Format according to conventional commits
        self.format_commit_message(&response.content)
    }
    
    fn analyze_changes(&self, diff: &Diff) -> Result<ChangeAnalysis> {
        let mut analysis = ChangeAnalysis::default();
        
        diff.foreach(
            &mut |delta, _| {
                analysis.files_changed += 1;
                analysis.file_types.insert(
                    Self::detect_file_type(&delta.new_file().path())
                );
                true
            },
            None,
            Some(&mut |_delta, _hunk, line| {
                match line.origin() {
                    '+' => analysis.lines_added += 1,
                    '-' => analysis.lines_removed += 1,
                    _ => {}
                }
                
                // Detect patterns in changes
                let content = String::from_utf8_lossy(line.content());
                if content.contains("TODO") || content.contains("FIXME") {
                    analysis.has_todos = true;
                }
                if content.contains("test") || content.contains("spec") {
                    analysis.has_tests = true;
                }
                
                true
            }),
            None,
        )?;
        
        Ok(analysis)
    }
}
```

### Week 1.5: Multi-file Operations
**Goal:** Efficient batch file operations with rollback
**Memory Target:** < 2MB

```rust
pub struct MultiFileOperator {
    workspace: Arc<WorkspaceManager>,
    transaction_log: Arc<TransactionLog>,
    file_watcher: Arc<FileWatcher>,
}

pub struct FileTransaction {
    id: Uuid,
    operations: Vec<FileOperation>,
    rollback_data: Vec<RollbackData>,
    status: TransactionStatus,
}

impl MultiFileOperator {
    pub async fn execute_refactoring(&self, refactoring: Refactoring) -> Result<()> {
        let transaction = self.begin_transaction().await?;
        
        match refactoring {
            Refactoring::Rename { from, to } => {
                self.rename_symbol(&transaction, from, to).await?
            }
            Refactoring::ExtractFunction { range, name } => {
                self.extract_function(&transaction, range, name).await?
            }
            Refactoring::MoveModule { module, target } => {
                self.move_module(&transaction, module, target).await?
            }
        }
        
        self.commit_transaction(transaction).await
    }
    
    async fn rename_symbol(&self, tx: &FileTransaction, old: &str, new: &str) -> Result<()> {
        // Find all occurrences across files
        let occurrences = self.workspace.find_symbol_occurrences(old).await?;
        
        // Group by file for efficient processing
        let mut files_to_modify: HashMap<PathBuf, Vec<TextEdit>> = HashMap::new();
        
        for occurrence in occurrences {
            files_to_modify.entry(occurrence.file.clone())
                .or_default()
                .push(TextEdit {
                    range: occurrence.range,
                    new_text: new.to_string(),
                });
        }
        
        // Apply edits with rollback capability
        for (file, edits) in files_to_modify {
            self.apply_edits_transactional(&tx, &file, edits).await?;
        }
        
        Ok(())
    }
    
    async fn apply_edits_transactional(
        &self,
        tx: &FileTransaction,
        file: &Path,
        mut edits: Vec<TextEdit>,
    ) -> Result<()> {
        // Read file once
        let original_content = tokio::fs::read_to_string(file).await?;
        
        // Store rollback data
        self.transaction_log.store_rollback(tx.id, file, &original_content).await?;
        
        // Sort edits by position (reverse order to maintain positions)
        edits.sort_by_key(|e| std::cmp::Reverse((e.range.start.line, e.range.start.character)));
        
        // Apply edits
        let mut content = original_content.clone();
        for edit in edits {
            content = self.apply_edit(&content, &edit)?;
        }
        
        // Write atomically
        self.atomic_write(file, &content).await?;
        
        Ok(())
    }
    
    async fn atomic_write(&self, path: &Path, content: &str) -> Result<()> {
        let temp_path = path.with_extension("tmp");
        
        // Write to temp file
        tokio::fs::write(&temp_path, content).await?;
        
        // Atomic rename
        tokio::fs::rename(&temp_path, path).await?;
        
        Ok(())
    }
}
```

### Project-wide Search & Replace
```rust
pub struct ProjectSearcher {
    index: Arc<SearchIndex>,
    regex_engine: Arc<RegexEngine>,
    parallel_executor: Arc<ParallelExecutor>,
}

impl ProjectSearcher {
    pub async fn search_and_replace(
        &self,
        pattern: &str,
        replacement: &str,
        options: SearchOptions,
    ) -> Result<SearchResults> {
        // Compile regex once
        let regex = self.regex_engine.compile(pattern, options.regex_flags)?;
        
        // Get file list based on options
        let files = self.get_target_files(&options).await?;
        
        // Process files in parallel with controlled concurrency
        let semaphore = Arc::new(Semaphore::new(num_cpus::get()));
        let results = Arc::new(Mutex::new(Vec::new()));
        
        let tasks: Vec<_> = files.into_iter().map(|file| {
            let regex = regex.clone();
            let replacement = replacement.to_string();
            let semaphore = semaphore.clone();
            let results = results.clone();
            
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await?;
                
                // Memory-mapped file reading
                let mmap = unsafe { 
                    MmapOptions::new()
                        .map(&File::open(&file)?)?
                };
                
                let content = std::str::from_utf8(&mmap)?;
                let matches = regex.find_iter(content)
                    .map(|m| Match {
                        file: file.clone(),
                        range: m.range(),
                        text: m.as_str().to_string(),
                    })
                    .collect::<Vec<_>>();
                
                if !matches.is_empty() {
                    results.lock().await.push((file, matches));
                }
                
                Ok::<_, anyhow::Error>(())
            })
        }).collect();
        
        // Wait for all tasks
        futures::future::try_join_all(tasks).await?;
        
        let results = Arc::try_unwrap(results)
            .unwrap()
            .into_inner();
            
        Ok(SearchResults {
            matches: results,
            total_files_searched: files.len(),
        })
    }
}
```

### Workspace Management
```rust
pub struct WorkspaceManager {
    root: PathBuf,
    file_index: Arc<FileIndex>,
    ignore_matcher: Arc<IgnoreMatcher>,
    dependency_graph: Arc<DependencyGraph>,
}

impl WorkspaceManager {
    pub async fn analyze_dependencies(&self) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        
        // Walk all source files
        let files = self.get_source_files().await?;
        
        for file in files {
            let deps = self.extract_dependencies(&file).await?;
            graph.add_file_dependencies(file, deps);
        }
        
        // Detect circular dependencies
        if let Some(cycle) = graph.find_cycle() {
            tracing::warn!("Circular dependency detected: {:?}", cycle);
        }
        
        Ok(graph)
    }
    
    pub async fn find_unused_code(&self) -> Vec<UnusedCode> {
        let mut unused = Vec::new();
        
        // Get all exported symbols
        let all_exports = self.file_index.get_all_exports().await;
        
        // Find which ones are never imported
        for (file, exports) in all_exports {
            for export in exports {
                if !self.is_symbol_used(&export).await {
                    unused.push(UnusedCode {
                        file,
                        symbol: export,
                        kind: UnusedKind::Export,
                    });
                }
            }
        }
        
        unused
    }
}
```

## Conflict Resolution
```rust
pub struct ConflictResolver {
    ai_provider: Arc<dyn AIProvider>,
    merge_engine: Arc<MergeEngine>,
}

impl ConflictResolver {
    pub async fn resolve_merge_conflict(&self, conflict: &MergeConflict) -> Result<Resolution> {
        // Try automatic resolution first
        if let Some(resolution) = self.merge_engine.try_auto_resolve(conflict)? {
            return Ok(resolution);
        }
        
        // Use AI for complex conflicts
        let prompt = format!(
            "Resolve this merge conflict:\n<<<<<<< HEAD\n{}\n=======\n{}\n>>>>>>> {}",
            conflict.ours, conflict.theirs, conflict.branch
        );
        
        let response = self.ai_provider.complete(/* ... */).await?;
        
        Ok(Resolution {
            content: response.content,
            confidence: self.calculate_confidence(&response),
        })
    }
}
```

## Dependencies
```toml
[dependencies]
# Git operations
git2 = "0.19"

# File watching
notify = "6.1"

# Parallel processing
rayon = "1.10"
num_cpus = "1.16"

# Regex
regex = "1.10"

# Transactions
uuid = { version = "1.10", features = ["v4"] }
```

## Expected Results - Phase 6
- **Total Memory**: < 5MB for all git/file operations
- **Diff Generation**: < 10ms for 1000-line file
- **Multi-file Rename**: < 100ms for 100 occurrences
- **Search & Replace**: < 500ms for 10K files
- **Commit Message Generation**: < 2s with AI
- **Dependency Analysis**: < 5s for large projects
