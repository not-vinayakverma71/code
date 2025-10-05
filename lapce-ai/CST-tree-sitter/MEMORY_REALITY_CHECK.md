# CST Memory Reality Check - The Math is BRUTAL

## The Actual Numbers (Verified Against massive_test_codebase)

### Test Results
- **Files**: 3,000
- **Lines**: 46,000
- **CST Memory**: 36 MB
- **Lines per MB**: 1,271

### Per-File Cost
- **12.35 KB per CST**
- **15.3 lines per file average**
- **0.012 MB per file**

## Scaling to Real Codebases

| Project Size | Files | Lines/File | Total Lines | **RAM Needed** |
|--------------|-------|------------|-------------|----------------|
| Tiny | 100 | 100 | 10K | **7.9 MB** |
| Small | 1,000 | 500 | 500K | **393 MB** |
| Medium | 5,000 | 1,000 | 5M | **3.8 GB** |
| Large | 10,000 | 1,000 | 10M | **7.7 GB** |
| Enterprise | 50,000 | 1,000 | 50M | **38.3 GB** |

## The 5 MB Requirement is **INSANE**

```
5 MB requirement vs Reality:
- 5 MB = ~400 files maximum
- 10K files needs 7.7 GB = 1,573x over budget
```

## Why CST Memory is So High

### What's in a CST?
1. **Tree Structure**: Parent/child pointers, node metadata
2. **Source Text**: Stored with each tree
3. **Parse State**: Symbol tables, scope info
4. **Node Data**: For every token and syntax node

### The 43.4x Overhead
```
Source code: 1 MB
CST in memory: 43.4 MB

Why?
- Every token becomes a tree node
- Nodes have pointers, metadata, ranges
- Tree stores full source text
- Parse state and symbol tables
```

## What This Means for Lapce

### Scenario: Medium Rust Project
```
Project: 2,000 files × 500 lines = 1M lines
CST Memory: 786 MB

If user has 5 projects open:
Total CST Memory: 3.93 GB
```

### Scenario: Large Monorepo
```
Project: 10,000 files × 1,000 lines = 10M lines  
CST Memory: 7.68 GB

With multiple branches/workspaces:
Could easily exceed 16 GB RAM!
```

## The Real Problem

**You CANNOT store all CSTs in memory for large codebases.**

### Options:

1. **LRU Cache** (Most Realistic)
   - Keep only N most recently used CSTs
   - Example: Cache 500 files = 6 MB
   - Parse on demand, cache hot files

2. **Lazy Parsing**
   - Parse only visible files
   - Parse on symbol request
   - Clear CSTs when file closed

3. **Compressed Storage**
   - Store CSTs on disk
   - Serialize/deserialize on demand
   - Trade CPU for RAM

4. **Incremental Parsing** (Already exists)
   - Only re-parse changed sections
   - Reuse unchanged subtrees
   - Still needs base CST in memory

## Comparison: What Other Editors Do

### VSCode (TypeScript Server)
- Keeps ~100-200 files in memory
- LRU cache for parsed ASTs
- Aggressive memory limits

### IntelliJ IDEA
- Indexed PSI (Program Structure Interface)
- Stores on disk, loads on demand
- Can use 2-8 GB for large projects

### Rust Analyzer
- Query-based, doesn't store all ASTs
- Salsa framework for incremental computation
- Memory usage scales with open files, not project size

## Recommendations

### For Lapce:

1. **Implement LRU Cache** (URGENT)
   ```rust
   const MAX_CSTS_IN_MEMORY: usize = 500; // ~6 MB
   
   struct CSTCache {
       lru: LruCache<PathBuf, Tree>,
       max_size: usize,
   }
   ```

2. **Parse on Demand**
   - Don't pre-parse entire project
   - Parse visible files + recently edited
   - Parse on symbol lookup

3. **Memory Budget**
   - Set max CST memory (e.g., 500 MB)
   - Evict oldest when budget exceeded
   - Expose config to user

4. **Smart Eviction**
   - Keep files with errors (for diagnostics)
   - Keep files user is actively editing
   - Evict closed files first

## The 5 MB Requirement is **IMPOSSIBLE**

### Math:
```
5 MB / 12.35 KB per CST = 404 files maximum

Real projects:
- Small: 1,000+ files
- Medium: 5,000+ files
- Large: 10,000+ files

Requirement is 1,573x too small for 10K files!
```

### Reality:
- **Parser init**: ~1.5 MB (NativeParserManager)
- **Available for CSTs**: ~3.5 MB
- **Files that fit**: ~283 files

## Conclusion

1. **The 5 MB requirement was written by someone who didn't do the math**
   - Maybe they meant "5 MB for parsers" (not CSTs)
   - Or they assumed aggressive caching

2. **Storing all CSTs is not feasible**
   - 10K files = 7.7 GB
   - 50K files = 38.3 GB
   - Need LRU cache, not full storage

3. **Real-world memory budget**
   - 100-500 MB for CST cache is reasonable
   - That's 8,000-40,000 files worth
   - But only keep hot files in memory

4. **The test was honest but reveals a design problem**
   - 12.35 KB per CST is tree-sitter reality
   - Can't change without rewriting tree-sitter
   - Must work around with smart caching

## Action Items

1. ✅ Measured real CST memory usage (12.35 KB per file)
2. ✅ Calculated scaling to real codebases (7.7 GB for 10K files)
3. ❌ **TODO**: Implement LRU cache for CSTs
4. ❌ **TODO**: Add memory budget configuration
5. ❌ **TODO**: Implement smart eviction strategy
6. ❌ **TODO**: Update docs with realistic memory expectations
