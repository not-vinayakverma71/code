#!/bin/bash

echo "FORCE REBUILDING FINAL 3 LANGUAGES"
echo "==================================="

# Clean all cached builds
echo "Cleaning build cache..."
cargo clean

# Clean and rebuild each grammar
cd external-grammars

echo ""
echo "1. Rebuilding SystemVerilog..."
cd tree-sitter-systemverilog
rm -rf target 2>/dev/null
# Double check version
current_version=$(grep '#define LANGUAGE_VERSION' src/parser.c | head -1)
echo "  Current: $current_version"
# Ensure bindings are correct
mkdir -p bindings/rust
if [ ! -f "bindings/rust/lib.rs" ]; then
cat > bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_verilog() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_verilog() }
}
EOF
fi
if [ ! -f "bindings/rust/build.rs" ]; then
cat > bindings/rust/build.rs << 'EOF'
use std::path::Path;

fn main() {
    let src_dir = Path::new("src");
    
    let mut c_files = vec![];
    if src_dir.join("parser.c").exists() {
        c_files.push(src_dir.join("parser.c"));
    }
    if src_dir.join("scanner.c").exists() {
        c_files.push(src_dir.join("scanner.c"));
    }
    
    let mut build = cc::Build::new();
    build.include(src_dir);
    build.std("c11");
    build.warnings(false);
    
    for file in c_files {
        build.file(file);
    }
    
    build.compile("tree-sitter-verilog");
}
EOF
fi
cd ..

echo ""
echo "2. Rebuilding Elm..."
cd tree-sitter-elm
rm -rf target 2>/dev/null
current_version=$(grep '#define LANGUAGE_VERSION' src/parser.c | head -1)
echo "  Current: $current_version"
cd ..

echo ""
echo "3. Rebuilding COBOL..."
cd tree-sitter-cobol
rm -rf target 2>/dev/null
current_version=$(grep '#define LANGUAGE_VERSION' src/parser.c | head -1)
echo "  Current: $current_version"

# Fix scanner includes more aggressively
if [ -f "src/scanner.c" ]; then
    if ! grep -q "#include <tree_sitter/parser.h>" src/scanner.c; then
        echo "  Fixing scanner.c includes..."
        cat > src/scanner_temp.c << 'EOF'
#include <tree_sitter/parser.h>
#include <wctype.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

EOF
        grep -v "^#include" src/scanner.c >> src/scanner_temp.c
        mv src/scanner_temp.c src/scanner.c
    fi
fi

cd ..

echo ""
echo "==================================="
echo "Force rebuild complete!"
cd ..
