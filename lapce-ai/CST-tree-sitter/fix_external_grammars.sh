#!/bin/bash

# Fix all external grammars that use tree-sitter-language to use tree-sitter 0.23.0

GRAMMARS=(
  "tree-sitter-asm"
  "tree-sitter-fsharp"
  "tree-sitter-gleam"
  "tree-sitter-glsl"
  "tree-sitter-hcl"
  "tree-sitter-powershell"
  "tree-sitter-scheme"
  "tree-sitter-solidity"
  "tree-sitter-tcl"
  "tree-sitter-toml"
  "tree-sitter-typescript"
)

for grammar in "${GRAMMARS[@]}"; do
  echo "Fixing $grammar..."
  
  # Update Cargo.toml
  if [ -f "external-grammars/$grammar/Cargo.toml" ]; then
    sed -i 's/tree-sitter-language = "0.1.0"/tree-sitter = "0.23.0"/g' "external-grammars/$grammar/Cargo.toml"
    echo "  - Updated Cargo.toml"
  fi
  
  # Update lib.rs
  if [ -f "external-grammars/$grammar/bindings/rust/lib.rs" ]; then
    # Replace LanguageFn with Language
    sed -i 's/use tree_sitter_language::LanguageFn;/use tree_sitter::Language;/g' "external-grammars/$grammar/bindings/rust/lib.rs"
    
    # Update the language constant type and initialization
    sed -i 's/pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(\(.*\)) };/pub const LANGUAGE: Language = unsafe { \1() };/g' "external-grammars/$grammar/bindings/rust/lib.rs"
    
    # Update test to use LANGUAGE directly
    sed -i 's/\.set_language(&super::LANGUAGE\.into())/.set_language(\&super::LANGUAGE)/g' "external-grammars/$grammar/bindings/rust/lib.rs"
    
    echo "  - Updated lib.rs"
  fi
done

echo "All grammars fixed!"
