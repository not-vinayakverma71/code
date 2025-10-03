#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Creating Cargo.toml for parsers that don't have them..."
echo "========================================================"

# Function to create standard Cargo.toml
create_cargo_toml() {
    local dir=$1
    local lang=${dir#tree-sitter-}
    
    cat > "$dir/Cargo.toml" << EOF
[package]
name = "$dir"
description = "$lang grammar for tree-sitter"
version = "0.1.0"
keywords = ["incremental", "parsing", "$lang"]
categories = ["parsing", "text-editors"]
repository = "https://github.com/tree-sitter-grammars/$dir"
edition = "2018"
license = "MIT"

build = "bindings/rust/build.rs"
include = [
  "bindings/rust/*",
  "grammar.js",
  "queries/*",
  "src/*",
]

[lib]
path = "bindings/rust/lib.rs"

[dependencies]
tree-sitter = "0.24"

[build-dependencies]
cc = "1.0"
EOF
    echo "✅ Created Cargo.toml for $dir"
}

# Function to create lib.rs
create_lib_rs() {
    local dir=$1
    local lang=${dir#tree-sitter-}
    local lang_fn=$(echo $lang | tr '-' '_')
    
    mkdir -p "$dir/bindings/rust"
    
    cat > "$dir/bindings/rust/lib.rs" << EOF
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_${lang_fn}() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_${lang_fn}() }
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading $lang language");
    }
}
EOF
    echo "✅ Created lib.rs for $dir"
}

# Function to create build.rs
create_build_rs() {
    local dir=$1
    
    cat > "$dir/bindings/rust/build.rs" << 'EOF'
fn main() {
    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    c_config.include(&src_dir);
    c_config
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs");
    let parser_path = src_dir.join("parser.c");
    c_config.file(&parser_path);
    
    // Check for scanner files
    let scanner_c = src_dir.join("scanner.c");
    let scanner_cc = src_dir.join("scanner.cc");
    if scanner_c.exists() {
        c_config.file(&scanner_c);
    } else if scanner_cc.exists() {
        c_config.file(&scanner_cc);
        c_config.cpp(true);
    }
    
    c_config.compile("parser");

    println!("cargo:rerun-if-changed={}", parser_path.display());
}
EOF
    echo "✅ Created build.rs for $dir"
}

# Check each directory
for dir in tree-sitter-*/; do
    name="${dir%/}"
    
    if [ ! -f "$dir/Cargo.toml" ]; then
        echo ""
        echo "Processing $name..."
        create_cargo_toml "$name"
        create_lib_rs "$name"
        create_build_rs "$name"
    fi
done

echo ""
echo "All missing Cargo.toml files created!"
