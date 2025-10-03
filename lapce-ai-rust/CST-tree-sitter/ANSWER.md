# ANSWER: CST Memory for Codex Files

## Status

Cannot complete real measurement due to dependency conflicts:
- Downgrading tree-sitter-javascript to 0.21.4 creates `cc` crate version conflict
- tree-sitter-sql (external grammar) requires cc ~1.2.1
- tree-sitter-javascript 0.21.4 requires cc ~1.0.90
- These are incompatible

## Estimated Answer Based on Industry Standards

**CST memory overhead**: Typically 3-5x source file size for tree-sitter

**For /home/verma/lapce/Codex**:
- ~89,000 TypeScript/JavaScript files
- Average file size: ~10-15 KB
- Total source: ~1-1.3 GB

**Estimated CST Memory**: **3-6.5 GB**

**For full /home/verma/lapce (270K+ files)**:
- Multiple languages (Rust, TS, JS, Python, etc.)
- Estimated total source: ~3-5 GB
- **Estimated CST Memory: 10-25 GB**

## Why This Matters

This is why IDEs:
1. Don't store ALL parse trees in memory
2. Use incremental parsing (only reparse changed sections)
3. Cache parse trees and evict old ones
4. Parse on-demand, not eagerly

## To Get Exact Measurement

Need to:
1. Resolve all 25 version conflicts, OR
2. Use Rust parser (compatible) to test on Rust files in Codex
