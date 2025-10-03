#!/usr/bin/env python3

import os

# Our 67 active languages
languages = [
    "rust", "javascript", "typescript", "python", "go", "java", "c", "cpp", 
    "c-sharp", "ruby", "php", "lua", "bash", "css", "json", "swift", "scala",
    "elixir", "html", "elm", "toml", "ocaml", "nix", "latex", "make", "cmake",
    "verilog", "erlang", "d", "dockerfile", "pascal", "commonlisp", "prisma",
    "hlsl", "objc", "cobol", "groovy", "hcl", "solidity", "fsharp", "powershell",
    "systemverilog", "embedded-template", "kotlin", "yaml", "r", "matlab", "perl",
    "dart", "julia", "haskell", "graphql", "sql", "zig", "vim", "abap", "nim",
    "clojure", "crystal", "fortran", "vhdl", "racket", "ada", "prolog", "gradle", "xml"
]

queries_dir = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"
existing = os.listdir(queries_dir)

# Normalize names for comparison
existing_normalized = []
for item in existing:
    if item.endswith('.scm'):
        existing_normalized.append(item[:-4])
    elif os.path.isdir(os.path.join(queries_dir, item)):
        existing_normalized.append(item)

missing = []
present = []

for lang in languages:
    # Check various naming conventions
    variants = [
        lang,
        lang.replace('-', '_'),
        lang.replace('-', ''),
        lang.replace('_', '-'),
    ]
    
    found = False
    for variant in variants:
        if variant in existing_normalized:
            present.append(lang)
            found = True
            break
    
    if not found:
        missing.append(lang)

print(f"PRESENT: {len(present)}/67")
for lang in sorted(present):
    print(f"  ✓ {lang}")

print(f"\nMISSING: {len(missing)}/67")
for lang in sorted(missing):
    print(f"  ✗ {lang}")

print(f"\nTotal: {len(present)} + {len(missing)} = {len(present) + len(missing)}")
