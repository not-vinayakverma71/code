#!/usr/bin/env python3

import os

queries_dir = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"

# The 17 I created
created_by_me = [
    "elixir", "nix", "latex", "make", "cmake", "verilog", "erlang",
    "commonlisp", "hlsl", "hcl", "solidity", "systemverilog",
    "embedded_template", "abap", "crystal", "vhdl", "prolog"
]

# Get all directories
all_dirs = []
for item in os.listdir(queries_dir):
    path = os.path.join(queries_dir, item)
    if os.path.isdir(path):
        all_dirs.append(item)

all_dirs.sort()

# Separate original vs created
original = []
created = []

for d in all_dirs:
    if d in created_by_me or d.replace('_', '-') in created_by_me:
        created.append(d)
    else:
        original.append(d)

print(f"TOTAL DIRECTORIES: {len(all_dirs)}")
print(f"ORIGINAL (pre-existing): {len(original)}")
print(f"CREATED BY ME: {len(created)}")
print()

print("=== ORIGINAL DIRECTORIES (need to analyze these deeply) ===")
for i, d in enumerate(original, 1):
    file_count = len([f for f in os.listdir(os.path.join(queries_dir, d)) if f.endswith('.scm')])
    print(f"{i:2}. {d:25} ({file_count} files)")

print()
print("=== DIRECTORIES I CREATED ===")
for i, d in enumerate(created, 1):
    file_count = len([f for f in os.listdir(os.path.join(queries_dir, d)) if f.endswith('.scm')])
    print(f"{i:2}. {d:25} ({file_count} files)")
