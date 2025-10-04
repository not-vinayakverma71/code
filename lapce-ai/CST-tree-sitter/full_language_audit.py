#!/usr/bin/env python3

import re

def analyze_cargo_toml():
    with open('/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/Cargo.toml', 'r') as f:
        lines = f.readlines()
    
    crates_io_langs = []
    path_langs = []
    commented_langs = []
    
    for line in lines:
        # Active crates.io dependencies
        match = re.match(r'^tree-sitter-([a-z-]+) = ".*"', line)
        if match:
            crates_io_langs.append(match.group(1))
            continue
        
        # Active path dependencies
        match = re.match(r'^tree-sitter-([a-z-]+) = \{ path', line)
        if match:
            path_langs.append(match.group(1))
            continue
            
        # Commented out languages
        match = re.match(r'^# tree-sitter-([a-z-]+)', line)
        if match:
            commented_langs.append(match.group(1))
    
    print("=== ACTIVE LANGUAGES ===")
    print("\nCrates.io dependencies:")
    for i, lang in enumerate(sorted(crates_io_langs), 1):
        print(f"  {i:2}. {lang}")
    
    print(f"\nPath dependencies:")
    for i, lang in enumerate(sorted(path_langs), 1):
        print(f"  {i:2}. {lang}")
    
    print("\n=== COMMENTED OUT LANGUAGES ===")
    for i, lang in enumerate(sorted(set(commented_langs)), 1):
        print(f"  {i:2}. {lang}")
    
    total_active = len(crates_io_langs) + len(path_langs)
    print(f"\n=== SUMMARY ===")
    print(f"Crates.io: {len(crates_io_langs)}")
    print(f"Path deps: {len(path_langs)}")
    print(f"TOTAL ACTIVE: {total_active}")
    print(f"Commented out: {len(set(commented_langs))}")
    
    print(f"\n=== ALL ACTIVE LANGUAGES ({total_active}) ===")
    all_langs = sorted(set(crates_io_langs + path_langs))
    for i, lang in enumerate(all_langs, 1):
        print(f"  {i:2}. {lang}")
    
    return total_active

if __name__ == "__main__":
    total = analyze_cargo_toml()
    print(f"\nNeed to reach 62+ languages. Currently have: {total}")
