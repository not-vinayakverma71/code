# Tools, Multi-file Operations, Git Diff, and MCP (Consolidated)

This document consolidates and reconciles the following sources:
- `docs/10-GIT-DIFF-OPERATIONS.md`
- `docs/10-MULTIFILE-OPERATIONS.md`
- `docs/10-MCP&TOOLS-IMPLEMENTATION.md`

It preserves all rules and success criteria as a strict TypeScript → Rust translation while presenting a coherent, single specification.

## ⚠️ Critical Rules: 1:1 Translation Only
- Translate line-by-line from Codex TypeScript sources in `/home/verma/lapce/Codex/`
- Preserve unified diff format, exact line-number semantics, and error strings
- Maintain tool validation, permission checks, and rate limits exactly
- Only change programming language to Rust; do not redesign logic

---

## ✅ Combined Success Criteria
- [ ] Memory: Tools framework total < 3MB (registry, sandbox, permissions, rate limits)
- [ ] Git Ops: Status/diff < 50ms; process 1K+ line diffs < 100ms; unified diff FORMAT EXACT
- [ ] Multi-file: 100+ file edits atomically in < 500ms; full rollback on any failure
- [ ] Line Numbers: TypeScript’s 1-based external, internal conversions correct and identical
- [ ] Errors: Character-for-character text match with Codex
- [ ] Sandboxing: Resource limits (CPU/mem), timeouts, privilege drop
- [ ] Rate Limiting: Adaptive per user/tool with governor
- [ ] Tests: ApplyDiff, MultiApplyDiff, EditFile, SearchReplace parity with fixtures

---

## Architecture Overview

### MCP vs Native Tools (Boundary)
- MCP is for EXTERNAL communication only (e.g., browser_action or any remote tool).
- Native Tools are INTERNAL IDE/file-system/git/terminal operations (no MCP overhead).
- Maintain the same behavioral interfaces so AI can invoke tools identically; only the transport differs.

```
Lapce UI (AI panel, commands)
  │
  ├─ Native Tools API (filesystem/git/terminal/multi-file/search)
  │    • Zero-cost direct calls inside the engine
  │
  └─ MCP (External-only)  ← browser_action (and any future external tool)
       • Permission + sandbox + rate limit at protocol boundary
```

The original MCP tool designs (registry, permissions, sandbox, rate limiter) remain valid as a pattern. For internal tools we reuse those components without MCP serialization to eliminate overhead.

---

## Git Diff Operations (EXACT FORMAT)

### Unified Diff Format
```typescript
--- a/file.ts
+++ b/file.ts
@@ -10,7 +10,7 @@
 context line
-removed line
+added line
 context line
```

### Rust Translation (Skeleton)
```rust
use git2::{Repository, DiffOptions};

pub struct GitManager { repo: Repository, diff_options: DiffOptions }

impl GitManager {
    pub fn create_diff(&self, old_content: &str, new_content: &str, file_path: &str) -> String {
        // EXACT headers and hunk formatting as TypeScript
        // 1-based/0-based handling MUST match
        // Emit '+', '-', ' ' prefixes precisely
        // Keep context lines identical
        // Return single unified diff string
        "".to_string()
    }
}
```

### Apply Diff (EXACT Behavior)
- Parse unified diff into hunks with exact header and line handling.
- Validate context lines before applying; on mismatch, return the same error string as Codex.
- Adjust offsets exactly like TypeScript when multiple hunks are applied.

---

## Multi-file Operations (Atomic Transactions)

### Edit File Tool
- Validate 1-based line ranges and content expectations exactly.
- Create a backup before any write; write atomically via temp + rename.

```rust
pub struct EditFileTool { validator: FileValidator, backup_manager: BackupManager }
```

### Multi-Apply Diff Tool
- Begin a transaction, apply diffs in sequence.
- On first error, rollback all changes and emit the EXACT error format
  (`BatchFailed { failed_file, reason }`).

### Search and Replace Tool
- Match TypeScript behavior for regex vs literal
- Write only when content changes
- Return list of `FileChange` with old/new content for audit

### Write to File Tool
- Enforce `overwrite` option; error text must match Codex
- Ensure permissions and parent directory creation are identical

### Transaction Manager
- Tracks backups per operation; `rollback()` restores all originals in reverse order
- `commit()` removes backup files

### Concurrent Processing
- Limit concurrency via semaphore; process in bounded parallelism identical to Codex

---

## Native Tools API (Former MCP Examples)

We retain the validated patterns from MCP (permissions, rate limiting, sandbox) but as native modules for internal operations:

- Filesystem: `read`, `write`, `list`, `search`, `watch`
- Git: `status`, `diff`, `commit`, `branch`, `log`
- Terminal: `create`, `execute`, `read`, `close`
- Code Search: `semantic`, `syntax`, `hybrid` (bridges to Step 6 + Step 7)

### Permissions & Rate Limiting
```rust
pub struct PermissionManager { /* user policies, denied ops, per-tool limits */ }
pub struct RateLimiter { /* governor-based per-user/tool */ }
```

### Sandboxed Execution (Terminal)
- Use `tokio::process::Command`
- Apply `setrlimit` for CPU/memory, drop privileges if root
- Enforce timeout; return stdout/stderr/exit_code with same fields as TS

---

## Line Number Handling (CRITICAL)

```rust
pub struct LineNumberHandler { use_one_based: bool }

impl LineNumberHandler {
    pub fn to_internal(&self, line: usize) -> usize { if self.use_one_based { line - 1 } else { line } }
    pub fn to_external(&self, line: usize) -> usize { if self.use_one_based { line + 1 } else { line } }
}
```

Ensure all tools use the same 1-based external convention Codex uses.

---

## Diff Caching & Merge Conflict Resolution

- Diff caching preserves CPU when repeatedly generating identical diffs.
- Conflict resolver emits markers and output in the same format Codex expects.

---

## Testing Requirements (Consolidated)

- Diff format parity: CHARACTER-FOR-CHARACTER with TS fixtures
- ApplyDiff parity: identical outputs and failure modes
- EditFile: 1-based index tests and atomicity
- MultiApplyDiff: rollback path correctness
- Terminal sandbox: timeouts and limits honored
- Git status/diff: same mapping of `git2::Status` to string states

```rust
#[test]
fn diff_format_matches_typescript() { /* compare fixtures */ }

#[tokio::test]
async fn transaction_rollback_works() { /* restore from backups */ }
```

---

## Memory Profile (Targets)
- Tool registry + metrics: ~100KB
- Sandbox + permissions + rate limits: ~1MB
- Session management: ~1MB
- Caches (diff/query): ~500KB
- Total: ~3MB (vs ~25MB Node.js)

---

## Implementation Checklist (Unified)
- [ ] Port unified diff generation and application EXACTLY
- [ ] Implement EditFile, MultiApplyDiff, SearchAndReplace, WriteToFile with 1-based handling
- [ ] Build TransactionManager and backups
- [ ] Native Tools API with permissions, rate limits, sandbox
- [ ] Git status/diff mapping and performance targets
- [ ] Diff caching and conflict resolution
- [ ] Full test suite parity with TypeScript fixtures
- [ ] Keep MCP for external-only (e.g., browser_action), route internal tools via native modules
