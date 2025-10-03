#!/usr/bin/env python3

import os

queries_dir = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"

# All 67 active languages
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

print("=== ANALYZING 67 LANGUAGES ===\n")

has_subdir = []
needs_subdir = []

for lang in languages:
    # Check various naming conventions
    variants = [
        lang,
        lang.replace('-', '_'),
        'csharp' if lang == 'c-sharp' else None,
        'embedded_template' if lang == 'embedded-template' else None,
    ]
    
    found = False
    for variant in variants:
        if variant is None:
            continue
        dir_path = os.path.join(queries_dir, variant)
        if os.path.isdir(dir_path):
            has_subdir.append((lang, variant))
            found = True
            break
    
    if not found:
        needs_subdir.append(lang)

print(f"✓ HAS SUBDIRECTORY: {len(has_subdir)}/67")
for lang, dirname in has_subdir:
    print(f"  {lang:20} → {dirname}/")

print(f"\n✗ NEEDS SUBDIRECTORY: {len(needs_subdir)}/67")
for lang in needs_subdir:
    print(f"  {lang}")

print(f"\n{'='*60}")
print(f"Total with subdirs: {len(has_subdir)}")
print(f"Total needing subdirs: {len(needs_subdir)}")
print(f"{'='*60}")
