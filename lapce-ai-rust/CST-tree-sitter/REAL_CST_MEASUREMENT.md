# 🔬 REAL CST MEMORY MEASUREMENT - /home/verma/lapce/Codex

## Test Method

**ACTUAL tree-sitter parsing** (not file I/O):
- Uses tree-sitter-javascript for .js/.jsx files
- Uses tree-sitter-typescript for .ts/.tsx files
- Parses each file into a real CST
- Stores ALL trees in memory simultaneously
- Counts every node in every tree
- Calculates memory: source bytes + (nodes × 50 bytes)

## Test Results

[Results from actual execution will be shown here]

## Memory Per Node

Each tree-sitter CST node:
```
Node kind ID:        2 bytes
Start/end byte:      8 bytes
Start/end point:     8 bytes
Parent pointer:      8 bytes
First child pointer: 8 bytes
Next sibling:        8 bytes
State/flags:         8 bytes
Total per node:     ~50 bytes
```

**ANSWER**: Total CST memory for Codex files = [from actual test output]
