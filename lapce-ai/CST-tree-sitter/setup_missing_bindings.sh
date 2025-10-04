#!/bin/bash
# Setup Rust bindings for all external grammars that don't have them

cd external-grammars

for dir in tree-sitter-*; do
    if [ -d "$dir" ] && [ ! -f "$dir/Cargo.toml" ]; then
        echo "Setting up Rust bindings for $dir..."
        
        lang_name="${dir#tree-sitter-}"
        lang_name_underscore="${lang_name//-/_}"
        
        # Create Cargo.toml
        cat > "$dir/Cargo.toml" << EOF
[package]
name = "$dir"
version = "0.1.0"
edition = "2021"
build = "bindings/rust/build.rs"

[lib]
path = "bindings/rust/lib.rs"

[dependencies]
tree-sitter-language = "0.1.0"

[build-dependencies]
cc = "1.0"
EOF
        
        # Create bindings directory
        mkdir -p "$dir/bindings/rust"
        
        # Create build.rs
        cat > "$dir/bindings/rust/build.rs" << 'EOF'
use std::path::Path;

fn main() {
    let src_dir = Path::new("../../src");
    let mut config = cc::Build::new();
    
    config.include(src_dir);
    config.warnings(false);
    
    // Add parser.c
    let parser_path = src_dir.join("parser.c");
    if parser_path.exists() {
        config.file(parser_path);
    }
    
    // Check for scanner.c
    let scanner_c = src_dir.join("scanner.c");
    if scanner_c.exists() {
        config.file(scanner_c);
    }
    
    // Check for scanner.cc
    let scanner_cc = src_dir.join("scanner.cc");
    if scanner_cc.exists() {
        config.file(scanner_cc);
        config.cpp(true);
        config.flag_if_supported("-std=c++14");
    }
    
    // Check for scanner.cpp
    let scanner_cpp = src_dir.join("scanner.cpp");
    if scanner_cpp.exists() {
        config.file(scanner_cpp);
        config.cpp(true);
        config.flag_if_supported("-std=c++14");
    }
    
    config.compile("parser");
}
EOF
        
        # Create lib.rs
        cat > "$dir/bindings/rust/lib.rs" << EOF
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_${lang_name_underscore}() -> Language;
}

/// Get the tree-sitter language for $lang_name
pub fn language() -> Language {
    unsafe { tree_sitter_${lang_name_underscore}() }
}

/// The source of the tree-sitter $lang_name grammar description
pub const GRAMMAR: &str = include_str!("../../grammar.js");

/// The syntax highlighting query for $lang_name
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

/// The content of the node-types.json file
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading $lang_name grammar");
    }
}
EOF
        
        # Check if src directory exists, if not, try to generate it
        if [ ! -d "$dir/src" ]; then
            echo "  Generating parser for $dir..."
            cd "$dir"
            
            # Install tree-sitter CLI if needed
            if ! command -v tree-sitter &> /dev/null; then
                npm install tree-sitter-cli
                export PATH="./node_modules/.bin:$PATH"
            fi
            
            # Generate parser
            if command -v tree-sitter &> /dev/null; then
                tree-sitter generate
            else
                echo "  Warning: tree-sitter CLI not available, skipping parser generation"
            fi
            
            cd ..
        fi
        
        echo "  ✅ Setup complete for $dir"
    fi
done

echo "✅ All Rust bindings setup complete!"
