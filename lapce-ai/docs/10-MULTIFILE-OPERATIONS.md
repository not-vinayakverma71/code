# Step 18: Multi-file Operations & Batch Edits
## Concurrent File Processing with Transaction Support

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**PRESERVE EXACT FILE OPERATION BEHAVIORS**

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex/`

## ✅ Success Criteria
- [ ] **Memory Usage**: < 1MB overhead for batch operations
- [ ] **Batch Size**: Handle 100+ file changes atomically
- [ ] **Transaction Speed**: < 500ms for 100 file operations
- [ ] **Rollback Support**: Full undo on any failure
- [ ] **Conflict Detection**: Same as TypeScript
- [ ] **Error Messages**: EXACT text for AI parsing
- [ ] **Validation**: Same file checks as Codex
- [ ] **Test Coverage**: All edge cases from TypeScript

## Overview
Multi-file operations enable AI to refactor entire codebases. Transaction support ensures consistency.

## Edit File Tool (EXACT BEHAVIOR)

### TypeScript Reference
```typescript
// From codex-reference/tools/editFileTool.ts
// Critical behaviors:
// - Line range validation
// - Content matching verification
// - Atomic file updates
```

### Rust Translation
```rust
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct EditFileTool {
    validator: FileValidator,
    backup_manager: BackupManager,
}

impl EditFileTool {
    pub async fn edit_file(&self, 
        path: &Path,
        start_line: usize,
        end_line: usize,
        new_content: &str
    ) -> Result<()> {
        // EXACT validation from TypeScript
        
        // Read file
        let content = tokio::fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().collect();
        
        // Validate line numbers (SAME as TypeScript)
        if start_line == 0 || start_line > lines.len() {
            return Err(Error::InvalidLineNumber(
                "Line numbers are 1-based".to_string() // EXACT error
            ));
        }
        
        if end_line < start_line || end_line > lines.len() {
            return Err(Error::InvalidRange);
        }
        
        // Create backup BEFORE edit
        self.backup_manager.backup(path).await?;
        
        // Build new content
        let mut result = Vec::new();
        result.extend_from_slice(&lines[..start_line - 1]); // 1-based!
        result.push(new_content);
        result.extend_from_slice(&lines[end_line..]); // Inclusive
        
        // Write atomically
        self.write_atomic(path, &result.join("\n")).await?;
        
        Ok(())
    }
    
    async fn write_atomic(&self, path: &Path, content: &str) -> Result<()> {
        // EXACT atomic write from TypeScript
        let temp_path = format!("{}.tmp", path.display());
        
        // Write to temp file
        tokio::fs::write(&temp_path, content).await?;
        
        // Atomic rename
        tokio::fs::rename(&temp_path, path).await?;
        
        Ok(())
    }
}
```

## Multi-Apply Diff Tool

### Batch Diff Application
```rust
pub struct MultiApplyDiffTool {
    applier: DiffApplier,
    transaction_manager: TransactionManager,
}

impl MultiApplyDiffTool {
    pub async fn apply_diffs(&self, diffs: Vec<DiffOperation>) -> Result<BatchResult> {
        // EXACT transaction behavior from TypeScript
        
        let transaction = self.transaction_manager.begin().await?;
        let mut results = Vec::new();
        
        for diff_op in diffs {
            match self.apply_single(&diff_op).await {
                Ok(()) => {
                    results.push(DiffResult::Success(diff_op.path.clone()));
                }
                Err(e) => {
                    // Rollback EVERYTHING on first failure
                    transaction.rollback().await?;
                    return Err(Error::BatchFailed {
                        failed_file: diff_op.path,
                        reason: e.to_string(), // EXACT error format
                    });
                }
            }
        }
        
        // Commit all changes atomically
        transaction.commit().await?;
        
        Ok(BatchResult {
            applied: results.len(),
            files: results,
        })
    }
}
```

## Search and Replace Tool

### Global Search/Replace
```rust
pub struct SearchAndReplaceTool {
    file_filter: FileFilter,
    content_matcher: ContentMatcher,
}

impl SearchAndReplaceTool {
    pub async fn search_and_replace(&self,
        pattern: &str,
        replacement: &str,
        options: SearchOptions
    ) -> Result<Vec<FileChange>> {
        // EXACT search logic from TypeScript
        
        let mut changes = Vec::new();
        let files = self.find_matching_files(&options).await?;
        
        for file_path in files {
            let content = tokio::fs::read_to_string(&file_path).await?;
            
            // Match EXACT replacement behavior
            let new_content = if options.use_regex {
                self.regex_replace(&content, pattern, replacement)?
            } else {
                content.replace(pattern, replacement) // Simple replace
            };
            
            if content != new_content {
                // Only write if changed
                changes.push(FileChange {
                    path: file_path.clone(),
                    old_content: content,
                    new_content: new_content.clone(),
                });
                
                // Write changes
                tokio::fs::write(&file_path, new_content).await?;
            }
        }
        
        Ok(changes)
    }
}
```

## Write to File Tool

### File Creation/Overwrite
```rust
pub struct WriteToFileTool {
    permission_checker: PermissionChecker,
}

impl WriteToFileTool {
    pub async fn write_file(&self,
        path: &Path,
        content: &str,
        options: WriteOptions
    ) -> Result<()> {
        // EXACT behavior from TypeScript
        
        // Check if file exists
        if path.exists() && !options.overwrite {
            return Err(Error::FileExists(
                format!("File {} already exists", path.display()) // EXACT message
            ));
        }
        
        // Check permissions
        if !self.permission_checker.can_write(path)? {
            return Err(Error::PermissionDenied);
        }
        
        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Write file
        tokio::fs::write(path, content).await?;
        
        Ok(())
    }
}
```

## Transaction Management

```rust
pub struct TransactionManager {
    backups: Arc<Mutex<Vec<Backup>>>,
}

impl TransactionManager {
    pub async fn begin(&self) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            backups: self.backups.clone(),
            operations: Vec::new(),
        }
    }
}

pub struct Transaction {
    id: Uuid,
    backups: Arc<Mutex<Vec<Backup>>>,
    operations: Vec<Operation>,
}

impl Transaction {
    pub async fn rollback(&self) -> Result<()> {
        // Restore ALL files to original state
        let backups = self.backups.lock().await;
        for backup in backups.iter().rev() {
            tokio::fs::copy(&backup.backup_path, &backup.original_path).await?;
        }
        Ok(())
    }
    
    pub async fn commit(&self) -> Result<()> {
        // Clean up backup files
        let backups = self.backups.lock().await;
        for backup in backups.iter() {
            tokio::fs::remove_file(&backup.backup_path).await.ok();
        }
        Ok(())
    }
}
```

## Concurrent File Processing

```rust
pub struct BatchProcessor {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl BatchProcessor {
    pub async fn process_files(&self, 
        files: Vec<PathBuf>,
        operation: impl Fn(PathBuf) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static
    ) -> Vec<Result<()>> {
        // Process files concurrently with limit
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        
        let futures: Vec<_> = files.into_iter().map(|file| {
            let sem = semaphore.clone();
            let op = operation.clone();
            
            async move {
                let _permit = sem.acquire().await.unwrap();
                op(file).await
            }
        }).collect();
        
        futures::future::join_all(futures).await
    }
}
```

## Testing Requirements

```rust
#[tokio::test]
async fn edit_file_matches_typescript() {
    let tool = EditFileTool::new();
    
    // Test 1-based line numbers
    let result = tool.edit_file(
        Path::new("test.txt"),
        1, // 1-based!
        3,
        "new content"
    ).await;
    
    let ts_result = load_typescript_result("edit_result.txt");
    assert_eq!(result, ts_result);
}

#[tokio::test]
async fn transaction_rollback_works() {
    let manager = TransactionManager::new();
    let tx = manager.begin().await;
    
    // Make changes
    // ...
    
    // Rollback
    tx.rollback().await.unwrap();
    
    // Verify all files restored
    // ...
}
```

## Implementation Checklist
- [ ] Port editFileTool.ts exactly
- [ ] Port multiApplyDiffTool.ts exactly
- [ ] Port searchAndReplaceTool.ts exactly
- [ ] Port writeToFileTool.ts exactly
- [ ] Preserve 1-based line numbers
- [ ] Match error messages exactly
- [ ] Implement transactions
- [ ] Test concurrent operations
