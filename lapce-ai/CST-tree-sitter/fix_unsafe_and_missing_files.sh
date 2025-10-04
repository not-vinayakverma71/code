#!/bin/bash

echo "Fixing unsafe extern blocks and missing node-types.json files..."

# Fix all lib.rs files to use unsafe extern blocks
for grammar_dir in external-grammars/tree-sitter-*/; do
  lib_file="$grammar_dir/bindings/rust/lib.rs"
  if [ -f "$lib_file" ]; then
    # Add unsafe to extern blocks
    sed -i 's/^extern "C" {/unsafe extern "C" {/g' "$lib_file"
    echo "Fixed unsafe extern in $(basename $grammar_dir)"
  fi
done

# Handle missing node-types.json files
for grammar_dir in external-grammars/tree-sitter-*/; do
  lib_file="$grammar_dir/bindings/rust/lib.rs"
  node_types_file="$grammar_dir/src/node-types.json"
  
  if [ -f "$lib_file" ]; then
    if [ ! -f "$node_types_file" ]; then
      # Comment out the NODE_TYPES constant if node-types.json doesn't exist
      echo "Fixing missing node-types.json for $(basename $grammar_dir)"
      sed -i 's/^pub const NODE_TYPES: &str = include_str/\/\/ pub const NODE_TYPES: &str = include_str/g' "$lib_file"
    fi
  fi
done

# Special fix for markdown which has a different structure
if [ -f "external-grammars/tree-sitter-markdown/bindings/rust/lib.rs" ]; then
  echo "Special fix for tree-sitter-markdown..."
  cat > external-grammars/tree-sitter-markdown/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

unsafe extern "C" {
    fn tree_sitter_md() -> Language;
    fn tree_sitter_markdown_inline() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_md() }
}

pub fn language_inline() -> Language {
    unsafe { tree_sitter_markdown_inline() }
}

// Compatibility aliases
pub const LANGUAGE: fn() -> Language = language;
pub const LANGUAGE_INLINE: fn() -> Language = language_inline;

// NODE_TYPES not available for markdown
// pub const NODE_TYPES: &str = "";

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading Markdown parser");
    }
}
EOF
fi

echo "All fixes applied!"
