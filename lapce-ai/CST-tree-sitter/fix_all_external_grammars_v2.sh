#!/bin/bash

# Fix all external grammars to properly handle tree-sitter 0.23 types

echo "Fixing all external grammars for tree-sitter 0.23 compatibility..."

# List of ALL external grammars that need fixing
GRAMMARS=(
  "tree-sitter-javascript"
  "tree-sitter-typescript"
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
  "tree-sitter-kotlin"
  "tree-sitter-yaml"
  "tree-sitter-r"
  "tree-sitter-matlab"
  "tree-sitter-perl"
  "tree-sitter-dart"
  "tree-sitter-julia"
  "tree-sitter-haskell"
  "tree-sitter-graphql"
  "tree-sitter-sql"
  "tree-sitter-zig"
  "tree-sitter-vim"
  "tree-sitter-abap"
  "tree-sitter-nim"
  "tree-sitter-crystal"
  "tree-sitter-fortran"
  "tree-sitter-vhdl"
  "tree-sitter-racket"
  "tree-sitter-ada"
  "tree-sitter-prolog"
  "tree-sitter-gradle"
  "tree-sitter-xml"
  "tree-sitter-clojure"
  "tree-sitter-svelte"
  "tree-sitter-fennel"
  "tree-sitter-astro"
  "tree-sitter-wgsl"
  "tree-sitter-cairo"
  "tree-sitter-markdown"
)

for grammar in "${GRAMMARS[@]}"; do
  if [ -d "external-grammars/$grammar" ]; then
    echo "Processing $grammar..."
    
    # Fix lib.rs to properly handle Language type
    lib_file="external-grammars/$grammar/bindings/rust/lib.rs"
    if [ -f "$lib_file" ]; then
      # Create a backup
      cp "$lib_file" "$lib_file.bak"
      
      # Get the grammar name without tree-sitter- prefix
      gram_name=${grammar#tree-sitter-}
      gram_name=${gram_name//-/_}
      
      # Special case for markdown which uses tree-sitter-md
      if [ "$grammar" = "tree-sitter-markdown" ]; then
        gram_name="md"
      fi
      
      # Write the fixed lib.rs file
      cat > "$lib_file" << EOF
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_${gram_name}() -> Language;
}

/// Returns the tree-sitter Language for this grammar
pub unsafe fn language() -> Language {
    tree_sitter_${gram_name}()
}

/// The tree-sitter Language for this grammar
pub const LANGUAGE: Language = unsafe { tree_sitter_${gram_name}() };

// Include node types if available
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

// Include queries if they exist
#[cfg(feature = "queries")]
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading ${gram_name} parser");
    }
}
EOF
      echo "  - Fixed lib.rs for $grammar"
    fi
  fi
done

echo "All external grammars fixed!"
