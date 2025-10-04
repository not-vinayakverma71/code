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

print("=== FINAL VERIFICATION: 67/67 LANGUAGES ===\n")

covered = []
missing = []

for lang in languages:
    variants = [
        lang,
        lang.replace('-', '_'),
        'csharp' if lang == 'c-sharp' else None,
        'embedded_template' if lang == 'embedded-template' else None,
    ]
    
    found = False
    location = ""
    
    for variant in variants:
        if variant is None:
            continue
        
        scm_file = os.path.join(queries_dir, f"{variant}.scm")
        dir_path = os.path.join(queries_dir, variant)
        
        if os.path.exists(scm_file):
            covered.append((lang, f"{variant}.scm"))
            found = True
            location = f"{variant}.scm"
            break
        elif os.path.isdir(dir_path):
            covered.append((lang, f"{variant}/"))
            found = True
            location = f"{variant}/"
            break
    
    if found:
        print(f"✓ {lang:25} → {location}")
    else:
        missing.append(lang)
        print(f"✗ {lang:25} MISSING!")

print(f"\n{'='*60}")
print(f"COVERED: {len(covered)}/67 ({len(covered)*100//67}%)")
print(f"MISSING: {len(missing)}/67")
print(f"{'='*60}")

if len(covered) == 67:
    print("\n🎉 SUCCESS: ALL 67 LANGUAGES HAVE QUERY FILES!")
    print("✅ TRUE 100% COMPLETION ACHIEVED!")
else:
    print(f"\n⚠️  Still missing {len(missing)} languages:")
    for lang in missing:
        print(f"  - {lang}")
