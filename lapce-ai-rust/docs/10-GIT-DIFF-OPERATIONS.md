# Step 17: Git Integration & Diff Management
## Native Git Operations with Exact Tool Behavior

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**PRESERVE YEARS OF GIT HANDLING LOGIC**

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/Codex/`

## ✅ Success Criteria
- [ ] **Memory Usage**: < 2MB for git operations
- [ ] **Diff Processing**: < 100ms for 1K+ line diffs
- [ ] **Git Commands**: < 50ms for status/diff
- [ ] **Diff Format**: EXACT unified diff format
- [ ] **Line Numbers**: Same 1-based/0-based handling
- [ ] **Error Messages**: CHARACTER-FOR-CHARACTER match
- [ ] **Patch Application**: 100% success rate
- [ ] **Test Coverage**: All edge cases from TypeScript

## Overview
Git integration handles version control operations. Line number handling and diff formats are CRITICAL for AI tools.

## Diff Format (MUST MATCH EXACTLY)

### Unified Diff Format from TypeScript
```typescript
// From codex-reference/diff/
// EXACT format AI expects:
--- a/file.ts
+++ b/file.ts
@@ -10,7 +10,7 @@
 context line
-removed line
+added line
 context line
```

### Rust Translation
```rust
use git2::{Repository, Diff, DiffOptions, DiffLine};

pub struct GitManager {
    repo: Repository,
    diff_options: DiffOptions,
}

impl GitManager {
    pub fn create_diff(&self, old_content: &str, new_content: &str, file_path: &str) -> String {
        // EXACT unified diff format from TypeScript
        let mut diff = String::new();
        
        // Headers MUST match exactly
        diff.push_str(&format!("--- a/{}\n", file_path));
        diff.push_str(&format!("+++ b/{}\n", file_path));
        
        // Generate hunks EXACTLY as TypeScript
        let hunks = self.generate_hunks(old_content, new_content);
        for hunk in hunks {
            // Line numbers format MUST match
            diff.push_str(&format!("@@ -{},{} +{},{} @@\n",
                hunk.old_start, hunk.old_lines,
                hunk.new_start, hunk.new_lines
            ));
            
            for line in hunk.lines {
                match line.line_type {
                    LineType::Context => diff.push_str(&format!(" {}\n", line.content)),
                    LineType::Deletion => diff.push_str(&format!("-{}\n", line.content)),
                    LineType::Addition => diff.push_str(&format!("+{}\n", line.content)),
                }
            }
        }
        
        diff
    }
}
```

## Apply Diff Tool (EXACT BEHAVIOR)

### TypeScript Reference
```typescript
// From codex-reference/tools/applyDiffTool.ts
// Critical behaviors:
// - Line number validation
// - Conflict detection
// - Partial application support
```

### Rust Translation
```rust
pub struct DiffApplier {
    conflict_strategy: ConflictStrategy,
}

impl DiffApplier {
    pub fn apply_diff(&self, file_content: &str, diff: &str) -> Result<String> {
        // EXACT application logic from TypeScript
        
        // Parse diff EXACTLY as TypeScript
        let hunks = self.parse_unified_diff(diff)?;
        
        let mut lines: Vec<&str> = file_content.lines().collect();
        let mut offset = 0i32;
        
        for hunk in hunks {
            // Line number handling MUST match
            // TypeScript uses 1-based, adjust if needed
            let start_line = (hunk.old_start as i32 - 1 + offset) as usize;
            
            // Validate context EXACTLY as TypeScript
            if !self.validate_context(&lines, &hunk, start_line) {
                return Err(Error::ContextMismatch);
            }
            
            // Apply changes
            self.apply_hunk(&mut lines, &hunk, start_line);
            offset += (hunk.new_lines as i32) - (hunk.old_lines as i32);
        }
        
        Ok(lines.join("\n"))
    }
    
    fn validate_context(&self, lines: &[&str], hunk: &Hunk, start: usize) -> bool {
        // EXACT validation from TypeScript
        // Check context lines match
        true
    }
}
```

## Git Status & File State

### File Status Detection
```rust
pub struct GitStatus {
    repo: Arc<Repository>,
}

impl GitStatus {
    pub fn get_file_status(&self, path: &Path) -> FileStatus {
        // EXACT status mapping from TypeScript
        let status = self.repo.status_file(path).unwrap();
        
        match status {
            s if s.contains(git2::Status::WT_NEW) => FileStatus::Untracked,
            s if s.contains(git2::Status::WT_MODIFIED) => FileStatus::Modified,
            s if s.contains(git2::Status::INDEX_NEW) => FileStatus::Added,
            s if s.contains(git2::Status::INDEX_DELETED) => FileStatus::Deleted,
            s if s.contains(git2::Status::CONFLICTED) => FileStatus::Conflicted,
            _ => FileStatus::Unchanged,
        }
    }
    
    pub fn get_repo_status(&self) -> RepoStatus {
        // Format EXACTLY as TypeScript for AI
        let statuses = self.repo.statuses(None).unwrap();
        
        RepoStatus {
            modified: self.count_status(&statuses, git2::Status::WT_MODIFIED),
            added: self.count_status(&statuses, git2::Status::INDEX_NEW),
            deleted: self.count_status(&statuses, git2::Status::INDEX_DELETED),
            untracked: self.count_status(&statuses, git2::Status::WT_NEW),
            // ... EXACT counts as TypeScript
        }
    }
}
```

## Line Number Handling (CRITICAL)

```rust
pub struct LineNumberHandler {
    use_one_based: bool, // TypeScript uses 1-based
}

impl LineNumberHandler {
    pub fn to_internal(&self, line: usize) -> usize {
        // Convert from TypeScript format to internal
        if self.use_one_based {
            line - 1 // Convert 1-based to 0-based
        } else {
            line
        }
    }
    
    pub fn to_external(&self, line: usize) -> usize {
        // Convert to TypeScript expected format
        if self.use_one_based {
            line + 1 // Convert 0-based to 1-based
        } else {
            line
        }
    }
}
```

## Diff Caching

```rust
pub struct DiffCache {
    cache: LruCache<DiffKey, String>,
}

impl DiffCache {
    pub fn get_or_compute(&mut self, key: &DiffKey, compute: impl FnOnce() -> String) -> String {
        // SAME caching strategy as TypeScript
        if let Some(diff) = self.cache.get(key) {
            diff.clone()
        } else {
            let diff = compute();
            self.cache.put(key.clone(), diff.clone());
            diff
        }
    }
}
```

## Merge Conflict Resolution

```rust
pub struct ConflictResolver {
    strategy: MergeStrategy,
}

impl ConflictResolver {
    pub fn resolve_conflicts(&self, ours: &str, theirs: &str, base: &str) -> String {
        // EXACT conflict markers from TypeScript
        let mut result = String::new();
        
        // Format MUST match for AI to understand
        result.push_str("<<<<<<< HEAD\n");
        result.push_str(ours);
        result.push_str("\n=======\n");
        result.push_str(theirs);
        result.push_str("\n>>>>>>> branch\n");
        
        result
    }
}
```

## Testing Requirements

```rust
#[test]
fn diff_format_matches_typescript() {
    let old = "line1\nline2\nline3";
    let new = "line1\nmodified\nline3";
    
    let diff = git_manager.create_diff(old, new, "test.ts");
    let ts_diff = load_typescript_fixture("diff_output.txt");
    
    // CHARACTER-FOR-CHARACTER match
    assert_eq!(diff, ts_diff);
}

#[test]
fn apply_diff_identical_behavior() {
    let file = "original content";
    let diff = load_fixture("test.diff");
    
    let rust_result = applier.apply_diff(file, diff);
    let ts_result = load_typescript_result("applied.txt");
    
    assert_eq!(rust_result, ts_result);
}
```

## Implementation Checklist
- [ ] Port diff/ logic exactly
- [ ] Port applyDiffTool.ts line-by-line
- [ ] Preserve unified diff format
- [ ] Match line number handling
- [ ] Keep error messages identical
- [ ] Test git operations
- [ ] Verify diff application
- [ ] Cache diffs appropriately
