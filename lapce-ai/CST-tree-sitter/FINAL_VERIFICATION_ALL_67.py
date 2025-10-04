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

required_files = ['highlights.scm', 'injections.scm', 'locals.scm', 'tags.scm', 'folds.scm']

print("=== FINAL VERIFICATION: ALL 67 LANGUAGES ===\n")

complete = []
incomplete = []

for lang in languages:
    variants = [
        lang,
        lang.replace('-', '_'),
        'csharp' if lang == 'c-sharp' else None,
        'embedded_template' if lang == 'embedded-template' else None,
    ]
    
    found_dir = None
    for variant in variants:
        if variant is None:
            continue
        dir_path = os.path.join(queries_dir, variant)
        if os.path.isdir(dir_path):
            found_dir = (variant, dir_path)
            break
    
    if not found_dir:
        incomplete.append((lang, "NO DIRECTORY"))
        continue
    
    variant, dir_path = found_dir
    missing_files = []
    
    for req_file in required_files:
        file_path = os.path.join(dir_path, req_file)
        if not os.path.exists(file_path):
            missing_files.append(req_file)
    
    if missing_files:
        incomplete.append((lang, f"{variant}/ missing: {', '.join(missing_files)}"))
    else:
        complete.append((lang, variant))

print(f"✅ COMPLETE: {len(complete)}/67")
for lang, dirname in complete:
    file_count = len([f for f in os.listdir(os.path.join(queries_dir, dirname)) if f.endswith('.scm')])
    print(f"  {lang:20} → {dirname:20} ({file_count} files)")

if incomplete:
    print(f"\n❌ INCOMPLETE: {len(incomplete)}/67")
    for lang, issue in incomplete:
        print(f"  {lang:20} → {issue}")

print(f"\n{'='*70}")
print(f"COMPLETE: {len(complete)}/67 ({len(complete)*100//67}%)")
print(f"INCOMPLETE: {len(incomplete)}/67")
print(f"{'='*70}")

if len(complete) == 67:
    print("\n🎉 SUCCESS: ALL 67 LANGUAGES HAVE COMPLETE 5-FILE STRUCTURE!")
    print("✅ TRUE 100% COMPLETION ACHIEVED!")
else:
    print(f"\n⚠️  {len(incomplete)} languages still need work")
