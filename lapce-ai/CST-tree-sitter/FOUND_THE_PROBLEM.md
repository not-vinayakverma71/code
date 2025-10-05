# FOUND THE ROOT CAUSE - MULTIPLE REDUNDANT CACHES

## The Problem

We have **MULTIPLE cache implementations** storing trees simultaneously:

### 1. TreeCache in NativeParserManager (src/native_parser_manager.rs:98)
```rust
pub struct NativeParserManager {
    tree_cache: Arc<TreeCache>,  // Stores up to 100 trees
}

pub struct CachedTree {
    pub tree: Tree,
    pub source: Bytes,
    pub version: u64,
    pub last_modified: SystemTime,
    pub file_type: FileType,
}
```

### 2. TreeSitterCache in IntegratedTreeSitter (src/integrated_system.rs:25)
```rust
pub struct IntegratedTreeSitter {
    parser_manager: Arc<NativeParserManager>,  // Has its own cache!
    cache: Arc<TreeSitterCache>,                 // DUPLICATE CACHE!
}
```

### 3. TreeSitterCache has TWO internal caches (src/cache_impl.rs:12-16)
```rust
pub struct TreeSitterCache {
    hot_trees: Cache<PathBuf, Arc<CachedTree>>,   // L1 cache
    warm_trees: Cache<PathBuf, Arc<CachedTree>>,  // L2 cache
}

// And it inserts into BOTH:
self.hot_trees.insert(path_buf.clone(), cached.clone());  // Line 137
self.warm_trees.insert(path_buf, cached.clone());         // Line 138
```

### 4. LRUParseCache ANOTHER cache (src/lru_cache.rs:19)
```rust
pub struct LRUParseCache {
    cache: Arc<RwLock<HashMap<PathBuf, CachedParseTree>>>,
}
```

### 5. IncrementalParserV2 storing trees (src/incremental_parser_v2.rs:42)
```rust
pub struct IncrementalParserV2 {
    old_trees: Arc<RwLock<HashMap<PathBuf, (Tree, Vec<u8>)>>>,
}
```

## The Memory Leak

When you parse a file through IntegratedTreeSitter:

1. NativeParserManager.parse_file() → stores in tree_cache
2. IntegratedTreeSitter.parse_file() → stores in cache (TreeSitterCache)
3. TreeSitterCache stores in BOTH hot_trees AND warm_trees
4. If incremental parsing is used → stores in IncrementalParserV2.old_trees

**Result: Same tree stored in 4-5 different places!**

## The Math

If each tree is 12 KB:
- 1 tree = 12 KB
- Stored 5x = 60 KB per file
- 10K files = 600 MB (just duplication)

Plus the actual tree memory (12 KB × 10K = 120 MB)
**Total: 720 MB just from duplication!**

## Why Test Shows 36 MB for 3K Files

3K files should be:
- 3K × 12 KB = 36 MB (what we measured)

But if we're duplicating 5x:
- 3K × 12 KB × 5 = 180 MB

We measured 36 MB, so maybe not all caches are active during the test?
Or Arc<CachedTree> is helping (reference counting)?

## Need To Verify

1. Are all these caches actually storing trees simultaneously?
2. Is Arc preventing full duplication?
3. How many times is the same tree actually duplicated in memory?
