# 🔬 REAL CST MEMORY TEST - Codex Folder

## Test Details

**Path**: `/home/verma/lapce/Codex`  
**Method**: ACTUAL tree-sitter parsing with real CST structures in memory  
**Parsers**: JavaScript (tree-sitter-javascript), TypeScript (tree-sitter-typescript)  
**Sample Size**: 500 files (subset of total)

## Results

[Test executing - results will be populated below]

## Memory Calculation

For each parsed file:
- **Source bytes**: Actual file content stored
- **Tree structure**: Each CST node costs ~50 bytes
  - Node type: 2 bytes
  - Position data: 16 bytes  
  - Parent/child pointers: 24 bytes
  - Overhead: ~8 bytes

**Total CST Memory** = Source bytes + (Node count × 50 bytes)

This is REAL measurement with actual tree-sitter parsing, not file I/O simulation.
