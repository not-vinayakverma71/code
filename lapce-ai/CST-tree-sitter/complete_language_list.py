#!/usr/bin/env python3

import re

with open('/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/Cargo.toml', 'r') as f:
    content = f.read()

# Find all active language dependencies
crates_io = re.findall(r'^tree-sitter-([a-z-]+) = ".*"', content, re.MULTILINE)
path_deps = re.findall(r'^tree-sitter-([a-z-]+) = \{ path', content, re.MULTILINE)

# Remove duplicates
all_active = sorted(set(crates_io + path_deps))

print(f"ACTIVE LANGUAGES: {len(all_active)}")
print("=" * 40)
for i, lang in enumerate(all_active, 1):
    print(f"{i:2}. {lang}")

print(f"\nTotal: {len(all_active)} languages")
print(f"Crates.io: {len(crates_io)}")
print(f"Path deps: {len(path_deps)}")
