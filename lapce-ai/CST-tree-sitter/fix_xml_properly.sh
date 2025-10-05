#!/bin/bash

echo "Properly fixing XML parser..."
echo "=============================="

cd external-grammars/tree-sitter-xml

# XML uses a non-standard structure with pre-built parsers
# Check if parser.c files already exist
echo "Checking existing parser files..."

for dir in xml dtd; do
    if [ -f "$dir/src/parser.c" ]; then
        version=$(grep -oP '#define LANGUAGE_VERSION \K\d+' "$dir/src/parser.c" | head -1)
        echo "$dir parser exists with version: $version"
        
        # If version is 15, patch it to 14
        if [ "$version" = "15" ]; then
            echo "Patching $dir parser from version 15 to 14..."
            sed -i 's/#define LANGUAGE_VERSION 15/#define LANGUAGE_VERSION 14/g' "$dir/src/parser.c"
            echo "Patched!"
        fi
    else
        echo "$dir parser missing - trying to generate..."
        if [ -f "$dir/grammar.js" ]; then
            cd "$dir"
            npx tree-sitter generate
            cd ..
        else
            echo "No grammar.js found for $dir"
        fi
    fi
done

# Update Cargo.toml
sed -i 's/tree-sitter = ".*"/tree-sitter = "0.23.0"/g' Cargo.toml
echo "Updated Cargo.toml"

# Create bindings if missing
if [ ! -f "bindings/rust/lib.rs" ]; then
    mkdir -p bindings/rust
    cat > bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_xml() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_xml() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language()).unwrap();
    }
}
EOF
    echo "Created bindings/rust/lib.rs"
fi

# Create build.rs if missing
if [ ! -f "bindings/rust/build.rs" ]; then
    cat > bindings/rust/build.rs << 'EOF'
use std::path::Path;

fn main() {
    let src_dir = Path::new("xml/src");
    
    let mut c_files = vec![];
    if src_dir.join("parser.c").exists() {
        c_files.push(src_dir.join("parser.c"));
    }
    if src_dir.join("scanner.c").exists() {
        c_files.push(src_dir.join("scanner.c"));
    }
    
    let mut build = cc::Build::new();
    build.include(src_dir);
    
    for file in c_files {
        build.file(file);
    }
    
    build.compile("tree-sitter-xml");
}
EOF
    echo "Created bindings/rust/build.rs"
fi

echo ""
echo "XML fix complete!"
cd ../..
