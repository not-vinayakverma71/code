# CHUNK-16: CORE/PROTECT - FILE ACCESS CONTROL

## 📁 MODULE STRUCTURE

```
Codex/src/core/protect/
├── RooProtectedController.ts        (112 lines)
└── __tests__/
    └── RooProtectedController.spec.ts (164 lines)
```

**Total**: 276 lines analyzed

---

## 🎯 PURPOSE

Prevent AI from auto-modifying sensitive configuration files without explicit user approval. This is a security layer that protects Kilocode/Roo configuration files from accidental or unauthorized modifications.

---

## 🔒 CORE: RooProtectedController.ts

### Architecture

```typescript
export class RooProtectedController {
    private cwd: string
    private ignoreInstance: Ignore  // From 'ignore' npm package
    
    private static readonly PROTECTED_PATTERNS = [
        ".kilocodeignore",
        ".kilocodemodes",
        ".kilocoderules",
        ".kilocode/**",
        ".kilocodeprotected",
        ".rooignore",
        ".roomodes",
        ".roorules*",
        ".clinerules*",
        ".roo/**",
        ".vscode/**",
        ".rooprotected",
        "AGENTS.md",
        "AGENT.md",
    ]
    
    constructor(cwd: string) {
        this.cwd = cwd
        this.ignoreInstance = ignore()
        this.ignoreInstance.add(RooProtectedController.PROTECTED_PATTERNS)
    }
}
```

**Key Design**: Uses the `ignore` library (same as .gitignore) for pattern matching.

---

## 📋 PROTECTED FILE PATTERNS

### 1. Kilocode Configuration Files
```
.kilocodeignore     - Ignore patterns for AI
.kilocodemodes      - AI mode configurations
.kilocoderules      - Custom rules
.kilocode/**        - All files in .kilocode directory
.kilocodeprotected  - Protection marker
```

### 2. Legacy Roo Files (Backward Compatibility)
```
.rooignore         - Legacy ignore file
.roomodes          - Legacy modes
.roorules*         - Legacy rules (with wildcard)
.clinerules*       - CLI-specific rules
.roo/**            - All files in .roo directory
.rooprotected      - Legacy protection marker
```

### 3. IDE Configuration
```
.vscode/**         - VSCode settings, launch configs, tasks
```

### 4. Agent Configuration
```
AGENTS.md          - Multi-agent configuration
AGENT.md           - Single agent configuration (alternative)
```

---

## 🛠️ API METHODS

### 1. Check Single File Protection

```typescript
isWriteProtected(filePath: string): boolean {
    try {
        // Convert to absolute path
        const absolutePath = path.resolve(this.cwd, filePath)
        
        // Make relative to workspace root
        const relativePath = path.relative(this.cwd, absolutePath).toPosix()
        
        // Use ignore library pattern matching
        return this.ignoreInstance.ignores(relativePath)
    } catch (error) {
        console.error(`Error checking protection for ${filePath}:`, error)
        return false  // Fail open (allow access on error)
    }
}
```

**Error Handling**: Paths outside cwd throw errors → returns `false` (not protected)

**Path Normalization**: 
- Converts to absolute
- Makes relative to cwd
- Converts to POSIX format (forward slashes)

### 2. Batch File Filtering

```typescript
getProtectedFiles(paths: string[]): Set<string> {
    const protectedFiles = new Set<string>()
    
    for (const filePath of paths) {
        if (this.isWriteProtected(filePath)) {
            protectedFiles.add(filePath)
        }
    }
    
    return protectedFiles
}
```

**Returns**: Set of protected file paths from input array

### 3. Annotate Paths

```typescript
annotatePathsWithProtection(paths: string[]): Array<{
    path: string
    isProtected: boolean
}> {
    return paths.map((filePath) => ({
        path: filePath,
        isProtected: this.isWriteProtected(filePath),
    }))
}
```

**Use Case**: UI display - show 🛡️ shield icon next to protected files

### 4. User Messages

```typescript
getProtectionMessage(): string {
    return "This is a Kilo Code configuration file and requires approval for modifications"
}

getInstructions(): string {
    const patterns = RooProtectedController.PROTECTED_PATTERNS.join(", ")
    return `# Protected Files

(The following Kilo Code configuration file patterns are write-protected and always require approval for modifications, regardless of autoapproval settings. When using list_files, you'll notice a 🛡️ next to files that are write-protected.)

Protected patterns: ${patterns}`
}
```

**Purpose**: Inject into AI system prompt to inform about restrictions

---

## 🧪 TEST COVERAGE (164 lines)

### Test Cases

**Basic Protection Tests**:
- ✅ `.rooignore` is protected
- ✅ `.roo/config.json` is protected
- ✅ `.roo/settings/user.json` is protected (nested)
- ✅ `.vscode/settings.json` is protected
- ✅ `AGENTS.md` and `AGENT.md` are protected

**Negative Tests**:
- ✅ `src/index.ts` is NOT protected
- ✅ `.roosettings` is NOT protected (doesn't match pattern)
- ✅ `src/roo-utils.ts` is NOT protected (roo not at start)

**Edge Cases**:
- ✅ Nested paths work: `nested/.rooignore` is protected
- ✅ Absolute paths converted to relative
- ✅ Path separators normalized (\ → /)

**Batch Operations**:
- ✅ `getProtectedFiles()` filters correctly
- ✅ `annotatePathsWithProtection()` annotates correctly

---

## 🔄 INTEGRATION POINTS

### 1. Tool Execution (write_to_file, replace_file_content)
```typescript
// Before file write
if (protectedController.isWriteProtected(filePath)) {
    // Require explicit user approval
    await requestUserApproval(filePath)
}
```

### 2. File Listing (list_files tool)
```typescript
const annotated = protectedController.annotatePathsWithProtection(files)
// Display: "📁 src/main.rs"
// Display: "🛡️ .kilocode/rules.md"
```

### 3. System Prompt Injection
```typescript
const instructions = protectedController.getInstructions()
systemPrompt += "\n\n" + instructions
```

---

## 🦀 RUST TRANSLATION

```rust
use ignore::{gitignore::GitignoreBuilder, Match};
use std::path::{Path, PathBuf};

pub const SHIELD_SYMBOL: &str = "🛡️";

pub struct RooProtectedController {
    cwd: PathBuf,
    patterns: Vec<String>,
}

impl RooProtectedController {
    const PROTECTED_PATTERNS: &'static [&'static str] = &[
        ".kilocodeignore",
        ".kilocodemodes",
        ".kilocoderules",
        ".kilocode/**",
        ".kilocodeprotected",
        ".rooignore",
        ".roomodes",
        ".roorules*",
        ".clinerules*",
        ".roo/**",
        ".vscode/**",
        ".rooprotected",
        "AGENTS.md",
        "AGENT.md",
    ];
    
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            patterns: Self::PROTECTED_PATTERNS.iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
    
    pub fn is_write_protected(&self, file_path: &Path) -> bool {
        // Convert to absolute path
        let absolute = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.cwd.join(file_path)
        };
        
        // Make relative to cwd
        let relative = match absolute.strip_prefix(&self.cwd) {
            Ok(rel) => rel,
            Err(_) => return false, // Outside cwd
        };
        
        // Check against patterns using glob matching
        for pattern in &self.patterns {
            if self.matches_pattern(relative, pattern) {
                return true;
            }
        }
        
        false
    }
    
    fn matches_pattern(&self, path: &Path, pattern: &str) -> bool {
        // Use glob crate for pattern matching
        if let Ok(glob) = glob::Pattern::new(pattern) {
            return glob.matches_path(path);
        }
        false
    }
    
    pub fn get_protected_files(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        paths.iter()
            .filter(|p| self.is_write_protected(p))
            .cloned()
            .collect()
    }
    
    pub fn annotate_paths_with_protection(&self, paths: &[PathBuf]) 
        -> Vec<(PathBuf, bool)> 
    {
        paths.iter()
            .map(|p| (p.clone(), self.is_write_protected(p)))
            .collect()
    }
    
    pub fn get_protection_message(&self) -> &'static str {
        "This is a Kilo Code configuration file and requires approval for modifications"
    }
    
    pub fn get_instructions(&self) -> String {
        let patterns = self.patterns.join(", ");
        format!(
            "# Protected Files\n\n\
            (The following Kilo Code configuration file patterns are write-protected \
            and always require approval for modifications, regardless of autoapproval \
            settings. When using list_files, you'll notice a {} next to files that are \
            write-protected.)\n\n\
            Protected patterns: {}",
            SHIELD_SYMBOL,
            patterns
        )
    }
}
```

### Alternative: Use `ignore` Crate

```rust
use ignore::gitignore::{Gitignore, GitignoreBuilder};

pub struct RooProtectedController {
    cwd: PathBuf,
    gitignore: Gitignore,
}

impl RooProtectedController {
    pub fn new(cwd: impl Into<PathBuf>) -> Result<Self> {
        let cwd = cwd.into();
        let mut builder = GitignoreBuilder::new(&cwd);
        
        for pattern in Self::PROTECTED_PATTERNS {
            builder.add_line(None, pattern)?;
        }
        
        Ok(Self {
            cwd,
            gitignore: builder.build()?,
        })
    }
    
    pub fn is_write_protected(&self, file_path: &Path) -> bool {
        let relative = match file_path.strip_prefix(&self.cwd) {
            Ok(rel) => rel,
            Err(_) => return false,
        };
        
        self.gitignore.matched(relative, false).is_ignore()
    }
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Fail-Open vs Fail-Closed
**Current**: Fail-open (returns `false` on error)
- Paths outside cwd → not protected
- Parse errors → not protected

**Rationale**: Avoid blocking valid operations due to edge cases

### 2. Pattern Matching Strategy
**Uses**: `ignore` npm library (gitignore syntax)
- Supports wildcards: `.roorules*`
- Supports directory globs: `.roo/**`
- Supports anywhere matching: `.rooignore` matches at any depth

### 3. Shield Symbol in UI
**Unicode**: U+1F6E1 (🛡️)
- Visual indicator in file listings
- Helps users identify protected files

### 4. Backward Compatibility
**Supports both**:
- Modern: `.kilocode*`
- Legacy: `.roo*`

**Why**: Migration path for existing users

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `ignore` (^5.2.0) - Gitignore pattern matching
- `path` (Node.js builtin)

**Rust Crates**:
- `ignore` (0.4) - Gitignore matching (BurntSushi)
- `glob` (0.3) - Alternative pattern matching

---

## 🚀 PERFORMANCE CONSIDERATIONS

### TypeScript
- **Pattern Matching**: O(n) where n = number of patterns
- **Batch Operations**: O(m × n) where m = files, n = patterns
- **Memory**: Negligible (pattern list cached)

### Rust
- **Pattern Matching**: Same O(n) but ~10x faster compilation
- **Memory**: Stack-allocated patterns
- **Zero-Copy**: Path operations use `&Path` references

---

## 🎓 KEY TAKEAWAYS

✅ **Simple Design**: Single file, single responsibility

✅ **Comprehensive Protection**: 14 pattern types covering all config files

✅ **Well-Tested**: 164 lines of tests for 112 lines of code

✅ **Reusable**: Uses standard `ignore` library

✅ **UI Integration**: Shield symbol for visual feedback

✅ **Rust Translation**: Straightforward with `ignore` crate or `glob`

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Low
**Estimated Effort**: 2-3 hours
**Lines of Rust**: ~150 lines (similar to TypeScript)
**Dependencies**: `ignore` crate (already used in Lapce)
**Risk**: Very low - simple pattern matching

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-17 (sliding-window/)
