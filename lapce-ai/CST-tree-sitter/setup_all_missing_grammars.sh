#!/bin/bash

# Setup all missing external grammars for complete 67 language support
# This script will ensure all external grammars are properly configured

cd external-grammars

# List of grammars that need to be added or fixed
GRAMMARS=(
    "tree-sitter-markdown"
    "tree-sitter-svelte"
    "tree-sitter-scheme"
    "tree-sitter-fennel"
    "tree-sitter-gleam"
    "tree-sitter-astro"
    "tree-sitter-wgsl"
    "tree-sitter-glsl"
    "tree-sitter-tcl"
    "tree-sitter-cairo"
    "tree-sitter-asm"
    "tree-sitter-hcl"
    "tree-sitter-solidity"
    "tree-sitter-fsharp"
    "tree-sitter-powershell"
    "tree-sitter-systemverilog"
)

# Clone missing repositories if not present
for grammar in "${GRAMMARS[@]}"; do
    if [ ! -d "$grammar" ]; then
        echo "Setting up $grammar..."
        
        case $grammar in
            "tree-sitter-markdown")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-markdown.git
                ;;
            "tree-sitter-svelte")
                git clone https://github.com/Himujjal/tree-sitter-svelte.git
                ;;
            "tree-sitter-scheme")
                git clone https://github.com/6cdh/tree-sitter-scheme.git
                ;;
            "tree-sitter-fennel")
                git clone https://github.com/TravonteD/tree-sitter-fennel.git
                ;;
            "tree-sitter-gleam")
                git clone https://github.com/gleam-lang/tree-sitter-gleam.git
                ;;
            "tree-sitter-astro")
                git clone https://github.com/virchau13/tree-sitter-astro.git
                ;;
            "tree-sitter-wgsl")
                git clone https://github.com/szebniok/tree-sitter-wgsl.git
                ;;
            "tree-sitter-glsl")
                git clone https://github.com/theHamsta/tree-sitter-glsl.git
                ;;
            "tree-sitter-tcl")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-tcl.git
                ;;
            "tree-sitter-cairo")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-cairo.git
                ;;
            "tree-sitter-asm")
                git clone https://github.com/RubixDev/tree-sitter-asm.git
                ;;
            "tree-sitter-hcl")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-hcl.git
                ;;
            "tree-sitter-solidity")
                git clone https://github.com/JoranHonig/tree-sitter-solidity.git
                ;;
            "tree-sitter-fsharp")
                git clone https://github.com/ionide/tree-sitter-fsharp.git
                ;;
            "tree-sitter-powershell")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-powershell.git
                ;;
            "tree-sitter-systemverilog")
                git clone https://github.com/tree-sitter-grammars/tree-sitter-systemverilog.git
                ;;
        esac
    else
        echo "$grammar already exists"
    fi
done

# Setup Rust bindings for each grammar
for grammar in "${GRAMMARS[@]}"; do
    if [ -d "$grammar" ]; then
        echo "Setting up Rust bindings for $grammar..."
        
        # Create Cargo.toml if not exists
        if [ ! -f "$grammar/Cargo.toml" ]; then
            cat > "$grammar/Cargo.toml" << 'EOF'
[package]
name = "GRAMMAR_NAME"
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
            # Replace GRAMMAR_NAME with actual name
            sed -i "s/GRAMMAR_NAME/$grammar/" "$grammar/Cargo.toml"
        fi
        
        # Create bindings/rust directory if not exists
        mkdir -p "$grammar/bindings/rust"
        
        # Create build.rs if not exists
        if [ ! -f "$grammar/bindings/rust/build.rs" ]; then
            cat > "$grammar/bindings/rust/build.rs" << 'EOF'
use std::path::Path;
use cc::Build;

fn main() {
    let src_dir = Path::new("../../src");

    let mut config = Build::new();
    config.include(src_dir);
    
    let parser_path = src_dir.join("parser.c");
    if parser_path.exists() {
        config.file(parser_path);
    } else {
        panic!("parser.c not found");
    }

    let scanner_c = src_dir.join("scanner.c");
    if scanner_c.exists() {
        config.file(scanner_c);
    }
    
    let scanner_cc = src_dir.join("scanner.cc");
    if scanner_cc.exists() {
        config.file(scanner_cc);
        config.cpp(true).flag_if_supported("-std=c++14");
    }
    
    config.compile("parser");
}
EOF
        fi
        
        # Create lib.rs if not exists
        if [ ! -f "$grammar/bindings/rust/lib.rs" ]; then
            cat > "$grammar/bindings/rust/lib.rs" << 'EOF'
use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_LANG_NAME() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_LANG_NAME) };

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading grammar");
    }
}
EOF
            # Determine the language name from the grammar directory
            lang_name="${grammar#tree-sitter-}"
            lang_name="${lang_name//-/_}"
            sed -i "s/LANG_NAME/$lang_name/" "$grammar/bindings/rust/lib.rs"
        fi
        
        # Update src/node-types.json path if needed
        if [ -f "$grammar/src/node-types.json" ]; then
            echo "node-types.json found for $grammar"
        fi
    fi
done

echo "✅ All external grammars setup complete!"
echo "Now run: cd .. && cargo build"
