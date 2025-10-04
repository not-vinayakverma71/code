#!/bin/bash

echo "Fixing const function call issues..."

# Fix TypeScript which has multiple languages
if [ -f "external-grammars/tree-sitter-typescript/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-typescript with lazy_static..."
  cat > external-grammars/tree-sitter-typescript/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_tsx() -> Language;
}

pub fn language_typescript() -> Language {
    unsafe { tree_sitter_typescript() }
}

pub fn language_tsx() -> Language {
    unsafe { tree_sitter_tsx() }
}

// Aliases for compatibility
pub fn language() -> Language {
    language_typescript()
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_typescript() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language_typescript())
            .expect("Error loading TypeScript parser");
    }
    
    #[test]
    fn test_can_load_tsx() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language_tsx())
            .expect("Error loading TSX parser");
    }
}
EOF
fi

# Fix F# which also has multiple languages
if [ -f "external-grammars/tree-sitter-fsharp/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-fsharp..."
  cat > external-grammars/tree-sitter-fsharp/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_fsharp() -> Language;
    fn tree_sitter_fsharp_signature() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_fsharp() }
}

pub fn language_fsharp() -> Language {
    unsafe { tree_sitter_fsharp() }
}

pub fn language_fsharp_signature() -> Language {
    unsafe { tree_sitter_fsharp_signature() }
}

// For compatibility
pub const LANGUAGE_FSHARP: fn() -> Language = language_fsharp;
pub const LANGUAGE_FSHARP_SIGNATURE: fn() -> Language = language_fsharp_signature;

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading F# parser");
    }
}
EOF
fi

# Fix JavaScript
if [ -f "external-grammars/tree-sitter-javascript/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-javascript..."
  cat > external-grammars/tree-sitter-javascript/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_javascript() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_javascript() }
}

// For compatibility
pub const LANGUAGE: fn() -> Language = language;

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
pub const HIGHLIGHT_QUERY: &str = include_str!("../../queries/highlights.scm");
pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");
pub const JSX_HIGHLIGHT_QUERY: &str = include_str!("../../queries/highlights-jsx.scm");
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading JavaScript parser");
    }
}
EOF
fi

# Fix all other grammars to use functions instead of consts
for grammar in tree-sitter-solidity tree-sitter-hcl tree-sitter-scheme tree-sitter-gleam tree-sitter-glsl tree-sitter-tcl tree-sitter-toml tree-sitter-powershell tree-sitter-asm tree-sitter-fennel tree-sitter-astro tree-sitter-wgsl tree-sitter-cairo; do
  if [ -d "external-grammars/$grammar" ]; then
    gram_name=${grammar#tree-sitter-}
    gram_name=${gram_name//-/_}
    
    echo "Fixing $grammar to use functions..."
    cat > "external-grammars/$grammar/bindings/rust/lib.rs" << EOF
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_${gram_name}() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_${gram_name}() }
}

// For compatibility  
pub const LANGUAGE: fn() -> Language = language;

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading ${gram_name} parser");
    }
}
EOF
  fi
done

# Fix markdown special case
if [ -f "external-grammars/tree-sitter-markdown/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-markdown..."
  cat > external-grammars/tree-sitter-markdown/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
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

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

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

echo "All const issues fixed!"
