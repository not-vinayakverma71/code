#!/usr/bin/env python3

import re

with open('Cargo.toml', 'r') as f:
    content = f.read()

# Find all active parsers
lines = content.split('\n')
crates_parsers = []
path_parsers = []

for line in lines:
    # Skip tree-sitter itself and highlight
    if line.startswith('tree-sitter = ') or line.startswith('tree-sitter-highlight = '):
        continue
    
    # Active crates.io parsers
    if re.match(r'^tree-sitter-[a-z-]+ = "', line):
        name = line.split(' = ')[0]
        crates_parsers.append(name.replace('tree-sitter-', ''))
    
    # Path dependencies
    elif re.match(r'^tree-sitter-[a-z-]+ = \{ path', line):
        name = line.split(' = ')[0]
        path_parsers.append(name.replace('tree-sitter-', ''))

print("CRATES.IO PARSERS:", len(crates_parsers))
for i, p in enumerate(sorted(crates_parsers), 1):
    print(f"  {i:2}. {p}")

print("\nPATH PARSERS:", len(path_parsers))
for i, p in enumerate(sorted(path_parsers), 1):
    print(f"  {i:2}. {p}")

total = len(crates_parsers) + len(path_parsers)
print(f"\nTOTAL: {total} languages")

if total >= 62:
    print(f"✅ SUCCESS: Have {total} languages!")
else:
    print(f"❌ NEED MORE: Have {total}, need 62+")
