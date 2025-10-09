# Troubleshooting Guide

## Common Issues

### Build Errors

#### Issue: Duplicate dependencies in Cargo.toml
```
error: duplicate key `package` in table `dependencies`
```
**Solution**: Remove duplicate entries, ensure each dependency appears only once.

#### Issue: Unresolved imports
```
error[E0432]: unresolved import `super::BytecodeStream`
```
**Solution**: Ensure proper module exports in `mod.rs`:
```rust
pub use opcodes::{BytecodeStream, BytecodeReader};
```

#### Issue: Lifetime errors
```
error: lifetime may not live long enough
```
**Solution**: Use explicit lifetime annotations or return owned types.

### Runtime Errors

#### Issue: Bytecode verify() fails with 0x00 byte
```
Error: Invalid opcode 0x00 at position X
```
**Root Cause**: Encoder not writing Exit opcode for leaf nodes.

**Solution**: Ensure Exit is written for all nodes:
```rust
if node.child_count() == 0 {
    self.stream.write_op(Opcode::Leaf);
    // Write node data...
    // No Exit needed for Leaf
} else {
    self.stream.write_op(Opcode::Enter);
    // Write node data...
    // Process children...
    self.stream.write_op(Opcode::Exit); // Required!
}
```

#### Issue: Cache hit ratio too low
```
Cache hit ratio: 45% (expected: >90%)
```
**Possible Causes**:
1. Working set larger than cache
2. Poor access patterns
3. Cache eviction too aggressive

**Solutions**:
- Increase memory budget: `CST_MEMORY_MB=200`
- Adjust tier ratios for workload
- Enable prefetching for predictable access

#### Issue: High parse latency
```
P99 parse latency: 25ms (target: <10ms)
```
**Diagnosis**:
```bash
# Check if using incremental parsing
grep "parse_incremental" logs.txt

# Profile parsing
perf record -g cargo run --release --bin benchmark_performance
perf report
```

**Solutions**:
- Enable incremental parsing
- Reduce segment size for faster loading
- Check for grammar complexity issues

### Memory Issues

#### Issue: Memory leak
**Symptoms**: Steady memory growth over time

**Diagnosis**:
```bash
# Monitor memory usage
watch -n 1 'ps aux | grep cst-tree-sitter'

# Memory profiler
valgrind --leak-check=full ./target/release/cst-server
```

**Common Causes**:
1. Arc reference cycles
2. Unbounded cache growth
3. Segment accumulation

**Solutions**:
```rust
// Break Arc cycles with Weak references
use std::sync::Weak;
struct Node {
    parent: Weak<Node>, // Not Arc<Node>
}

// Enforce cache limits
cache.set_max_entries(10000);
cache.set_max_memory(100 * 1024 * 1024);
```

#### Issue: Out of memory
```
memory allocation of X bytes failed
```
**Solutions**:
1. Reduce memory budget
2. Enable swap space
3. Use memory-mapped files
4. Increase compression ratio

### Performance Issues

#### Issue: Segment thrashing
**Symptoms**: High disk I/O, frequent segment loads

**Diagnosis**:
```promql
# Check segment load rate
rate(cst_segment_loads_total[5m])
```

**Solutions**:
- Increase segment cache size
- Adjust segment size for workload
- Enable segment prefetching

#### Issue: Lock contention
**Symptoms**: High CPU usage, poor scaling

**Diagnosis**:
```bash
# Check lock contention
perf record -e lock:* ./target/release/benchmark_performance
perf report
```

**Solutions**:
- Use `parking_lot::RwLock` instead of `std::sync::RwLock`
- Reduce lock scope
- Use lock-free data structures

### Corruption Issues

#### Issue: CRC32 mismatch
```
CRC32 verification failed: expected XXXX, got YYYY
```
**Causes**:
- Disk corruption
- Incomplete write
- Version mismatch

**Recovery**:
```bash
# Verify all segments
cst-cli verify --data-dir /data/cst

# Clear corrupted segments
cst-cli clear --corrupted

# Rebuild from source
cst-cli rebuild --force
```

#### Issue: Invalid bytecode format
```
Invalid magic bytes in header
```
**Solutions**:
1. Check format version compatibility
2. Clear cache and regenerate
3. Verify no partial writes

### Integration Issues

#### Issue: Grammar version mismatch
```
Grammar version 0.23.0 required, found 0.22.0
```
**Solution**: Update grammar dependencies:
```toml
[dependencies]
tree-sitter-rust = "=0.23.0"
tree-sitter-python = "=0.23.2"
```

#### Issue: Symbol extraction timeout
```
Symbol extraction took 150ms (limit: 50ms)
```
**Optimizations**:
1. Cache extracted symbols
2. Limit traversal depth
3. Use parallel extraction
4. Optimize queries

### Debugging Techniques

#### Enable Debug Logging
```bash
export CST_LOG_LEVEL=debug
export RUST_BACKTRACE=full
```

#### Trace Specific Module
```bash
export RUST_LOG=lapce_tree_sitter::cache=trace
```

#### Generate Core Dump
```bash
ulimit -c unlimited
./cst-server
# On crash, analyze core
gdb ./cst-server core
```

#### Use Debug Assertions
```rust
debug_assert!(self.cursor < self.stream.bytes.len());
debug_assert_eq!(opcode, Opcode::Exit);
```

### Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| E001 | Invalid opcode | Check bytecode encoding |
| E002 | CRC32 mismatch | Verify data integrity |
| E003 | Out of memory | Increase limits or optimize |
| E004 | Parse timeout | Use incremental parsing |
| E005 | Cache miss | Prefetch or increase cache |
| E006 | Lock timeout | Reduce contention |
| E007 | Grammar error | Update grammar version |
| E008 | I/O error | Check disk space/permissions |

### Getting Help

1. **Check Logs**
   ```bash
   journalctl -u cst-tree-sitter --since "1 hour ago"
   ```

2. **Collect Diagnostics**
   ```bash
   cst-cli diagnostics --output diagnostics.tar.gz
   ```

3. **Report Issue**
   - Include diagnostics archive
   - Provide reproduction steps
   - Note environment details
   - Submit to: https://github.com/project/issues

### Quick Fixes

```bash
# Reset everything
systemctl stop cst-tree-sitter
rm -rf /data/cst/*
systemctl start cst-tree-sitter

# Emergency performance mode
export CST_EMERGENCY_MODE=true
export CST_MEMORY_MB=50
export CST_COMPRESSION=false

# Bypass cache completely
export CST_CACHE_DISABLED=true
```
