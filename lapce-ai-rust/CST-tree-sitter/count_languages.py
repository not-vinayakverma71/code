#!/usr/bin/env python3

import re

with open('/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/Cargo.toml', 'r') as f:
    lines = f.readlines()

active_crates = []
active_paths = []
commented = []

for line in lines:
    # Skip tree-sitter itself and tree-sitter-highlight
    if line.startswith('tree-sitter = ') or line.startswith('tree-sitter-highlight = '):
        continue
    
    # Active crates.io dependencies
    if line.startswith('tree-sitter-') and ' = "' in line and not line.startswith('#'):
        lang = line.split(' = ')[0].replace('tree-sitter-', '')
        active_crates.append(lang)
    
    # Active path dependencies  
    elif line.startswith('tree-sitter-') and ' = { path' in line and not line.startswith('#'):
        lang = line.split(' = ')[0].replace('tree-sitter-', '')
        active_paths.append(lang)
    
    # Commented out
    elif line.startswith('# tree-sitter-'):
        parts = line.split()
        if len(parts) > 1:
            lang = parts[1].replace('tree-sitter-', '').split('=')[0]
            commented.append(lang)

print(f"ACTIVE CRATES.IO: {len(active_crates)}")
for i, lang in enumerate(sorted(active_crates), 1):
    print(f"  {i:2}. {lang}")

print(f"\nACTIVE PATH DEPS: {len(active_paths)}")  
for i, lang in enumerate(sorted(active_paths), 1):
    print(f"  {i:2}. {lang}")

total = len(active_crates) + len(active_paths)
print(f"\nTOTAL ACTIVE: {total}")
print(f"Commented out: {len(set(commented))}")

if total >= 62:
    print(f"\n✅ SUCCESS: Have {total} languages!")
else:
    print(f"\n❌ Need to activate more! Have {total}, need 62+")
    print(f"\nCan activate from commented: {sorted(set(commented))[:10]}")
