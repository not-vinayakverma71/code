#!/usr/bin/env python3

import os

queries_dir = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"

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

present = []
missing = []

for lang in languages:
    # Check variations
    variants = [
        lang,
        lang.replace('-', '_'),
        lang.replace('-', ''),
        'csharp' if lang == 'c-sharp' else None,
    ]
    
    found = False
    for variant in variants:
        if variant is None:
            continue
        
        # Check for .scm file
        if os.path.exists(os.path.join(queries_dir, f"{variant}.scm")):
            present.append(f"{lang} (.scm)")
            found = True
            break
        
        # Check for directory
        if os.path.isdir(os.path.join(queries_dir, variant)):
            present.append(f"{lang} (dir)")
            found = True
            break
    
    if not found:
        missing.append(lang)

print(f"=== PRESENT: {len(present)}/67 ===")
for item in sorted(present):
    print(f"  ✓ {item}")

print(f"\n=== MISSING: {len(missing)}/67 ===")
for lang in sorted(missing):
    print(f"  ✗ {lang}")

print(f"\n=== SUMMARY ===")
print(f"Total Present: {len(present)}")
print(f"Total Missing: {len(missing)}")
print(f"Total: {len(present) + len(missing)}")
