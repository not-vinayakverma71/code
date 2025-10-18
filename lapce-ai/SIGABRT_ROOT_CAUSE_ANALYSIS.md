# SIGABRT Root Cause Analysis

## Status: malloc_consolidate() crash during test cleanup

### Timeline of Fixes
1. ✅ Fixed sled path collision - unique UUID paths per instance
2. ✅ Removed directory cleanup from Drop - avoid race with sled background threads  
3. ❌ Still crashes AFTER all tests pass successfully

### Critical Observation
```
test working_cache_system::tests::test_cache_system ... ok
malloc_consolidate(): invalid chunk size
SIGABRT (signal 6)
```

**Crash occurs AFTER test passes** → Problem is in cleanup phase, not test execution

### Root Cause: Sled Background Flush Threads

Sled uses background threads for async flushing. When parallel tests finish:
1. Multiple sled instances created in parallel (real_l2_cache + working_cache)
2. Tests complete and drop sled Arc<Db> references
3. Sled background threads still running, trying to flush
4. Test harness exits → heap being torn down
5. Sled threads access freed memory → malloc_consolidate() crash

### Why Individual Tests Pass
Running tests individually (`--test-threads=1`) works because:
- Only one sled instance active at a time
- Background threads finish before next test starts
- No concurrent heap access during cleanup

### Why Parallel Tests Crash
Running parallel tests causes:
- Multiple sled instances with background threads
- Race between test cleanup and sled flush threads
- Heap corruption when threads access freed memory

### Solution Options

#### Option 1: Disable Parallel Tests ❌
```bash
cargo test -- --test-threads=1
```
**Downside**: Slow, not production-ready

#### Option 2: Use TempDir Properly ✅
Keep TempDir alive until sled is fully closed:
```rust
let temp_dir = TempDir::new().unwrap();
let db = sled::open(temp_dir.path())?;
// temp_dir stays alive, sled can cleanup safely
```

#### Option 3: Explicit Sled Shutdown ✅  
Call flush + close explicitly before Drop:
```rust
impl Drop for RealL2Cache {
    fn drop(&mut self) {
        // Wait for background threads
        std::thread::sleep(Duration::from_millis(100));
    }
}
```

#### Option 4: Global Sled Instance ✅
Use single shared sled instance with separate trees per test:
```rust
lazy_static! {
    static ref GLOBAL_DB: Db = sled::open("/tmp/lapce_test_db").unwrap();
}
// Each test uses different tree: db.open_tree(test_id)
```

### Recommended Fix: Option 4
**Rationale**:
- Avoids multiple sled instances competing
- Proper cleanup with single background thread
- Fast parallel test execution
- Production-ready pattern

### Implementation
1. Create global sled instance with lazy_static
2. Each test gets unique tree ID
3. Cleanup removes tree, not entire DB
4. Single set of background threads - no races

---

## Next Steps
Implement Option 4: Global sled instance with per-test trees
