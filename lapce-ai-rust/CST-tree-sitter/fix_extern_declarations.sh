#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Fixing extern C declarations in all lib.rs files..."
echo "==================================================="

for dir in tree-sitter-*/; do
    lib_file="$dir/bindings/rust/lib.rs"
    if [ -f "$lib_file" ]; then
        lang="${dir#tree-sitter-}"
        lang="${lang%/}"
        lang_fn=$(echo $lang | tr '-' '_')
        
        echo "Fixing $dir..."
        
        # Create a properly formatted lib.rs
        cat > "$lib_file" << EOF
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
            .expect("Error loading ${lang} language");
    }
}
EOF
    fi
done

echo ""
echo "All extern declarations fixed!"
