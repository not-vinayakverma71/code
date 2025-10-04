#!/bin/bash

cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter/external-grammars

echo "Fixing remaining tree_sitter_language imports..."
echo "================================================"

# Fix VHDL
if [ -f "tree-sitter-vhdl/bindings/rust/lib.rs" ]; then
    echo "Fixing tree-sitter-vhdl..."
    sed -i 's/use tree_sitter_language::LanguageFn;/use tree_sitter::Language;/' tree-sitter-vhdl/bindings/rust/lib.rs
    sed -i 's/pub const LANGUAGE: LanguageFn.*/pub fn language() -> Language { unsafe { tree_sitter::Language::from_raw(tree_sitter_vhdl as *const ()) } }/' tree-sitter-vhdl/bindings/rust/lib.rs
fi

# Fix Ada
if [ -f "tree-sitter-ada/bindings/rust/lib.rs" ]; then
    echo "Fixing tree-sitter-ada..."
    sed -i 's/use tree_sitter_language::LanguageFn;/use tree_sitter::Language;/' tree-sitter-ada/bindings/rust/lib.rs
    sed -i 's/pub const LANGUAGE: LanguageFn.*/pub fn language() -> Language { unsafe { tree_sitter::Language::from_raw(tree_sitter_ada as *const ()) } }/' tree-sitter-ada/bindings/rust/lib.rs
fi

# Fix Racket
if [ -f "tree-sitter-racket/bindings/rust/lib.rs" ]; then
    echo "Fixing tree-sitter-racket..."
    sed -i 's/use tree_sitter_language::LanguageFn;/use tree_sitter::Language;/' tree-sitter-racket/bindings/rust/lib.rs
    sed -i 's/pub const LANGUAGE: LanguageFn.*/pub fn language() -> Language { unsafe { tree_sitter::Language::from_raw(tree_sitter_racket as *const ()) } }/' tree-sitter-racket/bindings/rust/lib.rs
fi

# Check for any remaining tree_sitter_language imports
echo ""
echo "Checking for remaining tree_sitter_language imports..."
remaining=$(grep -r "tree_sitter_language" --include="*.rs" 2>/dev/null | wc -l)
if [ $remaining -eq 0 ]; then
    echo "✅ All tree_sitter_language imports fixed!"
else
    echo "⚠️ Still $remaining tree_sitter_language imports remaining:"
    grep -r "tree_sitter_language" --include="*.rs" 2>/dev/null | head -5
fi
