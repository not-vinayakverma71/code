#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Fixing ALL lib.rs files to use correct API..."
echo "=============================================="

for dir in tree-sitter-*/; do
    lib_file="$dir/bindings/rust/lib.rs"
    if [ -f "$lib_file" ]; then
        lang="${dir#tree-sitter-}"
        lang="${lang%/}"
        lang_fn=$(echo $lang | tr '-' '_')
        
        echo "Fixing $dir..."
        
        # Replace tree_sitter_language imports
        sed -i 's/use tree_sitter_language::LanguageFn;/use tree_sitter::Language;/' "$lib_file"
        
        # Replace LANGUAGE constant with function
        sed -i "s/pub const LANGUAGE: LanguageFn.*/pub fn language() -> Language { unsafe { tree_sitter::Language::from_raw(tree_sitter_${lang_fn} as *const ()) } }/" "$lib_file"
        
        # Fix extern C declaration if needed
        if ! grep -q "fn tree_sitter_${lang_fn}() -> \*const ()" "$lib_file"; then
            sed -i "s/fn tree_sitter_${lang_fn}() -> Language/fn tree_sitter_${lang_fn}() -> *const ()/" "$lib_file"
        fi
    fi
done

echo ""
echo "All lib.rs files fixed!"
